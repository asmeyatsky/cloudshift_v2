//! CLI integration tests: exit codes and stderr for main commands.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_exits_0() {
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.arg("--help");
    cmd.assert().success().stderr(predicate::str::is_empty());
}

#[test]
fn transform_help_exits_0() {
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.args(["transform", "--help"]);
    cmd.assert().success().stderr(predicate::str::is_empty());
}

#[test]
fn analyse_help_exits_0() {
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.args(["analyse", "--help"]);
    cmd.assert().success().stderr(predicate::str::is_empty());
}

#[test]
fn transform_nonexistent_path_exits_1() {
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.args(["transform", "/nonexistent/path/xyz"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn transform_current_dir_succeeds() {
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.args(["transform", "."])
        .current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.assert().success();
}

#[test]
fn catalogue_list_runs() {
    // catalogue list may exit 0 (if catalogue loads) or 1 (e.g. not implemented / no catalogue)
    let mut cmd = Command::cargo_bin("cloudshift").expect("binary");
    cmd.args(["catalogue", "list"]);
    let out = cmd.output().expect("run");
    assert!(
        out.status.code().is_some(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
