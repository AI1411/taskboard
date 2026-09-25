# CI typecheck and desktop check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CI runs `tsc`, the web build, and a macOS desktop-crate check, and `main` typechecks clean.

**Architecture:** Each TS package gets `typecheck`. The root `pnpm typecheck` runs them. The JS job also builds `web`. A `macos-14` job formats, clippy-checks, and `cargo check`s `apps/desktop/src-tauri`. Desktop UI sources must not import `HttpTransport`.

**Tech Stack:** TypeScript 5.9, pnpm, GitHub Actions, Rust 1.83, vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Do not add a full eslint setup
- Desktop crate stays outside the workspace so Linux `cargo test` does not need webkit
- Workspace CI stays on Rust 1.83.0
- The desktop job uses Rust 1.88.0 because the Tauri crate graph needs edition2024

User already chose sequential inline execution.

## File map

- Modify: `packages/client/tsconfig.json` (`lib` includes `DOM`)
- Modify: `packages/ui/src/Inspector.tsx` (narrow `task` after the null guard)
- Modify: `packages/ui/src/Inspector.test.tsx` (Run fixture includes `worktreePath` and `branch`)
- Modify: package `typecheck` scripts and root `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `apps/desktop/src-tauri` via `cargo fmt`
- Create: `apps/desktop/src/importBoundary.test.ts`
- Modify: `CHANGELOG.md`

---

### Task 1: Make tsc pass

- [ ] **Step 1: Run `tsc --noEmit` in client, ui, web, and desktop and record the errors**

Expected today: `RequestInfo` missing in client; `props.task` possibly null inside Inspector closures; one Run fixture missing `worktreePath` and `branch`.

- [ ] **Step 2: Fix those sites**

`packages/client/tsconfig.json` lib becomes `["ES2022", "DOM"]`.

After `if (!props.open || !props.task) return null`, bind `const task = props.task` and use `task` inside closures that read the card.

Add `worktreePath: null, branch: null` to the history-run fixture.

- [ ] **Step 3: Re-run tsc**

Expected: exit 0 for client, ui, web, and desktop.

### Task 2: CI scripts and desktop boundary

- [ ] **Step 1: Add `apps/desktop/src/importBoundary.test.ts`**

Walk `apps/desktop/src` and fail if a `.ts` or `.tsx` file other than the test imports `HttpTransport`.

- [ ] **Step 2: Add `typecheck` scripts and the CI steps**

Root script `pnpm typecheck`. JS job runs `pnpm typecheck` and `pnpm --filter web build`. New `desktop` job on `macos-14` installs Rust 1.83.0 with rustfmt and clippy, then:

```bash
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

- [ ] **Step 3: Format the desktop crate and run the new checks locally**

`cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml`, then `pnpm typecheck`, `pnpm --filter web build`, and `pnpm --filter @taskboard/desktop test`.

- [ ] **Step 4: Commit**
