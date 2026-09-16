# Per-card worktree and branch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record optional `worktree_path` and `branch` on a task. `task show` and `run current` return them. The app never runs git.

**Architecture:** Persist on `tasks`. `TaskUpdate` can set or clear either field. `Run` copies the parent task fields at read time so `run current` includes them. Card/inspector display only. Skills say: cd into `worktree_path` before the first edit.

**Tech Stack:** Rust crates, SQLite `0004_task_workspace.sql`, clap, React, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- No git subprocess
- CLI `--json` is snake_case
- Prefer CLI/domain

## File map

- `crates/core` — fields on `Task`, `TaskSummary`, `TaskDetail`, `Run`
- `crates/store-sqlite` — `0001` + `0004_task_workspace.sql`
- `crates/application` — `TaskUpdate` set/clear + tests
- `crates/cli` — `task update --worktree --branch`
- DTOs, types, Card, Inspector, AGENTS.md / skill

User already chose sequential inline execution.
