use axum::http::HeaderValue;
use taskboard_api::{generate_session, origin_allowed, SessionToken};

#[test]
fn origin_must_match_bound_port() {
    assert!(origin_allowed(
        Some(&HeaderValue::from_static("http://127.0.0.1:9876")),
        9876
    ));
    assert!(!origin_allowed(
        Some(&HeaderValue::from_static("http://localhost:9876")),
        9876
    ));
    assert!(!origin_allowed(
        Some(&HeaderValue::from_static("http://evil.example")),
        9876
    ));
    assert!(!origin_allowed(None, 9876));
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

struct TestServer {
    base: String,
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
    let base = format!("http://127.0.0.1:{}", bound.port());

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
        token,
        _tmp: tmp,
    }
}
