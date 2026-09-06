# Usability Mechanisms Design

Companion to `docs/superpowers/specs/2026-09-05-local-taskboard-design.md`. That document remains the product spec. This document proposes the first post-MVP usability layer: mechanisms that make the board easier to use for a person watching several agent runs, without adding cloud features, custom columns, or a second product.

Status: proposal. Do not implement until this spec is approved.

## 1. Why this exists

The MVP is functionally complete: projects, four columns, cards, runs, CLI, localhost web, and desktop all exist. Daily use still fights the product in three places that the original goals already named:

1. **The next thing that needs a person is not obvious.** Goal: "Make the next task requiring attention visually obvious." Today that is true only inside one project's board, and only if you already have the right project selected. Waiting and Failed live on card faces; they do not collect themselves.
2. **The UI hides actions that the domain already supports.** Trash, links, project rename/archive/path, restore, and New Task exist in CLI and Transport. The board either omits them or wires a no-op (Trash calls `trashList` and shows nothing). Keyboard `n` / `p` / `Cmd+Z` work; there is almost no visible control and no shortcut legend.
3. **Agents and people share IDs, but neither gets a short path to "what is this board waiting on?"** Agents must list projects, list tasks, then address `TASK-n`. There is no inbox command and no current-project detection from `repo_path`.

This spec is a usability layer, not a rewrite. Domain rules, four columns, local-only storage, and "finishing a run does not move the card" stay locked.

## 2. Observed friction (current code)

Evidence is the shared UI in `packages/ui` and the CLI in `crates/cli`, against the screens in the detailed design.

| Friction | What happens today | Spec already required? |
| --- | --- | --- |
| Untitled create | `n` / `p` insert title `"Untitled"` immediately | No. US-15 wants a first task in under a minute, not a board of Untitled cards |
| New Task is keyboard-only | No New Task button on the board | Partial. Empty-state copy mentions `n`; US-15 wants New Task as a first action |
| Trash is a no-op | Sidebar Trash calls `transport.trashList()` and does not render a panel | Yes. Screens: Trash entry, empty copy `Trash is empty` |
| Links are read-only | Inspector lists `link.value`; no add/remove | Yes. US-10 |
| Project settings missing | Cannot rename, set path, archive, reorder, or delete a project in the UI | Yes. US-02 |
| Restore missing | Inspector has Delete only | Yes. Inspector: Delete / Restore |
| Delete keyboard missing | `Delete` / `Backspace` not bound | Yes. Keyboard table |
| No shortcut legend | Keys exist (`j k h l 1-4 [ ] n p u e / Esc Enter Cmd+Z`) with no `?` overlay | No, but keyboard is otherwise undiscoverable |
| `h` / `l` mutate | With a card selected, `h`/`l` move it; 1–4 only change current column | Intentional in detailed design ("equivalent to move"). Easy to do by accident |
| No feedback | Undo, note save, and API errors have no toast | No. Conflicts and busy errors are defined on the API, unused in UI |
| Last project forgotten | App always opens `projects[0]` | No |
| Attention is per-board | Running / Waiting / Urgent badges exist on cards only | Goal 2 is board-local; no cross-project view |
| Agent ceremony | `project list` → `task list` → `task create` → `task move` → `run start` | AGENTS.md; no `inbox` or cwd project |

Out of this list, Trash / links / project settings / restore are **spec debt**. Untitled create, missing New Task, no legend, no toasts, no last-project, and no inbox are **new mechanisms**.

## 3. Approaches considered

### A. Spec-complete the existing screens

Finish the UI the detailed design already described: Trash panel, link add/remove, project rename/archive/path/reorder, restore, Delete key, New Task button, shortcut legend.

- Pros: smallest design risk; unblocks US-02, US-10, US-12 in the GUI; no new entities.
- Cons: does not make "what needs me" obvious across projects; agents still have the same ceremony; Untitled create remains.

### B. Attention routing (inbox) plus a thin agent context

Add a derived **Inbox** of cards that need a person, in the UI and as `tb inbox`. Detect the current project from `cwd` vs `repo_path` (and `TASKBOARD_PROJECT`) so agents skip `project list` when the mapping is unique.

- Pros: directly serves Goal 2; Waiting is the product's actual human-in-the-loop moment; CLI gets a read command agents already need; no new stored entity.
- Cons: a new surface to keep consistent with the board; must not become a fifth Kanban column.

### C. Power-user overlay (command palette, fuzzy IDs, composite `tb work start`)

Cmd-K for every action, title-based task addressing, one CLI command that creates + moves + starts a run.

- Pros: fast for experts; shorter agent transcripts.
- Cons: new interaction model on top of an unfinished board; fuzzy title IDs contradict the locked rule that titles are not identifiers; composite commands hide the column/run split the product is built on.

**Recommendation:** do **A scoped + B**. Call it the usability layer. Defer C. Do not add `tb work start`, command palette, or title-as-ID.

A without B leaves the main goal unsolved. B without A puts a new inbox on a board that still cannot restore, attach a link, or name a project. C waits until A+B exist.

## 4. Product principles for this layer

1. **Attention is a query, not a column.** Inbox never stores its own rank or status. Moving a card on the board is enough to change inbox membership.
2. **Visible first, keyboard still faster.** Every frequent action has a control. Keyboard shortcuts remain; `?` documents them.
3. **Create with a name.** The board must not insert `Untitled` as a real card. Draft titles live in the input until committed.
4. **Same rules on all three surfaces.** Inbox membership, project detection, and trash restore go through `App`. Desktop, web, and CLI call the same use cases.
5. **YAGNI.** No OS notifications, no markdown preview, no custom filters UI, no fifth column, no launching agents from the app.

## 5. Mechanism 1 — Named create and visible actions

### New Task

The board toolbar gains a **New task** button to the right of Search. Click, or `n`, focuses an inline composer at the top of the current column (the column highlighted by `1`–`4` or last selection).

Composer:

- Placeholder `Task title`.
- `Enter` creates with that trimmed title in the current column and selects the new card.
- `Esc` or empty `Enter` cancels; nothing is written.
- Empty title is `validation_error` on `title` if a client sends it; the composer must not send empty titles.

`p` / **New project** behaves the same: an inline name field in the sidebar. Default name is not `Untitled`. Empty cancel writes nothing.

### Shortcut legend

`?` toggles a panel listing the keyboard table from the detailed design, plus `?` itself. Copy stays English. The panel is not a command runner.

### Safer `h` / `l`

Keep move-on-`h`/`l` when a card is selected (locked in the detailed design). Add a 1.2s undo toast after keyboard moves so an accidental `l` is reversible without hunting. `Shift+h` / `Shift+l` only change the current column without moving. `1`–`4` stay jump-only.

### Last project

Persist `last_project_slug` in `{data_dir}/ui.toml` (App writes this file; it is not the user-edited `config.toml`, which remains read-only). On load, select that project if it still exists and is not archived; otherwise the first live project. Missing file keeps today's first-project behavior.

## 6. Mechanism 2 — Attention Inbox

### Membership (derived)

A live card is in the Inbox when **any** of these hold:

1. Display status is `waiting`.
2. Display status is `failed`.
3. `urgent` is true **and** column is not `done`.

Idle non-urgent cards are never in the Inbox. Completed runs in `done` are never in the Inbox. A card in `todo` with no run is not in the Inbox unless it is urgent.

Rationale: Waiting is "an agent needs a person." Failed is "an agent stopped badly." Urgent is the person's own pin. Done + completed is history, not attention.

### Sort

1. `waiting` first, then `failed`, then urgent-only.
2. Within a group: urgent before non-urgent.
3. Then newest `updated_at` descending.
4. Tie-break `TASK-n` ascending.

This is display order only. It does not rewrite `position`.

### UI

A compact **Inbox** strip sits above the column row, inside the board pane (not a fifth column, not a replacement for the sidebar).

- Collapsed: one line, `Inbox · N` with counts split as `Waiting a · Failed b · Urgent c`. Amber pip if any Waiting. Hidden entirely when N is 0.
- Expanded (`i` toggles, also click): list of rows `{project name} · TASK-n · title · badge · waiting reason or run message`. Clicking a row selects that project and card and opens the inspector.
- Scope toggle: **This project** (default on a selected board) vs **All projects**. State is session-only; do not persist.
- The strip uses existing Native Glass tokens. Waiting stays amber. Failed uses the existing failed badge color. Urgent uses the orange-red pip. No new accent.

Sidebar project rows show a small count badge when that project has inbox cards. The badge is the inbox count for that project, not a live run spinner.

### Card face (small addition)

When display status is `waiting` or `failed`, the existing one-line `runMessage` remains. If `waiting_reason` is set and `runMessage` is empty, show `waiting_reason` on the card face. Inspector run rows already show status and message; they must also show `waiting_reason` when present.

### CLI

```text
tb inbox [--project SLUG] [--archived] [--json]
```

- Default: all live (non-archived) projects. Agents do not have a "current board," so omitting `--project` is all-projects, not cwd detection.
- `--project SLUG` limits to one slug.
- `--archived` includes inbox cards from archived projects. Default excludes them. Do not name this flag `--all`; that word already means "all projects" in the UI.
- Human columns: `ID  PROJECT  COLUMN  URGENT  RUN  TITLE  REASON`
- JSON: `{ "ok": true, "entities": [ InboxItem, ... ] }` where `InboxItem` is `TaskSummary` plus `project_slug`, `project_name`, and `reason` (`waiting_reason` or `run_message` or empty).
- Empty human output: `inbox is empty`.

No mutation commands on inbox. Move, urgent, run finish, etc. stay as they are.

### HTTP / App

```text
App::inbox(scope: InboxScope) -> Vec<InboxItem>
GET /api/v1/inbox?project={slug}&archived=0|1
```

`InboxScope` is `{ project: Option<Slug>, include_archived: bool }`. Implementation filters `task_list` results in process for MVP. If this becomes slow with very large boards, add a SQL filter later; do not invent a new table now.

Transport: `inbox(opts?: { project?: string; includeArchived?: boolean }): Promise<InboxItem[]>`.

## 7. Mechanism 3 — Feedback and safety

### Toasts

A single toast region, bottom-center, auto-dismiss 2.4s (errors stay until dismissed or 8s).

| Event | Copy | Action |
| --- | --- | --- |
| Undo succeeded | `Undone` | none |
| Undo conflict | `Cannot undo — {field} changed` | none |
| Keyboard card move | `Moved to {Column}` | `Undo` (calls existing `undo`) |
| Note/title save conflict | `Updated elsewhere` | reload detail |
| Mutation error | `error.message` | none |
| Delete | `Deleted {TASK-n}` | `Undo` |

Do not toast routine note autosaves.

### Delete

- Inspector Delete asks `Delete {title}?` with Delete / Cancel. Soft-delete as today.
- Board-focused `Delete` / `Backspace` (not while typing) uses the same confirm. `Esc` cancels.
- After delete, toast with Undo.

### Inspector completeness (spec debt)

Required for this layer because Inbox jump-to-card is useless if the inspector cannot restore, attach a link, or show why a run is waiting.

- Links: add URL or path; remove. Existing `linkAdd` / `linkRemove`.
- Runs: show `displayId`, status, `message`, `waiting_reason`, `summary`. Still no "start run" from the UI (non-goal: launching agents).
- Activity: `operation · actorLabel · relative time`.
- Restore control when the selected entity is in trash (Trash panel selection). Board inspector of a live card keeps Delete only.

### Trash panel

Trash button opens a panel (sidebar overlay or inspector replacement) listing soft-deleted projects and tasks with Restore. Empty copy: `Trash is empty`. Uses existing `trashList` / restore commands. 30-day retention unchanged.

### Errors in UI

Transport failures that today are swallowed (`taskShow` catch that only clears selection) must toast the error unless the entity is genuinely gone (`not_found`), in which case clear selection as today.

## 8. Mechanism 4 — Agent project context (thin)

No new identity scheme. Titles still cannot address tasks.

### Current project

Resolution order for commands that take `--project` when the flag is omitted:

1. `--project` if passed.
2. `TASKBOARD_PROJECT` if set and the slug exists.
3. Unique live project whose `repo_path` is an ancestor of `cwd` (longest path wins if several match).
4. Else `project_required` with message `pass --project or set TASKBOARD_PROJECT`.

Commands that already require `--project`: `task create`, `task list`. `inbox` defaults to all projects when omitted (see §6); it does not use this resolver, so a waiting card in another repo is not hidden from an agent.

`tb project detect` prints the resolved project or `project_required`. JSON: the Project entity.

### AGENTS.md

After this ships, the agent workflow may replace the opening `project list` with `tb project detect --json` when the repo is linked. `task list --project` remains the way to find `TASK-n`. Do not allow title-only mutations.

This spec does not rewrite `AGENTS.md` until Mechanism 4 is implemented and tested.

## 9. Mechanism 5 — Project settings in the sidebar

Selected project name is editable on blur (same pattern as card title). A `⋯` menu on the selected project:

- Set repository path
- Archive / Unarchive
- Delete (confirm, soft-delete)
- Move up / Move down (or drag reorder if cheap to add with existing `projectReorder`)

Archived toggle stays as a filter. Showing archived projects still uses `projectList(true)`.

## 10. What we will not do in this layer

- OS or browser notifications.
- Command palette / Cmd-K.
- Fuzzy or title-based task IDs.
- Composite `tb work start` (create + move + run start).
- Markdown preview or WYSIWYG.
- Custom columns, labels, or assignees.
- Inbox as a stored column or a place you drag cards into.
- Auto-moving a card when a run finishes or fails.
- Changing poll interval, Native Glass palette, or the four-column workflow.

## 11. Screens (delta)

```text
Sidebar
  New project → inline name composer
  Project row → optional inbox count
  Selected name → editable
  ⋯ menu → path, archive, delete, reorder
  Project note (unchanged)
  Archived toggle (unchanged)
  Trash → panel with restore

Board
  Toolbar: Search | Inbox summary | New task
  Inbox strip (hidden if empty)
  Four columns (unchanged)
  Inline new-task composer in current column

Inspector
  Title, id, column, urgent, note
  Links add/remove
  Runs with waiting_reason
  Activity with time
  Delete (+ confirm)

Overlay
  ? shortcut legend
  Toast region
  Trash panel
```

Keyboard additions:

| Key | Action |
| --- | --- |
| `?` | toggle shortcut legend |
| `i` | toggle Inbox expanded |
| `n` | focus new-task composer (no longer inserts Untitled) |
| `p` | focus new-project composer |
| `Shift+h` / `Shift+l` | change current column without moving |
| `Delete` / `Backspace` | confirm and soft-delete selected card |

## 12. Data, config, testing

### ui.toml

```toml
last_project_slug = "taskboard"   # omitted until a project is selected
```

`config.toml` is unchanged and stays a read-only user file. Unknown keys in `ui.toml` are ignored. No new SQLite tables. No migration. `App` reads `ui.toml` on startup and writes it when the selected project changes (desktop/web via a `uiStateSet` use case; CLI does not change it).

### Types

`InboxItem` is a DTO, not a persisted entity:

```text
display_id, title, column, urgent, display_status,
run_message, waiting_reason, reason,
project_slug, project_name, revision, updated_at
```

`waiting_reason` on `TaskSummary` is added if the card face needs it without `taskShow`. Prefer extending `TaskSummary` over a second round-trip.

### Tests

- Core/application: inbox membership table (waiting, failed, urgent-not-done, done+completed excluded, archived excluded by default). Sort order.
- CLI: `tb inbox --json` contract; `tb project detect` unique path, collision, env override, missing.
- UI: New task composer does not call `taskCreate` until Enter with a non-empty title; `n` no longer creates Untitled; Inbox hidden at 0; click row selects project+card; `?` opens legend; Trash panel restore; toast on undo.
- Keyboard: `Shift+l` does not call `taskMove`; `l` still does when a card is selected.

## 13. Success criteria

- A person can name a project and first task from the UI without creating an Untitled card, in under one minute (US-15 still holds).
- With two projects each holding one Waiting card, Inbox All-projects shows both without opening the other board first.
- `tb inbox --json` lists the same membership as the UI All-projects scope.
- Trash restore works from the UI. Link add/remove works from the inspector.
- Accidental `l` move is undone from the toast without opening history.
- `tb project detect` from a linked repo prints that project; from an unlinked directory it fails with `project_required`.
- Desktop, web, and CLI still share `App` rules. No new network dependency.

## 14. Suggested implementation slices (not a plan)

These are reviewable units after this spec is approved. A writing-plans pass should turn them into TDD steps.

1. Named create (composer, New task button, stop Untitled) + `?` legend + last project + move toast + Shift+h/l.
2. Inspector/sidebar spec debt: links, trash panel, project ⋯ menu, delete confirm, transport error toasts.
3. Inbox query in `App` + CLI `tb inbox` + HTTP + UI strip + sidebar badges + `i`.
4. `tb project detect` and optional `--project` on `task create` / `task list`.

Slice 3 is the new product mechanism. Slices 1–2 make the board honest. Slice 4 is the agent convenience and can ship last.

## 15. Open decisions (locked here so implementation does not invent them)

| Topic | Decision |
| --- | --- |
| Inbox stored? | No. Derived query. |
| Inbox default UI scope | Current project. CLI default is all projects. |
| Failed in inbox? | Yes, while the card is live. |
| Urgent in `done` | Out of inbox. |
| Untitled cards | Never created by UI or by `n`/`p`. CLI `--title` still required. |
| `h`/`l` | Still move when a card is selected. Toast + Shift modifier for jump. |
| Notifications | Out of scope. |
| Command palette | Out of scope. |
| Title as task ID | Forbidden, unchanged. |
