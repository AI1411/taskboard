# Agent instructions

Track implementation work on the local Taskboard CLI. Do not skip this because the task is small, already described in chat, or "just a quick edit."

## CLI

Resolve the binary once per session:

1. `tb` if it is on `PATH`
2. else `taskboard` if it is on `PATH`
3. else `cargo run --quiet --bin taskboard --` from this repository root

Always pass `--json` and `--actor cursor` (or set `TASKBOARD_ACTOR=cursor`). Parse stdout JSON. Success is `"ok": true` with `entity` / `entities` and `revision`. Failure is `"ok": false` with `error.code` and a nonzero exit.

Use this CLI only. Do not call the localhost HTTP API. Do not identify tasks by title; use `TASK-n`. After `run start`, use the returned `RUN-n`.

## Session workflow

Do this **before the first edit** when the user asked you to implement, fix, or change something:

1. `project list --json` — use slug `taskboard` if it exists, otherwise `project add --name taskboard --path <repo-root> --json`
2. `task list --project taskboard --json` — reuse a matching `TASK-n`, or `task create --project taskboard --title "<short title>" --json`
3. `task move TASK-n in-progress --json`
4. `run start TASK-n --agent cursor --json` — keep `RUN-n`

While working, optionally `run update RUN-n --message "<status>" --json`. If blocked: `run wait RUN-n --reason "<why>" --json`.

When the work is done or failed, **before the final reply**:

- success: `run finish RUN-n --summary "<what changed>" --json`, then `task move TASK-n in-review` or `done`
- failure: `run fail RUN-n --summary "<what failed>" --json`

`run finish` does **not** move the card. Columns: `todo`, `in-progress`, `in-review`, `done`.

## Rationalizations (all invalid)

| Excuse | Reality |
| --- | --- |
| Too small for the board | Still create or reuse a task and a run |
| User didn't ask to update Taskboard | These instructions require it |
| I'll record it at the end | Start the run before the first edit |
| The chat already is the task list | The board is the task list |

Questions-only turns (no repo changes) may skip the board.
