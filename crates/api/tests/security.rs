use axum::http::HeaderValue;
use taskboard_api::{generate_session, host_allowed, origin_allowed, SessionToken};

#[test]
fn origin_must_match_bound_port() {
    assert!(origin_allowed(
        Some(&HeaderValue::from_static("http://127.0.0.1:9876")),
        9876
    ));
    assert!(origin_allowed(
        Some(&HeaderValue::from_static("http://localhost:9876")),
        9876
    ));
    assert!(!origin_allowed(
        Some(&HeaderValue::from_static("http://evil.example")),
        9876
    ));
    assert!(!origin_allowed(None, 9876));
}

#[test]
fn host_must_be_loopback_with_bound_port() {
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

#[tokio::test]
async fn mutation_without_session_is_401() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn mutation_with_foreign_origin_is_403() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .header("X-Taskboard-Session", &server.token)
        .header("Origin", "https://example.com")
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[test]
fn generate_session_is_64_lowercase_hex() {
    let SessionToken(token) = generate_session();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    let SessionToken(other) = generate_session();
    assert_ne!(token, other);
}

#[tokio::test]
async fn html_shell_sets_cookie_so_bootstrap_succeeds() {
    let server = start_test_server().await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let html = res.text().await.unwrap();
    assert!(html.contains("id=\"root\"") || html.contains("Taskboard"));

    let unauthorized = client
        .get(format!("{}/api/v1/bootstrap", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), 401);

    let bootstrap = client
        .get(format!("{}/api/v1/bootstrap", server.base))
        .header("Cookie", format!("taskboard_session={}", server.token))
        .send()
        .await
        .unwrap();
    assert_eq!(bootstrap.status(), 200);
    let body: serde_json::Value = bootstrap.json().await.unwrap();
    assert_eq!(body["session"], server.token);
}

#[tokio::test]
async fn mutation_with_same_origin_and_session_is_ok() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .header("X-Taskboard-Session", &server.token)
        .header("Origin", &server.base)
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

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
        .unwrap_or_else(|| panic!("empty response: {raw:?}"))
        .split_whitespace()
        .nth(1)
        .unwrap_or_else(|| panic!("no status in {raw:?}"))
        .parse()
        .unwrap_or_else(|_| panic!("bad status in {raw:?}"))
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
    )
    .await;
    assert_eq!(status_line(&root), 421);
    assert!(!root.to_ascii_lowercase().contains("set-cookie"));

    let projects = raw_exchange(
        server.port,
        &format!(
            "GET /api/v1/projects HTTP/1.1\r\nHost: evil.example:{port}\r\nCookie: taskboard_session={token}\r\nConnection: close\r\n\r\n",
            port = server.port,
            token = server.token
        ),
    )
    .await;
    assert_eq!(status_line(&projects), 421);

    let bootstrap = raw_exchange(
        server.port,
        &format!(
            "GET /api/v1/bootstrap HTTP/1.1\r\nHost: evil.example:{port}\r\nCookie: taskboard_session={token}\r\nConnection: close\r\n\r\n",
            port = server.port,
            token = server.token
        ),
    )
    .await;
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
    )
    .await;
    assert_eq!(status_line(&raw), 200);
    assert!(raw.to_ascii_lowercase().contains("set-cookie"));
}

struct TestServer {
    base: String,
    port: u16,
    token: String,
    _tmp: tempfile::TempDir,
}

async fn start_test_server() -> TestServer {
    use std::net::SocketAddr;

    use taskboard_api::serve;
    use taskboard_application::{App, SystemClock};
    use taskboard_store_sqlite::{open_db, SqliteStore};

    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = App::new(store, SystemClock);
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let bound = serve(app, addr, false).await.unwrap();
    let port = bound.port();
    let base = format!("http://127.0.0.1:{port}");

    let res = reqwest::Client::new()
        .get(format!("{base}/"))
        .send()
        .await
        .unwrap();
    let cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("GET / must set a session cookie");
    let token = cookie
        .split(';')
        .next()
        .and_then(|part| part.trim().strip_prefix("taskboard_session="))
        .expect("session cookie named taskboard_session")
        .to_string();

    TestServer {
        base,
        port,
        token,
        _tmp: tmp,
    }
}
