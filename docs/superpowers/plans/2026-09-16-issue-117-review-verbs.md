# In Review approve and request-changes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a person approve or bounce an In Review card in one command: comment plus column move, without starting a run or adding a fifth column.

**Architecture:** `App::review` writes the comment then `task_move_inner` to `done` (approve) or `in-progress` (changes) in one transaction. Only `in-review` cards are allowed. Unchecked items do not block. Inspector shows Approve / Request changes only when the selected card is In Review.

**Tech Stack:** Rust App/CLI/HTTP, React Inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card; these verbs do move the card
- Do not start a new run on request-changes
- Do not block Done on unchecked checklist items
- No fifth column
- CLI `--json` snake_case; HTTP camelCase
- No agent launch from the app

## File map

- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/review.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`
- Modify: `crates/api/src/dto.rs`, `routes.rs`
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `packages/types/src/index.ts`
- Modify: `packages/client/src/transport.ts`, `http.ts`, `tauri.ts`, `http.test.ts`, `tauri.test.ts`
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.test.tsx`, `TaskboardApp.tsx`, `fakeTransport.ts`

User already chose sequential inline execution.

---

### Task 1: App::review

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/review.rs`

**Interfaces:**
- Consumes: `comment_add_inner`, `task_move_inner`, `require_live_task`
- Produces:
  - `pub enum ReviewAction { Approve, Changes }`
  - `pub struct ReviewTask { task_display_id: String, action: ReviewAction, text: String, revision: Option<i64> }`
  - `App::review(actor, cmd) -> Result<TaskDetail, AppError>`
  - Approve → comment + `Column::Done`
  - Changes → comment + `Column::InProgress`, no new run
  - Column other than In Review → `validation_error` field `column`, message `task must be in-review`
  - Blank text → `validation_error` field `text` (via `comment_add_inner`)
  - Unchecked checks do not block approve
  - One transaction so a failed move leaves no comment

- [ ] **Step 1: Write failing tests**

Create `crates/application/tests/review.rs` with the `TestApp` / `cli_actor` / `test_app` helpers from `run_cancel.rs`, plus:

```rust
async fn seeded_in_review() -> TestApp {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Ship login".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn review_approve_comments_and_moves_to_done() {
    let app = seeded_in_review().await;
    app.check_add(&cli_actor(), CheckAdd { task_display_id: "TASK-1".into(), text: "Write tests".into() })
        .await
        .unwrap();
    let shown = app
        .review(
            &cli_actor(),
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Approve,
                text: "lgtm".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(shown.column, Column::Done);
    assert_eq!(shown.comments[0].body, "lgtm");
    assert_eq!(shown.checks[0].done, false);
    assert!(shown.runs.is_empty());
}

#[tokio::test]
async fn review_changes_moves_to_in_progress_without_starting_a_run() {
    let app = seeded_in_review().await;
    let shown = app
        .review(
            &cli_actor(),
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Changes,
                text: "fix the copy".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(shown.column, Column::InProgress);
    assert_eq!(shown.comments[0].body, "fix the copy");
    assert!(shown.runs.is_empty());
}

#[tokio::test]
async fn review_todo_is_validation_error_and_leaves_no_comment() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None })
        .await
        .unwrap();
    app.task_create(
        &actor,
        TaskCreate { project_slug: "renai-sim".into(), title: "Idle".into(), column: None, urgent: false },
    )
    .await
    .unwrap();
    let err = app
        .review(
            &actor,
            ReviewTask {
                task_display_id: "TASK-1".into(),
                action: ReviewAction::Approve,
                text: "nope".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
    assert!(app.comment_list("TASK-1").await.unwrap().is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test review -- --nocapture`

Expected: FAIL — `ReviewTask` / `review` not found.

- [ ] **Step 3: Minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Approve,
    Changes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTask {
    pub task_display_id: String,
    pub action: ReviewAction,
    pub text: String,
    pub revision: Option<i64>,
}
```

```rust
async fn review_inner(...) -> Result<TaskDetail, AppError> {
    let task = require_live_task(store, &cmd.task_display_id).await?;
    if task.column != Column::InReview {
        return Err(AppError::Validation {
            field: "column".into(),
            message: "task must be in-review".into(),
        });
    }
    comment_add_inner(
        store,
        actor,
        CommentAdd { task_display_id: cmd.task_display_id.clone(), body: cmd.text },
        now,
    )
    .await?;
    let dest = match cmd.action {
        ReviewAction::Approve => Column::Done,
        ReviewAction::Changes => Column::InProgress,
    };
    task_move_inner(store, actor, &cmd.task_display_id, dest, cmd.revision, now).await
}
```

- [ ] **Step 4: Run tests** — Expected: PASS

- [ ] **Step 5: Commit** `feat: approve or bounce In Review cards in one transaction`

---

### Task 2: CLI + MCP

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`

**Interfaces:**
- `Command::Review { display_id, approve: bool, changes: bool, text: String }` with required clap group `approve|changes`
- `tb review TASK-n --approve|--changes --text "…"`
- `--json` entity is `TaskDetail` (`column`, `comments`)
- MCP `review` with `display_id`, `action` (`approve`|`changes`), `text`

- [ ] **Step 1: Failing CLI tests**

```rust
#[test]
fn review_approve_json_moves_to_done() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Ship", "--column", "in-review"]).assert().success();
    let v = json_ok(&dir, &["review", "TASK-1", "--approve", "--text", "lgtm"]);
    assert_eq!(v["entity"]["column"], "done");
    assert_eq!(v["entity"]["comments"][0]["body"], "lgtm");
}

#[test]
fn review_changes_json_moves_to_in_progress() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Ship", "--column", "in-review"]).assert().success();
    let v = json_ok(&dir, &["review", "TASK-1", "--changes", "--text", "fix the copy"]);
    assert_eq!(v["entity"]["column"], "in-progress");
    let runs = json_ok(&dir, &["run", "list", "--task", "TASK-1"]);
    assert_eq!(runs["entities"].as_array().unwrap().len(), 0);
}

#[test]
fn review_todo_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Idle"]).assert().success();
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
```

Add `"review"` to the MCP required tools list. Confirm `run list --task` exists; if the filter flag differs, use the existing list invocation from `cli_json.rs`.

- [ ] **Step 2: Fail** — unrecognized command `review`.

- [ ] **Step 3: Implement CLI/MCP**

```rust
    #[command(group(
        clap::ArgGroup::new("verdict")
            .required(true)
            .args(["approve", "changes"])
    ))]
    Review {
        display_id: String,
        #[arg(long)]
        approve: bool,
        #[arg(long)]
        changes: bool,
        #[arg(long)]
        text: String,
    },
```

Map `approve` → `ReviewAction::Approve`, `changes` → `ReviewAction::Changes`. Human text: `Approved TASK-n` / `Requested changes on TASK-n`.

MCP: `action` string `approve` or `changes`.

- [ ] **Step 4: Tests pass**

- [ ] **Step 5: Commit** `feat(cli): add tb review approve and changes`

---

### Task 3: Inspector + HTTP/desktop

**Files:**
- Modify: `crates/api/src/dto.rs` — `ReviewBody { action, text }`
- Modify: `crates/api/src/routes.rs` — `POST /api/v1/tasks/:display_id/review`
- Modify: `crates/desktop-commands/src/commands.rs` — `review_inner`
- Modify: `apps/desktop/src-tauri/src/commands.rs` — `review` command
- Modify: `packages/types/src/index.ts` — `export type ReviewAction = "approve" | "changes"`
- Modify: `packages/client` Transport `review(displayId, { action, text })`
- Modify: `packages/ui` Inspector buttons only when `task.column === "in-review"`

**Interfaces:**
- `onReview?: (action: "approve" | "changes", text: string) => void`
- Buttons named `Approve` and `Request changes`
- Review text input placeholder `Review comment`
- Fake transport moves column and appends comment

- [ ] **Step 1: Failing Inspector tests**

```tsx
describe("Inspector review", () => {
  it("approves an in-review card", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Ship", column: "in-review" });
    task.column = "in-review";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Ship"));
    await userEvent.type(screen.getByPlaceholderText("Review comment"), "lgtm");
    await userEvent.click(screen.getByRole("button", { name: "Approve" }));
    await waitFor(() =>
      expect(transport.review).toHaveBeenCalledWith("TASK-1", { action: "approve", text: "lgtm" }),
    );
  });

  it("hides review verbs when the card is not in review", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Todo", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Todo"));
    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Request changes" })).toBeNull();
  });
});
```

- [ ] **Step 2: Fail** — missing buttons / `transport.review`.

- [ ] **Step 3: Implement** HTTP `ReviewBody.action` snake/camel `approve`|`changes`. Desktop `review_inner`. Inspector section after comments.

- [ ] **Step 4:** `pnpm --filter @taskboard/ui test` + `pnpm --filter @taskboard/client test`

- [ ] **Step 5: Commit** `feat(ui): approve or request changes from the inspector`

---

## Self-review

**1. Spec coverage:** approve comment+done; changes comment+in-progress without run; unchecked items do not block; Inspector only for In Review; no fifth column; snake_case JSON.

**2. Placeholder scan:** confirm `run list --task` flag against `cli_json.rs` before Task 2.

**3. Types:** `ReviewAction`, `ReviewTask`, Transport `review`.
