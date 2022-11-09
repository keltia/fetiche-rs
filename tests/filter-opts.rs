use assert_cmd::Command;

const BIN: &str = "cat21conv";

#[test]
fn test_file_with_today() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-F")
        .arg("aeroscope")
        .arg("-c")
        .arg("src/bin/cat21conv/config.toml")
        .arg("--today")
        .arg("testdata/csv13-10-2022")
        .assert()
        .success();
}

#[test]
fn test_file_with_begin_only() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-F")
        .arg("aeroscope")
        .arg("-c")
        .arg("src/bin/cat21conv/config.toml")
        .arg("-B")
        .arg("2022-01-01 23:00:00")
        .arg("testdata/csv13-10-2022")
        .assert()
        .failure();
}

#[test]
fn test_file_with_begin_end() {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.arg("-F")
        .arg("aeroscope")
        .arg("-c")
        .arg("src/bin/cat21conv/config.toml")
        .arg("-B")
        .arg("2022-01-01 23:00:00")
        .arg("-E")
        .arg("2022-01-01 23:00:01")
        .arg("testdata/csv13-10-2022")
        .assert()
        .failure();
}
