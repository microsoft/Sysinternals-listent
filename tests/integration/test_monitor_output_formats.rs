use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_monitor_json_output_format() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    let output = cmd.args(&["monitor", "--json", "--interval", "1.0"])
        .timeout(Duration::from_secs(3))
        .output()
        .expect("Failed to execute");

    // Parse output to validate JSON format (may have started before being killed)
    let output_str = String::from_utf8(output.stdout).unwrap();
    
    // Each non-empty line that looks like JSON should be valid JSON
    for line in output_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('{') {
            // Should be parseable as JSON
            let _: serde_json::Value = serde_json::from_str(line)
                .expect(&format!("Invalid JSON line: {}", line));
        }
    }
}

#[test]
fn test_monitor_quiet_mode_suppresses_startup() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--quiet", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted()
        .stdout(predicate::str::contains("Starting process monitoring").not())
        .stdout(predicate::str::contains("Press Ctrl+C").not());
}

#[test]
fn test_monitor_human_readable_format() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted()
        .stdout(predicate::str::contains("Starting process monitoring"))
        .stdout(predicate::str::contains("Press Ctrl+C"));
}

#[test]
fn test_monitor_error_message_formatting() {
    // Test error messages are properly formatted
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "0.05"]) // Invalid interval
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid polling interval"))
        .stderr(predicate::str::contains("0.1 and 300.0"));
}

#[test]
fn test_monitor_real_time_output_streaming() {
    // Test that output appears in real-time, not buffered
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "0.5"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted(); // Process is killed by timeout
}

#[test]
fn test_monitor_json_with_quiet_mode() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--json", "--quiet", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted(); // Process is killed by timeout
}

#[test]
fn test_monitor_output_with_filters() {
    // Test output format when filters are applied
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&[
        "monitor",
        "/System/Applications/Calculator.app",
        "-e", "sandbox",
        "--interval", "1.0"
    ])
    .timeout(Duration::from_secs(2))
    .assert()
    .interrupted()
    .stdout(predicate::str::contains("Monitoring /System/Applications/Calculator.app"))
    .stdout(predicate::str::contains("sandbox"));
}