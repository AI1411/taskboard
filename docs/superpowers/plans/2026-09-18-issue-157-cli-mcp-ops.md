# Share CLI/MCP ops layer (#157) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CLI and MCP share one `ops` execution layer for the overlapped command set, and MCP tool list + dispatch come from one non-drifting registry.

**Architecture:** Add `crates/cli/src/ops/` with typed async ops (`ops::task_create(app, actor, …) -> Result<Task, AppError>`, etc.) that both entrances call. Keep clap parsing and human/`--json` presentation in `main.rs`; keep MCP JSON-Schema arg parsing at the MCP edge. Replace MCP’s parallel `tools()` vec + `dispatch_tool` match with a single `ToolKind` enum that owns `name` / `description` / `schema` / `run` so list and dispatch cannot drift.

**Tech Stack:** Rust 2021, `taskboard-cli`, `taskboard-application`, `serde_json`, existing `crates/cli/tests/mcp.rs` / `cli_json.rs`.

## Global Constraints

- Behavior-preserving refactor only — no product feature changes
- Do not change agent-facing JSON field names
- Keep clap and MCP JSON Schema at the edges (no remote MCP)
- Existing CLI / MCP tests must stay green
- Four columns stay `todo | in-progress | in-review | done`

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.

## File map

- Create: `crates/cli/src/ops/mod.rs` — shared ops + `detect_project` + JSON envelope helpers used by MCP
- Modify: `crates/cli/src/mcp.rs` — `ToolKind` registry; `tools()` / `dispatch` derived from it; handlers call `ops`
- Modify: `crates/cli/src/main.rs` — `mod ops;`; overlapped arms call `ops::*` instead of `app.*` directly
- Test: `crates/cli/tests/mcp.rs`, `crates/cli/tests/cli_json.rs`; optional unit test in `mcp.rs` that every `ToolKind` has unique name

## Overlapped ops (MCP tool set ∩ CLI)

`project_list`, `project_detect`, `task_list`, `task_show`, `task_create`, `task_move`, `task_update`, `task_spawn`, `run_list`, `run_show`, `run_current`, `run_start`, `run_continue`, `run_wait`, `run_finish`, `run_fail`, `run_cancel`, `next`, `inbox`, `status`, `occupancy`, `activity`, `stale`, `comment_add` (+ continue), `comment_list`, `review`, `check_add`, `check_toggle`, `check_list`, `link_add`

CLI-only commands (trash, undo, backup, project CRUD, notes, prioritize, urgent, …) may keep calling `App` directly.

---

### Task 1: `ops` module + MCP `ToolKind` registry

**Files:**
- Create: `crates/cli/src/ops/mod.rs`
- Modify: `crates/cli/src/mcp.rs`
- Modify: `crates/cli/src/main.rs` (`mod ops;`)

**Interfaces:**
- Consumes: `App`, `Actor`, application command structs (`TaskCreate`, `RunStart`, …)
- Produces: `ops::{detect_project, task_show, task_create, …}`; `mcp::ToolKind::{ALL, name, description, schema, run}`

- [ ] **Step 1: Write failing registry uniqueness test**

In `mcp.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_kind_names_are_unique() {
        let mut names = std::collections::BTreeSet::new();
        for tool in ToolKind::ALL {
            assert!(names.insert(tool.name()), "duplicate tool {}", tool.name());
        }
        assert!(names.contains("task_show"));
        assert!(names.contains("project_list"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli tool_kind_names_are_unique -- --nocapture`
Expected: FAIL — `ToolKind` not found

- [ ] **Step 3: Implement `ops` + `ToolKind`**

`ops/mod.rs` sketch (typed returns; MCP wraps):

```rust
use taskboard_application::{Actor, App, AppError, /* command types */};
use taskboard_core::{Project, Run, TaskDetail, TaskSummary, /* … */};

pub async fn detect_project(app: &App) -> Result<Project, AppError> {
    let cwd = std::env::current_dir().map_err(|e| AppError::Io(e.to_string()))?;
    let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
    app.detect_project(&cwd, env_slug.as_deref().filter(|v| !v.is_empty()))
        .await
}

pub async fn task_show(app: &App, display_id: &str) -> Result<TaskDetail, AppError> {
    app.task_show(display_id).await
}

pub async fn task_create(
    app: &App,
    actor: &Actor,
    cmd: TaskCreate,
) -> Result<TaskDetail, AppError> {
    app.task_create(actor, cmd).await
}

// …one fn per overlapped verb; comment_add returns either Comment or ReplyContinueResult
// via an enum or two functions: comment_add / comment_add_and_continue

pub fn entity_ok<T: serde::Serialize>(entity: &T, revision: i64) -> serde_json::Value {
    serde_json::json!({ "ok": true, "entity": entity, "revision": revision })
}

pub fn entities_ok<T: serde::Serialize>(entities: &[T]) -> serde_json::Value {
    serde_json::json!({ "ok": true, "entities": entities })
}
```

`mcp.rs` registry:

```rust
#[derive(Clone, Copy)]
enum ToolKind {
    ProjectList,
    TaskShow,
    // …every current MCP tool
}

impl ToolKind {
    const ALL: &'static [ToolKind] = &[/* every variant once */];

    fn name(self) -> &'static str { match self { Self::TaskShow => "task_show", /* … */ } }
    fn description(self) -> &'static str { /* current descriptions verbatim */ }
    fn schema(self) -> Value { /* current schemas verbatim */ }

    async fn run(self, app: &App, actor: &Actor, args: Value) -> Result<Value, AppError> {
        match self {
            Self::TaskShow => {
                let task = ops::task_show(app, &require_string(&args, "display_id")?).await?;
                Ok(ops::entity_ok(&task, task.revision))
            }
            // …parse args at edge, call ops, wrap with entity_ok/entities_ok
        }
    }
}

fn tools() -> Vec<Value> {
    ToolKind::ALL
        .iter()
        .copied()
        .map(|t| tool(t.name(), t.description(), t.schema()))
        .collect()
}

async fn dispatch_tool(app: &App, actor: &Actor, name: &str, args: Value) -> Result<Value, AppError> {
    for tool in ToolKind::ALL {
        if tool.name() == name {
            return tool.run(app, actor, args).await;
        }
    }
    Err(AppError::Validation {
        field: "name".into(),
        message: format!("unknown tool `{name}`"),
    })
}
```

Move `detect_current_project` body into `ops::detect_project`. Keep `string_arg` / `require_string` / `parse_column` / `parse_statuses` / `optional_clearable` in `mcp.rs`.

- [ ] **Step 4: Run registry test**

Run: `cargo test -p taskboard-cli tool_kind_names_are_unique -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/ops crates/cli/src/mcp.rs crates/cli/src/main.rs docs/superpowers/plans/2026-09-18-issue-157-cli-mcp-ops.md
git commit -m "refactor(cli): add ops module and MCP ToolKind registry (#157)"
```

---

### Task 2: Wire CLI overlapped arms through `ops`

**Files:**
- Modify: `crates/cli/src/main.rs`

**Interfaces:**
- Consumes: `ops::*`
- Produces: same CLI stdout/stderr behavior

- [ ] **Step 1: Point overlapped `main.rs` arms at ops**

Examples:

```rust
Command::Inbox { project, archived } => {
    let items = ops::inbox(app, project, archived)
        .await
        .map_err(|err| output::print_error(&err, json))?;
    output::print_entities(json, &items, || output::print_inbox(&items));
    Ok(())
}
```

```rust
TaskCommand::Show { display_id } => {
    let task = ops::task_show(app, &display_id)
        .await
        .map_err(|err| output::print_error(&err, json))?;
    output::print_entity(json, &task, task.revision, || { /* unchanged human */ });
}
```

Same for create/move/update/spawn, runs, next/review/status/occupancy/stale/activity, comments, checks, link_add, project_list/detect.

Keep revision / clap-only fields (`revision` on update/move) passed through ops into the existing `App` command structs.

- [ ] **Step 2: Run CLI + MCP tests**

Run: `cargo test -p taskboard-cli -- --test-threads=1`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add crates/cli/src/main.rs crates/cli/src/ops
git commit -m "refactor(cli): route overlapped commands through shared ops (#157)"
```

---

### Task 3: Full verification + PR

- [ ] **Step 1:** `cargo fmt` and `cargo clippy -p taskboard-cli -- -D warnings`
- [ ] **Step 2:** `cargo test -p taskboard-cli -- --test-threads=1`
- [ ] **Step 3:** Open PR against `main`, wait for CI green, merge, delete remote branch

## Self-review

1. **Spec coverage:** Shared ops for overlapped set — Task 1–2. MCP registry — Task 1 (`ToolKind`). Clap/schema at edges — yes. Behavior/tests green — Task 3.
2. **Placeholders:** None; concrete types and commands listed.
3. **Type consistency:** Ops return domain types; MCP wraps with `entity_ok`/`entities_ok`; CLI prints via `output::*`.
