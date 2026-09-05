# Local Taskboard Detailed Design

Companion to `docs/superpowers/specs/2026-09-05-local-taskboard-design.md`. That document is the product spec. This document locks the remaining requirements so implementation does not have to invent identifiers, schemas, HTTP routes, screens, or file formats.

Implementation is split into three independently shippable plans:

1. Core domain, SQLite, and CLI (`tb` / `taskboard`) — `docs/superpowers/plans/2026-09-05-taskboard-core-cli.md`
2. Local HTTP API and web UI (`tb serve`) — `docs/superpowers/plans/2026-09-05-taskboard-api-web.md`
3. Tauri desktop shell using the shared UI — `docs/superpowers/plans/2026-09-05-taskboard-desktop.md`

Each plan must produce working, testable software on its own. Plan 1 is the source of truth for domain behavior. Plans 2 and 3 must call the same application use cases.

## 1. Locked decisions

These choices were left implicit in the product spec. They are now requirements.

| Topic | Decision |
| --- | --- |
| Internal IDs | UUID v7, stored as `BLOB(16)` |
| Human IDs | Monotonic `TASK-n` and `RUN-n` per database. Projects are addressed by slug, not `PROJECT-n` |
| Idle | Derived card status, not a stored run row. A card is Idle when it has no run in `running` or `waiting` |
| Run finish vs task Done | Unchanged: finishing a run never moves the card |
| Concurrent runs | Allowed. `tb run update/wait/finish/fail` always take a `RUN-n` |
| Notes | One Markdown document per project and per task. UI debounce is 400ms. `tb note add` appends a paragraph. `tb note set` replaces the document |
| Links | Ordered list of `{ kind: url \| path, value: string }` on each task |
| CLI language | English flags, English human output, English column slugs |
| UI language | English for MVP. Japanese copy is out of scope |
| Search | Title substring filter in the current project. Case-insensitive. Keyboard `/` focuses the filter |
| Data directory override | `TASKBOARD_DATA_DIR` wins over the platform default |
| Linux / test data dir | `$XDG_DATA_HOME/taskboard` or `~/.local/share/taskboard` |
| macOS data dir | `~/Library/Application Support/Taskboard/` |
| Busy timeout | 5 seconds, then `database_busy` |
| Poll interval | 1000ms while the UI is visible |
| Homebrew tap | `AI1411/taskboard`, formula name `taskboard` |
| Backup file format | Native SQLite database produced by SQLite backup. No custom archive format |
| Activity snapshots | Full entity JSON in the local DB so Undo can work. Diagnostic log export redacts note bodies, run summaries, and repository paths |
| Undo target | The latest undoable activity in the database, not per-actor. Personal local board, one user |

## 2. User stories and acceptance criteria

IDs are stable. Plans must cover every story in this table.

### Projects

**US-01.** Create a project from the UI or CLI in one step.  
Acceptance: `tb project add --name "Renai Sim"` creates slug `renai-sim`, opens an empty four-column board, and returns the project with revision `1`. A second project with the same name gets slug `renai-sim-2`. `tb project add --name "Other" --slug renai-sim` fails with `duplicate_slug`.

**US-02.** Rename, set repository path, archive, and reorder projects.  
Acceptance: archived projects leave the default sidebar and `tb project list`; `--archived` lists them. Reorder is a single transaction that rewrites `sort_order` to `0..n-1`.

**US-03.** Edit a project note in Markdown.  
Acceptance: UI saves 400ms after the last keystroke. `tb project-note set <slug> --file note.md` replaces the note. Each save increments revision and writes an activity row.

### Board and cards

**US-04.** Create a card on the current project.  
Acceptance: default column is `todo`. Title is required and trimmed. Empty title is `validation_error` on field `title`. New non-urgent cards go at the end of the non-urgent group.

**US-05.** Move a card between the four columns.  
Acceptance: `tb task move TASK-12 in-progress` places it at the end of the destination column's matching urgency group. Source and destination column positions are rewritten contiguously in one transaction.

**US-06.** Reorder a card in its column.  
Acceptance: `tb task prioritize TASK-12 --before TASK-8` fails with `different_column` if the two cards are not in the same column. Display order is urgent cards first (their positions), then non-urgent cards.

**US-07.** Toggle Urgent.  
Acceptance: turning Urgent on moves the card to the end of the urgent group in the same column. Turning it off moves it to the start of the non-urgent group. Orange-red styling is the only Urgent color.

**US-08.** Identify Running, Waiting, and Urgent cards without opening the inspector.  
Acceptance: every card face shows title, `TASK-n`, urgency mark when set, and a run badge when status is `running`, `waiting`, `failed`, or `completed`. Idle cards have no run badge.

### Notes, links, runs

**US-09.** Edit a task note.  
Acceptance: malformed Markdown stores as plain text and still saves. `tb note add TASK-12 --text "Check sessions"` appends after a blank line.

**US-10.** Attach URLs and local paths.  
Acceptance: `tb link add TASK-12 --url https://example.com` and `--path /tmp/log.txt`. Invalid URL is `validation_error` on `value`. Paths are stored as given; the app does not require the file to exist.

**US-11.** Record multiple agent runs on one card.  
Acceptance: `tb run start TASK-12 --agent codex --session abc123` returns `RUN-n` in status `running`. `tb run wait RUN-n --reason "..."` sets `waiting`. `tb run fail RUN-n --summary "..."` sets `failed`. `tb run finish RUN-n --summary "..."` sets `completed` and `ended_at`. The card column does not change.

### Trash, undo, sync

**US-12.** Soft-delete and restore a task or project.  
Acceptance: delete hides the item from the board. `tb trash list` shows it. Restore within 30 days returns it to its last column and position band. After 30 days, startup purge permanently removes the row and nonessential payloads.

**US-13.** Undo the latest safe mutation.  
Acceptance: `tb undo` creates a compensating mutation. If a newer mutation on the same entity makes reversal unsafe, the command exits nonzero with `undo_conflict` and the current entity. History is not rewritten.

**US-14.** See a CLI change in an open UI within two seconds.  
Acceptance: visible UI polls `GET /api/v1/sync?after={sequence}` every 1000ms and refetches only when the sequence advanced.

**US-15.** Create a project and first task in under one minute without reading docs.  
Acceptance: desktop and web both expose New Project and New Task as the first actions on an empty state. CLI examples in `--help` match this spec.

## 3. Identifiers and slugs

### UUID

Every `projects`, `tasks`, `runs`, `links`, and `activities` row has a UUID v7 primary key. JSON field name is `id` as a hyphenated lowercase string.

### Display IDs

| Entity | Display ID | Allocator |
| --- | --- | --- |
| Task | `TASK-{n}` | `counters` row `task`, starting at 1 |
| Run | `RUN-{n}` | `counters` row `run`, starting at 1 |

`n` never reuses a value, including after delete. CLI mutation arguments that take a task or run require this display ID. Titles are never accepted as identifiers.

### Project slugs

- Generate from `--name`: trim, lowercase, replace runs of non-`[a-z0-9]` with `-`, strip leading/trailing `-`.
- Allowed pattern after generation: `^[a-z0-9]+(?:-[a-z0-9]+)*$`.
- Unique among projects with `deleted_at IS NULL`.
- Collision: append `-2`, `-3`, … unless the caller passed `--slug` explicitly, in which case fail with `duplicate_slug`.
- Rename updates the slug only when `--slug` is passed. Changing the display name does not rewrite the slug.

## 4. Domain model

### Enumerations

```text
Column      = todo | in-progress | in-review | done
RunStatus   = running | waiting | failed | completed
LinkKind    = url | path
ActorKind   = cli | desktop | web
EntityType  = project | task | run | link
```

Card display status:

```text
if any run.status in {running, waiting}:
    show the newest such run (latest started_at, then display_id)
else if newest run exists:
    show that run's status (failed or completed)
else:
    show idle
```

`idle` is not stored.

### Entities

**Project**

| Field | Type | Notes |
| --- | --- | --- |
| id | Uuid | PK |
| slug | string | unique among live rows |
| name | string | 1..=120 chars after trim |
| repo_path | string? | local path, no existence check |
| archived | bool | default false |
| note_markdown | string | default `""` |
| sort_order | i64 | contiguous `0..n-1` among all live (non-deleted) projects, including archived. Sidebar shows non-archived rows in that order |
| revision | i64 | starts at 1 |
| created_at | DateTime<Utc> | |
| updated_at | DateTime<Utc> | |
| deleted_at | DateTime<Utc>? | |

**Task**

| Field | Type | Notes |
| --- | --- | --- |
| id | Uuid | PK |
| display_id | string | `TASK-n` |
| project_id | Uuid | |
| title | string | 1..=200 chars after trim |
| column | Column | default `todo` |
| urgent | bool | default false |
| note_markdown | string | default `""` |
| position | i64 | unique per `(project_id, column)` among live tasks |
| revision | i64 | starts at 1 |
| created_at | DateTime<Utc> | |
| updated_at | DateTime<Utc> | |
| deleted_at | DateTime<Utc>? | |

**Link**

| Field | Type | Notes |
| --- | --- | --- |
| id | Uuid | PK |
| task_id | Uuid | |
| kind | LinkKind | |
| value | string | URL must parse with a scheme of `http`, `https`, or `file`. Path kind is any non-empty string |
| sort_order | i64 | per task |

Deleting a task soft-deletes the task row. Links remain until purge.

**Run**

| Field | Type | Notes |
| --- | --- | --- |
| id | Uuid | PK |
| display_id | string | `RUN-n` |
| task_id | Uuid | |
| agent | string | 1..=64 chars, `[a-z0-9][a-z0-9._-]*` |
| session_id | string? | opaque, max 128 |
| status | RunStatus | |
| message | string? | current progress line, max 500 |
| waiting_reason | string? | required when status is `waiting` |
| summary | string? | required when status is `failed` or `completed` |
| started_at | DateTime<Utc> | |
| ended_at | DateTime<Utc>? | set on fail/finish |
| revision | i64 | |
| created_at | DateTime<Utc> | |
| updated_at | DateTime<Utc> | |

Runs are not soft-deleted with the task until purge. `run start` on a deleted task fails with `not_found`.

**Activity**

| Field | Type | Notes |
| --- | --- | --- |
| id | Uuid | PK |
| sequence | i64 | monotonic, unique, starts at 1 |
| actor_kind | ActorKind | |
| actor_label | string | CLI: `--actor` else `TASKBOARD_ACTOR` else `local-cli` |
| operation | string | see operations list |
| entity_type | EntityType | |
| entity_id | Uuid | |
| previous_revision | i64? | null on create |
| before_json | JSON? | null on create |
| after_json | JSON? | null on permanent purge |
| created_at | DateTime<Utc> | |

Operations: `project.create`, `project.update`, `project.reorder`, `project.archive`, `project.unarchive`, `project.delete`, `project.restore`, `project.note.set`, `task.create`, `task.update`, `task.move`, `task.reorder`, `task.urgent`, `task.delete`, `task.restore`, `task.note.set`, `link.add`, `link.remove`, `run.start`, `run.update`, `run.wait`, `run.fail`, `run.finish`, `undo`.

### Invariants

1. Live tasks belong to a live project. Deleting a project sets `projects.deleted_at` and does not cascade-delete tasks. Hidden tasks become visible again when the project is restored, unless a task's own `deleted_at` is set.
2. Display order in a column: `ORDER BY urgent DESC, position ASC` among live tasks.
3. After any move, reorder, or urgency toggle, positions in each affected column are rewritten to `0..n-1` in that display order. Because live positions are unique, the rewrite is two-phase: assign `position = -(old_position + 1)`, then assign the final `0..n-1` values.
4. Mutations compare `revision` when the caller sends one. Omit `revision` on CLI to mean "unconditional". UI always sends the revision it last read.
5. A revision mismatch returns the current entity and does not write.
6. Every successful mutation inserts exactly one activity row and increments the global sequence by 1.
7. Undo is allowed only when no later activity exists for the same `entity_id` with a mutating operation. `undo` itself is recorded as `operation = undo`.
8. Restoring a project fails with `duplicate_slug` if a live project already owns that slug.
9. Purge after 30 days deletes the project or task row, its links and runs (for tasks), and sets matching activity `before_json` and `after_json` to null. Sequence rows stay.

### Validation errors

Stable codes:

| Code | When |
| --- | --- |
| `validation_error` | field constraint failed |
| `not_found` | unknown slug or display ID, or deleted and not addressing trash |
| `duplicate_slug` | live project slug clash |
| `revision_conflict` | expected revision does not match |
| `different_column` | reorder across columns |
| `undo_conflict` | compensating undo is unsafe |
| `database_busy` | SQLite remained busy past 5s |
| `alias_skipped` | install found a foreign `tb` binary |
| `io_error` | backup/import file failure |
| `port_unavailable` | `tb serve --port` was required and could not bind |

JSON error shape:

```json
{
  "ok": false,
  "error": {
    "code": "revision_conflict",
    "message": "Task TASK-12 was updated by desktop:local-ui",
    "field": null,
    "current": { }
  }
}
```

`field` is set for `validation_error`. `current` is set for `revision_conflict` and `undo_conflict`. Human output prints `error: {message}` to stderr. Process exit code is `1` except `database_busy` which is `2`.

Success JSON shape:

```json
{
  "ok": true,
  "entity": { },
  "revision": 3
}
```

List endpoints use `"entities": [ ]` instead of `"entity"`. `tb undo` returns the compensated entity.

## 5. SQLite schema

File: `{data_dir}/taskboard.sqlite3`. WAL mode. `PRAGMA foreign_keys = ON`. `PRAGMA busy_timeout = 5000`.

```sql
CREATE TABLE counters (
    name TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT INTO counters (name, value) VALUES ('task', 0), ('run', 0), ('activity', 0);

CREATE TABLE projects (
    id BLOB NOT NULL PRIMARY KEY,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    repo_path TEXT,
    archived INTEGER NOT NULL DEFAULT 0,
    note_markdown TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX idx_projects_live_slug
    ON projects (slug)
    WHERE deleted_at IS NULL;

CREATE TABLE tasks (
    id BLOB NOT NULL PRIMARY KEY,
    display_id TEXT NOT NULL UNIQUE,
    project_id BLOB NOT NULL REFERENCES projects (id),
    title TEXT NOT NULL,
    column TEXT NOT NULL,
    urgent INTEGER NOT NULL DEFAULT 0,
    note_markdown TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX idx_tasks_live_position
    ON tasks (project_id, column, position)
    WHERE deleted_at IS NULL;

CREATE TABLE links (
    id BLOB NOT NULL PRIMARY KEY,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE TABLE runs (
    id BLOB NOT NULL PRIMARY KEY,
    display_id TEXT NOT NULL UNIQUE,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    agent TEXT NOT NULL,
    session_id TEXT,
    status TEXT NOT NULL,
    message TEXT,
    waiting_reason TEXT,
    summary TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE activities (
    id BLOB NOT NULL PRIMARY KEY,
    sequence INTEGER NOT NULL UNIQUE,
    actor_kind TEXT NOT NULL,
    actor_label TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    previous_revision INTEGER,
    before_json TEXT,
    after_json TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_activities_entity ON activities (entity_id, sequence);
```

Timestamps are ISO-8601 UTC with `Z` (`YYYY-MM-DDTHH:MM:SS.mmmZ`).

Migration `0001_init.sql` creates this schema. Before applying any migration, copy a consistent SQLite backup to `{data_dir}/backups/pre-migration-{schema_version}-{timestamp}.sqlite3`. If the migration fails, leave the original file untouched and keep that backup.

## 6. Application use cases

Rust crate `taskboard-application` exposes these functions. CLI, Axum, and Tauri commands call them. They do not talk to HTTP or clap.

```rust
pub struct Actor {
    pub kind: ActorKind,
    pub label: String,
}

pub struct App {
    // holds Store + clock + id generator
}

impl App {
    pub fn project_add(&self, actor: &Actor, cmd: ProjectAdd) -> Result<Project, AppError>;
    pub fn project_list(&self, include_archived: bool) -> Result<Vec<Project>, AppError>;
    pub fn project_update(&self, actor: &Actor, cmd: ProjectUpdate) -> Result<Project, AppError>;
    pub fn project_reorder(&self, actor: &Actor, slugs_in_order: &[String]) -> Result<Vec<Project>, AppError>;
    pub fn project_archive(&self, actor: &Actor, slug: &str, archived: bool, revision: Option<i64>) -> Result<Project, AppError>;
    pub fn project_delete(&self, actor: &Actor, slug: &str, revision: Option<i64>) -> Result<Project, AppError>;
    pub fn project_restore(&self, actor: &Actor, slug: &str) -> Result<Project, AppError>;
    pub fn project_note_set(&self, actor: &Actor, slug: &str, markdown: String, revision: Option<i64>) -> Result<Project, AppError>;

    pub fn task_create(&self, actor: &Actor, cmd: TaskCreate) -> Result<TaskDetail, AppError>;
    pub fn task_list(&self, project_slug: &str) -> Result<Vec<TaskSummary>, AppError>;
    pub fn task_show(&self, display_id: &str) -> Result<TaskDetail, AppError>;
    pub fn task_update(&self, actor: &Actor, cmd: TaskUpdate) -> Result<TaskDetail, AppError>;
    pub fn task_move(&self, actor: &Actor, display_id: &str, column: Column, revision: Option<i64>) -> Result<TaskDetail, AppError>;
    pub fn task_reorder(&self, actor: &Actor, display_id: &str, before_display_id: Option<&str>, revision: Option<i64>) -> Result<TaskDetail, AppError>;
    pub fn task_urgent(&self, actor: &Actor, display_id: &str, urgent: bool, revision: Option<i64>) -> Result<TaskDetail, AppError>;
    pub fn task_delete(&self, actor: &Actor, display_id: &str, revision: Option<i64>) -> Result<TaskDetail, AppError>;
    pub fn task_restore(&self, actor: &Actor, display_id: &str) -> Result<TaskDetail, AppError>;
    pub fn task_note_set(&self, actor: &Actor, display_id: &str, markdown: String, revision: Option<i64>) -> Result<TaskDetail, AppError>;
    pub fn task_note_add(&self, actor: &Actor, display_id: &str, paragraph: &str, revision: Option<i64>) -> Result<TaskDetail, AppError>;

    pub fn link_add(&self, actor: &Actor, cmd: LinkAdd) -> Result<TaskDetail, AppError>;
    pub fn link_remove(&self, actor: &Actor, link_id: Uuid, revision: Option<i64>) -> Result<TaskDetail, AppError>;

    pub fn run_start(&self, actor: &Actor, cmd: RunStart) -> Result<Run, AppError>;
    pub fn run_update(&self, actor: &Actor, cmd: RunUpdate) -> Result<Run, AppError>;
    pub fn run_wait(&self, actor: &Actor, cmd: RunWait) -> Result<Run, AppError>;
    pub fn run_fail(&self, actor: &Actor, cmd: RunFail) -> Result<Run, AppError>;
    pub fn run_finish(&self, actor: &Actor, cmd: RunFinish) -> Result<Run, AppError>;

    pub fn trash_list(&self) -> Result<Trash, AppError>;
    pub fn undo(&self, actor: &Actor) -> Result<UndoResult, AppError>;
    pub fn activity_head(&self) -> Result<i64, AppError>;
    pub fn sync(&self, after: i64) -> Result<SyncDelta, AppError>;
    pub fn purge_expired_trash(&self, now: DateTime<Utc>) -> Result<u64, AppError>;
    pub fn backup_export(&self, dest: &Path) -> Result<(), AppError>;
    pub fn backup_import(&self, src: &Path) -> Result<(), AppError>;
}
```

`TaskSummary` is the card face: identity, title, column, urgent, display run status, latest message. `TaskDetail` adds note, links, runs, and recent activities.

`Store` is a trait implemented by `taskboard-store-sqlite`. Tests in `application` may use an in-memory SQLite database through that same implementation; do not create a second fake store unless a test cannot open SQLite.

Startup always opens the DB, runs migrations, then `purge_expired_trash`. Retention is 30 days from `deleted_at`.

## 7. CLI contract (complete)

Binary crate `taskboard-cli`. Executables: `taskboard`. Installer creates symlink or copy `tb` only when that name is absent or already points at `taskboard`. If another `tb` exists, print `alias_skipped` and continue.

Global flags: `--json`, `--data-dir <path>`, `--actor <label>`, `--revision <n>` (mutation commands only).

| Command | Result entity |
| --- | --- |
| `tb project add --name NAME [--path PATH] [--slug SLUG]` | Project |
| `tb project list [--archived] [--all]` | Project[] |
| `tb project show SLUG` | Project |
| `tb project update SLUG [--name NAME] [--path PATH] [--slug SLUG]` | Project |
| `tb project archive SLUG` / `tb project unarchive SLUG` | Project |
| `tb project reorder SLUG...` | Project[] |
| `tb project delete SLUG` | Project |
| `tb project restore SLUG` | Project |
| `tb project-note set SLUG --file FILE` / `--text TEXT` | Project |
| `tb task create --project SLUG --title TITLE [--column COLUMN] [--urgent]` | TaskDetail |
| `tb task list --project SLUG [--column COLUMN]` | TaskSummary[] |
| `tb task show DISPLAY_ID` | TaskDetail |
| `tb task update DISPLAY_ID [--title TITLE]` | TaskDetail |
| `tb task move DISPLAY_ID COLUMN` | TaskDetail |
| `tb task prioritize DISPLAY_ID --before DISPLAY_ID` / `--end` | TaskDetail |
| `tb task urgent DISPLAY_ID on\|off` | TaskDetail |
| `tb task delete DISPLAY_ID` | TaskDetail |
| `tb task restore DISPLAY_ID` | TaskDetail |
| `tb note add DISPLAY_ID --text TEXT` / `--file FILE` | TaskDetail |
| `tb note set DISPLAY_ID --text TEXT` / `--file FILE` | TaskDetail |
| `tb link add DISPLAY_ID --url URL` / `--path PATH` | TaskDetail |
| `tb link remove LINK_UUID` | TaskDetail |
| `tb run start DISPLAY_ID --agent NAME [--session ID]` | Run |
| `tb run update RUN_ID [--message TEXT]` | Run |
| `tb run wait RUN_ID --reason TEXT` | Run |
| `tb run fail RUN_ID --summary TEXT` | Run |
| `tb run finish RUN_ID --summary TEXT` | Run |
| `tb trash list` | Trash |
| `tb undo` | UndoResult |
| `tb backup export FILE` | `{ "ok": true, "path": "..." }` |
| `tb backup import FILE` | `{ "ok": true }` |
| `tb serve [--port N] [--open]` | Plan 2. Plan 1 prints `error: tb serve is not available in this build` and exits `1` |

Human `task list` columns: `ID  COLUMN  URGENT  RUN  TITLE`. Urgent prints `U` or `-`. Run prints `idle`, `running`, `waiting`, `failed`, or `completed`.

`--all` on `project list` includes archived. Default list excludes archived and deleted.

## 8. HTTP API

Bound to `127.0.0.1` only. Random free port unless `--port` is set. If `--port` is set and bind fails, `port_unavailable`. If `--port` is omitted and the first candidate fails, try another port.

Every boot generates a 32-byte session token, hex-encoded. The HTML bootstrap page is served only from `/` on that origin. The page receives the token in the first-party document. Mutations require header `X-Taskboard-Session`. Reads also require it except `GET /` and static assets.

`Origin` on mutation requests must be exactly `http://127.0.0.1:{port}`. Any other origin is `403` with code `forbidden_origin`. Missing session is `401` with code `unauthorized`.

DTO JSON uses the same field names as CLI `--json` entities.

| Method | Path | Use case |
| --- | --- | --- |
| GET | `/api/v1/bootstrap` | `{ "session": "...", "activitySequence": N }` |
| GET | `/api/v1/sync?after=N` | `SyncDelta` |
| GET | `/api/v1/projects` | `project_list` |
| POST | `/api/v1/projects` | `project_add` |
| PATCH | `/api/v1/projects/{slug}` | `project_update` / archive / note |
| POST | `/api/v1/projects/reorder` | `project_reorder` body `{ "slugs": [] }` |
| DELETE | `/api/v1/projects/{slug}` | `project_delete` |
| POST | `/api/v1/projects/{slug}/restore` | `project_restore` |
| GET | `/api/v1/projects/{slug}/tasks` | `task_list` |
| POST | `/api/v1/projects/{slug}/tasks` | `task_create` |
| GET | `/api/v1/tasks/{displayId}` | `task_show` |
| PATCH | `/api/v1/tasks/{displayId}` | title, note, urgent, column, reorder |
| DELETE | `/api/v1/tasks/{displayId}` | `task_delete` |
| POST | `/api/v1/tasks/{displayId}/restore` | `task_restore` |
| POST | `/api/v1/tasks/{displayId}/links` | `link_add` |
| DELETE | `/api/v1/links/{id}` | `link_remove` |
| POST | `/api/v1/tasks/{displayId}/runs` | `run_start` |
| PATCH | `/api/v1/runs/{displayId}` | update / wait / fail / finish via `{ "op": "wait", ... }` |
| GET | `/api/v1/trash` | `trash_list` |
| POST | `/api/v1/undo` | `undo` |
| POST | `/api/v1/backups/export` | body `{ "path": "..." }` |
| POST | `/api/v1/backups/import` | body `{ "path": "..." }` |

Request JSON uses camelCase to match TypeScript. Rust serde rename is `rename_all = "camelCase"` on API DTOs only. CLI `--json` uses snake_case to match Rust field names in the product spec examples. This split is intentional: CLI is the machine interface for agents; HTTP is the UI interface.

Revision header: `If-Match: "{revision}"`. Absent header means unconditional, matching CLI without `--revision`.

## 9. Screens and state

### Desktop and web share these regions

1. **Sidebar** (dark): project list, New Project, Archived toggle, selected project name, project note editor, Trash entry.
2. **Board** (light): four columns Todo / In Progress / In Review / Done. Column headers show live counts. Horizontal scroll when the window is narrower than 1100px.
3. **Inspector** (collapsible, 320–420px): selected card. Empty state copy: `Select a card`.
4. **Search**: `/` opens an input above the board that filters card titles in the current project. Esc clears.

### Empty states

- No projects: sidebar shows `Create a project to start a board` and a primary New Project control.
- Project with no tasks: each column shows a dashed slot; keyboard `n` creates a card in Todo.
- Trash empty: `Trash is empty`.

### Keyboard

| Key | Action |
| --- | --- |
| `j` / `k` | next / previous card in the current column |
| `h` / `l` | previous / next column |
| `1`–`4` | jump to column |
| `[` / `]` | previous / next project |
| `n` | new task in current column |
| `p` | new project |
| `u` | toggle urgent |
| `e` | focus inspector title |
| `/` | search |
| `Delete` or `Backspace` | soft-delete selected card (confirm if title focused is not the case; when board focused, delete) |
| `Cmd+Z` / `Ctrl+Z` | undo |
| `Esc` | collapse inspector / clear search |
| `Enter` | open inspector |

### Card face

- Title (one line, ellipsis)
- `TASK-n` in muted type
- Urgent pip when urgent
- Run badge: amber `Waiting`, indigo `Running`, red-muted `Failed`, green `Done` for completed runs. No badge for Idle
- Latest run message truncated to one line when Running or Waiting

### Inspector sections, top to bottom

Title, display ID, column select, urgent switch, note editor, links list, run history (newest first), activity history (newest first), Delete / Restore.

### State transitions

```text
Board loaded
  -> select project (first live project, else empty state)
  -> select card (optional)
  -> inspector open if a card is selected

Mutation from UI or poll
  -> if sequence advanced, apply SyncDelta
  -> keep selection when the selected entity still exists
  -> if selected task was deleted, close inspector and keep project
```

Drag-and-drop uses dnd-kit. Drop on a column moves to that column. Drop on a card uses `--before` that card. Keyboard `h`/`l` is equivalent to move.

## 10. Config, logs, backups

`{data_dir}/config.toml`:

```toml
log_level = "info"          # error | warn | info | debug
poll_interval_ms = 1000
note_debounce_ms = 400
backup_retention = 30
busy_timeout_ms = 5000
```

Missing file uses these defaults. Unknown keys are ignored. The process does not write secrets. There are no account fields.

Logs: `{data_dir}/logs/taskboard.log`. Rotate at 2 MiB, keep 3 files. Do not write `note_markdown`, run `summary`, or `repo_path`.

Routine backup: at most one successful backup per local calendar day in `{data_dir}/backups/routine-YYYY-MM-DD.sqlite3`. Keep the newest `backup_retention` routine files. Use SQLite backup API.

`tb backup export FILE` writes a consistent copy to `FILE`. `tb backup import FILE` first writes `backups/pre-import-{timestamp}.sqlite3` from the current DB, then replaces the current DB with `FILE`. Import fails if `FILE` is not a Taskboard database (missing `projects` table). After import, the process must not keep using the old connection.

## 11. Transport, UI packages, generated types

```text
packages/types    generated from crates/core DTO structs during `pnpm generate`
packages/client   Transport interface + TauriTransport + HttpTransport
packages/ui       board, card, inspector, sidebar, note editor
apps/web          Vite app using HttpTransport
apps/desktop      Tauri 2 window loading the same UI with TauriTransport
```

`Transport` methods match `App` use cases 1:1 (`projectAdd`, `taskMove`, …). Desktop never talks HTTP. Web never talks Tauri.

Type generation: `crates/core` types that appear on the wire are annotated and emitted as `packages/types/src/index.ts`. Do not hand-edit that file.

Accessibility for MVP:

- Every column is a `role="list"` with `aria-label` equal to the column title.
- Every card is `role="listitem"` and `aria-grabbed` during drag.
- Focus ring is visible on keyboard focus (`:focus-visible`).
- Text contrast on the light board is at least 4.5:1.
- Inspector fields have `<label>` elements.

## 12. Distribution

- CLI release artifact: `taskboard-aarch64-apple-darwin` plus a `tb` symlink in the tarball.
- Homebrew formula `taskboard` in tap `AI1411/taskboard` installs both names, skipping `tb` when a foreign binary exists (`alias_skipped` printed during `brew postinstall`).
- Desktop: `.dmg` for Apple silicon, macOS 14+.
- Linux desktop packaging is out of scope. Linux remains the Cloud Agent / CI target for crate and CLI tests.

## 13. Testing mapping

| Layer | Command | Must cover |
| --- | --- | --- |
| core | `cargo test -p taskboard-core` | column enum, slug, display status, urgency order, validation |
| application + sqlite | `cargo test -p taskboard-application -- --test-threads=1` and `cargo test -p taskboard-store-sqlite` | revisions, reorder property tests, concurrent writers, trash purge, undo |
| cli | `cargo test -p taskboard-cli` | JSON contracts, help snapshots, `taskboard` vs `tb` argv, temp `TASKBOARD_DATA_DIR` |
| api | `cargo test -p taskboard-api` | session, Origin rejection, If-Match, bind 127.0.0.1 |
| ui | `pnpm test` | card badge, column order, keyboard, transport fake |
| web e2e | `tb serve` + browser tests | token, origin, poll refresh |
| desktop | Tauri smoke on macOS | launch, create, move, undo, CLI refresh |

Network-disabled success criterion: CLI and SQLite tests set `HTTP_PROXY` empty and do not perform outbound requests. `tb serve` binds localhost only.

## 14. Ambiguities resolved from the product spec

1. **UUID vs `TASK-142`.** Both exist. UUID is storage. `TASK-n` / `RUN-n` are the human and CLI IDs.
2. **`Idle` run state.** Derived, never inserted.
3. **`tb note add` vs a single note document.** Add appends a paragraph; the document remains one field.
4. **Install collision on `tb`.** Canonical binary always installs. Alias is best-effort.
5. **Redacted activity.** Redaction is for diagnostic export. The DB stores full snapshots for undo.
6. **CLI JSON naming vs HTTP.** snake_case on CLI, camelCase on HTTP.
7. **Project identity.** Slug, not a display ID.
8. **Data directory on Linux.** Defined so Cloud Agent and CI can run Plan 1 without macOS.

## 15. Out of scope (unchanged)

Cloud sync, accounts, billing, reading agent session files, inferring progress from process output, GitHub/Linear/Jira, chatting with or launching an agent, Windows/Linux desktop distribution, custom columns, mobile clients, Japanese UI, notifications, file attachments as blobs.
