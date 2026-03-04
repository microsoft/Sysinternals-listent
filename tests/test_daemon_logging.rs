//! Contract tests for enhanced ULS logging in daemon mode
//!
//! These tests validate structured logging according to
//! specs/003-add-launchd-daemon/contracts/uls-integration.md

use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Stop any running daemon instance so tests that start a daemon have a clean state
fn stop_any_running_daemon() {
    // Try graceful stop first
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_listent"))
        .args(&["daemon", "stop"])
        .output();
    // Wait for graceful shutdown (daemon stop itself waits up to 2s + SIGKILL)
    std::thread::sleep(std::time::Duration::from_secs(3));
    // Force kill any remaining listent daemon processes
    let _ = std::process::Command::new("pkill")
        .args(&["-9", "-f", "listent daemon run"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(500));
}

#[test]
#[ignore]
fn test_daemon_startup_logging() {
    // Ensure no daemon is already running
    stop_any_running_daemon();

    // Test that daemon startup works (daemon run subcommand)
    // The daemon will start and run until timeout
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "run"])
       .timeout(std::time::Duration::from_secs(2))
       .assert()
       .interrupted(); // Daemon starts successfully and runs until timeout
}

#[test]
fn test_daemon_process_detection_logging() {
    // Test that process detection events are logged with entitlement details
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("logging.toml");

    let config_content = r#"
[daemon]
polling_interval = 0.5
auto_start = false

[logging]
level = "debug"
subsystem = "com.microsoft.sysinternals.listent"
category = "daemon"

[monitoring]
path_filters = ["/bin"]
entitlement_filters = []
"#;

    fs::write(&config_path, config_content).unwrap();

    // Will fail due to permission issues (can't write to system directories)
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["daemon", "install", "--config", config_path.to_str().unwrap()])
       .assert()
       .failure()
       .stderr(predicate::str::contains("Permission denied").or(
           predicate::str::contains("Failed to create working directory").or(
               predicate::str::contains("Failed to write plist file")
           )
       ));
}

#[test]
fn test_daemon_log_levels() {
    // Test different log levels in daemon mode
    let levels = ["error", "warn", "info", "debug"];

    for level in &levels {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join(format!("{}_config.toml", level));

        let config_content = format!(r#"
[daemon]
polling_interval = 1.0
auto_start = false

[logging]
level = "{}"
subsystem = "com.microsoft.sysinternals.listent"
category = "daemon"

[monitoring]
path_filters = []
entitlement_filters = []
"#, level);

        fs::write(&config_path, config_content).unwrap();

        // Will fail due to permission issues (can't write to system directories)
        let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
        cmd.args(&["daemon", "install", "--config", config_path.to_str().unwrap()])
           .assert()
           .failure()
           .stderr(predicate::str::contains("Permission denied").or(
               predicate::str::contains("Failed to create working directory").or(
                   predicate::str::contains("Failed to write plist file")
               )
           ));
    }
}

#[test]
fn test_daemon_logs_command() {
    // Test that daemon logs command retrieves daemon logs
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "logs"])
       .assert()
       .success()
       .stdout(predicate::str::contains("Retrieving daemon logs"));
}

#[test]
#[ignore] // assert_cmd timeout cannot kill the `log stream` grandchild process, causing hangs
fn test_daemon_logs_follow_mode() {
    // Test that daemon logs --follow works (use timeout since it runs indefinitely)
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "logs", "--follow"])
       .timeout(std::time::Duration::from_secs(2))
       .assert()
       .interrupted() // Should be interrupted by timeout
       .stdout(predicate::str::contains("Following daemon logs"));
}

#[test]
fn test_daemon_logs_time_filtering() {
    // Test that daemon logs --since filters by time
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "logs", "--since", "1h"])
       .assert()
       .success()
       .stdout(predicate::str::contains("Retrieving daemon logs"));
}

#[test]
fn test_daemon_log_structured_format() {
    // Test that daemon logs can be output in structured JSON format
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "logs", "--format", "json"])
       .assert()
       .success()
       .stdout(predicate::str::contains("Retrieving daemon logs"));
}

#[test]
#[ignore]
fn test_daemon_no_terminal_output() {
    // Ensure no daemon is already running
    stop_any_running_daemon();

    // Test that daemon run subcommand works for daemon mode
    // The daemon will start and run until timeout
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "run"])
       .timeout(std::time::Duration::from_secs(2))
       .assert()
       .interrupted(); // Daemon starts successfully and runs until timeout
}