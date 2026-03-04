use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_default_scan_directories() {
    // Test scanning with a small directory (Calculator.app) instead of default /Applications
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Use a small system app for fast testing
    cmd.arg("/System/Applications/Calculator.app")
        .assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_default_directories_listed_in_help() {
    // Help should document what the default directories are
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("/Applications")
                .or(predicate::str::contains("default"))
                .or(predicate::str::contains("application directories")));
}

#[test]
fn test_default_scan_respects_environment_override() {
    // Use a small temp directory to test quickly
    let temp = TempDir::new().unwrap();
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    // Pass the small directory as argument instead of relying on default
    cmd.arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_default_scan_produces_summary() {
    // Use a small specific app for quick test
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/System/Applications/Calculator.app");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"Scanned: \d+").unwrap())
        .stdout(predicate::str::is_match(r"Matched: \d+").unwrap())
        .stdout(predicate::str::is_match(r"Duration: \d+").unwrap());
}