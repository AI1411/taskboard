use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;

struct TestServer {
    base: String,
    token: String,
    _tmp: tempfile::TempDir,
}

fn authed(s: &TestServer) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Taskboard-Session",
        HeaderValue::from_str(&s.token).unwrap(),
    );
    headers.insert("Origin", HeaderValue::from_str(&s.base).unwrap());
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
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

async fn seeded_task_server() -> TestServer {
    let s = start_test_server().await;
    let client = authed(&s);
    let proj = client
        .post(format!("{}/api/v1/projects", s.base))
        .json(&json!({"name": "Renai Sim"}))
        .send()
        .await
        .unwrap();
    assert_eq!(proj.status(), 200);
    let task = client
        .post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .json(&json!({"title": "Fix login error"}))
        .send()
        .await
        .unwrap();
    assert_eq!(task.status(), 200);
    s
}

#[tokio::test]
async fn create_project_and_task_round_trip() {
    let s = start_test_server().await;
    let client = authed(&s);
    let proj = client
        .post(format!("{}/api/v1/projects", s.base))
        .json(&json!({"name": "Renai Sim"}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(proj["entity"]["slug"], "renai-sim");
    let task = client
        .post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .json(&json!({"title": "Fix login error"}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(task["entity"]["displayId"], "TASK-1");
}

#[tokio::test]
async fn if_match_conflict() {
    let s = seeded_task_server().await;
    let res = authed(&s)
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .header("If-Match", "0")
        .json(&json!({"title": "Nope"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 409);
}

#[tokio::test]
async fn sync_after_zero_then_after_head() {
    let s = seeded_task_server().await;
    let head = authed(&s)
        .get(format!("{}/api/v1/sync?after=0", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let seq = head["sequence"].as_i64().unwrap();
    assert!(seq >= 1);
    let again = authed(&s)
        .get(format!("{}/api/v1/sync?after={}", s.base, seq))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(again["sequence"], seq);
    assert!(again["projects"].as_array().unwrap().is_empty());
}

async fn task_display_ids(s: &TestServer, slug: &str) -> Vec<String> {
    let listed = authed(s)
        .get(format!("{}/api/v1/projects/{slug}/tasks", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    listed["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["displayId"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn patch_task_before_display_id_null_moves_card_to_end() {
    let s = start_test_server().await;
    let client = authed(&s);
    assert_eq!(
        client
            .post(format!("{}/api/v1/projects", s.base))
            .json(&json!({"name": "Renai Sim"}))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    for title in ["First", "Second", "Third"] {
        assert_eq!(
            client
                .post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
                .json(&json!({ "title": title }))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }
    assert_eq!(
        task_display_ids(&s, "renai-sim").await,
        ["TASK-1", "TASK-2", "TASK-3"]
    );

    let res = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({ "beforeDisplayId": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        task_display_ids(&s, "renai-sim").await,
        ["TASK-2", "TASK-3", "TASK-1"]
    );
}

#[tokio::test]
async fn patch_task_chains_if_match_across_fields() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    let shown = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let revision = shown["entity"]["revision"].as_i64().unwrap();

    let ok = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .header("If-Match", revision.to_string())
        .json(&json!({ "title": "Updated title", "urgent": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    let body = ok.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["title"], "Updated title");
    assert_eq!(body["entity"]["urgent"], true);

    let conflict = client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .header("If-Match", revision.to_string())
        .json(&json!({ "title": "Should not apply", "urgent": false }))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status(), 409);
    let shown = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(shown["entity"]["title"], "Updated title");
    assert_eq!(shown["entity"]["urgent"], true);
}
