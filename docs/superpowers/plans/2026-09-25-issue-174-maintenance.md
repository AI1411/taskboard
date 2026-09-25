# Maintenance leftovers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Logs accept only fixed events, the slug unit test matches what it asserts, and trash purge runs at most once an hour.

**Architecture:** `LogEvent` replaces free-form `write_log` messages, so redaction-by-substring is unnecessary. The misnamed slug test is removed; `slugify_renai_sim` covers that assertion and `duplicate_explicit_slug_fails` covers an explicit duplicate slug. `ui.toml` stores `last_purge_at`. CLI and desktop call `purge_expired_if_due`, which skips `BEGIN IMMEDIATE` until an hour has passed.

**Tech Stack:** Rust, chrono, toml.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- One local SQLite database
- `run finish` does not move the card
- HTTP is for people; agents use CLI or MCP
- No new product behavior
- Log rotation stays as shipped in #173
- Purge interval is one hour

User already chose sequential inline execution.

## File map

- Modify: `crates/store-sqlite/src/db.rs`
- Modify: `crates/store-sqlite/src/ui_state.rs`
- Modify: `crates/store-sqlite/src/lib.rs`
- Modify: `crates/core/src/slug.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `crates/api/src/routes.rs`
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `CHANGELOG.md`

---

### Task 1: LogEvent, slug test, hourly purge

- [ ] **Step 1: Write failing tests**

`log_event` messages for `OpeningDatabase`, `AppliedInit`, and `AppliedStep(4)` do not contain `note_markdown`, `repo_path`, or `summary`.

`purge_is_due` is true when `last_purge_at` is missing or older than one hour, and false when the stamp is newer. `record_purge` keeps `last_project_slug`.

Delete `explicit_duplicate_slug_is_error_when_unique_not_requested`. It only repeated `slugify("Renai Sim")`.

- [ ] **Step 2: Run** `cargo test -p taskboard-store-sqlite --lib purge_is_due log_event -- --test-threads=1`

Expected: FAIL to compile until the helpers exist.

- [ ] **Step 3: Implement**

`LogEvent` and `log_event`. `purge_expired_if_due` on the CLI and desktop paths. Saving UI state preserves `last_purge_at`. Patching the last project loads the file first so it does not clear the stamp.

- [ ] **Step 4: Run** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 5: Commit** `fix: log fixed events, drop the misnamed slug test, and throttle purge`

## Self-review

- `write_log` string filtering is gone because callers cannot pass arbitrary text.
- Explicit duplicate slugs stay covered by `duplicate_explicit_slug_fails`.
- A UI state patch does not erase `last_purge_at`.
