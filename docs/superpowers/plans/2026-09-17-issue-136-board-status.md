# Thin board surfaces for status, occupancy, and spawn Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put `tb status` counts, occupancy collisions, and `task spawn` on the human board, with matching HTTP and desktop commands. Agents stay on CLI/MCP.

**Architecture:** Reuse `App::status`, `App::occupancy`, and `App::task_spawn`. HTTP `GET /api/v1/status`, `GET /api/v1/occupancy`, `POST /api/v1/tasks/{id}/spawn` and desktop commands wrap those. The board loads status + occupancy through Transport. A Status strip next to Inbox shows the same counts; click selects the first `*_head` card. One occupancy line when any group has 2+ open runs. Inspector Spawn creates a same-project todo child and parent `blocked-by` without moving columns.

**Tech Stack:** Axum, desktop-commands, `packages/types`, React, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- HTTP camelCase; CLI JSON stays snake_case
- HTTP/desktop are human surfaces, not agent surfaces
- Do not change CLI `status` / `occupancy` / `spawn` semantics
- No command palette
- No activity `--follow`
- No fifth column
- No OS notifications
- No auto-move of parent or child
- Occupancy line only when the same path has two or more open runs

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-136-board-status.md`
- Modify: `crates/api/src/dto.rs`, `crates/api/src/routes.rs`, `crates/api/tests/routes.rs`
- Modify: `crates/desktop-commands/src/commands.rs`, `crates/desktop-commands/tests/commands.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`, `apps/desktop/src-tauri/src/lib.rs`
- Modify: `packages/types/src/index.ts`
- Modify: `packages/client/src/transport.ts`, `http.ts`, `tauri.ts`, tests
- Modify: `packages/ui/src/fakeTransport.ts`, `TaskboardApp.tsx`, `Inspector.tsx`
- Create: `packages/ui/src/StatusStrip.tsx`, `StatusStrip.module.css`, `StatusStrip.test.tsx`
- Modify: `packages/ui/src/Inspector.test.tsx`
- Modify: `README.md` — one sentence that these HTTP routes are human-only

---

### Task 1: HTTP status / occupancy / spawn

**Files:**
- Modify: `crates/api/tests/routes.rs`
- Modify: `crates/api/src/dto.rs`
- Modify: `crates/api/src/routes.rs`

**Interfaces:**
- Consumes: `App::status(Option<String>)`, `App::occupancy(OccupancyQuery)`, `App::task_spawn(actor, TaskSpawn { parent_display_id, titles })`
- Produces:
  - `GET /api/v1/status?project=` → `{ ok, entity: BoardStatusDto }` camelCase
  - `GET /api/v1/occupancy?path=` → `{ ok, entities: OccupancyGroupDto[] }`
  - `POST /api/v1/tasks/{id}/spawn` body `{ titles: string[] }` → `{ ok, entities: TaskDetailDto[] }`
  - `SpawnBody { titles: Vec<String> }`

- [ ] **Step 1: Write failing HTTP tests**

Append to `crates/api/tests/routes.rs`:

```rust
#[tokio::test]
async fn get_status_returns_inbox_and_ready_counts() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    let res = client
        .get(format!("{}/api/v1/status", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["entity"]["inbox"]["total"], 0);
    assert_eq!(v["entity"]["ready"], 1);
    assert_eq!(v["entity"]["readyHead"][0]["displayId"], "TASK-1");
    assert_eq!(v["entity"]["openRuns"], 0);
    assert_eq!(v["entity"]["inReview"], 0);
    assert_eq!(v["entity"]["blocked"], 0);
}

#[tokio::test]
async fn get_occupancy_lists_collisions_only() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    client
        .post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .json(&json!({ "title": "Second" }))
        .send()
        .await
        .unwrap();
    for id in ["TASK-1", "TASK-2"] {
        client
            .patch(format!("{}/api/v1/tasks/{id}", s.base))
            .json(&json!({ "worktreePath": "/tmp/shared" }))
            .send()
            .await
            .unwrap();
        client
            .post(format!("{}/api/v1/tasks/{id}/runs", s.base))
            .json(&json!({ "agent": "cursor" }))
            .send()
            .await
            .unwrap();
    }
    let res = client
        .get(format!("{}/api/v1/occupancy", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["entities"][0]["worktreePath"], "/tmp/shared");
    assert_eq!(v["entities"][0]["runs"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn post_spawn_creates_todo_child_and_blocks_parent() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    let shown = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let parent_column = shown["entity"]["column"].clone();
    let res = client
        .post(format!("{}/api/v1/tasks/TASK-1/spawn", s.base))
        .json(&json!({ "titles": ["Child split"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["entities"][0]["displayId"], "TASK-2");
    assert_eq!(v["entities"][0]["title"], "Child split");
    assert_eq!(v["entities"][0]["column"], "todo");
    let parent = client
        .get(format!("{}/api/v1/tasks/TASK-1", s.base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(parent["entity"]["column"], parent_column);
    assert_eq!(parent["entity"]["blockedBy"][0], "TASK-2");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test routes get_status_returns get_occupancy_lists post_spawn_creates -- --nocapture`

Expected: FAIL — 404, routes missing.

- [ ] **Step 3: Add DTOs and routes**

In `crates/api/src/dto.rs` (use `json_keys_to_camel` on `BoardStatus` / `OccupancyGroup` which already serialize snake_case, or explicit camelCase DTOs). Prefer explicit:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineDto {
    pub display_id: String,
    pub status: String,
    pub agent: Option<String>,
    pub detail: String,
}

impl From<StatusLine> for StatusLineDto {
    fn from(line: StatusLine) -> Self {
        Self {
            display_id: line.display_id,
            status: line.status,
            agent: line.agent,
            detail: line.detail,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxCountsDto {
    pub total: usize,
    pub waiting: usize,
    pub failed: usize,
    pub stale: usize,
    pub review: usize,
    pub urgent: usize,
}

impl From<InboxCounts> for InboxCountsDto { /* field-wise copy */ }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardStatusDto {
    pub inbox: InboxCountsDto,
    pub inbox_head: Vec<StatusLineDto>,
    pub open_runs: usize,
    pub open_run_head: Vec<StatusLineDto>,
    pub stale: usize,
    pub stale_head: Vec<StatusLineDto>,
    pub ready: usize,
    pub ready_head: Vec<StatusLineDto>,
    pub in_review: usize,
    pub in_review_head: Vec<StatusLineDto>,
    pub blocked: usize,
    pub blocked_head: Vec<StatusLineDto>,
}

impl From<BoardStatus> for BoardStatusDto { /* map heads with StatusLineDto::from */ }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupancyRunDto {
    pub run_display_id: String,
    pub task_display_id: String,
    pub status: String,
    pub agent: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupancyGroupDto {
    pub worktree_path: String,
    pub runs: Vec<OccupancyRunDto>,
}

impl From<OccupancyGroup> for OccupancyGroupDto { /* map runs */ }

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OccupancyQueryDto {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnBody {
    pub titles: Vec<String>,
}
```

Import `BoardStatus`, `InboxCounts`, `OccupancyGroup`, `OccupancyRun`, `StatusLine` from `taskboard_application`.

Router additions:

```rust
.route("/api/v1/status", get(get_status))
.route("/api/v1/occupancy", get(get_occupancy))
.route("/api/v1/tasks/:display_id/spawn", post(spawn_task))
```

Handlers (read for GET, mutation for spawn):

```rust
async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<StatusQuery>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let snap = state.app.status(query.project).await.map_err(app_error)?;
    Ok(entity(BoardStatusDto::from(snap), 0))
}

async fn get_occupancy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<OccupancyQueryDto>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let groups = state
        .app
        .occupancy(OccupancyQuery { path: query.path })
        .await
        .map_err(app_error)?;
    Ok(entities(
        groups.into_iter().map(OccupancyGroupDto::from).collect::<Vec<_>>(),
    ))
}

async fn spawn_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<SpawnBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let children = state
        .app
        .task_spawn(
            &web_actor(),
            TaskSpawn {
                parent_display_id: display_id,
                titles: body.titles,
            },
        )
        .await
        .map_err(app_error)?;
    Ok(entities(
        children.into_iter().map(TaskDetailDto::from).collect::<Vec<_>>(),
    ))
}
```

Import `OccupancyQuery`, `TaskSpawn`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api --test routes get_status_returns get_occupancy_lists post_spawn_creates`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api
git commit -m "feat(api): add human status occupancy and spawn routes"
```

---

### Task 2: Desktop status / occupancy / spawn

**Files:**
- Modify: `crates/desktop-commands/tests/commands.rs`
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  - `status_inner(app, project: Option<String>) -> Result<BoardStatus, AppErrorDto>`
  - `occupancy_inner(app, path: Option<String>) -> Result<Vec<OccupancyGroup>, AppErrorDto>`
  - `task_spawn_inner(app, display_id, titles) -> Result<Vec<TaskDetail>, AppErrorDto>`

- [ ] **Step 1: Write failing desktop tests**

```rust
use taskboard_application::{OccupancyQuery, TaskUpdate};
use taskboard_desktop_commands::{
    occupancy_inner, run_start_inner, status_inner, task_spawn_inner, task_update_inner,
};

#[tokio::test]
async fn status_command_counts_ready_card() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None).await.unwrap();
    task_create_inner(&app, "renai-sim".into(), "Fix login".into(), None, None)
        .await
        .unwrap();
    let snap = status_inner(&app, None).await.unwrap();
    assert_eq!(snap.ready, 1);
    assert_eq!(snap.ready_head[0].display_id, "TASK-1");
}

#[tokio::test]
async fn occupancy_command_lists_collision() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None).await.unwrap();
    task_create_inner(&app, "renai-sim".into(), "A".into(), None, None).await.unwrap();
    task_create_inner(&app, "renai-sim".into(), "B".into(), None, None).await.unwrap();
    for id in ["TASK-1", "TASK-2"] {
        task_update_inner(
            &app,
            TaskPatchArgs {
                display_id: id.into(),
                title: None,
                note_markdown: None,
                urgent: None,
                column: None,
                before_display_id: None,
                worktree_path: Some("/tmp/shared".into()),
                branch: None,
                revision: None,
            },
        )
        .await
        .unwrap();
        run_start_inner(&app, id.into(), "cursor".into(), None).await.unwrap();
    }
    let groups = occupancy_inner(&app, None).await.unwrap();
    assert_eq!(groups[0].worktree_path, "/tmp/shared");
    assert_eq!(groups[0].runs.len(), 2);
}

#[tokio::test]
async fn spawn_command_creates_child_without_moving_parent() {
    let app = test_app().await;
    project_add_inner(&app, "Renai Sim".into(), None, None).await.unwrap();
    let parent = task_create_inner(&app, "renai-sim".into(), "Parent".into(), None, None)
        .await
        .unwrap();
    let kids = task_spawn_inner(&app, "TASK-1".into(), vec!["Child".into()])
        .await
        .unwrap();
    assert_eq!(kids[0].title, "Child");
    assert_eq!(kids[0].column, taskboard_core::Column::Todo);
    let shown = task_show_inner(&app, "TASK-1".into()).await.unwrap();
    assert_eq!(shown.column, parent.column);
    assert_eq!(shown.blocked_by, vec!["TASK-2".to_string()]);
}
```

Need `run_start_inner` / `task_show_inner` — they already exist. Export if needed.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-desktop-commands --test commands status_command occupancy_command spawn_command`

Expected: FAIL compile — missing inners.

- [ ] **Step 3: Implement inners and Tauri commands**

```rust
pub async fn status_inner(app: &App, project: Option<String>) -> Result<BoardStatus, AppErrorDto> {
    app.status(project).await.map_err(Into::into)
}

pub async fn occupancy_inner(app: &App, path: Option<String>) -> Result<Vec<OccupancyGroup>, AppErrorDto> {
    app.occupancy(OccupancyQuery { path }).await.map_err(Into::into)
}

pub async fn task_spawn_inner(
    app: &App,
    display_id: String,
    titles: Vec<String>,
) -> Result<Vec<TaskDetail>, AppErrorDto> {
    app.task_spawn(
        &actor(),
        TaskSpawn {
            parent_display_id: display_id,
            titles,
        },
    )
    .await
    .map_err(Into::into)
}
```

Tauri (`rename_all = "snake_case"`):

```rust
pub async fn status(state: ..., project: Option<String>) -> Result<BoardStatusDto, AppErrorDto>
pub async fn occupancy(state: ..., path: Option<String>) -> Result<Vec<OccupancyGroupDto>, AppErrorDto>
pub async fn task_spawn(state: ..., display_id: String, titles: Vec<String>) -> Result<Vec<TaskDetailDto>, AppErrorDto>
```

Register in `generate_handler!`. If `BoardStatusDto` lives in api crate, either move shared DTOs to desktop-commands (already has many DTOs) **or** serialize via `json_keys_to_camel` / duplicate thin DTOs in `crates/desktop-commands/src/dto.rs`. Put the same camelCase DTOs in desktop-commands `dto.rs` so Tauri returns camelCase to the UI (HttpTransport already expects camelCase). Mirror `BoardStatusDto` / `OccupancyGroupDto` there and `From` impls.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-desktop-commands --test commands status_command occupancy_command spawn_command`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/desktop-commands apps/desktop/src-tauri
git commit -m "feat(desktop): expose status occupancy and task_spawn"
```

---

### Task 3: Types + Transport + fakeTransport

**Files:**
- Modify: `packages/types/src/index.ts`
- Modify: `packages/client/src/transport.ts`, `http.ts`, `tauri.ts`, `http.test.ts`, `tauri.test.ts`
- Modify: `packages/ui/src/fakeTransport.ts`

**Interfaces:**
- Produces:
  - `status(project?: string): Promise<BoardStatus>`
  - `occupancy(path?: string): Promise<OccupancyGroup[]>`
  - `taskSpawn(displayId: string, titles: string[]): Promise<TaskDetail[]>`

- [ ] **Step 1: Write failing client tests**

`http.test.ts`:

```ts
  it("loads status occupancy and spawn as human routes", async () => {
    const fetches: Request[] = [];
    const fetchImpl: typeof fetch = async (input, init) => {
      fetches.push(new Request(input, init));
      return new Response(
        JSON.stringify({
          ok: true,
          entity: { ready: 1, inbox: { total: 0 } },
          entities: [{ worktreePath: "/tmp/shared" }, { displayId: "TASK-2" }],
          revision: 0,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    };
    const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
    await t.status("renai-sim");
    assert.equal(new URL(fetches[0].url).pathname, "/api/v1/status");
    assert.equal(new URL(fetches[0].url).searchParams.get("project"), "renai-sim");
    await t.occupancy();
    assert.equal(new URL(fetches[1].url).pathname, "/api/v1/occupancy");
    await t.taskSpawn("TASK-1", ["Child"]);
    assert.equal(fetches[2].method, "POST");
    assert.equal(new URL(fetches[2].url).pathname, "/api/v1/tasks/TASK-1/spawn");
    assert.deepEqual(await fetches[2].json(), { titles: ["Child"] });
  });
```

`tauri.test.ts`: add `await t.status("a"); await t.occupancy(); await t.taskSpawn("TASK-1", ["Child"]);` and extend the command list + args asserts.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/client test`

Expected: FAIL — methods missing.

- [ ] **Step 3: Add types and methods**

```ts
export interface StatusLine {
  displayId: string;
  status: string;
  agent: string | null;
  detail: string;
}
export interface InboxCounts {
  total: number;
  waiting: number;
  failed: number;
  stale: number;
  review: number;
  urgent: number;
}
export interface BoardStatus {
  inbox: InboxCounts;
  inboxHead: StatusLine[];
  openRuns: number;
  openRunHead: StatusLine[];
  stale: number;
  staleHead: StatusLine[];
  ready: number;
  readyHead: StatusLine[];
  inReview: number;
  inReviewHead: StatusLine[];
  blocked: number;
  blockedHead: StatusLine[];
}
export interface OccupancyRun {
  runDisplayId: string;
  taskDisplayId: string;
  status: string;
  agent: string;
}
export interface OccupancyGroup {
  worktreePath: string;
  runs: OccupancyRun[];
}
```

HttpTransport:

```ts
  status(project?: string): Promise<BoardStatus> {
    const q = project ? `?project=${enc(project)}` : "";
    return this.request("GET", `/api/v1/status${q}`);
  }
  occupancy(path?: string): Promise<OccupancyGroup[]> {
    const q = path ? `?path=${enc(path)}` : "";
    return this.request("GET", `/api/v1/occupancy${q}`, { unwrap: "entities" });
  }
  taskSpawn(displayId: string, titles: string[]): Promise<TaskDetail[]> {
    return this.request("POST", `/api/v1/tasks/${enc(displayId)}/spawn`, {
      body: { titles },
      unwrap: "entities",
    });
  }
```

TauriTransport: `this.call("status", { project })`, `this.call("occupancy", { path })`, `this.call("task_spawn", { displayId, titles })`.

`fakeTransport` implements enough for UI tests:

- `status`: derive counts from tasks (inbox via existing `inbox()`, ready = idle/todo and not blocked, open = running/waiting, stale flag, in-review column, blockedBy.length)
- heads = first matching task as `{ displayId, status: displayStatus, agent: null, detail: title }`
- `occupancy`: group running/waiting tasks by worktreePath; keep groups with 2+ runs
- `taskSpawn`: `taskCreate` + `linkAdd` blocked_by on parent; do not change parent column

- [ ] **Step 4: Run client tests**

Run: `pnpm --filter @taskboard/client test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/types packages/client packages/ui/src/fakeTransport.ts
git commit -m "feat(client): transport status occupancy and taskSpawn"
```

---

### Task 4: Status strip, occupancy line, Inspector Spawn

**Files:**
- Create: `packages/ui/src/StatusStrip.tsx`, `StatusStrip.module.css`, `StatusStrip.test.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`, `Inspector.tsx`, `Inspector.test.tsx`
- Modify: `README.md`

**Interfaces:**
- Consumes: `transport.status`, `transport.occupancy`, `transport.taskSpawn`
- Produces: click → `selectCard(head[0].displayId)`; Spawn → children + parent blocked-by

- [ ] **Step 1: Write failing UI tests**

`StatusStrip.test.tsx`:

```tsx
it("shows tb status counts and focuses the first ready card", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Ready one", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByRole("button", { name: /Ready 1/ }));
  await waitFor(() => expect(transport.taskShow).toHaveBeenCalledWith("TASK-1"));
  expect(await screen.findByLabelText("Title")).toBeTruthy();
});

it("shows one occupancy line for a colliding worktree", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  const a = await transport.taskCreate(project.slug, { title: "A", column: "todo" });
  const b = await transport.taskCreate(project.slug, { title: "B", column: "todo" });
  a.worktreePath = "/tmp/shared";
  b.worktreePath = "/tmp/shared";
  a.displayStatus = "running";
  b.displayStatus = "running";
  render(<TaskboardApp transport={transport} />);
  expect(await screen.findByLabelText("Occupancy")).toHaveTextContent("/tmp/shared");
  expect(screen.getByLabelText("Occupancy").textContent).toMatch(/TASK-1/);
  expect(screen.getByLabelText("Occupancy").textContent).toMatch(/TASK-2/);
});
```

Inspector spawn test:

```tsx
it("spawns a todo child and blocks the parent without moving columns", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Parent", column: "in-progress" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Parent"));
  await userEvent.type(screen.getByPlaceholderText("Spawn title"), "Child");
  await userEvent.click(screen.getByRole("button", { name: "Spawn" }));
  await waitFor(() => expect(transport.taskSpawn).toHaveBeenCalledWith("TASK-1", ["Child"]));
  expect(await screen.findByText("Child")).toBeTruthy();
  await userEvent.click(await screen.findByText("Parent"));
  expect(await screen.findByRole("button", { name: "TASK-2" })).toBeTruthy();
  expect((screen.getByLabelText("Column") as HTMLSelectElement).value).toBe("in-progress");
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test -- src/StatusStrip.test.tsx src/Inspector.test.tsx`

Expected: FAIL — missing Ready / Occupancy / Spawn.

- [ ] **Step 3: Implement UI**

`StatusStrip` (`aria-label="Status"`): buttons `Inbox {n}`, `Open {n}`, `Stale {n}`, `Ready {n}`, `In review {n}`, `Blocked {n}` plus muted inbox breakdown (`waiting · failed · stale · review · urgent` when > 0). `onSelect(displayId)` with first head id; disable or no-op when count is 0.

Occupancy line (`aria-label="Occupancy"`) in the same strip when `groups.length > 0`: `{path} · {taskIds join ", "}`. Click first run’s `taskDisplayId`. Reuse InboxStrip fonts (`.toggle` / `.counts`).

`TaskboardApp`: load `status` + `occupancy` after project/inbox refresh (scope: selected project when inbox scope is `this`, else undefined). Pass `onWorkspace` already present. `onSpawn(title)` → `transport.taskSpawn(id, [title])` then `refreshTasks` + `taskShow` parent.

Inspector after Branch fields:

```tsx
        <div className={styles.linkAdd}>
          <input
            className={styles.linkInput}
            placeholder="Spawn title"
            value={spawnValue}
            onChange={(e) => setSpawnValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              const title = spawnValue.trim();
              if (!title) return;
              props.onSpawn?.(title);
              setSpawnValue("");
            }}
          />
          <button
            type="button"
            className={styles.linkButton}
            onClick={() => {
              const title = spawnValue.trim();
              if (!title) return;
              props.onSpawn?.(title);
              setSpawnValue("");
            }}
          >
            Spawn
          </button>
        </div>
```

Reset `spawnValue` with other locals on task change.

README after the agent/HTTP sentence:

```
`GET /api/v1/status`, `GET /api/v1/occupancy`, and `POST /api/v1/tasks/TASK-n/spawn` are human board surfaces (`tb serve` / desktop). Agents use CLI or `tb mcp`.
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```
pnpm --filter @taskboard/ui test
cargo test -p taskboard-application --test occupancy --test spawn --test status
cargo test -p taskboard-api --test routes
```

Expected: PASS. Occupancy math unchanged.

- [ ] **Step 5: Commit**

```bash
git add packages/ui README.md
git commit -m "feat(ui): status strip occupancy line and inspector spawn"
```

---

## Self-review

1. Spec coverage: status counts + click first card; occupancy collision line; inspector spawn without column move; HTTP + desktop; documented human-only; no palette / `--follow`.
2. Placeholder scan: none.
3. Types: camelCase `openRuns` / `inReview` / `worktreePath` / `titles` consistent across DTO, TS, HTTP tests.
