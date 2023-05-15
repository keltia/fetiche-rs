use assert_cmd::Command;

const BIN: &str = "acutectl";

#[test]
fn test_empty_args() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.assert().failure();
}

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-h").assert().success();
}

#[test]
fn test_version_opt() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-V").assert().failure();
}

#[test]
fn test_version_keyword() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("help").assert().success();
}

#[test]
fn test_bad_keyword() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("bouh").assert().failure();
}

#[test]
fn test_list_empty() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("list").assert().failure();
}

#[test]
fn test_list_formats() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("list").arg("formats").assert().success();
}

#[test]
fn test_list_sources() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("list").arg("sources").assert().success();
}
