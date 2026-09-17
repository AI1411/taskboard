use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn tb() -> (Command, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (tb_in(&dir), dir)
}

fn tb_in(dir: &TempDir) -> Command {
    let mut c = Command::cargo_bin("taskboard").unwrap();
    c.env("TASKBOARD_DATA_DIR", dir.path());
    c.env("HTTP_PROXY", "");
    c.env("HTTPS_PROXY", "");
    c.env("http_proxy", "");
    c.env("https_proxy", "");
    c
}

#[test]
fn project_add_json() {
    let (mut cmd, _dir) = tb();
    let out = cmd
        .args(["project", "add", "--name", "Renai Sim", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["entity"]["slug"], "renai-sim");
    assert_eq!(v["revision"], 1);
}

#[test]
fn failed_mutation_is_not_ok_true() {
    let (mut cmd, _dir) = tb();
    let assert = cmd
        .args(["task", "show", "TASK-999", "--json"])
        .assert()
        .failure()
        .code(1);
    let stdout = &assert.get_output().stdout;
    let text = String::from_utf8_lossy(stdout);
    assert!(
        !text.contains("\"ok\":true") && !text.contains("\"ok\": true"),
        "failed show must not print ok:true on stdout, got: {text}"
    );
    let v: serde_json::Value = serde_json::from_slice(stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "not_found");
}

#[test]
fn serve_prints_localhost_url() {
    let dir = tempfile::tempdir().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("taskboard");
    let mut child = std::process::Command::new(bin)
        .args(["serve"])
        .env("TASKBOARD_DATA_DIR", dir.path())
        .env("HTTP_PROXY", "")
        .env("HTTPS_PROXY", "")
        .env("http_proxy", "")
        .env("https_proxy", "")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let mut reader = std::io::BufReader::new(stdout);
        let _ = std::io::BufRead::read_line(&mut reader, &mut line);
        let _ = tx.send(line);
    });
    let line = rx
        .recv_timeout(std::time::Duration::from_secs(15))
        .expect("serve should print a URL");
    assert!(
        line.contains("http://127.0.0.1:"),
        "expected localhost URL, got {line:?}"
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
async fn serve_page_contains_root_and_poll_sees_cli_write() {
    let dir = tempfile::tempdir().unwrap();
    let dist =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../api/tests/fixtures/web-dist");
    let bin = assert_cmd::cargo::cargo_bin("taskboard");
    let mut child = std::process::Command::new(&bin)
        .args(["serve", "--port", "0"])
        .env("TASKBOARD_DATA_DIR", dir.path())
        .env("TASKBOARD_WEB_DIST", &dist)
        .env("HTTP_PROXY", "")
        .env("HTTPS_PROXY", "")
        .env("http_proxy", "")
        .env("https_proxy", "")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let mut reader = std::io::BufReader::new(stdout);
        let _ = std::io::BufRead::read_line(&mut reader, &mut line);
        let _ = tx.send(line);
    });
    let line = rx
        .recv_timeout(std::time::Duration::from_secs(15))
        .expect("serve should print a URL");
    let url = line.trim().to_string();
    let html = reqwest::get(&url).await.unwrap().text().await.unwrap();
    assert!(
        html.contains("id=\"root\"") || html.contains("Taskboard"),
        "html={html}"
    );
    assert!(
        html.contains("data-taskboard-web=\"dist\""),
        "TASKBOARD_WEB_DIST should be served, got {html}"
    );
    Command::cargo_bin("taskboard")
        .unwrap()
        .env("TASKBOARD_DATA_DIR", dir.path())
        .args(["project", "add", "--name", "From CLI"])
        .assert()
        .success();
    let list = Command::cargo_bin("taskboard")
        .unwrap()
        .env("TASKBOARD_DATA_DIR", dir.path())
        .args(["project", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&list.stdout).contains("from-cli"),
        "stdout={}",
        String::from_utf8_lossy(&list.stdout)
    );
    let pid = child.id();
    let _ = child.kill();
    let _ = child.wait();
    let _ = pid;
}

#[test]
fn project_add_human() {
    let (mut cmd, _dir) = tb();
    cmd.args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created project  renai-sim  Renai Sim",
        ));
}

#[test]
fn task_create_human() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login error",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created TASK-1  Fix login error  [todo]",
        ));
}

#[test]
fn task_list_human_columns() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login error",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "list", "--project", "renai-sim"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID  COLUMN  URGENT  RUN  TITLE"))
        .stdout(predicate::str::contains("TASK-1"))
        .stdout(predicate::str::contains("todo"))
        .stdout(predicate::str::contains("idle"))
        .stdout(predicate::str::contains("Fix login error"));
}

#[test]
fn project_list_json_uses_entities_not_entity() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["project", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v.get("entity").is_none(), "list JSON must not fake entity");
    assert_eq!(v["entities"][0]["slug"], "renai-sim");
}

#[test]
fn json_commands_round_trip_board() {
    let dir = tempfile::tempdir().unwrap();
    let add = json_ok(&dir, &["project", "add", "--name", "Renai Sim"]);
    assert_eq!(add["entity"]["slug"], "renai-sim");

    let listed = json_ok(&dir, &["project", "list"]);
    assert!(listed.get("entity").is_none());
    assert_eq!(listed["entities"].as_array().unwrap().len(), 1);

    json_ok(&dir, &["project", "show", "renai-sim"]);
    json_ok(
        &dir,
        &[
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login error",
        ],
    );
    let tasks = json_ok(&dir, &["task", "list", "--project", "renai-sim"]);
    assert!(tasks.get("entity").is_none());
    assert_eq!(tasks["entities"][0]["display_id"], "TASK-1");

    json_ok(&dir, &["task", "show", "TASK-1"]);
    json_ok(
        &dir,
        &["task", "update", "TASK-1", "--title", "Fix login bug"],
    );
    json_ok(&dir, &["task", "move", "TASK-1", "in-progress"]);
    json_ok(&dir, &["task", "urgent", "TASK-1", "on"]);
    json_ok(&dir, &["task", "prioritize", "TASK-1", "--end"]);
    json_ok(&dir, &["note", "add", "TASK-1", "--text", "check sessions"]);
    json_ok(
        &dir,
        &["link", "add", "TASK-1", "--url", "https://example.com"],
    );
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    let link_id = shown["entity"]["links"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    json_ok(&dir, &["run", "start", "TASK-1", "--agent", "codex"]);
    json_ok(
        &dir,
        &["run", "update", "RUN-1", "--message", "adding tests"],
    );
    json_ok(&dir, &["run", "wait", "RUN-1", "--reason", "need spec"]);
    json_ok(&dir, &["run", "finish", "RUN-1", "--summary", "done"]);
    json_ok(
        &dir,
        &["project-note", "set", "renai-sim", "--text", "# Board"],
    );
    json_ok(&dir, &["project", "archive", "renai-sim"]);
    json_ok(&dir, &["project", "unarchive", "renai-sim"]);
    json_ok(&dir, &["link", "remove", &link_id]);
    json_ok(&dir, &["task", "delete", "TASK-1"]);
    let trash = json_ok(&dir, &["trash", "list"]);
    assert_eq!(trash["entity"]["tasks"][0]["display_id"], "TASK-1");
    json_ok(&dir, &["task", "restore", "TASK-1"]);
    json_ok(&dir, &["undo"]);
    let export = dir.path().join("export.sqlite3");
    json_ok(&dir, &["backup", "export", export.to_str().unwrap()]);
    json_ok(&dir, &["project", "delete", "renai-sim"]);
    json_ok(&dir, &["project", "restore", "renai-sim"]);
}

#[test]
fn inbox_json_lists_waiting_across_projects() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "B"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "a", "--title", "Wait A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "b", "--title", "Wait B"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "Need spec"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-2", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-2", "--reason", "Need B"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["inbox", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    let entities = v["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 2);
    let reasons: Vec<&str> = entities
        .iter()
        .map(|item| item["reason"].as_str().unwrap())
        .collect();
    let slugs: Vec<&str> = entities
        .iter()
        .map(|item| item["project_slug"].as_str().unwrap())
        .collect();
    assert!(reasons.contains(&"Need spec"));
    assert!(reasons.contains(&"Need B"));
    assert!(slugs.contains(&"a"));
    assert!(slugs.contains(&"b"));
}

#[test]
fn inbox_empty_human() {
    let (mut cmd, _dir) = tb();
    cmd.args(["inbox"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inbox is empty"));
}

#[test]
fn inbox_project_filter_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "B"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "a",
            "--title",
            "Pin A",
            "--urgent",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "b",
            "--title",
            "Pin B",
            "--urgent",
        ])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["inbox", "--project", "b", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entities"].as_array().unwrap().len(), 1);
    assert_eq!(v["entities"][0]["title"], "Pin B");
}

#[test]
fn project_detect_json_from_path() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    tb_in(&dir)
        .args([
            "project",
            "add",
            "--name",
            "Renai",
            "--path",
            repo.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = tb_in(&dir)
        .current_dir(&repo)
        .args(["project", "detect", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entity"]["slug"], "renai");
}

#[test]
fn task_list_without_project_uses_detect() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    tb_in(&dir)
        .args([
            "project",
            "add",
            "--name",
            "Renai",
            "--path",
            repo.to_str().unwrap(),
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai", "--title", "X"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .current_dir(&repo)
        .args(["task", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entities"][0]["title"], "X");
}

#[test]
fn task_list_without_project_unlinked_is_project_required() {
    let (mut cmd, _dir) = tb();
    let out = cmd
        .args(["task", "list", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "project_required");
}

#[test]
fn task_list_all_filters_status_column_agent() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Alpha"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "Beta"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "alpha",
            "--title",
            "Review me",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "beta",
            "--title",
            "Other",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "run",
            "start",
            "TASK-1",
            "--agent",
            "cursor",
            "--session",
            "s1",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-2", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-2", "--reason", "need spec"])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &[
            "task",
            "list",
            "--all",
            "--status",
            "running,waiting",
            "--column",
            "in-review",
            "--agent",
            "cursor",
        ],
    );
    assert!(v.get("entity").is_none());
    assert_eq!(v["entities"].as_array().unwrap().len(), 1);
    assert_eq!(v["entities"][0]["display_id"], "TASK-1");
    assert_eq!(v["entities"][0]["display_status"], "running");
}

#[test]
fn run_list_show_current_and_session() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "run",
            "start",
            "TASK-1",
            "--agent",
            "cursor",
            "--session",
            "abc123",
        ])
        .assert()
        .success();
    let listed = json_ok(&dir, &["run", "list", "--open"]);
    assert_eq!(listed["entities"][0]["display_id"], "RUN-1");
    assert_eq!(listed["entities"][0]["status"], "running");
    let shown = json_ok(&dir, &["run", "show", "RUN-1"]);
    assert_eq!(shown["entity"]["display_id"], "RUN-1");
    assert_eq!(shown["revision"], 1);
    let current = json_ok(&dir, &["run", "current", "TASK-1"]);
    assert_eq!(current["entity"]["display_id"], "RUN-1");
    let by_session = json_ok(&dir, &["run", "show", "--session", "abc123"]);
    assert_eq!(by_session["entity"]["session_id"], "abc123");
}

#[test]
fn run_show_missing_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let out = tb_in(&dir)
        .args(["run", "show", "RUN-9", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "not_found");
}

#[test]
fn run_continue_json_resumes_waiting() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "need spec"])
        .assert()
        .success();
    let continued = json_ok(&dir, &["run", "continue", "RUN-1", "--message", "got spec"]);
    assert_eq!(continued["entity"]["display_id"], "RUN-1");
    assert_eq!(continued["entity"]["status"], "running");
    assert_eq!(
        continued["entity"]["waiting_reason"],
        serde_json::Value::Null
    );
    assert_eq!(continued["entity"]["ended_at"], serde_json::Value::Null);
    assert_eq!(continued["entity"]["message"], "got spec");
}

#[test]
fn run_continue_running_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["run", "continue", "RUN-1", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "validation_error");
    assert_eq!(v["error"]["field"], "status");
}

#[test]
fn task_update_worktree_and_branch_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
        .assert()
        .success();
    let updated = json_ok(
        &dir,
        &[
            "task",
            "update",
            "TASK-1",
            "--worktree",
            "/tmp/wt",
            "--branch",
            "cursor/foo-88ba",
        ],
    );
    assert_eq!(updated["entity"]["worktree_path"], "/tmp/wt");
    assert_eq!(updated["entity"]["branch"], "cursor/foo-88ba");
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    let current = json_ok(&dir, &["run", "current", "TASK-1"]);
    assert_eq!(current["entity"]["worktree_path"], "/tmp/wt");
    assert_eq!(current["entity"]["branch"], "cursor/foo-88ba");
    let cleared = json_ok(
        &dir,
        &["task", "update", "TASK-1", "--worktree", "", "--branch", ""],
    );
    assert_eq!(cleared["entity"]["worktree_path"], serde_json::Value::Null);
    assert_eq!(cleared["entity"]["branch"], serde_json::Value::Null);
}

#[test]
fn check_add_toggle_list_json_keeps_note() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["note", "set", "TASK-1", "--text", "# Spec"])
        .assert()
        .success();
    let added = json_ok(
        &dir,
        &["check", "add", "TASK-1", "--text", "Add CLI JSON contract"],
    );
    assert_eq!(added["entity"]["display_id"], "CHECK-1");
    assert_eq!(added["entity"]["done"], false);
    json_ok(&dir, &["check", "add", "TASK-1", "--text", "Write tests"]);
    let toggled = json_ok(&dir, &["check", "toggle", "CHECK-1"]);
    assert_eq!(toggled["entity"]["done"], true);
    let listed = json_ok(&dir, &["check", "list", "TASK-1"]);
    assert_eq!(listed["entities"].as_array().unwrap().len(), 2);
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["note_markdown"], "# Spec");
    assert_eq!(shown["entity"]["checks"][0]["display_id"], "CHECK-1");
    let listed_tasks = json_ok(&dir, &["task", "list", "--project", "renai-sim"]);
    assert_eq!(listed_tasks["entities"][0]["checklist_done"], 1);
    assert_eq!(listed_tasks["entities"][0]["checklist_total"], 2);
}

#[test]
fn comment_add_list_json_keeps_note() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["note", "set", "TASK-1", "--text", "# Spec"])
        .assert()
        .success();
    let added = json_ok(&dir, &["comment", "add", "TASK-1", "--text", "use TDD"]);
    assert_eq!(added["entity"]["body"], "use TDD");
    let listed = json_ok(&dir, &["comment", "list", "TASK-1"]);
    assert_eq!(listed["entities"].as_array().unwrap().len(), 1);
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["note_markdown"], "# Spec");
    assert_eq!(shown["entity"]["comments"][0]["body"], "use TDD");
    assert_eq!(shown["entity"]["reply"], serde_json::Value::Null);
}

#[test]
fn comment_reply_on_waiting_task_show() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "need spec"])
        .assert()
        .success();
    json_ok(
        &dir,
        &["comment", "add", "TASK-1", "--text", "here is spec"],
    );
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["reply"], "here is spec");
}

#[test]
fn link_add_blocked_by_json_and_filters() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Blocker",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Blocked",
        ])
        .assert()
        .success();
    let added = json_ok(&dir, &["link", "add", "TASK-2", "--blocked-by", "TASK-1"]);
    assert_eq!(added["entity"]["links"][0]["kind"], "blocked_by");
    assert_eq!(added["entity"]["links"][0]["value"], "TASK-1");
    assert_eq!(added["entity"]["column"], "todo");
    let blocked = json_ok(
        &dir,
        &["task", "list", "--project", "renai-sim", "--blocked"],
    );
    assert_eq!(blocked["entities"].as_array().unwrap().len(), 1);
    assert_eq!(blocked["entities"][0]["display_id"], "TASK-2");
    assert_eq!(blocked["entities"][0]["blocked_by"][0], "TASK-1");
    let ready = json_ok(&dir, &["task", "list", "--project", "renai-sim", "--ready"]);
    let ids: Vec<_> = ready["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["display_id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"TASK-1".to_string()));
    assert!(!ids.contains(&"TASK-2".to_string()));
}

#[test]
fn link_add_blocked_by_cycle_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "B"])
        .assert()
        .success();
    json_ok(&dir, &["link", "add", "TASK-2", "--blocked-by", "TASK-1"]);
    let out = tb_in(&dir)
        .args(["link", "add", "TASK-1", "--blocked-by", "TASK-2", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "validation_error");
}

#[test]
fn activity_json_lists_and_filters() {
    let dir = tempfile::tempdir().unwrap();
    json_ok(&dir, &["project", "add", "--name", "Renai Sim"]);
    json_ok(
        &dir,
        &["task", "create", "--project", "renai-sim", "--title", "Fix"],
    );
    let all = json_ok(&dir, &["activity"]);
    let entities = all["entities"].as_array().unwrap();
    assert!(entities.iter().any(|row| row["operation"] == "task.create"));
    assert!(entities.iter().any(|row| row["target"] == "TASK-1"));
    let filtered = json_ok(&dir, &["activity", "--task", "TASK-1"]);
    assert!(filtered["entities"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["target"] == "TASK-1"));
}

#[test]
fn stale_json_skips_fresh_running_run() {
    let dir = tempfile::tempdir().unwrap();
    json_ok(&dir, &["project", "add", "--name", "Renai Sim"]);
    json_ok(
        &dir,
        &["task", "create", "--project", "renai-sim", "--title", "Fix"],
    );
    json_ok(&dir, &["run", "start", "TASK-1", "--agent", "cursor"]);
    let stale = json_ok(&dir, &["stale", "--minutes", "30"]);
    assert_eq!(stale["entities"].as_array().unwrap().len(), 0);
    let out = tb_in(&dir)
        .args(["stale", "--minutes", "0", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "validation_error");
}

#[test]
fn comment_add_continue_json_resumes_waiting() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Wait",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "Need spec"])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &[
            "comment",
            "add",
            "TASK-1",
            "--text",
            "here is spec",
            "--continue",
        ],
    );
    assert_eq!(v["entity"]["comment"]["body"], "here is spec");
    assert_eq!(v["entity"]["run"]["status"], "running");
}

#[test]
fn comment_add_continue_on_idle_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Idle",
        ])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args([
            "comment",
            "add",
            "TASK-1",
            "--text",
            "nope",
            "--continue",
            "--json",
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "validation_error");
}

#[test]
fn run_continue_reply_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Wait",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "Need spec"])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &["run", "continue", "RUN-1", "--reply", "here is spec"],
    );
    assert_eq!(v["entity"]["status"], "running");
    let comments = json_ok(&dir, &["comment", "list", "TASK-1"]);
    assert_eq!(comments["entities"][0]["body"], "here is spec");
}

#[test]
fn task_spawn_json_creates_children_and_blocks() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Parent",
        ])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &["task", "spawn", "TASK-1", "--title", "API", "--title", "UI"],
    );
    assert_eq!(v["entities"].as_array().unwrap().len(), 2);
    assert_eq!(v["entities"][0]["display_id"], "TASK-2");
    assert_eq!(v["entities"][0]["column"], "todo");
    assert_eq!(v["entities"][0]["worktree_path"], serde_json::Value::Null);
    let parent = json_ok(&dir, &["task", "show", "TASK-1"]);
    let kinds: Vec<_> = parent["entity"]["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|link| {
            (
                link["kind"].as_str().unwrap().to_string(),
                link["value"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert!(kinds.contains(&("blocked_by".into(), "TASK-2".into())));
    assert!(kinds.contains(&("blocked_by".into(), "TASK-3".into())));
    assert_eq!(parent["entity"]["column"], "todo");
    let listed = json_ok(&dir, &["task", "list", "--project", "renai-sim"]);
    let parent_row = listed["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["display_id"] == "TASK-1")
        .unwrap();
    let blocked_by: Vec<_> = parent_row["blocked_by"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_string())
        .collect();
    assert!(blocked_by.contains(&"TASK-2".to_string()));
    assert!(blocked_by.contains(&"TASK-3".to_string()));
}

#[test]
fn run_start_exclusive_conflicts_when_open() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "One"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "codex"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args([
            "run",
            "start",
            "TASK-1",
            "--agent",
            "cursor",
            "--exclusive",
            "--json",
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "conflict");
}

#[test]
fn next_json_claims_first_ready() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "First",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Second",
        ])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &["next", "--project", "renai-sim", "--agent", "cursor"],
    );
    assert_eq!(v["entity"]["display_id"], "RUN-1");
    assert_eq!(v["entity"]["agent"], "cursor");
    assert_eq!(v["entity"]["status"], "running");
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["column"], "todo");
    let second = json_ok(&dir, &["next", "--move"]);
    assert_eq!(second["entity"]["display_id"], "RUN-2");
    let moved = json_ok(&dir, &["task", "show", "TASK-2"]);
    assert_eq!(moved["entity"]["column"], "in-progress");
}

#[test]
fn status_json_counts_and_heads() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Wait me",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Ready me",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "Need spec"])
        .assert()
        .success();
    let ready = json_ok(&dir, &["task", "list", "--project", "renai-sim", "--ready"]);
    let ready_ids: Vec<_> = ready["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["display_id"].as_str().unwrap().to_string())
        .collect();
    let v = json_ok(&dir, &["status"]);
    assert_eq!(v["entity"]["inbox"]["waiting"], 1);
    assert_eq!(v["entity"]["open_runs"], 1);
    assert_eq!(v["entity"]["ready"], ready_ids.len());
    assert_eq!(v["entity"]["inbox_head"][0]["display_id"], "TASK-1");
    assert_eq!(v["entity"]["inbox_head"][0]["detail"], "Need spec");
    let status_ready: Vec<_> = v["entity"]["ready_head"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["display_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        status_ready,
        ready_ids.into_iter().take(3).collect::<Vec<_>>()
    );
}

#[test]
fn status_project_scope_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "B"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "a", "--title", "In A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "b", "--title", "In B"])
        .assert()
        .success();
    let v = json_ok(&dir, &["status", "--project", "b"]);
    assert_eq!(v["entity"]["ready"], 1);
    assert_eq!(v["entity"]["ready_head"][0]["display_id"], "TASK-2");
}

#[test]
fn run_cancel_json_defaults_summary() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Stuck",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    let v = json_ok(&dir, &["run", "cancel", "RUN-1"]);
    assert_eq!(v["entity"]["status"], "failed");
    assert_eq!(v["entity"]["summary"], "canceled");
}

#[test]
fn run_cancel_completed_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Done",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "finish", "RUN-1", "--summary", "shipped"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["run", "cancel", "RUN-1", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "validation_error");
}

#[test]
fn review_approve_json_moves_to_done() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Ship",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    let v = json_ok(&dir, &["review", "TASK-1", "--approve", "--text", "lgtm"]);
    assert_eq!(v["entity"]["column"], "done");
    assert_eq!(v["entity"]["comments"][0]["body"], "lgtm");
}

#[test]
fn review_changes_json_moves_to_in_progress() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Ship",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &["review", "TASK-1", "--changes", "--text", "fix the copy"],
    );
    assert_eq!(v["entity"]["column"], "in-progress");
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["runs"].as_array().unwrap().len(), 0);
}

#[test]
fn review_todo_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Idle",
        ])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["review", "TASK-1", "--approve", "--text", "nope", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "validation_error");
}

#[test]
fn occupancy_json_lists_collisions() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "B"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "update", "TASK-1", "--worktree", "/tmp/shared"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "update", "TASK-2", "--worktree", "/tmp/shared"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-2", "--agent", "cursor"])
        .assert()
        .success();
    let v = json_ok(&dir, &["occupancy"]);
    assert_eq!(v["entities"][0]["worktree_path"], "/tmp/shared");
    assert_eq!(v["entities"][0]["runs"].as_array().unwrap().len(), 2);
}

#[test]
fn occupancy_path_json_includes_singleton() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Alone",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "update", "TASK-1", "--worktree", "/tmp/alone"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    let empty = json_ok(&dir, &["occupancy"]);
    assert_eq!(empty["entities"].as_array().unwrap().len(), 0);
    let v = json_ok(&dir, &["occupancy", "--path", "/tmp/alone"]);
    assert_eq!(v["entities"][0]["runs"][0]["task_display_id"], "TASK-1");
}

fn json_ok(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    let mut argv = args.to_vec();
    argv.push("--json");
    let out = tb_in(dir)
        .args(&argv)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true, "stdout={v}");
    v
}
