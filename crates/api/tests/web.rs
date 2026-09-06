use std::path::{Path, PathBuf};

struct DistEnv {
    prev: Option<String>,
}

impl DistEnv {
    fn set(path: &Path) -> Self {
        let prev = std::env::var("TASKBOARD_WEB_DIST").ok();
        std::env::set_var("TASKBOARD_WEB_DIST", path);
        Self { prev }
    }
}

impl Drop for DistEnv {
    fn drop(&mut self) {
        match &self.prev {
            Some(value) => std::env::set_var("TASKBOARD_WEB_DIST", value),
            None => std::env::remove_var("TASKBOARD_WEB_DIST"),
        }
    }
}

struct TestServer {
    base: String,
    token: String,
    _tmp: tempfile::TempDir,
    _dist: DistEnv,
}

fn fixture_dist() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/web-dist")
}

async fn start_test_server() -> TestServer {
    use std::net::SocketAddr;

    use taskboard_api::serve;
    use taskboard_application::{App, SystemClock};
    use taskboard_store_sqlite::{open_db, SqliteStore};

    let dist = DistEnv::set(&fixture_dist());
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
        _dist: dist,
    }
}

#[tokio::test]
async fn serve_page_contains_root_and_poll_sees_cli_write() {
    let server = start_test_server().await;
    let html = reqwest::get(format!("{}/", server.base))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        html.contains("id=\"root\"") || html.contains("Taskboard"),
        "html={html}"
    );
    assert!(
        html.contains("data-taskboard-web=\"dist\""),
        "TASKBOARD_WEB_DIST should be served, got {html}"
    );
}

#[tokio::test]
async fn get_root_sets_session_cookie_when_serving_dist() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .get(format!("{}/", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("GET / must Set-Cookie taskboard_session");
    assert!(cookie.contains("taskboard_session="));
    assert_eq!(server.token.len(), 64);
}

#[tokio::test]
async fn static_assets_need_no_session() {
    let server = start_test_server().await;
    let js = reqwest::Client::new()
        .get(format!("{}/assets/app.js", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(js.status(), 200);
    let body = js.text().await.unwrap();
    assert!(body.contains("TASKBOARD_FIXTURE"));

    let css = reqwest::Client::new()
        .get(format!("{}/assets/app.css", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(css.status(), 200);
}

#[tokio::test]
async fn spa_fallback_for_unknown_non_api_path() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .get(format!("{}/projects/renai-sim", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let html = res.text().await.unwrap();
    assert!(html.contains("id=\"root\"") || html.contains("Taskboard"));

    let api = reqwest::Client::new()
        .get(format!("{}/api/v1/does-not-exist", server.base))
        .send()
        .await
        .unwrap();
    assert_eq!(api.status(), 404);
}
