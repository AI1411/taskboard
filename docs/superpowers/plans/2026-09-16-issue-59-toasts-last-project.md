# Toasts, Safer h/l, Last Project Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a toast region, make `Shift+h`/`Shift+l` jump columns without moving, and persist the last live project in `{data_dir}/ui.toml`.

**Architecture:** `UiState` lives in `ui.toml` via store-sqlite helpers. HTTP `GET/PATCH /api/v1/ui-state` and desktop `ui_state` / `ui_state_set` expose it on Transport. `TaskboardApp` loads the last live slug, writes on project select, shows toasts for keyboard moves / delete / undo / transport errors, and treats Shift+h/l as jump-only.

**Tech Stack:** Rust (store-sqlite, API, desktop), React, Vitest, Testing Library. Spec §§5, 7, 12. Issue: https://github.com/AI1411/taskboard/issues/59. Existing plan Task 6 plus Task 7 transport-error toasts.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `h` / `l` still move a selected card
- `Shift+h` / `Shift+l` change `currentColumn` only (no `taskMove`)
- Do not write `config.toml`; CLI does not write `ui.toml`
- Toast copy is English; auto-dismiss 2.4s, errors 8s
- Do not toast routine note autosaves
- No OS notifications

## File map

- Create: `crates/store-sqlite/src/ui_state.rs`
- Create: `packages/ui/src/Toast.tsx`, `Toast.module.css`, `Toast.test.tsx`
- Modify: `crates/store-sqlite/src/lib.rs`
- Modify: `crates/api/src/{server.rs,routes.rs,dto.rs,lib.rs}`
- Modify: `crates/api/tests/routes.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/desktop-commands/src/{commands.rs,dto.rs}`
- Modify: `apps/desktop/src-tauri/src/{lib.rs,state.rs,commands.rs}`
- Modify: `packages/client/src/{transport.ts,http.ts,tauri.ts,http.test.ts,tauri.test.ts,index.ts}`
- Modify: `packages/ui/src/{TaskboardApp.tsx,TaskboardApp.module.css,fakeTransport.ts,keyboard.test.tsx,index.ts}`

---

### Task 1: ui.toml + Transport + toasts + Shift+h/l

**Files:** listed above

**Interfaces:**
- Consumes: `transport.undo`, `transport.taskMove`, `transport.projectList`, `transport.taskShow`, `transport.taskDelete`
- Produces:
  - `UiState { last_project_slug: Option<String> }` in `{data_dir}/ui.toml`
  - `load_ui_state` / `save_ui_state`; missing file = empty; unknown keys ignored
  - `GET/PATCH /api/v1/ui-state` body `{ lastProjectSlug }`
  - `Transport.uiState()` / `uiStateSet(slug | null)`
  - Toast `{ message, action?, error? }` bottom-center
  - Keyboard move toast `Moved to {Column}` + Undo
  - Delete toast `Deleted {TASK-n}` + Undo
  - Undo success `Undone`; undo conflict `Cannot undo — {field} changed`
  - Title/note `revision_conflict` → `Updated elsewhere` + reload detail
  - Other mutation / `taskShow` errors toast `error.message` unless `not_found`
  - On load, select last live project else first project
  - After `applyProject`, `uiStateSet(slug)`

- [x] **Step 1: Write the failing tests** (`ui_state` unit, API route, keyboard Shift+l / move toast, last project, taskShow error).
- [x] **Step 2: Run them** — FAIL missing module / toast / Shift+l still moves.
- [x] **Step 3: Implement** helpers, routes, Transport, Toast, TaskboardApp wiring.
- [x] **Step 4: Re-run tests** — PASS.
- [x] **Step 5: Commit** `feat: add move toasts, shift column jump, and last project ui.toml`

---

## Self-review

1. **Spec coverage:** Toasts table, Shift+h/l, last project, taskShow errors (§5, §7, §12).
2. **Placeholder scan:** No TBD.
3. **Type consistency:** `lastProjectSlug` camelCase on the wire; `last_project_slug` in toml.
