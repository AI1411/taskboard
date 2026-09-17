use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

fn tb_in(dir: &TempDir) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin("taskboard"));
    c.env("TASKBOARD_DATA_DIR", dir.path());
    c.env("TASKBOARD_ACTOR", "cursor");
    c.env("HTTP_PROXY", "");
    c.env("HTTPS_PROXY", "");
    c.env("http_proxy", "");
    c.env("https_proxy", "");
    c
}

fn seed(dir: &TempDir) {
    tb_in(dir)
        .args(["project", "add", "--name", "Renai Sim", "--json"])
        .output()
        .unwrap();
    tb_in(dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix",
            "--json",
        ])
        .output()
        .unwrap();
}

fn mcp_rpc(dir: &TempDir, requests: &[Value]) -> Vec<Value> {
    mcp_rpc_with_env(dir, &[], requests)
}

fn mcp_rpc_with_env(dir: &TempDir, env: &[(&str, &str)], requests: &[Value]) -> Vec<Value> {
    let mut command = tb_in(dir);
    command
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in requests {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let stdout = child.stdout.take().unwrap();
    let mut responses = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        responses.push(serde_json::from_str(&line).unwrap());
    }
    let status = child.wait().unwrap();
    assert!(status.success(), "mcp exited {status}");
    responses
}

#[test]
fn mcp_initialize_lists_core_tools_and_calls_app() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir);
    let responses = mcp_rpc(
        &dir,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"task_show",
                "arguments":{"display_id":"TASK-1"}
            }}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
                "name":"task_show",
                "arguments":{"display_id":"TASK-999"}
            }}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
                "name":"project_list",
                "arguments":{}
            }}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{
                "name":"inbox",
                "arguments":{}
            }}),
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
                "name":"activity",
                "arguments":{"after":0}
            }}),
        ],
    );
    assert_eq!(responses.len(), 7);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "taskboard");
    let names: Vec<&str> = responses[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for required in [
        "project_list",
        "task_list",
        "task_show",
        "run_list",
        "inbox",
    ] {
        assert!(names.contains(&required), "missing {required} in {names:?}");
    }
    assert!(names.contains(&"activity"));
    assert!(names.contains(&"comment_add"));
    assert!(names.contains(&"run_continue"));
    assert!(names.contains(&"run_start"));
    assert!(names.contains(&"next"));
    for required in [
        "task_create",
        "task_move",
        "task_update",
        "run_wait",
        "run_finish",
        "run_fail",
        "run_cancel",
        "check_add",
        "check_toggle",
        "check_list",
        "stale",
        "project_detect",
        "link_add",
    ] {
        assert!(names.contains(&required), "missing {required} in {names:?}");
    }
    let shown: Value = serde_json::from_str(
        responses[2]["result"]["structuredContent"]
            .to_string()
            .as_str(),
    )
    .unwrap_or_else(|_| responses[2]["result"]["structuredContent"].clone());
    assert_eq!(shown["ok"], true);
    assert_eq!(shown["entity"]["display_id"], "TASK-1");
    let missing = &responses[3]["result"];
    assert_eq!(missing["isError"], true);
    assert_eq!(missing["structuredContent"]["error"]["code"], "not_found");
    assert_eq!(
        responses[4]["result"]["structuredContent"]["entities"][0]["slug"],
        "renai-sim"
    );
    assert_eq!(responses[5]["result"]["structuredContent"]["ok"], true);
    assert!(responses[6]["result"]["structuredContent"]["entities"].is_array());
}

#[test]
fn mcp_next_and_exclusive_start() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir);
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Two",
            "--json",
        ])
        .output()
        .unwrap();
    let responses = mcp_rpc(
        &dir,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"next",
                "arguments":{"project":"renai-sim","agent":"cursor"}
            }}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"run_start",
                "arguments":{"display_id":"TASK-1","agent":"codex","exclusive":true}
            }}),
        ],
    );
    assert_eq!(
        responses[0]["result"]["structuredContent"]["entity"]["display_id"],
        "RUN-1"
    );
    assert_eq!(responses[1]["result"]["isError"], true);
    assert_eq!(
        responses[1]["result"]["structuredContent"]["error"]["code"],
        "conflict"
    );
}

#[test]
fn mcp_write_path_parity() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir);
    let responses = mcp_rpc_with_env(
        &dir,
        &[("TASKBOARD_PROJECT", "renai-sim")],
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"project_detect","arguments":{}
            }}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"task_create",
                "arguments":{"project":"renai-sim","title":"Write me","urgent":true}
            }}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"task_move",
                "arguments":{"display_id":"TASK-2","column":"in-progress"}
            }}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
                "name":"task_update",
                "arguments":{"display_id":"TASK-2","title":"Wrote me","worktree":"/tmp/wt","branch":"feat/x"}
            }}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
                "name":"run_start",
                "arguments":{"display_id":"TASK-2","agent":"cursor"}
            }}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{
                "name":"run_wait",
                "arguments":{"display_id":"RUN-1","reason":"Need spec"}
            }}),
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
                "name":"check_add",
                "arguments":{"display_id":"TASK-2","text":"Covered"}
            }}),
            json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{
                "name":"check_toggle",
                "arguments":{"display_id":"CHECK-1"}
            }}),
            json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{
                "name":"check_list",
                "arguments":{"display_id":"TASK-2"}
            }}),
            json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{
                "name":"link_add",
                "arguments":{"display_id":"TASK-1","blocked_by":"TASK-2"}
            }}),
            json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{
                "name":"stale",
                "arguments":{"minutes":30}
            }}),
            json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{
                "name":"run_finish",
                "arguments":{"display_id":"RUN-1","summary":"done"}
            }}),
            json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{
                "name":"task_create",
                "arguments":{"project":"renai-sim","title":"Fail me"}
            }}),
            json!({"jsonrpc":"2.0","id":14,"method":"tools/call","params":{
                "name":"run_start",
                "arguments":{"display_id":"TASK-3","agent":"cursor"}
            }}),
            json!({"jsonrpc":"2.0","id":15,"method":"tools/call","params":{
                "name":"run_fail",
                "arguments":{"display_id":"RUN-2","summary":"boom"}
            }}),
        ],
    );
    assert_eq!(
        responses[0]["result"]["structuredContent"]["entity"]["slug"],
        "renai-sim"
    );
    assert_eq!(
        responses[1]["result"]["structuredContent"]["entity"]["display_id"],
        "TASK-2"
    );
    assert_eq!(
        responses[2]["result"]["structuredContent"]["entity"]["column"],
        "in-progress"
    );
    assert_eq!(
        responses[3]["result"]["structuredContent"]["entity"]["title"],
        "Wrote me"
    );
    assert_eq!(
        responses[3]["result"]["structuredContent"]["entity"]["worktree_path"],
        "/tmp/wt"
    );
    assert_eq!(
        responses[5]["result"]["structuredContent"]["entity"]["status"],
        "waiting"
    );
    assert_eq!(
        responses[8]["result"]["structuredContent"]["entities"][0]["done"],
        true
    );
    assert_eq!(
        responses[9]["result"]["structuredContent"]["entity"]["links"][0]["kind"],
        "blocked_by"
    );
    assert!(responses[10]["result"]["structuredContent"]["entities"].is_array());
    assert_eq!(
        responses[11]["result"]["structuredContent"]["entity"]["status"],
        "completed"
    );
    assert_eq!(
        responses[14]["result"]["structuredContent"]["entity"]["status"],
        "failed"
    );
}
