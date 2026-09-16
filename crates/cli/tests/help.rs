use assert_cmd::Command;

#[test]
fn help_lists_project_add() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.arg("--help").output().unwrap().stdout).unwrap();
    assert!(out.contains("project"), "help missing project:\n{out}");
    assert!(out.contains("task"), "help missing task:\n{out}");
    assert!(out.contains("run"), "help missing run:\n{out}");
    assert!(out.contains("comment"), "help missing comment:\n{out}");
    assert!(out.contains("inbox"), "help missing inbox:\n{out}");
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
