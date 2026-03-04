use predicates::prelude::*;

#[test]
fn test_monitor_with_path_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "-p", "/Applications", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .success(); // Will fail until path filtering in monitor mode is implemented
}

#[test]
fn test_monitor_with_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "-e", "com.apple.security.camera", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .success(); // Will fail until entitlement filtering in monitor mode is implemented
}

#[test]
fn test_monitor_with_json_output() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--json", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .success(); // Will fail until JSON output in monitor mode is implemented
}

#[test]
fn test_monitor_with_quiet_mode() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--quiet", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not()) // Should still show process detections
        .stderr(predicate::str::is_empty()); // No startup messages in quiet mode
}

#[test]
fn test_monitor_with_multiple_path_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor", 
        "-p", "/Applications",
        "-p", "/System/Applications",
        "--interval", "1.0"
    ])
    .timeout(std::time::Duration::from_secs(2))
    .assert()
    .success(); // Will fail until multiple path filters are supported
}

#[test]
fn test_monitor_with_multiple_entitlement_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.camera",
        "-e", "com.apple.security.microphone", 
        "--interval", "1.0"
    ])
    .timeout(std::time::Duration::from_secs(2))
    .assert()
    .success(); // Will fail until multiple entitlement filters are supported
}

#[test]
fn test_monitor_combined_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-p", "/Applications",
        "-e", "com.apple.security.camera",
        "--json",
        "--quiet",
        "--interval", "2.0"
    ])
    .timeout(std::time::Duration::from_secs(3))
    .assert()
    .success(); // Will fail until all filter combinations work together
}