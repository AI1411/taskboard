# Stale running runs (heartbeat)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface running runs whose `updated_at` is older than N minutes (default 30) via `tb stale` and a card-face Stale badge. No new table.

**Architecture:** Derived query on existing `runs`. `App::stale_list(minutes)` returns running runs where `now - updated_at >= minutes`. `TaskSummary.stale` is true when the winning run is stale at the default 30-minute threshold. Card shows a `Stale` badge instead of `Running` when `stale` is true.

**Tech Stack:** Existing Rust crates, clap, React card, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Inbox membership unchanged
- No new table, no OS notification, no auto-fail
- CLI `--json` is snake_case
- Prefer CLI/domain; card badge is the only UI

## File map

- Modify: `crates/core/src/models.rs` — `TaskSummary.stale`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/stale.rs`
- Modify: `crates/cli/src/{args.rs,main.rs,output.rs}`
- Modify: `crates/cli/tests/{cli_json.rs,help.rs}`
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs`
- Modify: `packages/types/src/index.ts`, `packages/ui/src/{summary.ts,fakeTransport.ts,Card.tsx,Card.test.tsx,Card.module.css,columns.ts}`

---

### Task 1: `App::stale_list` and `TaskSummary.stale`

**Interfaces:**
- `App::stale_list(&self, minutes: i64) -> Result<Vec<Run>, AppError>`
- `minutes < 1` → `validation_error` field `minutes`
- Running + `now - updated_at >= Duration::minutes(minutes)`
- Waiting/failed/completed never stale
- `TaskSummary.stale` uses default 30 minutes and `self.clock.now()`
- Use a stepping `Clock` in tests

- [ ] Write failing tests in `crates/application/tests/stale.rs`
- [ ] Run: `cargo test -p taskboard-application --test stale` (FAIL: missing method)
- [ ] Implement `stale_list` + `stale` on summaries
- [ ] Run tests (PASS)
- [ ] Commit

### Task 2: CLI `tb stale`

- `tb stale [--minutes 30]`
- JSON `entities` of runs
- Human: `ID  AGENT  TASK  UPDATED`
- [ ] Failing CLI test: fresh run is absent from `--minutes 30`; `--minutes 0` is validation_error
- [ ] Implement
- [ ] Commit

### Task 3: Card Stale badge

- `stale: boolean` on TaskSummary
- Badge text `Stale`, class distinct from Waiting
- [ ] Failing Card test
- [ ] Implement
- [ ] Commit

User already chose sequential inline execution.
