use predicates::prelude::*;

#[test]
fn test_version_prints_semantic_version() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--version");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"listent \d+\.\d+\.\d+").unwrap());
}

#[test]
fn test_short_version_flag() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("-V");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"listent \d+\.\d+\.\d+").unwrap());
}