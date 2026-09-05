# Taskboard Desktop Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri 2 macOS desktop window that uses the shared UI and `TauriTransport`, talking to the same `App` / SQLite database as the CLI, with no HTTP server required for normal use.

**Architecture:** `apps/desktop` is a Tauri 2 project. Rust commands in the Tauri crate call `taskboard_application::App` on a shared pool. `TauriTransport` in `packages/client` invokes `invoke`. The webview loads `packages/ui` `TaskboardApp`. CLI mutations appear because the UI polls `sync` every 1000ms against the same database file.

**Tech Stack:** Tauri 2, React, Vite, existing Plan 1 crates and Plan 2 UI. Apple silicon, macOS 14+. Linux is not a release target; Cloud Agent may compile-check the Rust side only.

## Global Constraints

- Desktop shell: Tauri 2
- macOS 14 or later; Apple silicon release target
- No telemetry, crash upload, or network sync
- Desktop never uses HTTP; `TauriTransport` only
- Same data directory rules as CLI (`TASKBOARD_DATA_DIR`, macOS Application Support path)
- Actor kind `desktop`, label `local-ui` unless overridden
- Native Glass UI from Plan 2; do not fork components
- Specs: `docs/superpowers/specs/2026-09-05-local-taskboard-design.md` and `docs/superpowers/specs/2026-09-05-local-taskboard-detailed-design.md`

## File map

- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `Cargo.toml` workspace members to include `apps/desktop/src-tauri` if that crate is named `taskboard-desktop`
- Modify: `packages/client/src/tauri.ts`
- Modify: `packages/client/src/index.ts`
- Test: `packages/client/src/tauri.test.ts`
- Test: `apps/desktop/src-tauri/src/commands.rs` (or `apps/desktop/src-tauri/tests/commands.rs`)

---

### Task 1: TauriTransport

**Files:**
- Create: `packages/client/src/tauri.ts`
- Modify: `packages/client/src/index.ts`
- Test: `packages/client/src/tauri.test.ts`

**Interfaces:**
- Consumes: `Transport` from `packages/client/src/transport.ts` (Plan 2 Task 3). Method names stay identical.
- Produces: `export class TauriTransport implements Transport` with constructor `(invokeFn: InvokeFn)` where `type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>`

Command names match snake_case Rust commands: `project_add`, `project_list`, `task_create`, `task_move`, `task_reorder`, `task_urgent`, `task_delete`, `task_restore`, `task_note_set`, `link_add`, `link_remove`, `run_start`, `run_patch`, `trash_list`, `undo`, `sync`, `project_update`, `project_reorder`, `project_archive`, `project_delete`, `project_restore`, `project_note_set`, `task_list`, `task_show`, `task_update`.

- [ ] **Step 1: Write the failing test**

```ts
it("invokes project_add with camelCase payload mapped to rust args", async () => {
  const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
  const invoke: InvokeFn = async (cmd, args) => {
    calls.push({ cmd, args });
    return { id: "00000000-0000-7000-0000-000000000001", slug: "renai-sim", name: "Renai Sim", revision: 1 };
  };
  const t = new TauriTransport(invoke);
  const p = await t.projectAdd({ name: "Renai Sim" });
  assert.equal(calls[0].cmd, "project_add");
  assert.equal(p.slug, "renai-sim");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/client test`

Expected: FAIL `TauriTransport` not found.

- [ ] **Step 3: Implement TauriTransport**

Each `Transport` method calls `this.invoke(command, args)` and returns the entity. Errors from invoke must surface `error.code` matching AppError codes. Do not call `fetch`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/client test`

Expected: PASS including Plan 2 HttpTransport tests.

- [ ] **Step 5: Commit**

```bash
git add packages/client
git commit -m "feat: add TauriTransport for desktop invoke"
```

---

### Task 2: Tauri crate and commands

**Files:**
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/commands.rs`
- Create: `apps/desktop/src-tauri/src/state.rs`
- Modify: workspace `Cargo.toml` members
- Test: `apps/desktop/src-tauri/tests/commands.rs`

**Interfaces:**
- Consumes: `App` from Plan 1
- Produces: Tauri commands listed in Task 1. Shared state:

```rust
pub struct DesktopState {
    pub app: tokio::sync::Mutex<App>,
}

#[tauri::command]
pub async fn project_add(
    state: tauri::State<'_, DesktopState>,
    name: String,
    repo_path: Option<String>,
    slug: Option<String>,
) -> Result<Project, AppErrorDto> { ... }
```

`AppErrorDto` serializes `{ "code": "validation_error", "message": "...", "field": "title", "current": null }` so the UI can display the same errors as HTTP.

`tauri.conf.json` identifier `com.taskboard.app`, productName `Taskboard`, windows title `Taskboard`, CSP that does not allow remote scripts. `withGlobalTauri` false.

On setup: `resolve_data_dir(None, std::env::var_os("TASKBOARD_DATA_DIR"))`, `open_db`, `purge_expired_trash(Utc::now())`, store `App` in `DesktopState`. Actor is `Actor { kind: ActorKind::Desktop, label: "local-ui".into() }`.

- [ ] **Step 1: Write the failing test**

This test does not start a webview. It constructs `DesktopState` with a temp dir `App` and calls the command functions directly:

```rust
#[tokio::test]
async fn project_add_command_creates_slug() {
    let state = test_state().await;
    let p = project_add(state_handle(&state), "Renai Sim".into(), None, None).await.unwrap();
    assert_eq!(p.slug, "renai-sim");
}

#[tokio::test]
async fn sync_sees_cli_equivalent_write_on_same_app() {
    let state = test_state().await;
    project_add(state_handle(&state), "A".into(), None, None).await.unwrap();
    let delta = sync(state_handle(&state), 0).await.unwrap();
    assert!(delta.sequence >= 1);
    assert_eq!(delta.projects[0].slug, "a");
}
```

If command signatures need `tauri::State`, extract inner functions `pub async fn project_add_inner(app: &App, ...)` and test those.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-desktop --test commands`

Expected: FAIL crate missing.

- [ ] **Step 3: Implement commands wrapping App**

Register all commands in `tauri::Builder`. Map `If-Match`-style revision as an optional `revision: Option<i64>` argument. Do not bind a TCP port.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-desktop -- --test-threads=1`

Expected: PASS. If webkit2gtk is missing on Linux, skip linking the webview binary and keep `taskboard-desktop` lib tests compiling with Tauri `custom-protocol` only when possible. If the crate cannot compile without webkit, put inner functions in `taskboard-application` tests already covered and keep this crate macOS-only with `cargo test -p taskboard-desktop` documented as macOS. Prefer extracting `*_inner` into `crates/desktop-commands` without Tauri so Linux CI still runs them. If extracted, crate name is `taskboard-desktop-commands`, files `crates/desktop-commands/src/lib.rs`, and Tauri `commands.rs` becomes one-line wrappers. Use that split when `pkg-config webkit2gtk-4.1` fails.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml apps/desktop/src-tauri crates/desktop-commands
git commit -m "feat: add Tauri command wrappers around App"
```

---

### Task 3: Desktop frontend entry and smoke

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/vite.config.ts`
- Modify: `apps/desktop/src-tauri/tauri.conf.json` build distDir `../dist`
- Test: `apps/desktop/src/App.test.tsx`

**Interfaces:**
- Consumes: `TaskboardApp` from `@taskboard/ui`, `TauriTransport` from `@taskboard/client`
- Produces: desktop entry that never imports `HttpTransport`

```tsx
import { invoke } from "@tauri-apps/api/core";
import { TauriTransport } from "@taskboard/client";
import { TaskboardApp } from "@taskboard/ui";

export function App() {
  const transport = new TauriTransport(invoke);
  return <TaskboardApp transport={transport} />;
}
```

- [ ] **Step 1: Write the failing test**

```tsx
it("does not call fetch on first render", async () => {
  const invoke = async () => ({ projects: [], sequence: 0 });
  const fetchCalls: string[] = [];
  const original = globalThis.fetch;
  globalThis.fetch = async (input) => {
    fetchCalls.push(String(input));
    return original(input);
  };
  render(<AppWithInvoke invoke={invoke} />);
  expect(fetchCalls).toEqual([]);
  globalThis.fetch = original;
});
```

Expose `AppWithInvoke` for tests; production `App` uses `invoke` from `@tauri-apps/api/core`.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/desktop test`

Expected: FAIL.

- [ ] **Step 3: Implement desktop frontend**

`index.html` has `<div id="root"></div>`. Window min size 960x640. Keyboard shortcuts from Plan 2 apply because `TaskboardApp` is reused. Poll: same `refetchInterval: 1000` inside `TaskboardApp` via `transport.sync`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/desktop test` and `pnpm --filter @taskboard/ui test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop
git commit -m "feat: load shared board UI in the Tauri window"
```

---

### Task 4: Undo, CLI refresh, packaging notes

**Files:**
- Modify: `packages/ui/src/TaskboardApp.tsx` (or keyboard handler file from Plan 2) to bind `Meta+Z` / `Control+Z` to `transport.undo()`
- Modify: `apps/desktop/src-tauri/tauri.conf.json` bundle icons placeholder only if files exist; skip binary assets if none, document formula
- Create: `docs/superpowers/plans/desktop-smoke.md` is not allowed as a substitute for tests. Put a Rust test that writes via `App` while another `App` on the same file reads `sync`:

**Interfaces:**
- Consumes: `undo`, `sync`
- Produces: Cmd+Z path; two-process SQLite refresh behavior required by US-14

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn second_connection_sees_commit_within_two_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let app_a = open_app(dir.path()).await;
    let app_b = open_app(dir.path()).await;
    app_a.project_add(&cli_actor(), ProjectAdd { name: "A".into(), repo_path: None, slug: None }).await.unwrap();
    let start = std::time::Instant::now();
    let mut seq = 0;
    loop {
        let delta = app_b.sync(seq).await.unwrap();
        if delta.projects.iter().any(|p| p.slug == "a") {
            assert!(start.elapsed() <= std::time::Duration::from_secs(2));
            break;
        }
        seq = delta.sequence;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }
}
```

UI test:

```tsx
it("meta+z calls undo", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.keyboard("{Meta>}z{/Meta}");
  expect(transport.undo).toHaveBeenCalled();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test concurrency -- --exact second_connection_sees_commit_within_two_seconds` and `pnpm --filter @taskboard/ui test`

Expected: FAIL if undo shortcut or WAL visibility is missing.

- [ ] **Step 3: Enable WAL (already in Plan 1) and bind undo shortcut**

Confirm `PRAGMA journal_mode=WAL` in `open_db`. If the UI test fails because Plan 2 used a different key map, add `meta+z` / `ctrl+z` there without duplicating board components.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -- --test-threads=1` and `pnpm test`

Expected: PASS on Linux for crates + JS. Full `.dmg` smoke is macOS-only and is not required to merge this plan if the command and UI tests pass.

- [ ] **Step 5: Commit**

```bash
git add packages/ui crates apps/desktop
git commit -m "feat: undo shortcut and CLI-visible desktop sync"
```

---

## Spec coverage

| Story / rule | Task |
| --- | --- |
| Tauri 2 shell | 2, 3 |
| Shared UI / no HTTP on desktop | 1, 3 |
| US-14 CLI appears in two seconds | 4 |
| US-13 undo | 4 |
| macOS 14 / Apple silicon packaging | 2 `tauri.conf.json`; `.dmg` produced on a Mac builder, not in Cloud Agent |
| Homebrew CLI tap | out of this plan (release engineering after binaries exist) |
