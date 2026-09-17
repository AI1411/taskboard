# Reply + `run continue` in one beat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a person answer a Waiting card and resume the winning run in one command (CLI) and optionally from the Inspector.

**Architecture:** New `App::comment_add_and_continue` writes the comment then continues the winning run if it is `waiting`; otherwise `validation_error` (comment is not left behind — one transaction). `RunContinue.reply` does the same keyed by `RUN-n`. Inspector “If Waiting, Continue” posts `{ body, continue: true }` through the existing comment HTTP route.

**Tech Stack:** Rust App/CLI/HTTP, React Inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Failed re-runs stay a new `run start`
- Do not change `run continue` waiting → running semantics
- CLI `--json` snake_case; HTTP camelCase
- No agent launch from the app

## File map

- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/reply_continue.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`
- Modify: `crates/api/src/dto.rs`, `routes.rs`
- Modify: `packages/client/src/transport.ts`, `http.ts`, `tauri.ts`
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.test.tsx`, `TaskboardApp.tsx`, `fakeTransport.ts`

User already chose sequential inline execution.

---

### Task 1: App reply-and-continue

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/reply_continue.rs`

**Interfaces:**
- Produces:
  - `pub struct ReplyContinueResult { comment: Comment, run: Run }`
  - `App::comment_add_and_continue(actor, CommentAdd) -> Result<ReplyContinueResult, AppError>`
  - `RunContinue.reply: Option<String>`
  - Winning run missing or not `waiting` → `validation_error` field `status`, message `run must be waiting`
  - Same transaction as comment insert + continue
  - Failed card (no waiting winning run) → `validation_error` (do not start a new run)

- [x] **Step 1: Write failing tests**

Create `crates/application/tests/reply_continue.rs` using the `seeded_waiting` setup from `run_continue.rs` (copy TestApp helpers):

```rust
#[tokio::test]
async fn comment_add_and_continue_resumes_waiting_run() {
    let app = seeded_waiting().await;
    let result = app
        .comment_add_and_continue(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "here is spec".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.comment.body, "here is spec");
    assert_eq!(result.run.display_id, "RUN-1");
    assert_eq!(result.run.status, RunStatus::Running);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.comments[0].body, "here is spec");
}

#[tokio::test]
async fn comment_add_and_continue_on_idle_is_validation_error() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "renai-sim".into(), title: "Idle".into(), column: None, urgent: false }).await.unwrap();
    let err = app
        .comment_add_and_continue(
            &actor,
            CommentAdd { task_display_id: "TASK-1".into(), body: "nope".into() },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    assert!(app.comment_list("TASK-1").await.unwrap().is_empty());
}

#[tokio::test]
async fn run_continue_reply_writes_comment() {
    let app = seeded_waiting().await;
    let run = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                reply: Some("here is spec".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Running);
    let comments = app.comment_list("TASK-1").await.unwrap();
    assert_eq!(comments[0].body, "here is spec");
}
```

Also add `reply: None` to existing `RunContinue {` literals in `run_continue.rs` after the field exists (impl step). The new test will fail to compile until the field exists — keep `reply` only in the new file for the red step; update old literals in the impl step.

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test reply_continue -- --nocapture`

Expected: FAIL — `comment_add_and_continue` / `ReplyContinueResult` not found.

- [x] **Step 3: Minimal implementation**

`commands.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyContinueResult {
    pub comment: taskboard_core::Comment,
    pub run: taskboard_core::Run,
}
```

Add `pub reply: Option<String>` to `RunContinue`.

Export `ReplyContinueResult`.

`app.rs` `comment_add_and_continue` + inner; `run_continue` passes `cmd.reply` into a shared helper:

```rust
async fn continue_waiting_run(
    store: &mut dyn Store,
    actor: &Actor,
    run_display_id: &str,
    message: Option<String>,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    run_continue_inner(
        store,
        actor,
        RunContinue {
            run_display_id: run_display_id.to_string(),
            message,
            reply: None,
            revision,
        },
        now,
    )
    .await
}
```

`comment_add_and_continue_inner`:

```rust
    let comment = comment_add_inner(store, actor, cmd, now).await?;
    let task = require_live_task(store, &comment is on task — use cmd.task_display_id).await?;
    let runs = store.list_runs(task.id).await?;
    let winning = winning_run(&runs).ok_or_else(|| AppError::Validation {
        field: "status".into(),
        message: "run must be waiting".into(),
    })?;
    if winning.status != RunStatus::Waiting {
        return Err(AppError::Validation {
            field: "status".into(),
            message: "run must be waiting".into(),
        });
    }
    let run = continue_waiting_run(store, actor, &winning.display_id.clone(), None, None, now).await?;
    Ok(ReplyContinueResult { comment, run })
```

`run_continue`:

```rust
        store.begin().await?;
        let result = async {
            if let Some(reply) = cmd.reply.clone() {
                let run = require_run(store, &cmd.run_display_id).await?;
                let task = store.get_task(run.task_id).await?.ok_or_else(|| AppError::NotFound {
                    entity: "task".into(),
                    id: run.task_id.to_string(),
                })?;
                comment_add_inner(
                    store,
                    actor,
                    CommentAdd { task_display_id: task.display_id, body: reply },
                    now,
                )
                .await?;
            }
            run_continue_inner(store, actor, RunContinue { reply: None, ..cmd }, now).await
        }
        .await;
        commit_or_rollback(store, result).await
```

Check `Store::get_task`. If missing, `require_live_task` after mapping task_id via existing helper. Prefer `store.list` / get_task_by id.

Update all existing `RunContinue {` to include `reply: None`.

- [x] **Step 4: Run tests**

Run: `cargo test -p taskboard-application --test reply_continue -- --nocapture`

Run: `cargo test -p taskboard-application --test run_continue -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/application
git commit -m "feat: comment and continue a waiting run in one transaction"
```

---

### Task 2: CLI + MCP flags

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`

**Interfaces:**
- `CommentCommand::Add { continue_waiting: bool }` via `--continue`
- `RunCommand::Continue { reply: Option<String> }` via `--reply`
- `--json` for comment --continue prints `{ ok, entity: ReplyContinueResult }`
- MCP `comment_add` gains `continue` bool; `run_continue` gains `reply`

- [x] **Step 1: Failing CLI tests**

```rust
#[test]
fn comment_add_continue_json_resumes_waiting() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Wait"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    tb_in(&dir).args(["run", "wait", "RUN-1", "--reason", "Need spec"]).assert().success();
    let v = json_ok(&dir, &["comment", "add", "TASK-1", "--text", "here is spec", "--continue"]);
    assert_eq!(v["entity"]["comment"]["body"], "here is spec");
    assert_eq!(v["entity"]["run"]["status"], "running");
}

#[test]
fn comment_add_continue_on_idle_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Idle"]).assert().success();
    let out = tb_in(&dir)
        .args(["comment", "add", "TASK-1", "--text", "nope", "--continue", "--json"])
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
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Wait"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    tb_in(&dir).args(["run", "wait", "RUN-1", "--reason", "Need spec"]).assert().success();
    let v = json_ok(&dir, &["run", "continue", "RUN-1", "--reply", "here is spec"]);
    assert_eq!(v["entity"]["status"], "running");
    let comments = json_ok(&dir, &["comment", "list", "TASK-1"]);
    assert_eq!(comments["entities"][0]["body"], "here is spec");
}
```

- [x] **Step 2: Run to fail**

Run: `cargo test -p taskboard-cli --test cli_json comment_add_continue -- --nocapture`

Expected: FAIL — unexpected argument `--continue`.

- [x] **Step 3: Implement CLI/MCP**

Wire `--continue` to `comment_add_and_continue`. Wire `--reply` into `RunContinue.reply`. MCP `comment_add` if `continue` then `comment_add_and_continue`.

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `feat(cli): add comment --continue and run continue --reply`

---

### Task 3: Inspector checkbox + HTTP

**Files:**
- Modify: `crates/api/src/dto.rs` — `AddCommentBody.continue_waiting` (`continue`, default false)
- Modify: `crates/api/src/routes.rs` — call `comment_add_and_continue` when set
- Modify: `packages/client/src/transport.ts`, `http.ts`, `tauri.ts` — `commentAdd(id, body, continueWaiting?)`
- Modify: `packages/ui/src/Inspector.tsx` — checkbox “If Waiting, Continue”
- Modify: `packages/ui/src/Inspector.test.tsx`, `TaskboardApp.tsx`, `fakeTransport.ts`
- Modify: `crates/desktop-commands/src/commands.rs` — optional continue flag

- [x] **Step 1: Failing Inspector test**

```tsx
  it("can continue a waiting run when sending a comment", async () => {
    const task = detail();
    task.displayStatus = "waiting";
    const onCommentAdd = vi.fn();
    renderInspector(task, { onCommentAdd });
    await userEvent.click(screen.getByLabelText("If Waiting, Continue"));
    await userEvent.type(screen.getByPlaceholderText("Add a comment"), "here is spec{Enter}");
    expect(onCommentAdd).toHaveBeenCalledWith("here is spec", true);
  });
```

Change `onCommentAdd?: (body: string, continueWaiting?: boolean) => void`.

- [x] **Step 2: Fail** — checkbox missing.

- [x] **Step 3: Implement checkbox + HTTP `continue` + TaskboardApp `transport.commentAdd(id, body, continueWaiting)`**

Desktop `comment_add_inner` takes `continue_waiting: bool`.

- [ ] **Step 4: `npm test` Inspector + `cargo test -p taskboard-api` if a route test is added.

- [x] **Step 5: Commit** `feat(ui): continue waiting run from inspector comment`

---

## Self-review

**1. Spec coverage:** one command reply+continue; non-waiting validation; failed still needs new start; Inspector checkbox; snake_case JSON.

**2. Placeholder scan:** none

**3. Types:** `ReplyContinueResult`, `RunContinue.reply`, `AddCommentBody.continue`.
