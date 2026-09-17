use assert_cmd::Command;

#[test]
fn help_lists_project_add() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.arg("--help").output().unwrap().stdout).unwrap();
    assert!(out.contains("project"), "help missing project:\n{out}");
    assert!(out.contains("task"), "help missing task:\n{out}");
    assert!(out.contains("run"), "help missing run:\n{out}");
    assert!(out.contains("comment"), "help missing comment:\n{out}");
    assert!(out.contains("check"), "help missing check:\n{out}");
    assert!(out.contains("activity"), "help missing activity:\n{out}");
    assert!(out.contains("stale"), "help missing stale:\n{out}");
    assert!(out.contains("mcp"), "help missing mcp:\n{out}");
    assert!(out.contains("inbox"), "help missing inbox:\n{out}");
    assert!(out.contains("status"), "help missing status:\n{out}");
    assert!(out.contains("next"), "help missing next:\n{out}");
}

#[test]
fn check_help_lists_remove() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.args(["check", "--help"]).output().unwrap().stdout).unwrap();
    assert!(out.contains("remove"), "check help missing remove:\n{out}");
}

#[test]
fn comment_help_lists_remove() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.args(["comment", "--help"]).output().unwrap().stdout).unwrap();
    assert!(
        out.contains("remove"),
        "comment help missing remove:\n{out}"
    );
}

#[test]
fn project_help_lists_detect() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.args(["project", "--help"]).output().unwrap().stdout).unwrap();
    assert!(
        out.contains("detect"),
        "project help missing detect:\n{out}"
    );
}
