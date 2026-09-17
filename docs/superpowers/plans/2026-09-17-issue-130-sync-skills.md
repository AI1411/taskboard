# Sync skills and AGENTS.md to current verbs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `AGENTS.md` and the four `using-taskboard` skill copies onto the shipped session loop (`status`, `occupancy`, `next` / `--exclusive`, `comment add --continue`, `run cancel`, `task spawn`, `review`) so agents stop reinventing `create` → `move` → `run start`.

**Architecture:** Documentation-only contract sync. Canonical skill text lives in `skills/using-taskboard/SKILL.md` and is copied byte-for-byte to `.cursor` / `.claude` / `.agents`. `AGENTS.md` is the same loop with this repo's actor (`cursor`) and project (`taskboard`). A CLI integration test pins the copies and required verb names so the contract cannot drift back.

**Tech Stack:** Markdown skill/AGENTS contract, Rust `assert_cmd` integration test, existing clap CLI (no flag changes).

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- Do not invent `tb work start` (stays deferred)
- Do not add, rename, or change CLI flags
- Do not rewrite product code (App / HTTP / desktop / UI)
- JSON contract stays snake_case with `--json` / `--actor`
- Use MCP (`tb mcp`) when the host exposes it; otherwise CLI
- Do not call the localhost HTTP API as an agent
- Identify tasks by `TASK-n`, runs by `RUN-n`
- Existing cards: `tb next [--move]` or `run start --exclusive`. Create only when there is no card
- Waiting: `comment add --continue` / `run continue --reply`. Dead Running: `run cancel`. Split: `task spawn`. Human review: `review --approve|--changes`
- Session start: `tb status`; same checkout: `occupancy`

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-130-sync-skills.md` — this plan
- Create: `crates/cli/tests/skill_contract.rs` — copies identical + required verbs present + `work start` not taught
- Modify: `AGENTS.md` — repo-specific contract (`cursor` / `taskboard`)
- Modify: `skills/using-taskboard/SKILL.md` — generic host contract
- Modify: `.cursor/skills/using-taskboard/SKILL.md` — identical copy of the skill
- Modify: `.claude/skills/using-taskboard/SKILL.md` — identical copy of the skill
- Modify: `.agents/skills/using-taskboard/SKILL.md` — identical copy of the skill

---

### Task 1: Pin the skill contract with a failing test

**Files:**
- Create: `crates/cli/tests/skill_contract.rs`

**Interfaces:**
- Consumes: repo-root Markdown at `skills/using-taskboard/SKILL.md`, `.cursor/skills/using-taskboard/SKILL.md`, `.claude/skills/using-taskboard/SKILL.md`, `.agents/skills/using-taskboard/SKILL.md`, `AGENTS.md`
- Produces: two integration tests that fail until Task 2 updates the docs
  - `skill_copies_are_byte_identical` — three host copies `==` canonical skill
  - `skill_and_agents_name_current_verbs` — both documents contain the required verb tokens and do not teach `work start`

- [ ] **Step 1: Write the failing test**

Create `crates/cli/tests/skill_contract.rs`:

```rust
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read_repo(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn skill_copies_are_byte_identical() {
    let canonical = read_repo("skills/using-taskboard/SKILL.md");
    for rel in [
        ".cursor/skills/using-taskboard/SKILL.md",
        ".claude/skills/using-taskboard/SKILL.md",
        ".agents/skills/using-taskboard/SKILL.md",
    ] {
        let other = read_repo(rel);
        assert_eq!(
            canonical, other,
            "{rel} drifted from skills/using-taskboard/SKILL.md"
        );
    }
}

#[test]
fn skill_and_agents_name_current_verbs() {
    let skill = read_repo("skills/using-taskboard/SKILL.md");
    let agents = read_repo("AGENTS.md");
    for (name, body) in [("SKILL.md", skill.as_str()), ("AGENTS.md", agents.as_str())] {
        for token in [
            "status",
            "occupancy",
            "next",
            "--exclusive",
            "--continue",
            "run cancel",
            "task spawn",
            "review",
            "--approve",
            "--changes",
            "--json",
            "--actor",
        ] {
            assert!(
                body.contains(token),
                "{name} must name current verb {token:?}"
            );
        }
        assert!(
            !body.contains("work start"),
            "{name} must not invent tb work start"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test skill_contract -- --nocapture`

Expected: `skill_copies_are_byte_identical` PASS (copies already match). `skill_and_agents_name_current_verbs` FAIL with `SKILL.md must name current verb "occupancy"` (or the first missing token: `status` / `occupancy` / `next` / `--exclusive`).

- [ ] **Step 3: Do not change product code**

Leave `crates/cli/src/args.rs` and `crates/cli/src/main.rs` untouched. This failure is missing documentation, not missing CLI.

---

### Task 2: Rewrite AGENTS.md to the current loop

**Files:**
- Modify: `AGENTS.md` (replace entire file)

**Interfaces:**
- Consumes: shipped CLI from `crates/cli/src/args.rs` — `status`, `occupancy`, `next` / `--move`, `run start --exclusive`, `comment add --continue`, `run continue --reply`, `run cancel`, `task spawn --title`, `review --approve|--changes --text`
- Produces: repo contract agents follow in this checkout (`--actor cursor`, project `taskboard`)

- [ ] **Step 1: Replace `AGENTS.md` with this exact file**

```markdown
# Agent instructions

Track implementation work on the local Taskboard CLI. Do not skip this because the task is small, already described in chat, or "just a quick edit."

## CLI

Resolve the binary once per session:

1. `tb` if it is on `PATH`
2. else `taskboard` if it is on `PATH`
3. else `cargo run --quiet --bin taskboard --` from this repository root

Always pass `--json` and `--actor cursor` (or set `TASKBOARD_ACTOR=cursor`). Parse stdout JSON. Success is `"ok": true` with `entity` / `entities` and `revision`. Failure is `"ok": false` with `error.code` and a nonzero exit.

Use MCP (`tb mcp`) when the host exposes it; otherwise use this CLI. Do not call the localhost HTTP API. Do not identify tasks by title; use `TASK-n`. After a claim (`next` or `run start`), use the returned `RUN-n`. If you forget `RUN-n`, use `run current TASK-n` or `run list --open`. `task list --all --status running,waiting` lists cards across projects.

## Session start

Do this **before the first edit** when the user asked you to implement, fix, or change something:

1. `tb project detect --json` — if `project_required`, `project list --json` then `project add --name taskboard --path <repo-root> --json`. If `task show` has `worktree_path`, `cd` there before the first edit.
2. `tb status --json` — board-wide snapshot (inbox / open / stale / ready / in-review / blocked).
3. `tb occupancy --json` when this checkout may already have an open run — collisions are grouped by `worktree_path`.
4. Claim an existing card. Prefer `tb next --json` (first ready card + start a run) or `tb next --move --json` (also moves the card to `in-progress`). If you already know `TASK-n`, `run start TASK-n --agent cursor --exclusive --json`. Create only when there is no card: `task create --project taskboard --title "<short title>" --json`, then `task move TASK-n in-progress --json`, then `run start TASK-n --agent cursor --json`.
5. Keep `RUN-n`. If a later `run start --exclusive` returns `conflict`, that card already has a running or waiting run — pick another card or `run current TASK-n`.

Do not invent a combined claim command. `tb next` and `run start --exclusive` already claim existing cards.

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
| I'll create+move+start even though a card exists | Existing cards use `tb next` or `run start --exclusive` |

Questions-only turns (no repo changes) may skip the board.
```

- [ ] **Step 2: Confirm `AGENTS.md` has no `work start` and names `next` / `--exclusive`**

Run: `rg -n "work start|tb next|--exclusive|occupancy|review" AGENTS.md`

Expected: `work start` is absent. `next`, `--exclusive`, `occupancy`, `review` are present.

---

### Task 3: Rewrite the skill and copy it to the three hosts

**Files:**
- Modify: `skills/using-taskboard/SKILL.md` (replace entire file)
- Modify: `.cursor/skills/using-taskboard/SKILL.md` (identical bytes)
- Modify: `.claude/skills/using-taskboard/SKILL.md` (identical bytes)
- Modify: `.agents/skills/using-taskboard/SKILL.md` (identical bytes)

**Interfaces:**
- Consumes: the same shipped verbs as Task 2
- Produces: generic host contract (`<agent>` / `<slug>`) that matches `AGENTS.md`'s session loop

- [ ] **Step 1: Replace `skills/using-taskboard/SKILL.md` with this exact file**

```markdown
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
3. else `cargo run --quiet --bin taskboard --` from a Taskboard checkout

Set `<agent>` to the host: `cursor`, `claude`, or `codex`. Always pass `--json` and `--actor <agent>` (or set `TASKBOARD_ACTOR`). Parse stdout JSON. Success is `"ok": true` with `entity` / `entities` and `revision`. Failure is `"ok": false` with `error.code` and a nonzero exit.

Use MCP (`tb mcp`) when the host exposes it; otherwise use this CLI. Do not call the localhost HTTP API. Do not identify tasks by title; use `TASK-n`. After a claim (`next` or `run start`), use the returned `RUN-n`. If you forget `RUN-n`, use `run current TASK-n` or `run list --open`. `task list --all --status running,waiting` lists cards across projects.

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
```

- [ ] **Step 2: Copy the canonical skill to the three host trees**

```bash
cp skills/using-taskboard/SKILL.md .cursor/skills/using-taskboard/SKILL.md
cp skills/using-taskboard/SKILL.md .claude/skills/using-taskboard/SKILL.md
cp skills/using-taskboard/SKILL.md .agents/skills/using-taskboard/SKILL.md
```

Expected: `diff` of each copy against `skills/using-taskboard/SKILL.md` is empty.

- [ ] **Step 3: Run the contract tests and make sure they pass**

Run: `cargo test -p taskboard-cli --test skill_contract -- --nocapture`

Expected: both tests PASS.

- [ ] **Step 4: Confirm `work start` is not introduced**

Run: `rg -n "work start" AGENTS.md skills/using-taskboard/SKILL.md`

Expected: no matches.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-09-17-issue-130-sync-skills.md \
  crates/cli/tests/skill_contract.rs \
  AGENTS.md \
  skills/using-taskboard/SKILL.md \
  .cursor/skills/using-taskboard/SKILL.md \
  .claude/skills/using-taskboard/SKILL.md \
  .agents/skills/using-taskboard/SKILL.md
git commit -m "$(cat <<'EOF'
docs: sync skills and AGENTS.md to current verbs

Teach status, occupancy, next/--exclusive, comment --continue,
run cancel, task spawn, and review so agents stop reinventing
create → move → run start. Keep skill copies byte-identical.
EOF
)"
```

---

## Self-review

**1. Spec coverage**

| Acceptance / spec item | Task |
| --- | --- |
| Skill copies and `AGENTS.md` describe the same session loop | Tasks 2–3 |
| Existing-card path prefers `tb next` / `--exclusive` | Tasks 2–3 step 1, new rationalization row |
| Waiting / dead-run / spawn / review / status / occupancy named | Tasks 2–3 + Task 1 token list |
| `tb work start` is still not introduced | Docs never name it; test forbids the `work start` substring |
| JSON contract stays snake_case with `--json` / `--actor` | Both docs keep the CLI preamble; test requires those flags |
| No new verbs / CLI flag changes / product code | Task 1 step 3; file map is docs + one test |

**2. Placeholder scan:** no TBD / "add tests later" / "similar to Task N".

**3. Type consistency:** verb spellings match `crates/cli/src/args.rs` (`--exclusive`, `--continue`, `--approve`, `--changes`, `task spawn --title`, `run cancel`, `occupancy`, `status`, `next --move`).
