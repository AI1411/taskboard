# Document local MCP host setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Check in local stdio `mcp.json` / Codex TOML snippets and point README, Homebrew caveats, and the skill at them so hosts actually launch `tb mcp` instead of falling back to CLI parsing.

**Architecture:** Documentation plus example host configs. No new MCP tools, no remote server, no App changes. Canonical snippets live under `packaging/mcp/`. README and Formula caveats copy the same `command` / `args: ["mcp"]` / `TASKBOARD_ACTOR` shape. Skills tell agents to add that file and keep the CLI fallback. A CLI integration test pins the examples and the documented smoke check (`tools/list` contains `next` / `review` / `run_cancel`).

**Tech Stack:** Host MCP config JSON/TOML, Markdown, Homebrew Formula caveats, existing `tb mcp` stdio server.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- Local stdio only — no remote / cloud MCP
- Do not add MCP tools (`status` / `occupancy` / `task_spawn` stay a sibling issue)
- Do not change App validation or HTTP agent access
- `command` is `tb` or `taskboard`; `args` is exactly `["mcp"]`
- Actor via `TASKBOARD_ACTOR` (`cursor` / `claude` / `codex`)
- Smoke check is `tools/list` containing `next`, `review`, `run_cancel`
- CLI fallback stays when the host config is missing
- JSON CLI contract stays snake_case with `--json` / `--actor`

User already chose sequential inline execution.

## File map

- Create: `docs/superpowers/plans/2026-09-17-issue-131-mcp-host-setup.md` — this plan
- Create: `packaging/mcp/cursor.mcp.json` — Cursor `.cursor/mcp.json` / `~/.cursor/mcp.json`
- Create: `packaging/mcp/claude.mcp.json` — Claude Code `.mcp.json`
- Create: `packaging/mcp/codex.config.toml` — Codex `~/.codex/config.toml` fragment
- Create: `crates/cli/tests/mcp_host_docs.rs` — examples + README / caveats / skill pins
- Modify: `README.md` — Local MCP section + smoke check
- Modify: `Formula/taskboard.rb` — caveats point at the same snippet
- Modify: `packaging/homebrew/README.md` — same snippet
- Modify: `AGENTS.md` — add the host file; keep CLI fallback
- Modify: `skills/using-taskboard/SKILL.md` — same (host copies are the same inode)

---

### Task 1: Failing docs contract test

**Files:**
- Create: `crates/cli/tests/mcp_host_docs.rs`

**Interfaces:**
- Consumes: repo files listed in File map
- Produces: tests that fail until Tasks 2–3 land the examples and copy

- [ ] **Step 1: Write the failing test**

Create `crates/cli/tests/mcp_host_docs.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read_repo(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn assert_stdio_server(value: &Value, actor: &str) {
    let server = &value["mcpServers"]["taskboard"];
    let command = server["command"].as_str().unwrap();
    assert!(
        command == "tb" || command == "taskboard",
        "command must be tb or taskboard, got {command}"
    );
    assert_eq!(
        server["args"],
        serde_json::json!(["mcp"]),
        "args must be [\"mcp\"]"
    );
    assert_eq!(
        server["env"]["TASKBOARD_ACTOR"].as_str(),
        Some(actor),
        "TASKBOARD_ACTOR"
    );
    assert!(
        server.get("url").is_none(),
        "local snippet must not set a remote url"
    );
}

#[test]
fn host_snippets_are_local_stdio() {
    let cursor: Value = serde_json::from_str(&read_repo("packaging/mcp/cursor.mcp.json")).unwrap();
    let claude: Value = serde_json::from_str(&read_repo("packaging/mcp/claude.mcp.json")).unwrap();
    assert_stdio_server(&cursor, "cursor");
    assert_stdio_server(&claude, "claude");

    let codex = read_repo("packaging/mcp/codex.config.toml");
    assert!(codex.contains("[mcp_servers.taskboard]"));
    assert!(codex.contains("command = \"tb\"") || codex.contains("command = \"taskboard\""));
    assert!(codex.contains("args = [\"mcp\"]"));
    assert!(codex.contains("TASKBOARD_ACTOR") && codex.contains("codex"));
    assert!(!codex.contains("url"));
}

#[test]
fn readme_formula_skill_point_at_snippets_and_smoke() {
    let readme = read_repo("README.md");
    let formula = read_repo("Formula/taskboard.rb");
    let brew = read_repo("packaging/homebrew/README.md");
    let skill = read_repo("skills/using-taskboard/SKILL.md");
    let agents = read_repo("AGENTS.md");
    for (name, body) in [
        ("README.md", readme.as_str()),
        ("Formula/taskboard.rb", formula.as_str()),
        ("packaging/homebrew/README.md", brew.as_str()),
    ] {
        assert!(body.contains("mcp.json"), "{name} must mention mcp.json");
        assert!(body.contains("TASKBOARD_ACTOR"), "{name} must set TASKBOARD_ACTOR");
        assert!(body.contains("tools/list"), "{name} must document tools/list smoke");
        for tool in ["next", "review", "run_cancel"] {
            assert!(body.contains(tool), "{name} smoke must name {tool}");
        }
    }
    for (name, body) in [("SKILL.md", skill.as_str()), ("AGENTS.md", agents.as_str())] {
        assert!(
            body.contains("packaging/mcp") || body.contains("mcp.json"),
            "{name} must tell the agent to add the host MCP file"
        );
        assert!(
            body.contains("otherwise use this CLI") || body.contains("CLI fallback"),
            "{name} must keep the CLI fallback"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test mcp_host_docs -- --nocapture`

Expected: FAIL — `packaging/mcp/cursor.mcp.json` does not exist (`read cursor.mcp.json: No such file or directory`).

---

### Task 2: Check in host snippets

**Files:**
- Create: `packaging/mcp/cursor.mcp.json`
- Create: `packaging/mcp/claude.mcp.json`
- Create: `packaging/mcp/codex.config.toml`

**Interfaces:**
- Consumes: existing `tb mcp` stdio server (`crates/cli/src/mcp.rs`)
- Produces: copy-paste host configs with no `url`

- [ ] **Step 1: Write `packaging/mcp/cursor.mcp.json`**

```json
{
  "mcpServers": {
    "taskboard": {
      "command": "tb",
      "args": ["mcp"],
      "env": {
        "TASKBOARD_ACTOR": "cursor"
      }
    }
  }
}
```

Host paths: project `.cursor/mcp.json`, or user `~/.cursor/mcp.json`. If `tb` is not on `PATH`, set `"command": "taskboard"`.

- [ ] **Step 2: Write `packaging/mcp/claude.mcp.json`**

```json
{
  "mcpServers": {
    "taskboard": {
      "command": "tb",
      "args": ["mcp"],
      "env": {
        "TASKBOARD_ACTOR": "claude"
      }
    }
  }
}
```

Host paths: project `.mcp.json`, or user-scope `~/.claude.json` `mcpServers`. `claude mcp add --scope project --env TASKBOARD_ACTOR=claude -- tb mcp` writes the same shape.

- [ ] **Step 3: Write `packaging/mcp/codex.config.toml`**

```toml
[mcp_servers.taskboard]
command = "tb"
args = ["mcp"]

[mcp_servers.taskboard.env]
TASKBOARD_ACTOR = "codex"
```

Host paths: user `~/.codex/config.toml`, or project `.codex/config.toml` (trusted workspaces).

---

### Task 3: Point README, Homebrew, and the skill at the snippets

**Files:**
- Modify: `README.md` (insert a "Local MCP" section after "CLI を入れる")
- Modify: `Formula/taskboard.rb` `caveats`
- Modify: `packaging/homebrew/README.md`
- Modify: `AGENTS.md`
- Modify: `skills/using-taskboard/SKILL.md`

**Interfaces:**
- Consumes: Task 2 files
- Produces: humans and agents can attach `tb mcp` without inventing a remote server

- [ ] **Step 1: Add this section to `README.md` after the `tb` / `taskboard` PATH sentence**

```markdown
## Local MCP

`tb mcp` is a local stdio server. Do not use the HTTP API as an agent. Add one of the checked-in snippets to the host config (if `tb` is not on `PATH`, use `taskboard`):

| Host | Config file | Snippet |
| --- | --- | --- |
| Cursor | `.cursor/mcp.json` or `~/.cursor/mcp.json` | [`packaging/mcp/cursor.mcp.json`](packaging/mcp/cursor.mcp.json) |
| Claude Code | `.mcp.json` or `~/.claude.json` | [`packaging/mcp/claude.mcp.json`](packaging/mcp/claude.mcp.json) |
| Codex | `~/.codex/config.toml` | [`packaging/mcp/codex.config.toml`](packaging/mcp/codex.config.toml) |

Every snippet is `command: tb` (or `taskboard`), `args: ["mcp"]`, and `TASKBOARD_ACTOR` set to `cursor` / `claude` / `codex`.

Smoke check after the host loads the server — `tools/list` must include `next`, `review`, and `run_cancel`:

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | tb mcp
```

If the host config is missing, agents keep using the CLI.
```

- [ ] **Step 2: Replace `Formula/taskboard.rb` `caveats` with**

```ruby
  def caveats
    <<~EOS
      Apple silicon (arm64) on macOS 14+ is the supported release target.
      `taskboard` is always linked. `tb` is created in postinstall unless a
      foreign `tb` already exists; then postinstall prints alias_skipped.

      Local MCP (stdio, not remote): copy packaging/mcp/cursor.mcp.json to
      Cursor `.cursor/mcp.json`, packaging/mcp/claude.mcp.json to Claude
      `.mcp.json`, or packaging/mcp/codex.config.toml into Codex
      `~/.codex/config.toml`. Each snippet is command tb (or taskboard),
      args ["mcp"], env TASKBOARD_ACTOR=cursor|claude|codex.

      Smoke: tools/list must include next, review, and run_cancel:
        printf '%s\\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | tb mcp
    EOS
  end
```

- [ ] **Step 3: Append this section to `packaging/homebrew/README.md`**

```markdown
## Local MCP

Same stdio snippets as the root README (`command: tb` or `taskboard`, `args: ["mcp"]`, `TASKBOARD_ACTOR`). Copy:

- Cursor: `packaging/mcp/cursor.mcp.json` → `.cursor/mcp.json`
- Claude Code: `packaging/mcp/claude.mcp.json` → `.mcp.json`
- Codex: `packaging/mcp/codex.config.toml` → `~/.codex/config.toml`

Homebrew `caveats` repeat this. Smoke: `tools/list` includes `next`, `review`, `run_cancel`.

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | tb mcp
```
```

- [ ] **Step 4: In `AGENTS.md` and `skills/using-taskboard/SKILL.md`, replace the MCP sentence**

`AGENTS.md` (keep `--actor cursor`):

```markdown
Use MCP (`tb mcp`) when the host has the local stdio config from `packaging/mcp/` (Cursor `.cursor/mcp.json`, Claude Code `.mcp.json`, Codex `~/.codex/config.toml`). If that file is missing, otherwise use this CLI. Do not call the localhost HTTP API.
```

`SKILL.md`:

```markdown
Use MCP (`tb mcp`) when the host has the local stdio config from `packaging/mcp/` (or the `mcp.json` snippet in the README). If that file is missing, otherwise use this CLI. Do not call the localhost HTTP API.
```

Keep the rest of each file unchanged so `skill_contract` still passes.

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p taskboard-cli --test mcp_host_docs -- --nocapture
cargo test -p taskboard-cli --test skill_contract -- --nocapture
sh packaging/homebrew/tests/run.sh
```

Expected: all PASS. Homebrew script still finds `alias_skipped` in the Formula.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/plans/2026-09-17-issue-131-mcp-host-setup.md \
  packaging/mcp crates/cli/tests/mcp_host_docs.rs \
  README.md Formula/taskboard.rb packaging/homebrew/README.md \
  AGENTS.md skills/using-taskboard/SKILL.md
git commit -m "$(cat <<'EOF'
docs: add local MCP host setup snippets

Check in Cursor / Claude / Codex stdio configs and point README,
Homebrew caveats, and the skill at them. Smoke check is tools/list
containing next, review, and run_cancel. No remote MCP.
EOF
)"
```

---

## Self-review

**1. Spec coverage**

| Acceptance | Task |
| --- | --- |
| Repo documents local stdio `mcp.json` for Cursor, Claude Code, Codex | Task 2 + README table |
| Homebrew caveats point at the same snippet | Task 3 Formula + brew README |
| Skills tell the agent to add that file, CLI fallback stays | Task 3 step 4 |
| Smoke check is `tools/list` containing `next` / `review` / `run_cancel` | README / caveats / test |
| No remote MCP server | snippets have no `url`; constraint |

**2. Placeholder scan:** no TBD. Complete JSON/TOML and exact caveats text.

**3. Type consistency:** `args: ["mcp"]`, tool names match `crates/cli/src/mcp.rs` (`next`, `review`, `run_cancel`).
