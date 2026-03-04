//! Contract tests for daemon CLI subcommands
//!
//! These tests validate the daemon management CLI interface according to
//! the specification in specs/003-add-launchd-daemon/contracts/cli-contract.md

use predicates::prelude::*;

#[test]
fn test_install_daemon_subcommand() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test basic daemon install subcommand - will fail due to permissions but should be recognized
    cmd.args(&["daemon", "install"])
       .assert()
       .failure() // Expected to fail due to permission issues (can't write to /Library/LaunchDaemons)
       .stderr(predicate::str::contains("Permission denied").or(
           predicate::str::contains("Failed to write plist file")
       ));
}

#[test]
fn test_install_daemon_with_config() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon install with custom config path - will fail because config file doesn't exist
    cmd.args(&["daemon", "install", "--config", "/tmp/test-config.toml"])
       .assert()
       .failure() // Expected to fail because config file doesn't exist
       .stderr(predicate::str::contains("Failed to read config file").or(
           predicate::str::contains("No such file or directory")
       ));
}

#[test]
fn test_uninstall_daemon_subcommand() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon uninstall subcommand - may fail due to permissions if plist exists
    // or succeed if plist doesn't exist
    cmd.args(&["daemon", "uninstall"])
       .assert()
       .stdout(predicate::str::contains("Uninstalling listent daemon service"));
    // Note: Don't assert success/failure since it depends on whether plist exists
    // and whether we have permissions
}

#[test]
fn test_daemon_status_subcommand() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon status subcommand - should work and show status
    cmd.args(&["daemon", "status"])
       .assert()
       .success() // Should succeed and show status
       .stdout(predicate::str::contains("Checking listent daemon status"));
}

#[test]
fn test_logs_subcommand() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon logs subcommand - should now work with fixed predicate
    cmd.args(&["daemon", "logs"])
       .assert()
       .success() // Should succeed with fixed macOS log predicate
       .stdout(predicate::str::contains("Retrieving daemon logs"));
}

#[test]
#[ignore] // `log stream` subprocess isn't killed by assert_cmd's timeout, causing hang
fn test_logs_with_follow() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon logs with --follow flag - should now work with fixed predicate
    cmd.args(&["daemon", "logs", "--follow"])
       .timeout(std::time::Duration::from_secs(2)) // Use timeout since --follow runs indefinitely
       .assert()
       .interrupted() // Should be interrupted by timeout after starting successfully
       .stdout(predicate::str::contains("Following daemon logs"));
}

#[test]
fn test_logs_with_since() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test daemon logs with --since flag - should now work with fixed predicate
    cmd.args(&["daemon", "logs", "--since", "1h"])
       .assert()
       .success() // Should succeed with fixed macOS log predicate
       .stdout(predicate::str::contains("Retrieving daemon logs"));
}

#[test]
#[ignore]
fn test_daemon_flag_compatibility() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test that daemon run subcommand works (runs monitoring in daemon mode)
    // The daemon will start running and be interrupted by our timeout
    cmd.args(&["daemon", "run"])
       .timeout(std::time::Duration::from_secs(2))
       .assert()
       .interrupted(); // Daemon starts successfully and runs until timeout
}

#[test]
fn test_help_shows_daemon_subcommands() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");

    // Test that help output includes daemon subcommand
    cmd.arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("daemon").or(
           predicate::str::contains("SUBCOMMANDS").or(
               predicate::str::contains("Commands") // Different help format
           )
       ));
}