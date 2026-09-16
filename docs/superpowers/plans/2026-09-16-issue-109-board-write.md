# Board write comments / checks / blocked-by Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a person add comments, toggle/add checks, and add `blocked_by` from the inspector, through the same `App` verbs HTTP and desktop already share for other writes.

**Architecture:** Domain verbs already exist (`App::comment_add`, `check_add`, `check_toggle`, `link_add` with `LinkKind::BlockedBy`). This slice adds HTTP routes, desktop IPC, Transport methods, and inspector composers. Comment/check routes return the new `Comment` / `Check` entity; the UI then `taskShow` + `taskList` so the thread, checklist counts, and card face stay consistent. `blocked_by` reuses `POST /api/v1/tasks/:id/links` with `kind: "blocked_by"` and a separate inspector input from URL/path. Cycles stay `validation_error` field `blocked_by`.

**Tech Stack:** Existing Rust crates (`taskboard-api`, `taskboard-desktop-commands`), Axum, Tauri commands, `@taskboard/client` Transport, React inspector, Vitest, reqwest route tests.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers
- Comments must not overwrite `note_markdown`
- No markdown preview
- The app does not launch an agent
- URL/path link CRUD (#56) stays a separate input
- Cycles and self-edges stay `validation_error` with field `blocked_by`
- CLI `--json` stays snake_case; HTTP / desktop DTOs stay camelCase
- Actor is `web` / `local-web` on HTTP and `desktop` / `local-ui` on desktop
- Empty comment `body` and empty check `text` stay `validation_error` (existing `require_non_blank`)

## File map

- Modify: `crates/api/src/dto.rs` — `AddCommentBody`, `AddCheckBody`
- Modify: `crates/api/src/routes.rs` — `POST …/comments`, `POST …/checks`, `PATCH /api/v1/checks/:display_id`
- Modify: `crates/api/tests/routes.rs` — comment / check / blocked-by / cycle
- Modify: `crates/desktop-commands/src/commands.rs` — `comment_add_inner`, `check_add_inner`, `check_toggle_inner`
- Modify: `crates/desktop-commands/tests/commands.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs` — IPC wrappers
- Modify: `apps/desktop/src-tauri/src/lib.rs` — register commands
- Modify: `packages/types/src/index.ts` — `LinkKind` includes `blocked_by`
- Modify: `packages/client/src/transport.ts` — `commentAdd`, `checkAdd`, `checkToggle`; `linkAdd` kind union
- Modify: `packages/client/src/http.ts`, `packages/client/src/http.test.ts`
- Modify: `packages/client/src/tauri.ts`, `packages/client/src/tauri.test.ts`
- Modify: `packages/ui/src/fakeTransport.ts`
- Modify: `packages/ui/src/Inspector.tsx`, `packages/ui/src/Inspector.module.css`, `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`

User already chose sequential inline execution.

---

### Task 1: HTTP comment / check / blocked-by write routes

**Files:**
- Modify: `crates/api/src/dto.rs`
- Modify: `crates/api/src/routes.rs`
- Modify: `crates/api/tests/routes.rs`

**Interfaces:**
- Consumes: `App::comment_add`, `App::check_add`, `App::check_toggle`, `App::link_add`, `web_actor()`, `require_mutation`
- Produces:
  - `POST /api/v1/tasks/:display_id/comments` body `{ "body": string }` → `{ ok, entity: CommentDto, revision }`
  - `POST /api/v1/tasks/:display_id/checks` body `{ "text": string }` → `{ ok, entity: CheckDto, revision }`
  - `PATCH /api/v1/checks/:display_id` empty/ignored body → toggles and returns `CheckDto`
  - Existing `POST /api/v1/tasks/:display_id/links` with `kind: "blocked_by"` (already accepted by `LinkKind`) returns `TaskDetailDto`
  - Comment/check `revision` is `0` (those entities have no card revision)
  - Cycle → HTTP 400 `{ error: { code: "validation_error", field: "blocked_by", message: "blocked-by cycle" } }`
  - Blank body/text → HTTP 400 `validation_error`

- [ ] **Step 1: Write the failing HTTP tests**

Append to `crates/api/tests/routes.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api comment_add_does_not_change_note check_add_and_toggle blocked_by_link_and_cycle -- --nocapture`

Expected: FAIL — 404 (no `/comments` or `/checks` route). `blocked_by_link_and_cycle` may already pass the add path; the cycle assertion is the new guarantee. If add already works, keep the test.

- [ ] **Step 3: Write minimal implementation**

In `crates/api/src/dto.rs` after `AddLinkBody`:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCommentBody {
    pub body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCheckBody {
    pub text: String,
}
```

In `crates/api/src/routes.rs`:

Import `CheckAdd`, `CommentAdd`, `AddCheckBody`, `AddCommentBody`, `CheckDto`, `CommentDto`.

Add routes:

```rust
        .route(
            "/api/v1/tasks/:display_id/comments",
            post(add_comment),
        )
        .route("/api/v1/tasks/:display_id/checks", post(add_check))
        .route("/api/v1/checks/:display_id", patch(toggle_check))
```

Handlers (place after `add_link`):

```rust
async fn add_comment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<AddCommentBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let comment = state
        .app
        .comment_add(
            &web_actor(),
            CommentAdd {
                task_display_id: display_id,
                body: body.body,
            },
        )
        .await
        .map_err(app_error)?;
    Ok(entity(CommentDto::from(comment), 0))
}

async fn add_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<AddCheckBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let check = state
        .app
        .check_add(
            &web_actor(),
            CheckAdd {
                task_display_id: display_id,
                text: body.text,
            },
        )
        .await
        .map_err(app_error)?;
    Ok(entity(CheckDto::from(check), 0))
}

async fn toggle_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let check = state
        .app
        .check_toggle(&web_actor(), &display_id)
        .await
        .map_err(app_error)?;
    Ok(entity(CheckDto::from(check), 0))
}
```

`add_link` already forwards `body.kind`; no change unless a compile error appears.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api comment_add_does_not_change_note check_add_and_toggle blocked_by_link_and_cycle -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/dto.rs crates/api/src/routes.rs crates/api/tests/routes.rs docs/superpowers/plans/2026-09-16-issue-109-board-write.md
git commit -m "feat(api): write comments, checks, and blocked-by"
```

---

### Task 2: Desktop command inners

**Files:**
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `crates/desktop-commands/tests/commands.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `App::comment_add`, `App::check_add`, `App::check_toggle`, `desktop_actor()`
- Produces:
  - `comment_add_inner(app, display_id: String, body: String) -> Result<Comment, AppErrorDto>`
  - `check_add_inner(app, display_id: String, text: String) -> Result<Check, AppErrorDto>`
  - `check_toggle_inner(app, display_id: String) -> Result<Check, AppErrorDto>` (`CHECK-n`)
  - Tauri commands `comment_add`, `check_add`, `check_toggle` with `rename_all = "snake_case"`
  - Actor is desktop `local-ui`

- [ ] **Step 1: Write the failing desktop tests**

In `crates/desktop-commands/tests/commands.rs` add imports for the new inners and `link_add_inner`, `task_create_inner`. Then:

```rust
use taskboard_core::LinkKind;
use taskboard_desktop_commands::{
    check_add_inner, check_toggle_inner, comment_add_inner, link_add_inner, project_add_inner,
    sync_inner, task_create_inner, AppErrorDto,
};

#[tokio::test]
async fn comment_add_command_uses_desktop_actor_and_keeps_note() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    let task = task_create_inner(&app, "renai-sim".into(), "Talk".into(), None, None)
        .await
        .unwrap();
    assert_eq!(task.note_markdown, "");
    let comment = comment_add_inner(&app, "TASK-1".into(), "use TDD".into())
        .await
        .unwrap();
    assert_eq!(comment.body, "use TDD");
    assert_eq!(comment.actor_label, "local-ui");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "");
    assert_eq!(shown.comments[0].body, "use TDD");
}

#[tokio::test]
async fn check_add_and_toggle_commands() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "DoD".into(), None, None)
        .await
        .unwrap();
    let check = check_add_inner(&app, "TASK-1".into(), "Write tests".into())
        .await
        .unwrap();
    assert_eq!(check.display_id, "CHECK-1");
    assert!(!check.done);
    let toggled = check_toggle_inner(&app, "CHECK-1".into()).await.unwrap();
    assert!(toggled.done);
}

#[tokio::test]
async fn link_add_blocked_by_cycle_is_validation_error() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "A".into(), None, None)
        .await
        .unwrap();
    task_create_inner(&app, "renai-sim".into(), "B".into(), None, None)
        .await
        .unwrap();
    link_add_inner(
        &app,
        "TASK-2".into(),
        LinkKind::BlockedBy,
        "TASK-1".into(),
        None,
    )
    .await
    .unwrap();
    let err = link_add_inner(
        &app,
        "TASK-1".into(),
        LinkKind::BlockedBy,
        "TASK-2".into(),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code, "validation_error");
    assert_eq!(err.field.as_deref(), Some("blocked_by"));
}
```

`AppErrorDto` field names must match `crates/desktop-commands/src/error.rs` (`code`, `field`). If `field` is `Option<String>`, use `.as_deref()`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-desktop-commands comment_add_command check_add_and_toggle_commands link_add_blocked_by -- --nocapture`

Expected: FAIL — `comment_add_inner` / `check_add_inner` / `check_toggle_inner` not found.

- [ ] **Step 3: Write minimal implementation**

In `crates/desktop-commands/src/commands.rs` add imports `CheckAdd`, `CommentAdd`, `Check`, `Comment` and:

```rust
pub async fn comment_add_inner(
    app: &App,
    display_id: String,
    body: String,
) -> Result<Comment, AppErrorDto> {
    app.comment_add(
        &actor(),
        CommentAdd {
            task_display_id: display_id,
            body,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn check_add_inner(
    app: &App,
    display_id: String,
    text: String,
) -> Result<Check, AppErrorDto> {
    app.check_add(
        &actor(),
        CheckAdd {
            task_display_id: display_id,
            text,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn check_toggle_inner(app: &App, display_id: String) -> Result<Check, AppErrorDto> {
    app.check_toggle(&actor(), &display_id)
        .await
        .map_err(Into::into)
}
```

In `apps/desktop/src-tauri/src/commands.rs` import the new inners and `CheckDto`, `CommentDto`. Add:

```rust
#[tauri::command(rename_all = "snake_case")]
pub async fn comment_add(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    body: String,
) -> Result<CommentDto, AppErrorDto> {
    let app = state.app.lock().await;
    comment_add_inner(&app, display_id, body)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_add(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    text: String,
) -> Result<CheckDto, AppErrorDto> {
    let app = state.app.lock().await;
    check_add_inner(&app, display_id, text).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_toggle(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
) -> Result<CheckDto, AppErrorDto> {
    let app = state.app.lock().await;
    check_toggle_inner(&app, display_id).await.map(Into::into)
}
```

Register `commands::comment_add`, `commands::check_add`, `commands::check_toggle` in `apps/desktop/src-tauri/src/lib.rs` `generate_handler!`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-desktop-commands -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/desktop-commands/src/commands.rs crates/desktop-commands/tests/commands.rs apps/desktop/src-tauri/src/commands.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): comment, check, and blocked-by writes"
```

---

### Task 3: Transport methods

**Files:**
- Modify: `packages/types/src/index.ts`
- Modify: `packages/client/src/transport.ts`
- Modify: `packages/client/src/http.ts`
- Modify: `packages/client/src/http.test.ts`
- Modify: `packages/client/src/tauri.ts`
- Modify: `packages/client/src/tauri.test.ts`
- Modify: `packages/ui/src/fakeTransport.ts`

**Interfaces:**
- Consumes: HTTP routes and Tauri commands from Tasks 1–2
- Produces:
  - `LinkKind = "url" | "path" | "blocked_by"`
  - `Transport.linkAdd(displayId, { kind: LinkKind, value })`
  - `Transport.commentAdd(displayId, body): Promise<Comment>`
  - `Transport.checkAdd(displayId, text): Promise<Check>`
  - `Transport.checkToggle(displayId): Promise<Check>` (`CHECK-n`)
  - HTTP: `POST /api/v1/tasks/:id/comments` `{ body }`, `POST …/checks` `{ text }`, `PATCH /api/v1/checks/:id`
  - Tauri: `comment_add` `{ display_id, body }`, `check_add` `{ display_id, text }`, `check_toggle` `{ display_id }`

- [ ] **Step 1: Write the failing client tests**

In `packages/client/src/http.test.ts`:

```ts
  it("posts comments and checks and patches toggle", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({ ok: true, entity: { displayId: "CHECK-1", body: "hi", done: true }, revision: 0 }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.commentAdd("TASK-1", "hi");
    await t.checkAdd("TASK-1", "Write tests");
    await t.checkToggle("CHECK-1");
    await t.linkAdd("TASK-1", { kind: "blocked_by", value: "TASK-2" });
    assert.equal(fetches[0].method, "POST");
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/tasks/TASK-1/comments");
    assert.deepEqual(await fetches[0].json(), { body: "hi" });
    assert.equal(new URL(fetches[1].url).pathname, "/api/v1/tasks/TASK-1/checks");
    assert.deepEqual(await fetches[1].json(), { text: "Write tests" });
    assert.equal(fetches[2].method, "PATCH");
    assert.equal(new URL(fetches[2].url).pathname, "/api/v1/checks/CHECK-1");
    assert.equal(new URL(fetches[3].url).pathname, "/api/v1/tasks/TASK-1/links");
    assert.deepEqual(await fetches[3].json(), { kind: "blocked_by", value: "TASK-2" });
  });
```

In `packages/client/src/tauri.test.ts` `maps every Transport method`, after `linkRemove` add:

```ts
    await t.commentAdd("TASK-1", "hi");
    await t.checkAdd("TASK-1", "Write tests");
    await t.checkToggle("CHECK-1");
```

Insert `"comment_add"`, `"check_add"`, `"check_toggle"` into the expected `cmds` list immediately after `"link_remove"`. Update later `calls[N]` indexes: each insert shifts subsequent indexes by +3. After this change, `runStart` is `calls[23]`, `runPatch` `24`, `inbox` `25`, `trashList` `26`, `undo` `27`, `sync` `28`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/client test`

Expected: FAIL — `commentAdd` is not a function / Transport missing methods.

- [ ] **Step 3: Write minimal implementation**

`packages/types/src/index.ts`:

```ts
export type LinkKind = "url" | "path" | "blocked_by";
```

`packages/client/src/transport.ts` — import `Check`, `Comment`, `LinkKind` and add:

```ts
  linkAdd(displayId: string, input: { kind: LinkKind; value: string }): Promise<TaskDetail>;
  commentAdd(displayId: string, body: string): Promise<Comment>;
  checkAdd(displayId: string, text: string): Promise<Check>;
  checkToggle(displayId: string): Promise<Check>;
```

`packages/client/src/http.ts` — same imports; replace `linkAdd` kind type; add:

```ts
  commentAdd(displayId: string, body: string): Promise<Comment> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/comments`, { body: { body } });
  }

  checkAdd(displayId: string, text: string): Promise<Check> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/checks`, { body: { text } });
  }

  checkToggle(displayId: string): Promise<Check> {
    return this.request("PATCH", `/api/v1/checks/${enc(displayId)}`, { body: {} });
  }
```

`packages/client/src/tauri.ts`:

```ts
  commentAdd(displayId: string, body: string): Promise<Comment> {
    return this.call("comment_add", { displayId, body });
  }

  checkAdd(displayId: string, text: string): Promise<Check> {
    return this.call("check_add", { displayId, text });
  }

  checkToggle(displayId: string): Promise<Check> {
    return this.call("check_toggle", { displayId });
  }
```

`packages/ui/src/fakeTransport.ts` — implement the three methods and accept `blocked_by` on `linkAdd`:

```ts
    async commentAdd(displayId, body) {
      const detail = details.get(displayId);
      if (!detail) throw new Error("not found");
      const comment = {
        id: `c${detail.comments.length + 1}`,
        taskId: detail.id,
        actorKind: "desktop" as const,
        actorLabel: "local-ui",
        body,
        createdAt: now(),
      };
      detail.comments.push(comment);
      return comment;
    },
    async checkAdd(displayId, text) {
      const detail = details.get(displayId);
      const task = tasks.find((t) => t.displayId === displayId);
      if (!detail || !task) throw new Error("not found");
      const check = {
        id: `k${detail.checks.length + 1}`,
        displayId: `CHECK-${detail.checks.length + 1}`,
        taskId: detail.id,
        text,
        done: false,
        sortOrder: detail.checks.length,
      };
      detail.checks.push(check);
      task.checklistTotal += 1;
      Object.assign(detail, task);
      return check;
    },
    async checkToggle(displayId) {
      for (const detail of details.values()) {
        const check = detail.checks.find((item) => item.displayId === displayId);
        if (!check) continue;
        check.done = !check.done;
        const task = tasks.find((t) => t.displayId === detail.displayId);
        if (task) {
          task.checklistDone = detail.checks.filter((item) => item.done).length;
          task.checklistTotal = detail.checks.length;
          Object.assign(detail, task);
        }
        return { ...check };
      }
      throw new Error("not found");
    },
```

In `linkAdd`, after pushing the link, if `input.kind === "blocked_by"`:

```ts
      const task = tasks.find((t) => t.displayId === displayId);
      if (task && !task.blockedBy.includes(input.value)) task.blockedBy.push(input.value);
      detail.blockedBy = task?.blockedBy ?? detail.blockedBy;
      const blocker = tasks.find((t) => t.displayId === input.value);
      if (blocker && !blocker.blocks.includes(displayId)) blocker.blocks.push(displayId);
      const blockerDetail = details.get(input.value);
      if (blockerDetail && blocker) Object.assign(blockerDetail, blocker);
```

If the blocker already has `blockedBy` containing `displayId`, throw:

```ts
      if (blocker?.blockedBy.includes(displayId) || task?.blockedBy.includes(input.value) === false && blocker?.blocks.includes(displayId)) {
        // simpler: detect reverse edge
      }
```

Use this exact cycle check before mutating:

```ts
      const task = tasks.find((t) => t.displayId === displayId);
      const blocker = tasks.find((t) => t.displayId === input.value);
      if (input.kind === "blocked_by" && blocker?.blockedBy.includes(displayId)) {
        const { TransportError } = await import("@taskboard/client");
        throw new TransportError({
          code: "validation_error",
          message: "blocked-by cycle",
          field: "blocked_by",
        });
      }
```

Prefer a static import of `TransportError` at the top of `fakeTransport.ts` instead of a dynamic import:

```ts
import { TransportError, type Transport } from "@taskboard/client";
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/client test`

Expected: PASS. Then `pnpm --filter @taskboard/ui test` — existing inspector tests must still pass (fakeTransport now implements the new methods).

- [ ] **Step 5: Commit**

```bash
git add packages/types/src/index.ts packages/client/src/transport.ts packages/client/src/http.ts packages/client/src/http.test.ts packages/client/src/tauri.ts packages/client/src/tauri.test.ts packages/ui/src/fakeTransport.ts
git commit -m "feat(client): transport writes for comments, checks, blocked-by"
```

---

### Task 4: Inspector composers

**Files:**
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Inspector.module.css`
- Modify: `packages/ui/src/Inspector.test.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`

**Interfaces:**
- Consumes: `transport.commentAdd`, `checkAdd`, `checkToggle`, `linkAdd`, `taskShow`, `taskList`
- Produces:
  - `onCommentAdd?: (body: string) => void`
  - `onCheckAdd?: (text: string) => void`
  - `onCheckToggle?: (displayId: string) => void`
  - `onBlockedByAdd?: (displayId: string) => void`
  - Comment composer: placeholder `Add a comment`, Enter submits (no Shift+Enter newline — use `<input>`)
  - Check composer: placeholder `Add a check`, button `Add check`
  - Checkboxes are not `readOnly`; `onChange` calls `onCheckToggle(check.displayId)`
  - Blocked-by composer: placeholder `TASK-n`, button `Add blocked-by`, separate from `https:// or /path`
  - After comment/check mutations: `taskShow` then `applyDetail` and `refreshTasks`
  - After blocked-by: `linkAdd({ kind: "blocked_by", value })` then `applyDetail` + `refreshTasks`; on error toast `errorMessage(err)`

- [ ] **Step 1: Write the failing inspector tests**

In `packages/ui/src/Inspector.test.tsx` replace the read-only checklist assertion's exclusive behavior by adding new describes (keep the existing note-preservation test):

```tsx
describe("Inspector comments", () => {
  it("shows a plain comment thread", async () => {
    // existing test stays
  });

  it("adds a comment on Enter without changing the note", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Talk", column: "todo" });
    task.noteMarkdown = "# Spec";
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    await userEvent.type(screen.getByPlaceholderText("Add a comment"), "use TDD{Enter}");
    await waitFor(() => expect(transport.commentAdd).toHaveBeenCalledWith("TASK-1", "use TDD"));
    expect(await screen.findByText(/local-ui · use TDD/)).toBeTruthy();
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe("# Spec");
  });
});

describe("Inspector checklists", () => {
  it("toggles a check and adds a row", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "DoD", column: "todo" });
    task.checks = [
      {
        id: "k1",
        displayId: "CHECK-1",
        taskId: task.id,
        text: "Write tests",
        done: false,
        sortOrder: 0,
      },
    ];
    task.checklistDone = 0;
    task.checklistTotal = 1;
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("DoD"));
    await userEvent.click(await screen.findByRole("checkbox", { name: "Write tests" }));
    await waitFor(() => expect(transport.checkToggle).toHaveBeenCalledWith("CHECK-1"));
    await userEvent.type(screen.getByPlaceholderText("Add a check"), "Ship it");
    await userEvent.click(screen.getByRole("button", { name: "Add check" }));
    await waitFor(() => expect(transport.checkAdd).toHaveBeenCalledWith("TASK-1", "Ship it"));
    expect(await screen.findByRole("checkbox", { name: "Ship it" })).toBeTruthy();
  });
});

describe("Inspector blocked-by", () => {
  it("adds a TASK-n blocked-by link from a separate input", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "Blocker", column: "todo" });
    await transport.taskCreate(project.slug, { title: "Blocked", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Blocked"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-1");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    await waitFor(() =>
      expect(transport.linkAdd).toHaveBeenCalledWith("TASK-2", {
        kind: "blocked_by",
        value: "TASK-1",
      }),
    );
    expect(await screen.findByRole("button", { name: "TASK-1" })).toBeTruthy();
    expect(screen.getByPlaceholderText("https:// or /path")).toBeTruthy();
  });

  it("toasts validation_error on a blocked-by cycle", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    await transport.taskCreate(project.slug, { title: "A", column: "todo" });
    await transport.taskCreate(project.slug, { title: "B", column: "todo" });
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("B"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-1");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    await waitFor(() => expect(transport.linkAdd).toHaveBeenCalled());
    await userEvent.click(await screen.findByText("A"));
    await userEvent.type(screen.getByPlaceholderText("TASK-n"), "TASK-2");
    await userEvent.click(screen.getByRole("button", { name: "Add blocked-by" }));
    expect(await screen.findByText("blocked-by cycle")).toBeTruthy();
  });
});
```

Update the existing comments test that searches `/2026-09-16T12:00:00Z · alice · use TDD/` — keep it. The new add test matches `/local-ui · use TDD/` because `fakeTransport` uses `now()` for `createdAt`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- Inspector.test.tsx`

Expected: FAIL — no `Add a comment` placeholder / `checkToggle` not called because checkbox is `readOnly`.

- [ ] **Step 3: Write minimal implementation**

`Inspector` props add:

```ts
  onCommentAdd?: (body: string) => void;
  onCheckAdd?: (text: string) => void;
  onCheckToggle?: (displayId: string) => void;
  onBlockedByAdd?: (displayId: string) => void;
```

State: `commentValue`, `checkValue`, `blockedByValue` — reset when `displayId` changes (same `lastId` effect as `linkValue`).

Checklist list item:

```tsx
                  <input
                    type="checkbox"
                    checked={check.done}
                    onChange={() => props.onCheckToggle?.(check.displayId)}
                  />
```

After the check `<ul>`:

```tsx
          <div className={styles.linkAdd}>
            <input
              className={styles.linkInput}
              placeholder="Add a check"
              value={checkValue}
              onChange={(e) => setCheckValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key !== "Enter") return;
                e.preventDefault();
                const text = checkValue.trim();
                if (!text) return;
                props.onCheckAdd?.(text);
                setCheckValue("");
              }}
            />
            <button
              type="button"
              className={styles.linkButton}
              onClick={() => {
                const text = checkValue.trim();
                if (!text) return;
                props.onCheckAdd?.(text);
                setCheckValue("");
              }}
            >
              Add check
            </button>
          </div>
```

After the comments `<ul>`:

```tsx
          <input
            className={styles.linkInput}
            placeholder="Add a comment"
            value={commentValue}
            onChange={(e) => setCommentValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              const body = commentValue.trim();
              if (!body) return;
              props.onCommentAdd?.(body);
              setCommentValue("");
            }}
          />
```

After the URL/path add row, a second row:

```tsx
          <div className={styles.linkAdd}>
            <input
              className={styles.linkInput}
              placeholder="TASK-n"
              value={blockedByValue}
              onChange={(e) => setBlockedByValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key !== "Enter") return;
                e.preventDefault();
                const value = blockedByValue.trim();
                if (!value) return;
                props.onBlockedByAdd?.(value);
                setBlockedByValue("");
              }}
            />
            <button
              type="button"
              className={styles.linkButton}
              onClick={() => {
                const value = blockedByValue.trim();
                if (!value) return;
                props.onBlockedByAdd?.(value);
                setBlockedByValue("");
              }}
            >
              Add blocked-by
            </button>
          </div>
```

In `TaskboardApp` Inspector props:

```tsx
        onCommentAdd={(body) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.commentAdd(id, body);
              applyDetail(await transport.taskShow(id));
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCheckAdd={(text) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.checkAdd(id, text);
              applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onCheckToggle={(checkId) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void (async () => {
            try {
              await transport.checkToggle(checkId);
              applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
        onBlockedByAdd={(value) => {
          const id = selectedIdRef.current;
          if (!id) return;
          void transport
            .linkAdd(id, { kind: "blocked_by", value })
            .then(async (updated) => {
              applyDetail(updated);
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            })
            .catch((err) => setToast({ message: errorMessage(err), error: true }));
        }}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test -- Inspector.test.tsx`

Expected: PASS

Then: `pnpm --filter @taskboard/ui test` and `pnpm --filter @taskboard/client test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/ui/src/Inspector.tsx packages/ui/src/Inspector.module.css packages/ui/src/Inspector.test.tsx packages/ui/src/TaskboardApp.tsx
git commit -m "feat(ui): write comments, checks, and blocked-by from inspector"
```

---

## Self-review

**1. Spec coverage**
- Inspector comment input + Enter → Task 4
- Toggle + add `CHECK-n` → Tasks 1, 2, 3, 4
- Separate `blocked_by` `TASK-n` input; cycles `validation_error` → Tasks 1, 2, 4
- HTTP + desktop write routes → Tasks 1, 2
- JSON snake_case CLI unchanged; HTTP camelCase unchanged → no CLI edits
- No markdown preview, no agent launch, no #56 URL/path merge → Task 4 keeps two inputs

**2. Placeholder scan:** none

**3. Type consistency:** `commentAdd(displayId, body)`, `checkAdd(displayId, text)`, `checkToggle(CHECK-n)`, `linkAdd(..., { kind: "blocked_by", value })` used the same way in HTTP, desktop, Transport, and UI.
