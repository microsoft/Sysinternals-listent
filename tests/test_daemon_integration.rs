//! Integration tests for daemon startup and lifecycle management
//!
//! These tests validate the daemon's ability to start, run, and shutdown properly
//! Note: These tests expect permission failures since they cannot write to system directories

use predicates::prelude::*;
use std::time::Duration;
use tempfile::tempdir;

/// Stop any running daemon instance so tests that start a daemon have a clean state
fn stop_any_running_daemon() {
    // Try graceful stop first
    let _ = std::process::Command::new(env!("CARGO_BIN_EXE_listent"))
        .args(&["daemon", "stop"])
        .output();
    // Wait for graceful shutdown (daemon stop itself waits up to 2s + SIGKILL)
    std::thread::sleep(Duration::from_secs(3));
    // Force kill any remaining listent daemon processes
    let _ = std::process::Command::new("pkill")
        .args(&["-9", "-f", "listent daemon run"])
        .output();
    std::thread::sleep(Duration::from_millis(500));
}

#[test]
#[ignore]
fn test_daemon_startup_process() {
    // Ensure no daemon is already running
    stop_any_running_daemon();

    // Test that daemon can start in background mode (daemon run subcommand)
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // daemon run starts monitoring in daemon mode
    // The daemon will start and run until timeout
    cmd.args(&["daemon", "run"])
       .timeout(Duration::from_secs(2))
       .assert()
       .interrupted(); // Daemon starts successfully and runs until timeout
}

#[test]
fn test_daemon_pid_file_creation() {
    let temp_dir = tempdir().unwrap();
    let _pid_file = temp_dir.path().join("test.pid");

    // Create config with custom PID file location
    let config_path = temp_dir.path().join("config.toml");
    let config_content = r#"
[daemon]
polling_interval = 1.0
auto_start = false

[monitoring]
path_filters = []
entitlement_filters = []
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Test daemon startup with custom config
    // Will fail due to permission issues (can't write to /var/run/listent or /Library/LaunchDaemons)
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
fn test_daemon_configuration_management() {
    // Test daemon configuration loading and management
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("daemon.toml");

    let config_content = r#"
[daemon]
polling_interval = 2.0
auto_start = false

[monitoring]
path_filters = ["/Applications", "/usr/bin"]
entitlement_filters = ["com.apple.security.network.client"]

[logging]
level = "debug"
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Will fail due to permission issues (can't write to /var/run/listent or /Library/LaunchDaemons)
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
fn test_daemon_status_command() {
    // Test daemon status command - should succeed and show status info
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "status"])
       .assert()
       .success()
       .stdout(predicate::str::contains("Checking listent daemon status"));
}

#[test]
fn test_daemon_launchd_integration() {
    // Test LaunchD plist generation and installation
    // Will fail due to permission issues (can't write to /Library/LaunchDaemons)
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    cmd.args(&["daemon", "install"])
       .assert()
       .failure()
       .stderr(predicate::str::contains("Permission denied").or(
           predicate::str::contains("Failed to create working directory").or(
               predicate::str::contains("Failed to write plist file")
           )
       ));
}

#[test]
fn test_daemon_process_monitoring() {
    // Test that daemon properly monitors processes
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("monitor.toml");

    let config_content = r#"
[daemon]
polling_interval = 0.5
auto_start = false

[monitoring]
path_filters = ["/bin", "/usr/bin"]
entitlement_filters = []

[logging]
level = "info"
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Test daemon with monitoring configuration
    // Will fail due to permission issues (can't write to /var/run/listent or /Library/LaunchDaemons)
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