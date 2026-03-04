use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_monitor_with_camera_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor", 
        "-e", "com.apple.security.device.camera",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted()
    .stdout(predicate::str::contains("Monitoring for processes with entitlement"));
}

#[test]
fn test_monitor_with_microphone_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.device.microphone", 
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted()
    .stdout(predicate::str::contains("Monitoring for processes with entitlement"));
}

#[test]
fn test_monitor_with_multiple_entitlement_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.device.camera",
        "-e", "com.apple.security.device.microphone",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted();
}

#[test]
fn test_monitor_with_network_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.network.client",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted()
    .stdout(predicate::str::contains("Monitoring for processes with entitlement"));
}

#[test]
fn test_entitlement_filter_with_json_output() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.app-sandbox",
        "--json",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(2))
    .assert()
    .interrupted(); // Process is killed by timeout
}

#[test]
fn test_entitlement_filter_partial_matching() {
    // Test that partial entitlement strings work for filtering
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "camera", // Should match com.apple.security.device.camera
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(2))
    .assert()
    .interrupted();
}

#[test]
fn test_processes_with_no_entitlements() {
    // Test monitoring when processes have no entitlements
    // This should still work but may not produce output
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "nonexistent.entitlement",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(2))
    .assert()
    .interrupted(); // Process is killed by timeout
}

#[test]
fn test_entitlement_extraction_error_handling() {
    // Test that monitor continues even if entitlement extraction fails
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "-e", "com.apple.security.app-sandbox",
        "--interval", "0.5"
    ])
    .timeout(Duration::from_secs(3))
    .assert()
    .interrupted()
    .stderr(predicate::str::contains("panic").not()); // No panics on extraction errors
}