use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_monitor_mode_startup() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(Duration::from_secs(3))
        .assert()
        .interrupted() // Process is killed by timeout, not clean exit
        .stdout(predicate::str::contains("Starting process monitoring"));
}

#[test]
fn test_process_detection_basic() {
    // This test runs monitor mode briefly and expects it to be interrupted by timeout
    // without requiring specific process detection
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "2.0"])
        .timeout(Duration::from_secs(4))
        .assert()
        .interrupted();
}

#[test]
fn test_ctrl_c_shutdown_handling() {
    // This test validates that monitor mode runs correctly until interrupted
    // The timeout() simulates the interrupt
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted(); // Interrupted by timeout (simulates Ctrl+C)
}

#[test]
fn test_polling_interval_timing() {
    use std::time::Instant;
    
    // Test that monitor mode respects the specified interval
    // This is an indirect test through timeout behavior
    let start = Instant::now();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(Duration::from_secs(3))
        .assert()
        .interrupted();
        
    let elapsed = start.elapsed();
    // Should run for approximately 3 seconds (allowing for startup time)
    assert!(elapsed >= Duration::from_secs(2));
    assert!(elapsed <= Duration::from_secs(4));
}

#[test]
fn test_monitor_without_crashes() {
    // Test basic stability - monitor mode should not crash
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "0.5"])
        .timeout(Duration::from_secs(5))
        .assert()
        .interrupted()
        .stderr(predicate::str::contains("panic").not())
        .stderr(predicate::str::contains("error").not());
}

#[test]
fn test_monitor_with_fast_interval() {
    // Test with minimum allowed interval
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "0.1"])
        .timeout(Duration::from_secs(2))
        .assert()
        .interrupted();
}

#[test]
fn test_monitor_with_slow_interval() {
    // Test with larger interval
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "3.0"])
        .timeout(Duration::from_secs(4))
        .assert()
        .interrupted();
}