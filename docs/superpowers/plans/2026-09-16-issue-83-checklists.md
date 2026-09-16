# Definition-of-done checklists

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Machine-readable per-card checklists (`CHECK-n`) that do not live in the spec note and do not block `run finish` or Done.

**Architecture:** New `checks` table. `App::check_add` / `check_toggle` / `check_list`. `TaskDetail.checks` chronological by sort_order. `TaskSummary.checklist_done` / `checklist_total`. Card face `2/5`. Inspector lists checkboxes (display). Finish/move stay ungated.

**Tech Stack:** Rust crates, SQLite migrations, clap, React, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Comments/notes stay separate
- CLI `--json` is snake_case
- Prefer CLI/domain

## File map

- `crates/core` — `Check`, `DisplayKind::Check`, summary counts
- `crates/store-sqlite` — `0001` + `0003_checks.sql`, store methods
- `crates/application` — commands + app + tests
- `crates/cli` — `check add|toggle|list`
- DTOs, types, Card, Inspector

User already chose sequential inline execution.
