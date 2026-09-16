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
    let mut child = tb_in(dir)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
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
