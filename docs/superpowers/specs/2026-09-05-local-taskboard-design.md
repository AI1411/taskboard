# Local Taskboard Design

## 1. Summary

Local Taskboard is a macOS-first, fully local task manager for people running several development tasks at once. Each project has a four-column Kanban board. Users and command-line tools can create, prioritize, move, annotate, review, complete, and delete cards. A card can retain multiple agent-run records so that implementation, review, and follow-up work remain visible in one place.

The product is available through three interfaces backed by one local database:

- A Tauri desktop application for normal use.
- A localhost-only web application started with `tb serve`.
- The `taskboard` command-line application, also installed as `tb`.

No account, cloud service, telemetry, LAN server, or cross-device synchronization is included.

## 2. Goals

- Show the status and priority of every active task per project in one view.
- Make the next task requiring attention visually obvious.
- Allow both people and command-line tools to manage the complete board.
- Preserve task notes, project notes, run history, and a reversible activity history.
- Keep all repository names, paths, task text, and run details on the Mac.
- Share domain rules between desktop, web, and CLI instead of implementing them three times.

## 3. Non-goals

- Cloud synchronization or multi-user collaboration.
- Accounts, authentication, or subscription billing.
- Reading the private internal session formats of Codex, Claude Code, or Cursor.
- Automatically inferring task progress from process output.
- Replacing GitHub Issues, Linear, or Jira.
- Chatting with or launching an agent from the application.
- Windows or Linux desktop distribution in the first release.

## 4. Target platform

- macOS 14 or later.
- Apple silicon is the supported release target for the MVP.
- The localhost web UI runs in current Safari, Chrome, and Firefox versions on the same Mac.
- The CLI is distributed as a standalone arm64 binary and through a Homebrew tap.

Intel macOS builds can be added later if user demand justifies the additional release and test surface.

## 5. Core interaction model

### Projects

The left sidebar lists projects. A project has a name, optional repository path, archived flag, and Markdown project note. Selecting a project opens its board. Project order is manually adjustable.

### Kanban board

Every project uses the same fixed workflow:

1. Todo
2. In Progress
3. In Review
4. Done

Cards are manually ordered within a column. An `Urgent` flag pins a card above non-urgent cards in that column. Urgent cards can still be ordered relative to other urgent cards.

### Task details

Selecting a card opens a right-hand inspector containing:

- Title and task ID.
- Current Kanban column.
- Urgent state.
- Markdown task note.
- Linked URLs and local file paths.
- Agent-run history.
- Activity history.
- Delete and restore controls.

### Run status

Kanban workflow and run status are separate concepts. Finishing a run does not automatically move its task to Done.

Run states are:

- Idle
- Running
- Waiting
- Failed
- Completed

A card can contain multiple runs from different tools or repeated attempts. Each run records its agent/tool name, optional external session ID, status, current message, waiting reason, summary, start time, and end time.

### Notes

Both project notes and task notes use Markdown. Notes save automatically after a short debounce. Every saved version is recoverable from activity history.

## 6. Visual design

The approved direction is `Native Glass`:

- A dark, compact project sidebar.
- A light board surface with restrained translucency and depth.
- Rounded cards with subtle borders rather than heavy shadows.
- Purple and indigo as product accents.
- Orange-red only for Urgent.
- Amber only for Waiting or attention-required states.
- Green only for successful completion.
- Motion is limited to short card movement, inspector transitions, and state changes.

The desktop layout has three regions:

1. Project navigation and project note on the left.
2. Four-column Kanban board in the center.
3. Selected-card details on the right.

The window supports horizontal board scrolling at smaller widths. The inspector can collapse. Keyboard navigation must support project switching, card selection, card movement, search, and new-card creation.

## 7. CLI contract

The canonical executable name is `taskboard`; `tb` is an installed alias. Installation must not overwrite an unrelated existing `tb` executable. If a collision is detected, `taskboard` remains usable and installation reports the skipped alias.

Representative commands:

```bash
tb project add --name renai-sim --path /path/to/repo
tb project list --json

tb task create --project renai-sim --title "Fix login error"
tb task list --project renai-sim --json
tb task show TASK-142 --json
tb task move TASK-142 in-progress
tb task prioritize TASK-142 --before TASK-108
tb task urgent TASK-142 on
tb task delete TASK-142
tb task restore TASK-142

tb note add TASK-142 --text "Check compatibility with existing sessions"
tb project-note set renai-sim --file project-note.md

tb run start TASK-142 --agent codex --session abc123
# Returns a run ID such as RUN-37.
tb run update RUN-37 --message "Adding test cases"
tb run wait RUN-37 --reason "Specification decision required"
tb run finish RUN-37 --summary "Implementation and verification complete"
```

All commands support human-readable output. Read and mutation commands support `--json`. Successful JSON responses include the updated entity and revision. Error responses use stable error codes and a nonzero process exit code.

Task mutations require an exact task ID. A title alone is insufficient because titles are not unique. Commands that update an existing record accept an optional `--revision` argument for strict optimistic concurrency.

Run mutations after `run start` require the returned run ID, because one task may have concurrent or historical runs. CLI activity uses `--actor` when supplied, then the `TASKBOARD_ACTOR` environment variable, and otherwise records the actor as `local-cli`.

## 8. Architecture

### Technology choices

- Desktop shell: Tauri 2
- Shared UI: React, TypeScript, and Vite
- Drag and drop: dnd-kit
- Server-state cache: TanStack Query
- Styling: CSS Variables and CSS Modules
- Core and application logic: Rust
- CLI parsing: clap
- Local HTTP server: Axum
- Database: SQLite in WAL mode
- Database access and migrations: SQLx
- Serialization: Serde
- Async runtime: Tokio
- Entity IDs: UUID version 7
- Stored timestamps: UTC

### Repository layout

```text
apps/
  desktop/          Tauri desktop shell
  web/              React and Vite application

crates/
  core/             Entities, validation, and domain rules
  application/      Use cases and transaction boundaries
  store-sqlite/     SQLite repositories and migrations
  api/              Local Axum HTTP API
  cli/              taskboard and tb executable

packages/
  ui/               Kanban and inspector components
  client/           Tauri and HTTP transport adapters
  types/            Generated TypeScript DTO definitions
```

The UI calls a transport interface rather than Tauri or HTTP directly. Desktop uses `TauriTransport`; localhost web uses `HttpTransport`. Both transports expose the same application operations. Rust DTO definitions generate the corresponding TypeScript definitions during the build.

The CLI calls the Rust application layer directly. It does not depend on the desktop app or local server being open.

### Local web mode

`tb serve` starts an Axum server bound only to `127.0.0.1`. The port is randomly selected unless the user supplies `--port`. `tb serve --open` opens the default browser.

Each server start creates an unguessable session token. The generated page receives it through a secure bootstrap response, and every mutation request must include it. The server validates the Origin header and rejects requests from other origins. This prevents an unrelated website from silently modifying the local board through localhost requests.

## 9. Persistence and concurrency

The default data directory is:

```text
~/Library/Application Support/Taskboard/
├── taskboard.sqlite3
├── backups/
├── logs/
└── config.toml
```

SQLite runs in WAL mode with a bounded busy timeout. Each mutable entity has an integer revision. Mutations update records only when the expected revision still matches. A conflict returns the current entity without overwriting it.

Card reordering occurs in a single transaction. Each card has an integer position within its project and column. The transaction rewrites the affected column positions into a contiguous sequence. This is appropriate for a personal local board and avoids fractional-rank precision or normalization complexity.

Desktop and localhost web clients poll the latest activity sequence once per second while visible. They refetch changed entities only when the sequence advances. This makes CLI-originated changes appear without requiring a permanent background daemon.

## 10. Activity, deletion, and Undo

Every successful mutation creates an activity record containing:

- Monotonic local sequence.
- Actor type and actor label.
- Operation name.
- Entity type and ID.
- Previous revision.
- Redacted before and after representations.
- UTC timestamp.

Deleting a task or project performs a soft delete. Deleted items remain in Trash for 30 days and can be restored. Automatic purging permanently removes expired Trash entries and their nonessential payloads.

Undo creates a new compensating mutation; it does not erase history. Undo is rejected when a newer incompatible change makes automatic reversal unsafe. The UI then shows the conflicting fields and offers manual restoration.

## 11. Backups and privacy

- The app creates a backup before every schema migration.
- It creates at most one routine backup per day and retains the latest 30 routine backups.
- Backup creation uses SQLite's consistent backup mechanism rather than copying a live database file directly.
- No telemetry, crash upload, or network sync is enabled.
- Repository paths are displayed locally and excluded from any manually exported diagnostic log unless explicitly requested.
- Application logs do not include note bodies or run summaries.

## 12. Error handling

- Validation errors identify the exact field and allowed values.
- Revision conflicts return the current revision and actor responsible for the intervening change.
- Database-busy errors retry briefly and then return a stable `database_busy` error without hanging.
- A malformed note or unknown Markdown syntax remains stored as plain text; it never blocks saving.
- If `tb serve` cannot bind its requested port, it chooses another port unless the port was explicitly required.
- If a database migration fails, the app leaves the original database untouched and offers the pre-migration backup.
- A failed CLI mutation never emits a success-shaped JSON object.

## 13. Testing strategy

### Rust core

- Unit tests for workflow validation, Urgent ordering, revisions, soft deletion, and compensating Undo.
- Property tests for arbitrary card-reordering sequences.
- Concurrent-write tests with separate SQLite connections.

### CLI

- Snapshot tests for help and human-readable output.
- JSON contract tests for every command and error type.
- End-to-end tests using a temporary application data directory.
- Alias tests proving `taskboard` and `tb` behave identically.

### UI

- Component tests for cards, board columns, notes, run badges, and inspector history.
- Drag-and-drop tests for same-column and cross-column moves.
- Keyboard-navigation and accessibility tests.
- Transport-contract tests run against both Tauri and HTTP adapters.

### Desktop and local web

- Tauri smoke tests for launch, create, move, undo, and external CLI refresh.
- Browser tests for `tb serve`, session-token enforcement, Origin rejection, and live refresh.
- Visual regression checks for the approved Native Glass board at supported window sizes.

## 14. MVP scope

The first release includes:

- Project creation, editing, ordering, archiving, and project notes.
- Fixed four-column Kanban boards.
- Card creation, editing, ordering, Urgent pinning, soft deletion, and restoration.
- Markdown task notes.
- Multiple run records per card.
- Activity history and safe Undo.
- `taskboard` and `tb` CLI commands.
- Native Glass desktop UI.
- `tb serve` localhost web UI.
- SQLite backups and import/export of a complete local backup.

The first release excludes automatic agent discovery, external issue trackers, notifications, file attachments, cloud features, team features, custom columns, and mobile clients.

## 15. Success criteria

- A new project and first task can be created in under one minute without documentation.
- A user can identify all Running, Waiting, and Urgent cards without opening the inspector.
- A CLI mutation appears in an open desktop or localhost web view within two seconds.
- Concurrent conflicting edits are detected rather than silently overwritten.
- Any soft-deleted item can be restored during the retention period.
- Desktop, web, and CLI pass the same domain-rule contract suite.
- The application remains fully functional with network access disabled.
