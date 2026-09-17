# README capability map and CHANGELOG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put a short current-capability table on the README and a root `CHANGELOG.md` that starts at `v0.1.0`.

**Architecture:** Docs only. A CLI file-reading test pins the table tokens, the changelog header, spec links, and that deferred items are not listed as capabilities. No product verbs change.

**Tech Stack:** Markdown, `crates/cli/tests/readme_docs.rs`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- Do not rewrite `docs/superpowers/` plans
- Do not present deferred items as current capabilities: OS notifications, command palette, `tb work start`, markdown preview, fifth column, title-as-ID
- Specs are linked, not duplicated as unimplemented checklists
- No desktop `.dmg` docs as if they shipped

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-139-readme-changelog.md`
- Create: `CHANGELOG.md`
- Create: `crates/cli/tests/readme_docs.rs`
- Modify: `README.md` — capability table after the intro, link CHANGELOG

---

### Task 1: Pin then write the docs

**Files:**
- Create: `crates/cli/tests/readme_docs.rs`
- Modify: `README.md`
- Create: `CHANGELOG.md`

**Interfaces:**
- README section heading `## Capability`
- Table names Board / CLI / Agents / Distribution
- Tokens: Inbox, inspector, review, `status`, `next`, `spawn`, `occupancy`, skills, `tb mcp`, Homebrew, `tb serve`
- `CHANGELOG.md` contains `## [0.1.0]`
- README links `[CHANGELOG](CHANGELOG.md)` and `[AGENTS.md](AGENTS.md)`
- README capability section must not contain `work start`, `command palette`, `markdown preview`

```rust
#[test]
fn readme_capability_table_and_changelog() {
    let readme = read_repo("README.md");
    let cap = readme.split("## Capability").nth(1).expect("Capability heading");
    let cap = cap.split("\n## ").next().unwrap();
    for token in ["Inbox", "inspector", "review", "status", "next", "spawn", "occupancy", "tb mcp", "Homebrew", "tb serve"] {
        assert!(cap.contains(token), "capability table missing {token}");
    }
    for banned in ["work start", "command palette", "markdown preview"] {
        assert!(!cap.contains(banned), "capability table must not list {banned}");
    }
    let log = read_repo("CHANGELOG.md");
    assert!(log.contains("## [0.1.0]"));
    assert!(readme.contains("[CHANGELOG](CHANGELOG.md)"));
}
```

- [x] **Step 1: Write the failing test**

- [x] **Step 2: Run** `cargo test -p taskboard-cli --test readme_docs` — expect FAIL missing files/heading

- [x] **Step 3: Add README `## Capability` table and root `CHANGELOG.md` starting at 0.1.0**

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `docs: add README capability map and CHANGELOG`

---

## Self-review

1. Spec: table + CHANGELOG from v0.1.0 + links not checklists + deferred not current.
2. No placeholders.
3. Existing MCP/Homebrew README sections stay.
