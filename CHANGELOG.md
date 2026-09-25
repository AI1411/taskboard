# Changelog

All notable product changes are listed here. The first tagged release is `v0.1.0`.

## [Unreleased]

- Migrations record `PRAGMA user_version`, each step is one transaction with a pre-migration backup, and import migrates through the same path. Import returns conflict when `taskboard.lock` is held. `logs/taskboard.log` rotates at 1 MiB.
- Inbox and status read runs, comments, and checks once per request, and the board fetches inbox once.
- CI typechecks the TypeScript packages, builds the web UI, and checks the desktop crate on macOS.

## [0.1.1] - 2026-09-25

- Release CLI and Homebrew HEAD embed the built web UI. A release build fails when `apps/web/dist/index.html` is missing, and the tag workflow refuses a binary whose `/` has no `<script`.
- Inbox includes completed In Review cards (before Urgent).
- Board writes worktree and branch; HTTP/desktop reject unknown task patch fields.
- Board status strip, occupancy line, and inspector spawn (human HTTP/desktop only).
- Comment and check writes record activity; undo hits the latest event; checks and the latest comment can be deleted.
- Contract tests lock `TaskSummary` / `TaskUpdate` / `EntityType` against `packages/types`.

## [0.1.0] - 2026-09-17

First tagged CLI release (`taskboard-aarch64-apple-darwin.tar.gz`) and Homebrew Formula.

- Local four-column board (desktop / `tb serve` / CLI) on one SQLite database.
- Agent contract: `tb next`, `run start --exclusive`, `status`, `occupancy`, `task spawn`, `review`, `comment add --continue`, `run cancel`.
- Local stdio MCP (`tb mcp`) with host snippets under `packaging/mcp/`.
- `brew tap AI1411/taskboard && brew install taskboard`.
