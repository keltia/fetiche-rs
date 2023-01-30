use assert_cmd::Command;

const BIN: &str = "cat21conv";

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
fn test_version() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-V").assert().success();
}

#[test]
fn test_invalid_no_file() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("blah").assert().failure();
}

#[test]
fn test_both_flags() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-S nope")
        .arg("testdata/csv13-10-2022.csv")
        .assert()
        .failure();
}

#[test]
fn test_file_without_format() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("testdata/csv13-10-2022.csv").assert().failure();
}

#[test]
fn test_file_with_format() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-F")
        .arg("aeroscope")
        .arg("-c")
        .arg("src/sites/src/config.toml")
        .arg("testdata/csv13-10-2022.csv")
        .assert()
        .success();
}
