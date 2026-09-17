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
fn readme_capability_table_and_changelog() {
    let readme = read_repo("README.md");
    let cap = readme
        .split("## Capability")
        .nth(1)
        .expect("README must have ## Capability");
    let cap = cap.split("\n## ").next().unwrap();
    for token in [
        "Inbox",
        "inspector",
        "review",
        "status",
        "next",
        "spawn",
        "occupancy",
        "tb mcp",
        "Homebrew",
        "tb serve",
    ] {
        assert!(
            cap.contains(token),
            "capability table missing {token:?}:\n{cap}"
        );
    }
    for banned in ["work start", "command palette", "markdown preview"] {
        assert!(
            !cap.contains(banned),
            "capability table must not list {banned}"
        );
    }
    assert!(
        readme.contains("CHANGELOG.md"),
        "README must link CHANGELOG.md"
    );
    assert!(
        readme.contains("AGENTS.md"),
        "README must link AGENTS.md instead of copying spec checklists"
    );
    let log = read_repo("CHANGELOG.md");
    assert!(
        log.contains("## [0.1.0]"),
        "CHANGELOG.md must start at v0.1.0"
    );
}
