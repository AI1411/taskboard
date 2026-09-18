# Share TestApp harness (#156) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One `tests/common` harness for `TestApp` / `test_app` / `cli_actor` / seeders used by application integration tests.

**Architecture:** `crates/application/tests/common/mod.rs` owns the boilerplate. Each `tests/*.rs` does `mod common; use common::*;` and drops local copies. Keep `stale.rs` SharedClock locally.

**Tech Stack:** Rust integration tests, tempfile, SqliteStore.

## Global Constraints

- Behavior-preserving scenarios
- No production API changes for tests

User already chose sequential inline execution.

## File map

- Create: `crates/application/tests/common/mod.rs`
- Modify: all `crates/application/tests/*.rs` that redefine TestApp (except SharedClock bits in stale)

### Task 1

```rust
// common/mod.rs
pub struct TestApp { pub app: App, _tmp: TempDir }
impl TestApp { pub fn path(&self) -> &Path { self._tmp.path() } }
impl Deref<Target=App> for TestApp { ... }
pub fn cli_actor() -> Actor { ... }
pub async fn test_app() -> TestApp { ... }
pub async fn seed_project(app: &App, name: &str) -> String { /* returns slug */ }
pub async fn create_task(app: &App, project_slug: &str, title: &str, column: Option<Column>, urgent: bool) { ... }
```

- [ ] Add common module; migrate all SystemClock TestApp files; leave stale SharedClock
- [ ] `cargo test -p taskboard-application -- --test-threads=1`
- [ ] PR

## Self-review

Majority of files use shared harness; boilerplate not re-copied — yes.
