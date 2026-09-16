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

    use taskboard_api::serve_with_data_dir;
    use taskboard_application::{App, SystemClock};
    use taskboard_store_sqlite::{open_db, SqliteStore};

    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    let app = App::new(store, SystemClock);
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let bound = serve_with_data_dir(app, addr, false, tmp.path())
        .await
        .unwrap();
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
    assert_eq!(body["revision"], revision + 2);

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

#[tokio::test]
async fn inbox_returns_waiting_card() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    client
        .post(format!("{}/api/v1/tasks/TASK-1/runs", s.base))
        .json(&json!({"agent": "codex"}))
        .send()
        .await
        .unwrap();
    client
        .patch(format!("{}/api/v1/runs/RUN-1", s.base))
        .json(&json!({"op": "wait", "reason": "Need spec"}))
        .send()
        .await
        .unwrap();
    let res = client
        .get(format!("{}/api/v1/inbox", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["entities"][0]["displayId"], "TASK-1");
    assert_eq!(v["entities"][0]["reason"], "Need spec");
    assert_eq!(v["entities"][0]["projectSlug"], "renai-sim");
}

#[tokio::test]
async fn ui_state_get_and_patch_round_trip() {
    let s = start_test_server().await;
    let client = authed(&s);
    let empty = client
        .get(format!("{}/api/v1/ui-state", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(empty["entity"]["lastProjectSlug"], serde_json::Value::Null);
    let patched = client
        .patch(format!("{}/api/v1/ui-state", s.base))
        .json(&json!({"lastProjectSlug": "taskboard"}))
        .send()
        .await
        .unwrap();
    assert_eq!(patched.status(), 200);
    let after = client
        .get(format!("{}/api/v1/ui-state", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(after["entity"]["lastProjectSlug"], "taskboard");
}

#[tokio::test]
async fn comment_add_does_not_change_note() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    client
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({"noteMarkdown": "# Spec"}))
        .send()
        .await
        .unwrap();
    let added = client
        .post(format!("{}/api/v1/tasks/TASK-1/comments", s.base))
        .json(&json!({"body": "use TDD"}))
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), 200);
    let body = added.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["entity"]["body"], "use TDD");
    assert_eq!(body["entity"]["actorKind"], "web");
    assert_eq!(body["entity"]["actorLabel"], "local-web");
    let shown = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(shown["entity"]["noteMarkdown"], "# Spec");
    assert_eq!(shown["entity"]["comments"][0]["body"], "use TDD");
}

#[tokio::test]
async fn check_add_and_toggle() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    let added = client
        .post(format!("{}/api/v1/tasks/TASK-1/checks", s.base))
        .json(&json!({"text": "Write tests"}))
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), 200);
    let body = added.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["displayId"], "CHECK-1");
    assert_eq!(body["entity"]["done"], false);
    let toggled = client
        .patch(format!("{}/api/v1/checks/CHECK-1", s.base))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(toggled.status(), 200);
    let toggle_body = toggled.json::<serde_json::Value>().await.unwrap();
    assert_eq!(toggle_body["entity"]["done"], true);
    let listed = client
        .get(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(listed["entities"][0]["checklistDone"], 1);
    assert_eq!(listed["entities"][0]["checklistTotal"], 1);
}

#[tokio::test]
async fn blocked_by_link_and_cycle() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    assert_eq!(
        client
            .post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
            .json(&json!({"title": "Blocker"}))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let added = client
        .post(format!("{}/api/v1/tasks/TASK-2/links", s.base))
        .json(&json!({"kind": "blocked_by", "value": "TASK-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(added.status(), 200);
    let body = added.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["entity"]["links"][0]["kind"], "blocked_by");
    assert_eq!(body["entity"]["links"][0]["value"], "TASK-1");
    let listed = client
        .get(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let two = listed["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["displayId"] == "TASK-2")
        .unwrap();
    assert_eq!(two["blockedBy"][0], "TASK-1");
    let cycle = client
        .post(format!("{}/api/v1/tasks/TASK-1/links", s.base))
        .json(&json!({"kind": "blocked_by", "value": "TASK-2"}))
        .send()
        .await
        .unwrap();
    assert_eq!(cycle.status(), 400);
    let err = cycle.json::<serde_json::Value>().await.unwrap();
    assert_eq!(err["error"]["code"], "validation_error");
    assert_eq!(err["error"]["field"], "blocked_by");
    assert_eq!(err["error"]["message"], "blocked-by cycle");
}
