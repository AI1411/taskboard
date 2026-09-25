---
name: using-taskboard
description: Track implementation work on the local Taskboard CLI. Use when implementing, fixing, debugging, or changing code, and when starting or finishing an agent coding session that should appear on the local Taskboard.
---

# Using Taskboard

Track implementation work with the local Taskboard CLI. Do not skip this because the task is small, already described in chat, or "just a quick edit." If the repo has `AGENTS.md` with a Taskboard section, follow that contract.

## CLI

Resolve the binary once per session:

1. `tb` if it is on `PATH`
2. else `taskboard` if it is on `PATH`
3. else on Linux, fetch the latest release into the current directory (then run `./taskboard`):

```bash
triple=$(uname -s)-$(uname -m); case $triple in Linux-aarch64|Linux-arm64) triple=aarch64-unknown-linux-gnu ;; Linux-*) triple=x86_64-unknown-linux-musl ;; *) triple= ;; esac; [ -z "$triple" ] || curl -fsSL "https://github.com/AI1411/taskboard/releases/latest/download/taskboard-${triple}.tar.gz" | tar -xz
```

4. else `cargo run --quiet --bin taskboard --` from a Taskboard checkout

Set `<agent>` to the host: `cursor`, `claude`, or `codex`. Always pass `--json` and `--actor <agent>` (or set `TASKBOARD_ACTOR`). Parse stdout JSON. Success is `"ok": true` with `entity` / `entities` and `revision`. Failure is `"ok": false` with `error.code` and a nonzero exit.

Use MCP (`tb mcp`) when the host has the local stdio config from `packaging/mcp/` (or the `mcp.json` snippet in the README); otherwise use this CLI. Do not call the localhost HTTP API. Do not identify tasks by title; use `TASK-n`. After a claim (`next` or `run start`), use the returned `RUN-n`. If you forget `RUN-n`, use `run current TASK-n` or `run list --open`. `task list --all --status running,waiting` lists cards across projects.

## Session start

Do this **before the first edit** when the user asked you to implement, fix, or change something:

1. `project detect --json` — if `project_required`, `project list --json` then `project add --name <name> --path <repo-root> --json`. If `task show` has `worktree_path`, `cd` there before the first edit.
2. `status --json` — board-wide snapshot (inbox / open / stale / ready / in-review / blocked).
3. `occupancy --json` when this checkout may already have an open run — collisions are grouped by `worktree_path`.
4. Claim an existing card. Prefer `next --json` (first ready card + start a run) or `next --move --json` (also moves the card to `in-progress`). If you already know `TASK-n`, `run start TASK-n --agent <agent> --exclusive --json`. Create only when there is no card: `task create --project <slug> --title "<short title>" --json`, then `task move TASK-n in-progress --json`, then `run start TASK-n --agent <agent> --json`.
5. Keep `RUN-n`. If a later `run start --exclusive` returns `conflict`, that card already has a running or waiting run — pick another card or `run current TASK-n`.

Do not invent a combined claim command. `next` and `run start --exclusive` already claim existing cards.

## While working

- Progress: `run update RUN-n --message "<status>" --json`
- Split: `task spawn TASK-n --title "<child>" --json` (repeat `--title` for more children). The parent becomes blocked-by the children; do not move columns.
- Waiting: `run wait RUN-n --reason "<why>" --json`. Replies go on the comment thread, not the spec note. Prefer `comment add TASK-n --text "..." --continue --json` (comment + resume) or `run continue RUN-n --reply "..." --json`. `comment add` / `comment list` without `--continue` stay available. Waiting cards expose the latest comment as `reply` on `task show` / `task list`.
- Dead Running (stuck or superseded): `run cancel RUN-n --summary "<why>" --json`. Do not leave a zombie Running.
- Definition-of-done items live on `check add` / `check toggle` / `check list` (`CHECK-n`), not the spec note, and do not block `run finish` or Done.

## Session end

When the work is done or failed, **before the final reply**:

- success: `run finish RUN-n --summary "<what changed>" --json`, then `task move TASK-n in-review` or `done`
- failure: `run fail RUN-n --summary "<what failed>" --json`

`run finish` does **not** move the card. Columns: `todo`, `in-progress`, `in-review`, `done`.

Human review of an In Review card: `review TASK-n --approve --text "<why>" --json` (moves to `done`) or `review TASK-n --changes --text "<why>" --json` (moves to `in-progress`, does not start a run).

## Rationalizations (all invalid)

| Excuse | Reality |
| --- | --- |
| Too small for the board | Still create or reuse a task and a run |
| User didn't ask to update Taskboard | These instructions require it |
| I'll record it at the end | Start the run before the first edit |
| The chat already is the task list | The board is the task list |
| I'll create+move+start even though a card exists | Existing cards use `next` or `run start --exclusive` |

Questions-only turns (no repo changes) may skip the board.
