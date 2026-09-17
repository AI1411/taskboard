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
        assert!(
            body.contains("TASKBOARD_ACTOR"),
            "{name} must set TASKBOARD_ACTOR"
        );
        assert!(
            body.contains("tools/list"),
            "{name} must document tools/list smoke"
        );
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
