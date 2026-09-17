# Add status, occupancy, and task_spawn to MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the already-shipped App verbs `status`, `occupancy`, and `task_spawn` on `tb mcp` so a coordinator host does not drop to the terminal for snapshot, collision, or split.

**Architecture:** Add three tools to the existing stdio MCP dispatcher in `crates/cli/src/mcp.rs`. Each tool calls the same `App` method the CLI uses, with the MCP actor (`--actor` / `TASKBOARD_ACTOR`). Do not add HTTP routes, do not add `project_add` / `undo` / `note_set`, and do not change occupancy math.

**Tech Stack:** Existing Rust `tb mcp` JSON-RPC, `App::status` / `App::occupancy` / `App::task_spawn`, assert_cmd MCP tests.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- Same App validation and actor as CLI
- HTTP stays closed to agents
- Do not add MCP `project add` / `undo` / `note set`
- Do not change occupancy grouping (#118)
- CLI / MCP JSON stays snake_case
- Tool names: `status`, `occupancy`, `task_spawn`

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-132-mcp-status-occupancy-spawn.md`
- Modify: `crates/cli/src/mcp.rs` — tool schemas + dispatch
- Modify: `crates/cli/tests/mcp.rs` — `tools/list` + behavior tests

---

### Task 1: Failing MCP tests

**Files:**
- Modify: `crates/cli/tests/mcp.rs`

**Interfaces:**
- Consumes: existing `mcp_rpc` / `seed` helpers
- Produces: tests that fail until Task 2 registers and dispatches the three tools

- [ ] **Step 1: Require the new names in `tools/list`**

In `mcp_initialize_lists_core_tools_and_calls_app`, add to the second `for required` list:

```rust
        "status",
        "occupancy",
        "task_spawn",
```

Also assert the out-of-scope tools stay absent:

```rust
    for forbidden in ["project_add", "undo", "note_set"] {
        assert!(
            !names.contains(&forbidden),
            "MCP must not grow {forbidden}: {names:?}"
        );
    }
```

- [ ] **Step 2: Add a behavior test at the end of `crates/cli/tests/mcp.rs`**

```rust
#[test]
fn mcp_status_occupancy_and_task_spawn() {
    let dir = tempfile::tempdir().unwrap();
    seed(&dir);
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Sibling",
            "--json",
        ])
        .output()
        .unwrap();
    tb_in(&dir)
        .args(["task", "update", "TASK-1", "--worktree", "/tmp/shared", "--json"])
        .output()
        .unwrap();
    tb_in(&dir)
        .args(["task", "update", "TASK-2", "--worktree", "/tmp/shared", "--json"])
        .output()
        .unwrap();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor", "--json"])
        .output()
        .unwrap();
    tb_in(&dir)
        .args(["run", "start", "TASK-2", "--agent", "cursor", "--json"])
        .output()
        .unwrap();

    let responses = mcp_rpc(
        &dir,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
                "name":"status",
                "arguments":{"project":"renai-sim"}
            }}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"occupancy",
                "arguments":{}
            }}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"occupancy",
                "arguments":{"path":"/tmp/shared"}
            }}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
                "name":"task_spawn",
                "arguments":{"display_id":"TASK-1","titles":["API","UI"]}
            }}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
                "name":"task_spawn",
                "arguments":{"display_id":"TASK-1","titles":[]}
            }}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{
                "name":"project_add",
                "arguments":{"name":"Nope"}
            }}),
        ],
    );

    let status = &responses[0]["result"]["structuredContent"]["entity"];
    assert_eq!(status["open_runs"], 2);
    assert!(status["ready"].is_number());

    let collision = &responses[1]["result"]["structuredContent"]["entities"][0];
    assert_eq!(collision["worktree_path"], "/tmp/shared");
    assert_eq!(collision["runs"].as_array().unwrap().len(), 2);

    assert_eq!(
        responses[2]["result"]["structuredContent"]["entities"][0]["runs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let children = responses[3]["result"]["structuredContent"]["entities"]
        .as_array()
        .unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0]["display_id"], "TASK-3");
    assert_eq!(children[0]["column"], "todo");
    assert_eq!(children[1]["display_id"], "TASK-4");

    assert_eq!(responses[4]["result"]["isError"], true);
    assert_eq!(
        responses[4]["result"]["structuredContent"]["error"]["code"],
        "validation_error"
    );

    assert_eq!(responses[5]["result"]["isError"], true);
    assert_eq!(
        responses[5]["result"]["structuredContent"]["error"]["code"],
        "validation_error"
    );
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p taskboard-cli --test mcp -- --nocapture mcp_status`

Expected: `mcp_initialize_lists_core_tools_and_calls_app` FAIL `missing status`. `mcp_status_occupancy_and_task_spawn` FAIL unknown tool / missing structuredContent.

---

### Task 2: Register and dispatch the three tools

**Files:**
- Modify: `crates/cli/src/mcp.rs`

**Interfaces:**
- Consumes:
  - `App::status(project: Option<String>) -> Result<BoardStatus, AppError>`
  - `App::occupancy(OccupancyQuery { path: Option<String> }) -> Result<Vec<OccupancyGroup>, AppError>`
  - `App::task_spawn(actor, TaskSpawn { parent_display_id, titles }) -> Result<Vec<TaskDetail>, AppError>`
- Produces:
  - `status` → `{ ok: true, entity: BoardStatus, revision: 0 }` (same as CLI `print_entity` revision 0)
  - `occupancy` → `{ ok: true, entities: Vec<OccupancyGroup> }`
  - `task_spawn` → `{ ok: true, entities: Vec<TaskDetail> }`
  - `titles` is a JSON array of strings; empty array uses App validation (`field: "title"`, `must not be empty`)
  - unknown `project_add` still hits the existing `unknown tool` validation

- [ ] **Step 1: Add imports**

In `crates/cli/src/mcp.rs` change the `taskboard_application` import to include `OccupancyQuery` and `TaskSpawn`:

```rust
use taskboard_application::{
    ActivityQuery, Actor, App, CheckAdd, CommentAdd, InboxScope, LinkAdd, NextClaim, OccupancyQuery,
    ReviewAction, ReviewTask, RunCancel, RunContinue, RunFail, RunFinish, RunListQuery, RunStart,
    RunWait, TaskCreate, TaskListQuery, TaskSpawn, TaskUpdate,
};
```

- [ ] **Step 2: Add tool schemas after the `inbox` tool**

```rust
        tool(
            "status",
            "Board-wide snapshot of inbox, open runs, ready, review, and blocked",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" }
                }
            }),
        ),
        tool(
            "occupancy",
            "Group running and waiting runs by worktree path",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
        ),
        tool(
            "task_spawn",
            "Create child tasks and block the parent on them",
            json!({
                "type": "object",
                "properties": {
                    "display_id": { "type": "string" },
                    "titles": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["display_id", "titles"]
            }),
        ),
```

- [ ] **Step 3: Add dispatch arms before the `other =>` arm**

```rust
        "status" => {
            let snap = app.status(string_arg(&args, "project")).await?;
            Ok(json!({ "ok": true, "entity": snap, "revision": 0 }))
        }
        "occupancy" => {
            let groups = app
                .occupancy(OccupancyQuery {
                    path: string_arg(&args, "path"),
                })
                .await?;
            Ok(json!({ "ok": true, "entities": groups }))
        }
        "task_spawn" => {
            let titles = args
                .get("titles")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|title| !title.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let children = app
                .task_spawn(
                    actor,
                    TaskSpawn {
                        parent_display_id: require_string(&args, "display_id")?,
                        titles,
                    },
                )
                .await?;
            Ok(json!({ "ok": true, "entities": children }))
        }
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p taskboard-cli --test mcp -- --nocapture
```

Expected: PASS, including `mcp_status_occupancy_and_task_spawn`.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/mcp.rs crates/cli/tests/mcp.rs \
  docs/superpowers/plans/2026-09-17-issue-132-mcp-status-occupancy-spawn.md
git commit -m "$(cat <<'EOF'
feat(mcp): add status, occupancy, and task_spawn

Coordinator hosts can snapshot the board, list worktree collisions,
and split a parent without dropping to the CLI. Same App validation
and actor as the existing verbs. No HTTP, no project_add/undo/note_set.
EOF
)"
```

---

## Self-review

**1. Spec coverage**

| Acceptance | Task |
| --- | --- |
| `tools/list` includes `status`, `occupancy`, `task_spawn` | Task 1 list + Task 2 schemas |
| Same App validation and actor | Task 2 dispatch calls `App` with `actor` |
| HTTP still not an agent surface | no api crate edits |
| Not expanded to `project add` / `undo` / `note set` | forbidden-name assert + `project_add` call is validation_error |
| snake_case JSON | existing MCP `output` / serde rename |

**2. Placeholder scan:** complete test and dispatch code.

**3. Type consistency:** `OccupancyQuery.path`, `TaskSpawn { parent_display_id, titles }`, `status(Option<String>)` match `crates/application`.
