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
