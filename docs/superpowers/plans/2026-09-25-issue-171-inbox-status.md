# Inbox and status batching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `inbox` and `status` stay under a loose time bound at 1000 tasks by reading related rows once per request.

**Architecture:** `inbox_inner` groups `list_all_runs` (plus one query per archived project) and decides membership from column, urgent, and run display before building an `InboxItem`. `task_query_inner` keeps a single `load_block_index` and groups bulk comment, check, and run rows. Migration `0005` adds the missing lookup indexes. The board fetches inbox once and filters by `projectSlug` on the client, then loads status and occupancy together.

**Tech Stack:** Rust, SQLite, sqlx, React, vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- One local SQLite database
- `run finish` does not move the card
- HTTP is for people; agents use CLI or MCP
- Inbox membership rules do not change
- CI time bound is loose (2000ms per call after the rows exist)

User already chose sequential inline execution.

## File map

- Create: `crates/application/tests/inbox_perf.rs`
- Create: `crates/store-sqlite/migrations/0005_lookup_indexes.sql`
- Create: `packages/ui/src/inboxScope.ts`
- Create: `packages/ui/src/inboxScope.test.ts`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/application/src/app/helpers.rs`
- Modify: `crates/application/src/app/board.rs`
- Modify: `crates/application/src/app/tasks.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/store-sqlite/src/migrations.rs`
- Modify: `crates/store-sqlite/src/db.rs`
- Modify: `crates/store-sqlite/migrations/0001_init.sql`
- Modify: `packages/ui/src/hooks/useBoardData.ts`
- Modify: `CHANGELOG.md`

---

### Task 1: Bulk reads and indexes

**Interfaces:**
- Produces: `Store::list_all_comments`, `Store::list_all_checks`, `Store::list_runs_for_project`
- Produces: `helpers::group_by_task`

- [ ] **Step 1: Write the failing perf and index tests**

`crates/application/tests/inbox_perf.rs` seeds 1000 live todo tasks (one urgent) with sqlx, then asserts `inbox` and `status` each finish in under 2000ms and the urgent card is the only inbox row.

`crates/store-sqlite/src/db.rs` asserts `idx_runs_task_id`, `idx_links_task_id`, and `idx_links_kind_value` exist after `open_db`, including when `idx_runs_task_id` was dropped first.

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p taskboard-application --test inbox_perf -- --nocapture`
Expected: FAIL (over 2000ms, or missing sqlx until the dev-dependency is added)

Run: `cargo test -p taskboard-store-sqlite --lib open_db_adds_lookup_indexes -- --nocapture`
Expected: FAIL (index missing)

- [ ] **Step 3: Implement bulk queries, inbox, task query, and migration 0005**

`0005_lookup_indexes.sql`:

```sql
CREATE INDEX IF NOT EXISTS idx_runs_task_id ON runs (task_id);
CREATE INDEX IF NOT EXISTS idx_links_task_id ON links (task_id);
CREATE INDEX IF NOT EXISTS idx_links_kind_value ON links (kind, value);
```

Append the same three statements to `0001_init.sql`. `open_db` applies `0005` when `idx_runs_task_id` is absent.

`list_all_comments` / `list_all_checks` use the same live non-archived task filter as `list_all_runs`, ordered `created_at ASC, id ASC` and `sort_order ASC, id ASC`. `list_runs_for_project` orders `started_at DESC, display_id DESC`.

`inbox_inner` does not call `to_task_summary` or `load_block_index`. Archived projects load runs with `list_runs_for_project`.

`task_query_inner` calls `load_block_index` once and looks up grouped runs, comments, and checks.

- [ ] **Step 4: Re-run the tests**

Expected: perf under 2000ms, index tests pass, existing `inbox` and `task_query` tests pass.

### Task 2: One inbox fetch on the board

**Interfaces:**
- Produces: `inboxItemsForScope(all, scope, slug)`

- [ ] **Step 1: Write `inboxScope.test.ts`**

A `"this"` scope keeps only `projectSlug === slug`. `"all"` returns every item.

- [ ] **Step 2: Run `pnpm --filter @taskboard/ui test -- src/inboxScope.test.ts`**

Expected: FAIL (module missing)

- [ ] **Step 3: Implement `inboxItemsForScope` and use it in `refreshInbox`**

`refreshInbox` calls `transport.inbox()` once. `status` and `occupancy` run in `Promise.all`.

- [ ] **Step 4: Re-run the ui tests**

Expected: PASS

- [ ] **Step 5: Commit**
