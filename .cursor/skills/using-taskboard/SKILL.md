---
name: using-taskboard
description: Use when implementing, fixing, debugging, or changing code in this repository, and when starting or finishing any agent coding session that should appear on the local Taskboard.
---

# Using Taskboard

Track work with the local Taskboard CLI. Full contract: `AGENTS.md`.

Resolve `$TB` as `tb`, else `taskboard`, else `cargo run --quiet --bin taskboard --`. Always `--json --actor cursor`.

1. Before the first edit: ensure project `taskboard`, create or reuse `TASK-n`, `task move` to `in-progress`, `run start --agent cursor`.
2. Optionally `run update` / `run wait` while working.
3. Before the final reply: `run finish` or `run fail`, then `task move` to `in-review` or `done`.

`run finish` does not move the card. Do not use the HTTP API. Do not identify tasks by title.
