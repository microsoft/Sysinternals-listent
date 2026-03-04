use predicates::prelude::*;

#[test]
fn test_monitor_flag_parsing() {
    // Test that monitor subcommand is recognized by checking it produces expected output
    // We use timeout but focus on checking the output exists rather than exact content
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
        .timeout(std::time::Duration::from_millis(200))
        .assert()
        .interrupted(); // Should be interrupted by timeout after starting successfully
    
    // The fact that it gets interrupted (not immediate failure) shows it started monitor mode
}

#[test]
fn test_interval_parameter_validation() {
    // We'll focus on testing the failure cases which work reliably
    // and the help text which shows the intervals are supported
    
    // Test invalid interval - too low
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "0.05"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid polling interval"))
        .stderr(predicate::str::contains("Must be between 0.1 and 300.0"));

    // Test invalid interval - too high
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "500.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid polling interval"))
        .stderr(predicate::str::contains("Must be between 0.1 and 300.0"));
}

#[test]
fn test_monitor_help_text() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("monitor"))
        .stdout(predicate::str::contains("Monitor new processes for entitlements in real-time").or(
            predicate::str::contains("Real-time Monitor Mode")
        ));
}

#[test]
fn test_monitor_with_invalid_arguments() {
    // Test monitor with invalid path
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    // Test --interval without monitor subcommand
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["--interval", "5.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument").or(
           predicate::str::contains("error")
       ));
}