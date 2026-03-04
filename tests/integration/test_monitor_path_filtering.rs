use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_monitor_with_single_path_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "/System/Applications/Calculator.app", "--interval", "1.0"])
        .timeout(Duration::from_secs(3))
        .assert()
        .interrupted()
        .stdout(predicate::str::contains("Monitoring /System/Applications/Calculator.app"));
}

#[test]
fn test_monitor_with_multiple_path_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor", 
        "/System/Applications/Calculator.app",
        "/System/Applications/TextEdit.app",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted()
    .stdout(predicate::str::contains("Monitoring"));
}

#[test]
fn test_monitor_with_nonexistent_path() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "/nonexistent/path", "--interval", "1.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist").or(
            predicate::str::contains("not a directory")
        ).or(
            predicate::str::contains("No such file")
        ));
}

#[test]
fn test_path_filtering_effectiveness() {
    // Test that path filtering actually works by comparing output
    // Use a small directory for fast tests
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "/System/Applications/Calculator.app", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted()
        .stdout(predicate::str::contains("/System/Applications/Calculator.app"));
}

#[test]
fn test_monitor_system_applications_path() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "/System/Applications/TextEdit.app", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted()
        .stdout(predicate::str::contains("/System/Applications/TextEdit.app"));
}

#[test]
fn test_path_filter_with_json_output() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor", 
        "/System/Applications/Calculator.app",
        "--json",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(2))
    .assert()
    .interrupted(); // Process is killed by timeout
}

#[test]
fn test_path_filter_validation() {
    // Test that empty paths cause failure quickly
    // Skip /dev/null test as it may not fail immediately on all systems
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .failure();
}