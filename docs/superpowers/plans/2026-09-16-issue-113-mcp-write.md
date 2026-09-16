# MCP write-path parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `tb mcp` the same write verbs the CLI already has so hosts do not fork to the terminal for create / move / update / run lifecycle / checks / links / stale / detect.

**Architecture:** Add MCP tools that call existing `App` methods with the same structs and `--actor`. `run_start` (including `exclusive`) already shipped in #112. No new HTTP agent API. No remote MCP. JSON payloads stay snake_case `{ ok, entity|entities, revision? }`.

**Tech Stack:** Existing Rust MCP stdio server (`crates/cli/src/mcp.rs`), assert_cmd MCP tests.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Same `App` validation and actor as CLI
- HTTP stays closed to agents
- No remote MCP
- CLI `--json` / MCP structuredContent stay snake_case
- Do not replace the CLI
- `run_start` / `next` already exist — do not regress them

## File map

- Modify: `crates/cli/src/mcp.rs` — tool list + dispatch
- Modify: `crates/cli/tests/mcp.rs` — required names + write-path integration test

User already chose sequential inline execution.

---

### Task 1: Task, run, check, link, stale, detect tools

**Files:**
- Modify: `crates/cli/src/mcp.rs`
- Modify: `crates/cli/tests/mcp.rs`

**Interfaces:**
- Consumes: `App::task_create`, `task_move`, `task_update`, `run_start`/`run_start_exclusive` (existing), `run_wait`, `run_finish`, `run_fail`, `check_add`, `check_toggle`, `check_list`, `link_add`, `stale_list`, `detect_project`
- Produces MCP tools:
  - `task_create` — `project?`, `title` required, `column?`, `urgent?`. Missing project uses `detect_project(cwd, TASKBOARD_PROJECT)` then `TaskCreate`
  - `task_move` — `display_id`, `column` required
  - `task_update` — `display_id` required, `title?`, `worktree?`, `branch?` (empty string clears, same as CLI `empty_to_none`)
  - `run_wait` — `display_id`, `reason`
  - `run_finish` / `run_fail` — `display_id`, `summary`
  - `check_add` — `display_id` (TASK-n), `text`
  - `check_toggle` — `display_id` (CHECK-n)
  - `check_list` — `display_id` (TASK-n)
  - `link_add` — `display_id` plus exactly one of `url` / `path` / `blocked_by`
  - `stale` — `minutes` default 30
  - `project_detect` — `detect_project(cwd, TASKBOARD_PROJECT)`
  - Unknown column / missing exclusive link target → `validation_error`

- [ ] **Step 1: Write the failing tests**

In `mcp_initialize_lists_core_tools_and_calls_app`, require these names in addition to the existing ones:

```rust
    for required in [
        "task_create",
        "task_move",
        "task_update",
        "run_start",
        "run_wait",
        "run_finish",
        "run_fail",
        "check_add",
        "check_toggle",
        "check_list",
        "stale",
        "project_detect",
        "link_add",
    ] {
        assert!(names.contains(&required), "missing {required} in {names:?}");
    }
```

Add `crates/cli/tests/mcp.rs`:

```rust
#[test]
fn mcp_write_path_parity() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir);
    let mut child_env = vec![("TASKBOARD_PROJECT", "renai-sim")];
    let _ = child_env;
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
    assert_eq!(responses[0]["result"]["structuredContent"]["entity"]["slug"], "renai-sim");
    assert_eq!(responses[1]["result"]["structuredContent"]["entity"]["display_id"], "TASK-2");
    assert_eq!(responses[2]["result"]["structuredContent"]["entity"]["column"], "in-progress");
    assert_eq!(responses[3]["result"]["structuredContent"]["entity"]["title"], "Wrote me");
    assert_eq!(responses[3]["result"]["structuredContent"]["entity"]["worktree_path"], "/tmp/wt");
    assert_eq!(responses[5]["result"]["structuredContent"]["entity"]["status"], "waiting");
    assert_eq!(responses[8]["result"]["structuredContent"]["entities"][0]["done"], true);
    assert_eq!(responses[9]["result"]["structuredContent"]["entity"]["links"][0]["kind"], "blocked_by");
    assert!(responses[10]["result"]["structuredContent"]["entities"].is_array());
    assert_eq!(responses[11]["result"]["structuredContent"]["entity"]["status"], "completed");
    assert_eq!(responses[14]["result"]["structuredContent"]["entity"]["status"], "failed");
}
```

Add helper next to `mcp_rpc`:

```rust
fn mcp_rpc_with_env(dir: &TempDir, env: &[(&str, &str)], requests: &[Value]) -> Vec<Value> {
    let mut child = tb_in(dir)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        child.env(key, value);
    }
    let mut child = child.spawn().unwrap();
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
```

Refactor `mcp_rpc` to call `mcp_rpc_with_env(dir, &[], requests)`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-cli --test mcp -- --nocapture`

Expected: FAIL — missing `task_create` (and the write-path calls return unknown-tool validation).

- [ ] **Step 3: Write minimal implementation**

In `mcp.rs` imports add `CheckAdd, LinkAdd, RunFail, RunFinish, RunWait, TaskCreate, TaskUpdate` and `LinkKind`.

Add each `tool(...)` entry after the existing `run_start` / `next` tools. Dispatch arms:

```rust
        "task_create" => {
            let project = match string_arg(&args, "project") {
                Some(slug) => slug,
                None => {
                    let cwd = std::env::current_dir().map_err(|err| {
                        taskboard_application::AppError::Io(err.to_string())
                    })?;
                    let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
                    app.detect_project(
                        &cwd,
                        env_slug.as_deref().filter(|value| !value.is_empty()),
                    )
                    .await?
                    .slug
                }
            };
            let task = app
                .task_create(
                    actor,
                    TaskCreate {
                        project_slug: project,
                        title: require_string(&args, "title")?,
                        column: parse_column(args.get("column"))?,
                        urgent: args.get("urgent").and_then(Value::as_bool).unwrap_or(false),
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "task_move" => {
            let column = parse_column(args.get("column"))?.ok_or_else(|| {
                taskboard_application::AppError::Validation {
                    field: "column".into(),
                    message: "column is required".into(),
                }
            })?;
            let task = app
                .task_move(actor, &require_string(&args, "display_id")?, column, None)
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "task_update" => {
            let task = app
                .task_update(
                    actor,
                    TaskUpdate {
                        display_id: require_string(&args, "display_id")?,
                        title: string_arg(&args, "title"),
                        worktree_path: args.get("worktree").and_then(Value::as_str).map(|s| {
                            if s.is_empty() { None } else { Some(s.to_string()) }
                        }),
                        branch: args.get("branch").and_then(Value::as_str).map(|s| {
                            if s.is_empty() { None } else { Some(s.to_string()) }
                        }),
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "run_wait" => {
            let run = app
                .run_wait(
                    actor,
                    RunWait {
                        run_display_id: require_string(&args, "display_id")?,
                        reason: require_string(&args, "reason")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_finish" => {
            let run = app
                .run_finish(
                    actor,
                    RunFinish {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "run_fail" => {
            let run = app
                .run_fail(
                    actor,
                    RunFail {
                        run_display_id: require_string(&args, "display_id")?,
                        summary: require_string(&args, "summary")?,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": run, "revision": run.revision }))
        }
        "check_add" => {
            let check = app
                .check_add(
                    actor,
                    CheckAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        text: require_string(&args, "text")?,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": check, "revision": 0 }))
        }
        "check_toggle" => {
            let check = app
                .check_toggle(actor, &require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entity": check, "revision": 0 }))
        }
        "check_list" => {
            let checks = app
                .check_list(&require_string(&args, "display_id")?)
                .await?;
            Ok(json!({ "ok": true, "entities": checks }))
        }
        "link_add" => {
            let (kind, value) = if let Some(url) = string_arg(&args, "url") {
                (LinkKind::Url, url)
            } else if let Some(path) = string_arg(&args, "path") {
                (LinkKind::Path, path)
            } else if let Some(blocked_by) = string_arg(&args, "blocked_by") {
                (LinkKind::BlockedBy, blocked_by)
            } else {
                return Err(taskboard_application::AppError::Validation {
                    field: "target".into(),
                    message: "url, path, or blocked_by is required".into(),
                });
            };
            let task = app
                .link_add(
                    actor,
                    LinkAdd {
                        task_display_id: require_string(&args, "display_id")?,
                        kind,
                        value,
                        revision: None,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entity": task, "revision": task.revision }))
        }
        "stale" => {
            let minutes = args.get("minutes").and_then(Value::as_i64).unwrap_or(30);
            let runs = app.stale_list(minutes).await?;
            Ok(json!({ "ok": true, "entities": runs }))
        }
        "project_detect" => {
            let cwd = std::env::current_dir()
                .map_err(|err| taskboard_application::AppError::Io(err.to_string()))?;
            let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
            let project = app
                .detect_project(&cwd, env_slug.as_deref().filter(|value| !value.is_empty()))
                .await?;
            Ok(json!({ "ok": true, "entity": project, "revision": project.revision }))
        }
```

Tool schemas (copy into `tools()`):

```rust
        tool("task_create", "Create a task", json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "title": { "type": "string" },
                "column": { "type": "string" },
                "urgent": { "type": "boolean" }
            },
            "required": ["title"]
        })),
        tool("task_move", "Move a task to a column", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "column": { "type": "string" }
            },
            "required": ["display_id", "column"]
        })),
        tool("task_update", "Update a task title, worktree, or branch", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "title": { "type": "string" },
                "worktree": { "type": "string" },
                "branch": { "type": "string" }
            },
            "required": ["display_id"]
        })),
        tool("run_wait", "Mark a run as waiting", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "reason": { "type": "string" }
            },
            "required": ["display_id", "reason"]
        })),
        tool("run_finish", "Mark a run as completed", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "summary": { "type": "string" }
            },
            "required": ["display_id", "summary"]
        })),
        tool("run_fail", "Mark a run as failed", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "summary": { "type": "string" }
            },
            "required": ["display_id", "summary"]
        })),
        tool("check_add", "Add a checklist item", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "text": { "type": "string" }
            },
            "required": ["display_id", "text"]
        })),
        tool("check_toggle", "Toggle a checklist item", json!({
            "type": "object",
            "properties": { "display_id": { "type": "string" } },
            "required": ["display_id"]
        })),
        tool("check_list", "List checklist items on a task", json!({
            "type": "object",
            "properties": { "display_id": { "type": "string" } },
            "required": ["display_id"]
        })),
        tool("link_add", "Add a URL, path, or blocked-by link", json!({
            "type": "object",
            "properties": {
                "display_id": { "type": "string" },
                "url": { "type": "string" },
                "path": { "type": "string" },
                "blocked_by": { "type": "string" }
            },
            "required": ["display_id"]
        })),
        tool("stale", "List stale running runs", json!({
            "type": "object",
            "properties": { "minutes": { "type": "integer" } }
        })),
        tool("project_detect", "Resolve the current project from cwd or TASKBOARD_PROJECT", json!({
            "type": "object",
            "properties": {}
        })),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-cli --test mcp -- --nocapture`

Expected: PASS. If `run_finish` on a waiting run is `validation_error`, `run_continue` first or `run_fail` the waiting run and finish a fresh running one. Prefer: finish only after continue, or fail the waiting run then start+finish another. The test above finishes a waiting RUN-1 — check `run_finish` rules. If waiting cannot finish, change id 12 to `run_continue` then `run_finish`, or use `run_fail` on RUN-1 and keep `run_finish` for a third running run.

Look up `run_finish_inner`: if status must be running, insert a `run_continue` call before finish in the test (update the test, not the App).

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/mcp.rs crates/cli/tests/mcp.rs docs/superpowers/plans/2026-09-16-issue-113-mcp-write.md
git commit -m "feat(mcp): add write-path task, run, check, and detect tools"
```

---

## Self-review

**1. Spec coverage**
- create / move / update through App — `task_*`
- start / wait / finish / fail / exclusive start — `run_start` already + new wait/finish/fail
- checks, links, stale, project detect — Task 1
- Actor/validation match CLI — same App methods
- HTTP closed / no remote MCP — no api/desktop changes
- snake_case JSON — structuredContent uses serde models

**2. Placeholder scan:** none

**3. Type consistency:** `TaskCreate` / `TaskUpdate` / `RunWait` / `RunFinish` / `RunFail` / `CheckAdd` / `LinkAdd` match application crate.
