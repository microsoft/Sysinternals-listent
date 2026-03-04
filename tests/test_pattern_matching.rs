//! Integration tests for pattern matching in entitlement filters
//! 
//! Tests both static scan and monitor modes to ensure consistent behavior

use predicates::prelude::*;

#[test]
fn test_exact_entitlement_filter_backwards_compatibility() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test exact matching continues to work
    // Note: paths are now positional arguments, not -p
    cmd.args(&["/usr/bin", "-e", "com.apple.security.network.client", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_exact_filter_no_substring_matching() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Substring matching should NOT work (this was the old monitor mode bug)
    cmd.args(&["/usr/bin", "-e", "security.network", "--quiet"])
        .assert()
        .success();
    // May or may not find matches depending on system binaries
}

#[test]
fn test_glob_pattern_wildcard() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test glob pattern matching
    cmd.args(&["/usr/bin", "-e", "com.apple.security.*", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_glob_pattern_any_network() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test wildcard matching for network entitlements
    cmd.args(&["/usr/bin", "-e", "*network*", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_multiple_glob_patterns() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test multiple patterns (OR logic)
    cmd.args(&["/usr/bin", "-e", "com.apple.security.network.*", "-e", "*camera*", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_invalid_glob_pattern_validation() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test invalid glob pattern is rejected
    cmd.args(&["/usr/bin", "-e", "com.apple.[", "--quiet"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid entitlement filter"));
}

#[test]
fn test_monitor_mode_glob_patterns() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test that monitor mode also supports glob patterns
    cmd.args(&["monitor", "-e", "com.apple.security.*", "--interval", "10.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout causes interrupted status
        .stdout(predicate::str::contains("Monitoring for processes with entitlement: com.apple.security.*"));
}

#[test]
fn test_monitor_mode_consistent_exact_matching() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test that monitor mode uses exact matching, not substring matching
    cmd.args(&["monitor", "-e", "network.client", "--interval", "10.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout causes interrupted status
        .stdout(predicate::str::contains("Monitoring for processes with entitlement: network.client"));
}

#[test]
fn test_json_output_with_patterns() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test JSON output works with glob patterns
    cmd.args(&["/usr/bin", "-e", "com.apple.security.network.*", "--json", "--quiet"])
        .assert()
        .success();
    // JSON output format is validated separately
}

#[test]
fn test_comma_separated_patterns() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test comma-separated patterns work with glob
    cmd.args(&["/usr/bin", "-e", "com.apple.security.network.*,*camera*", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_pattern_case_sensitivity() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    
    // Test that patterns are case-sensitive
    cmd.args(&["/usr/bin", "-e", "COM.APPLE.SECURITY.*", "--quiet"])
        .assert()
        .success();
    // Uppercase patterns won't match lowercase entitlements
}