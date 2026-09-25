# Localhost Host check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reject any `Host` other than `127.0.0.1:PORT` or `localhost:PORT` before a session cookie is set, and allow `http://localhost:PORT` writes.

**Architecture:** `host_allowed` and `origin_allowed` live together in `crates/api/src/origin.rs`. An axum layer on the whole router returns 421 when `Host` is missing or unexpected, so `GET /` never calls `set_session_cookie`. The existing Origin check stays on mutations; its allow-list gains `http://localhost:PORT`. No new auth scheme.

**Tech Stack:** axum 0.7, existing `crates/api/tests/security.rs`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- Keep the existing Origin check
- Allow legitimate localhost origins
- Do not redesign token / cookie / Origin auth
- HTTP stays bound to `127.0.0.1` only

User already chose sequential inline execution.

## File map

- Modify: `crates/api/src/origin.rs` — `host_allowed`; localhost origin allowed
- Modify: `crates/api/src/lib.rs` — export `host_allowed`
- Modify: `crates/api/src/server.rs` — router layer returns 421
- Modify: `crates/api/tests/security.rs` — spoofed Host and localhost Origin

---

### Task 1: Failing Host and localhost-Origin tests

**Files:**
- Modify: `crates/api/tests/security.rs`
- Modify: `crates/api/src/origin.rs`
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/api/src/server.rs`

**Interfaces:**
- Consumes: `AppState.port`, `header::HOST`
- Produces: `pub fn host_allowed(host: Option<&HeaderValue>, port: u16) -> bool` and `origin_allowed` that accepts both `http://127.0.0.1:{port}` and `http://localhost:{port}`

- [ ] **Step 1: Update the origin unit test and add Host tests**

In `origin_must_match_bound_port`, change the localhost assertion to `assert!(origin_allowed(... "http://localhost:9876" ...))`.

Add:

```rust
#[test]
fn host_must_be_loopback_with_bound_port() {
    use axum::http::HeaderValue;
    use taskboard_api::host_allowed;

    assert!(host_allowed(
        Some(&HeaderValue::from_static("127.0.0.1:9876")),
        9876
    ));
    assert!(host_allowed(
        Some(&HeaderValue::from_static("localhost:9876")),
        9876
    ));
    assert!(!host_allowed(
        Some(&HeaderValue::from_static("evil.example:9876")),
        9876
    ));
    assert!(!host_allowed(
        Some(&HeaderValue::from_static("127.0.0.1:1")),
        9876
    ));
    assert!(!host_allowed(None, 9876));
}
```

Add a raw-HTTP helper and these tokio tests (they use `TestServer`, so add `port: u16` parsed from `base`):

```rust
async fn raw_exchange(port: u16, request: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buf = vec![0_u8; 8192];
    let n = tokio::time::timeout(std::time::Duration::from_secs(2), stream.read(&mut buf))
        .await
        .unwrap_or(Ok(0))
        .unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

fn status_line(raw: &str) -> u16 {
    raw.lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
async fn spoofed_host_does_not_set_a_cookie_or_return_the_board() {
    let server = start_test_server().await;
    let root = raw_exchange(
        server.port,
        &format!(
            "GET / HTTP/1.1\r\nHost: evil.example:{}\r\nConnection: close\r\n\r\n",
            server.port
        ),
    );
    assert_eq!(status_line(&root), 421);
    assert!(!root.to_ascii_lowercase().contains("set-cookie"));

    let projects = raw_exchange(
        server.port,
        &format!(
            "GET /api/v1/projects HTTP/1.1\r\nHost: evil.example:{port}\r\nCookie: taskboard_session={token}\r\nConnection: close\r\n\r\n",
            port = server.port,
            token = server.token
        ),
    );
    assert_eq!(status_line(&projects), 421);

    let bootstrap = raw_exchange(
        server.port,
        &format!(
            "GET /api/v1/bootstrap HTTP/1.1\r\nHost: evil.example:{port}\r\nCookie: taskboard_session={token}\r\nConnection: close\r\n\r\n",
            port = server.port,
            token = server.token
        ),
    );
    assert_eq!(status_line(&bootstrap), 421);
    assert!(!bootstrap.contains(&server.token));
}

#[tokio::test]
async fn localhost_origin_write_succeeds() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .header("X-Taskboard-Session", &server.token)
        .header("Origin", format!("http://localhost:{}", server.port))
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn localhost_host_still_sets_a_cookie() {
    let server = start_test_server().await;
    let raw = raw_exchange(
        server.port,
        &format!(
            "GET / HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
            server.port
        ),
    );
    assert_eq!(status_line(&raw), 200);
    assert!(raw.to_ascii_lowercase().contains("set-cookie"));
}
```

- [ ] **Step 2: Run the tests and watch them fail**

Run: `cargo test -p taskboard-api --test security -- --test-threads=1`

Expected: compile error on `host_allowed`, or `origin_must_match_bound_port` / `localhost_origin_write_succeeds` fail with localhost still forbidden. Spoofed Host currently returns 200.

- [ ] **Step 3: Implement**

`crates/api/src/origin.rs`:

```rust
pub fn origin_allowed(origin: Option<&HeaderValue>, port: u16) -> bool {
    let Some(value) = header_str(origin) else {
        return false;
    };
    value == format!("http://127.0.0.1:{port}") || value == format!("http://localhost:{port}")
}

pub fn host_allowed(host: Option<&HeaderValue>, port: u16) -> bool {
    let Some(value) = header_str(host) else {
        return false;
    };
    let value = value.to_ascii_lowercase();
    value == format!("127.0.0.1:{port}") || value == format!("localhost:{port}")
}

fn header_str(value: Option<&HeaderValue>) -> Option<&str> {
    value.and_then(|value| value.to_str().ok())
}
```

Export `host_allowed` from `lib.rs`.

In `server.rs`, layer the router:

```rust
use axum::middleware::{self, Next};

fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(html_shell))
        .merge(crate::routes::api_router())
        .fallback(static_or_spa)
        .layer(middleware::from_fn_with_state(state.clone(), reject_foreign_host))
        .with_state(state)
}

async fn reject_foreign_host(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    if !crate::origin::host_allowed(request.headers().get(header::HOST), state.port) {
        return StatusCode::MISDIRECTED_REQUEST.into_response();
    }
    next.run(request).await
}
```

- [ ] **Step 4: Re-run security tests and clippy**

Run: `cargo test -p taskboard-api --test security -- --test-threads=1`

Expected: PASS.

Run: `cargo clippy -p taskboard-api --all-targets -- -D warnings`

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/api docs/superpowers/plans/2026-09-25-issue-169-host-check.md
git commit -m "fix(api): reject unexpected Host and allow localhost Origin"
```

## Self-review

- Spoofed Host on `/`, bootstrap, and reads: Task 1 tests.
- No cookie on rejection: `set-cookie` assertion.
- Localhost Origin writes: `localhost_origin_write_succeeds`.
- Existing 127.0.0.1 Origin writes stay covered by `mutation_with_same_origin_and_session_is_ok`.
- Auth scheme unchanged.
