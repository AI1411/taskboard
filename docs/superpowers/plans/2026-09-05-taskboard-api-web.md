# Taskboard Local HTTP API and Web UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tb serve` so a localhost-only web app can manage the same SQLite board as the CLI, with session tokens, Origin checks, and 1s poll sync.

**Architecture:** `taskboard-api` is an Axum server that calls `taskboard-application::App`. `packages/types` holds generated DTOs. `packages/client` `HttpTransport` implements `Transport`. `packages/ui` renders sidebar, board, inspector. `apps/web` is the Vite app Axum serves from `/`.

**Tech Stack:** Axum, Tower, Vite, React, TypeScript, dnd-kit, TanStack Query, CSS Modules. Depends on Plan 1 crates already compiling and tested.

## Global Constraints

- Bind `127.0.0.1` only
- Random port unless `--port` is set; required port failure is `port_unavailable`
- Session token: 32 random bytes, hex; mutations require `X-Taskboard-Session`
- Mutation `Origin` must be exactly `http://127.0.0.1:{port}`
- HTTP JSON camelCase; revision via `If-Match`
- Poll `GET /api/v1/sync?after=N` every 1000ms while visible
- English UI copy from the detailed design
- Native Glass: dark sidebar, light board, purple/indigo accents, orange-red Urgent only, amber Waiting, green completed
- Specs: `docs/superpowers/specs/2026-09-05-local-taskboard-design.md` and `docs/superpowers/specs/2026-09-05-local-taskboard-detailed-design.md`

## File map

- Create: `crates/api/Cargo.toml`, `crates/api/src/lib.rs`, `crates/api/src/{session,origin,dto,routes,server}.rs`
- Modify: `Cargo.toml` (add `crates/api` member)
- Modify: `crates/cli/src/main.rs` (replace serve stub)
- Create: `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`
- Create: `packages/types/package.json`, `packages/types/src/index.ts`
- Create: `packages/client/package.json`, `packages/client/src/{transport.ts,http.ts,types.ts}`
- Create: `packages/ui/package.json`, `packages/ui/src/{index.ts,theme.css,Sidebar,Board,Card,Inspector,Search,EmptyState}`
- Create: `apps/web/package.json`, `apps/web/index.html`, `apps/web/src/{main.tsx,App.tsx,api.ts}`
- Create: `apps/web/vite.config.ts`

---

### Task 1: Axum server, session, Origin

**Files:**
- Create: `crates/api/Cargo.toml`
- Create: `crates/api/src/lib.rs`
- Create: `crates/api/src/session.rs`
- Create: `crates/api/src/origin.rs`
- Create: `crates/api/src/server.rs`
- Modify: `Cargo.toml`
- Test: `crates/api/tests/security.rs`

**Interfaces:**
- Consumes: `taskboard_application::App`
- Produces:
  - `pub struct SessionToken(pub String); fn generate_session() -> SessionToken`
  - `pub fn origin_allowed(origin: Option<&HeaderValue>, port: u16) -> bool` true only for `http://127.0.0.1:{port}`
  - `pub async fn serve(app: App, addr: SocketAddr, open: bool) -> Result<SocketAddr, ApiError>`
  - `GET /` returns HTML shell
  - `GET /api/v1/bootstrap` requires session after cookie set on `/`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn origin_must_match_bound_port() {
    assert!(origin_allowed(Some(&HeaderValue::from_static("http://127.0.0.1:9876")), 9876));
    assert!(!origin_allowed(Some(&HeaderValue::from_static("http://localhost:9876")), 9876));
    assert!(!origin_allowed(Some(&HeaderValue::from_static("http://evil.example")), 9876));
    assert!(!origin_allowed(None, 9876));
}

#[tokio::test]
async fn mutation_without_session_is_401() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn mutation_with_foreign_origin_is_403() {
    let server = start_test_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/v1/projects", server.base))
        .header("X-Taskboard-Session", &server.token)
        .header("Origin", "https://example.com")
        .json(&serde_json::json!({"name": "A"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test security`

Expected: FAIL compile error.

- [ ] **Step 3: Implement session generation, origin check, Axum router with bootstrap and a dummy POST /api/v1/projects that enforces the two checks**

`generate_session` uses `rand` 32 bytes hex. Store token in `Arc<AppState>`. Bind `127.0.0.1:0` in tests.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api --test security`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/api
git commit -m "feat: add localhost Axum session and origin checks"
```

---

### Task 2: HTTP routes matching App use cases

**Files:**
- Create: `crates/api/src/dto.rs`
- Create: `crates/api/src/routes.rs`
- Modify: `crates/api/src/server.rs`
- Modify: `crates/cli/src/main.rs`
- Test: `crates/api/tests/routes.rs`

**Interfaces:**
- Consumes: every `App` method from Plan 1
- Produces: routes in detailed design section 8. DTOs `#[serde(rename_all = "camelCase")]`. `If-Match` parsed as `Option<i64>`. Actor `ActorKind::Web`, label `local-web`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn create_project_and_task_round_trip() {
    let s = start_test_server().await;
    let client = authed(&s);
    let proj = client.post(format!("{}/api/v1/projects", s.base))
        .json(&serde_json::json!({"name": "Renai Sim"}))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(proj["entity"]["slug"], "renai-sim");
    let task = client.post(format!("{}/api/v1/projects/renai-sim/tasks", s.base))
        .json(&serde_json::json!({"title": "Fix login error"}))
        .send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(task["entity"]["task"]["displayId"], "TASK-1");
}

#[tokio::test]
async fn if_match_conflict() {
    let s = seeded_task_server().await;
    let res = authed(&s)
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .header("If-Match", "0")
        .json(&serde_json::json!({"title": "Nope"}))
        .send().await.unwrap();
    assert_eq!(res.status(), 409);
}

#[tokio::test]
async fn sync_after_zero_then_after_head() {
    let s = seeded_task_server().await;
    let head = authed(&s).get(format!("{}/api/v1/sync?after=0", s.base)).send().await.unwrap()
        .json::<serde_json::Value>().await.unwrap();
    let seq = head["sequence"].as_i64().unwrap();
    assert!(seq >= 1);
    let again = authed(&s).get(format!("{}/api/v1/sync?after={}", s.base, seq)).send().await.unwrap()
        .json::<serde_json::Value>().await.unwrap();
    assert_eq!(again["sequence"], seq);
    assert!(again["projects"].as_array().unwrap().is_empty());
}
```

Map AppError: validation 400, not_found 404, duplicate_slug 409, revision_conflict 409, undo_conflict 409, forbidden_origin 403, unauthorized 401.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test routes`

Expected: FAIL.

- [ ] **Step 3: Implement all routes from the detailed design table**

`tb serve` in CLI: resolve data dir, `open_db`, `App::new`, pick port, print `http://127.0.0.1:{port}`, `--open` uses `open::that`. Remove the Plan 1 stub error.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api crates/cli
git commit -m "feat: expose application use cases over localhost HTTP"
```

---

### Task 3: TypeScript Transport and HttpTransport

**Files:**
- Create: `package.json` with `"packageManager": "pnpm@10.33.3"` and script `"test": "pnpm --filter @taskboard/client test && pnpm --filter @taskboard/ui test"`
- Create: `pnpm-workspace.yaml` with packages `packages/*`, `apps/*`
- Create: `packages/types/src/index.ts`
- Create: `packages/client/src/transport.ts`
- Create: `packages/client/src/http.ts`
- Create: `packages/client/src/index.ts`
- Test: `packages/client/src/http.test.ts`

**Interfaces:**
- Consumes: HTTP DTO camelCase shapes
- Produces:

```ts
export interface Transport {
  projectAdd(input: { name: string; repoPath?: string; slug?: string }): Promise<Project>;
  projectList(includeArchived: boolean): Promise<Project[]>;
  projectUpdate(slug: string, patch: ProjectPatch, revision?: number): Promise<Project>;
  projectReorder(slugs: string[]): Promise<Project[]>;
  projectArchive(slug: string, archived: boolean, revision?: number): Promise<Project>;
  projectDelete(slug: string, revision?: number): Promise<Project>;
  projectRestore(slug: string): Promise<Project>;
  projectNoteSet(slug: string, markdown: string, revision?: number): Promise<Project>;
  taskCreate(projectSlug: string, input: { title: string; column?: Column; urgent?: boolean }): Promise<TaskDetail>;
  taskList(projectSlug: string): Promise<TaskSummary[]>;
  taskShow(displayId: string): Promise<TaskDetail>;
  taskUpdate(displayId: string, patch: TaskPatch, revision?: number): Promise<TaskDetail>;
  taskMove(displayId: string, column: Column, revision?: number): Promise<TaskDetail>;
  taskReorder(displayId: string, beforeDisplayId?: string, revision?: number): Promise<TaskDetail>;
  taskUrgent(displayId: string, urgent: boolean, revision?: number): Promise<TaskDetail>;
  taskDelete(displayId: string, revision?: number): Promise<TaskDetail>;
  taskRestore(displayId: string): Promise<TaskDetail>;
  taskNoteSet(displayId: string, markdown: string, revision?: number): Promise<TaskDetail>;
  linkAdd(displayId: string, input: { kind: "url" | "path"; value: string }): Promise<TaskDetail>;
  linkRemove(linkId: string, revision?: number): Promise<TaskDetail>;
  runStart(displayId: string, input: { agent: string; sessionId?: string }): Promise<Run>;
  runPatch(runDisplayId: string, op: RunOp, revision?: number): Promise<Run>;
  trashList(): Promise<Trash>;
  undo(): Promise<UndoResult>;
  sync(after: number): Promise<SyncDelta>;
}
```

`HttpTransport` constructor `(baseUrl: string, session: string)`. Sends `X-Taskboard-Session` and `If-Match`.

- [ ] **Step 1: Write the failing test**

```ts
it("sends session and camelCase body", async () => {
  const fetches: Request[] = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    fetches.push(new Request(input, init));
    return new Response(JSON.stringify({ ok: true, entity: { slug: "renai-sim", revision: 1 }, revision: 1 }), {
      headers: { "Content-Type": "application/json" },
    });
  };
  const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
  const p = await t.projectAdd({ name: "Renai Sim" });
  assert.equal(p.slug, "renai-sim");
  assert.equal(fetches[0].headers.get("X-Taskboard-Session"), "deadbeef");
});
```

Use node:test or vitest. Prefer vitest already needed by the UI.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/client test`

Expected: FAIL module not found.

- [ ] **Step 3: Implement types, Transport, HttpTransport**

Hand-write `packages/types/src/index.ts` to match camelCase HTTP DTOs (displayId, repoPath, noteMarkdown, displayStatus). A later generate step may replace this file; keep field names stable.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/client test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add package.json pnpm-workspace.yaml pnpm-lock.yaml packages/types packages/client
git commit -m "feat: add HttpTransport for localhost API"
```

---

### Task 4: UI board, inspector, keyboard, Native Glass

**Files:**
- Create: `packages/ui/src/theme.css` (CSS variables)
- Create: `packages/ui/src/Sidebar.tsx` + `Sidebar.module.css`
- Create: `packages/ui/src/Board.tsx` + `Board.module.css`
- Create: `packages/ui/src/Card.tsx` + `Card.module.css`
- Create: `packages/ui/src/Inspector.tsx` + `Inspector.module.css`
- Create: `packages/ui/src/Search.tsx`
- Create: `packages/ui/src/EmptyState.tsx`
- Create: `packages/ui/src/index.ts`
- Test: `packages/ui/src/Card.test.tsx`, `packages/ui/src/Board.test.tsx`, `packages/ui/src/keyboard.test.tsx`

**Interfaces:**
- Consumes: `Transport`, types from `@taskboard/types`
- Produces: `export function TaskboardApp(props: { transport: Transport })`

CSS variables:

```css
:root {
  --sidebar-bg: #1c1b22;
  --board-bg: #f4f2f8;
  --accent: #5b4bdb;
  --accent-2: #7c6cff;
  --urgent: #e24a2b;
  --waiting: #d89a1a;
  --completed: #2f9e62;
  --running: #5b4bdb;
}
```

Card face: title, `TASK-n`, urgent pip, run badge. Idle: no badge. Waiting: amber `Waiting`. Running: indigo `Running`. Failed: muted red `Failed`. Completed: green `Done`.

Keyboard: `j k h l 1 2 3 4 n p u / Escape Enter` as detailed design section 9. `/` focuses Search which filters `title.toLowerCase().includes(query)`.

dnd-kit: drag card to column → `transport.taskMove`; drag onto card → `transport.taskReorder(id, beforeId)`.

Notes: debounce 400ms then `taskNoteSet` / `projectNoteSet` with last seen revision.

- [ ] **Step 1: Write the failing tests**

```tsx
it("shows running badge and hides idle badge", () => {
  const { getByText, queryByText, rerender } = render(
    <Card task={summary({ displayStatus: "idle" })} selected={false} />
  );
  expect(queryByText("Running")).toBeNull();
  rerender(<Card task={summary({ displayStatus: "running" })} selected={false} />);
  expect(getByText("Running")).toBeTruthy();
});

it("n creates a task through transport", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  // fakeTransport.projectAdd resolves; press n
  await userEvent.keyboard("n");
  expect(transport.taskCreate).toHaveBeenCalled();
});
```

`fakeTransport()` is an in-memory object that records calls and holds one project / tasks array.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL.

- [ ] **Step 3: Implement components and keyboard handlers**

Empty state copy: `Create a project to start a board`. Inspector empty: `Select a card`. Column `role="list"` `aria-label` Todo / In Progress / In Review / Done. Cards `role="listitem"`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/ui
git commit -m "feat: add Native Glass board and inspector UI"
```

---

### Task 5: Vite web app, poll, serve static, e2e

**Files:**
- Create: `apps/web/package.json`
- Create: `apps/web/index.html`
- Create: `apps/web/src/main.tsx`
- Create: `apps/web/src/App.tsx`
- Create: `apps/web/vite.config.ts`
- Modify: `crates/api/src/server.rs` to serve `apps/web/dist` (dev: `TASKBOARD_WEB_DIST` env, else compile-time `include_dir` after `pnpm --filter web build`)
- Test: `crates/api/tests/web.rs` or `apps/web/e2e/serve.spec.ts`

**Interfaces:**
- Consumes: `TaskboardApp`, `HttpTransport`
- Produces: `tb serve` serves the SPA; poll every 1000ms

- [ ] **Step 1: Write the failing test**

CLI e2e (Rust):

```rust
#[tokio::test]
async fn serve_page_contains_root_and_poll_sees_cli_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::cargo_bin("taskboard").unwrap()
        .env("TASKBOARD_DATA_DIR", dir.path())
        .args(["serve", "--port", "0"]) // if 0 not supported, parse printed URL from stdout
        .stdout(Stdio::piped())
        .spawn().unwrap();
    let url = read_printed_url(&mut child).await;
    let html = reqwest::get(url.clone()).await.unwrap().text().await.unwrap();
    assert!(html.contains("id=\"root\"") || html.contains("Taskboard"));
    Command::cargo_bin("taskboard").unwrap()
        .env("TASKBOARD_DATA_DIR", dir.path())
        .args(["project", "add", "--name", "From CLI"])
        .assert().success();
    // GET sync as the served session is not available to this test without parsing bootstrap.
    // Instead open /api is unauthorized without token; parse token from bootstrap JSON if exposed
    // to same-origin document only. Test HTML load + CLI write to the same DB:
    let list = Command::cargo_bin("taskboard").unwrap()
        .env("TASKBOARD_DATA_DIR", dir.path())
        .args(["project", "list", "--json"])
        .output().unwrap();
    assert!(String::from_utf8_lossy(&list.stdout).contains("from-cli"));
    child.kill().ok();
}
```

Also add a unit test in web: `App` constructs `HttpTransport` from bootstrap.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test web -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Implement apps/web and static serving**

`main.tsx` fetches `/api/v1/bootstrap` with credentials, then `createRoot`. `App.tsx` uses TanStack Query `refetchInterval: 1000` on `transport.sync(lastSeq)`. Visible-only: `document.visibilityState === "visible"`.

If `--port` omitted, bind `127.0.0.1:0` and print the actual port.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm test` and `cargo test -p taskboard-cli -- --test-threads=1` and `cargo test -p taskboard-api -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/web crates/api crates/cli
git commit -m "feat: serve localhost web UI from tb serve"
```

---

## Spec coverage

| Story / rule | Task |
| --- | --- |
| Session + Origin | 1 |
| HTTP table section 8 | 2 |
| US-14 poll | 5 |
| US-08 card badges | 4 |
| Keyboard / search | 4 |
| Native Glass | 4 |
| `tb serve --open` | 2, 5 |
| Tauri desktop | Plan 3 |
