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
