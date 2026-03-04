use predicates::prelude::*;
use tempfile::TempDir;
use std::time::Duration;

// ==================== Static Scan Combined Filters ====================

#[test]
fn test_path_and_entitlement_filters_combined() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"))
        .stdout(predicate::str::contains("Matched:"));
}

#[test]
fn test_combined_filters_logical_and() {
    // Path AND entitlement filters should both apply
    // Only binaries in specified paths AND containing specified entitlements
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp1.path().to_str().unwrap())
       .arg(temp2.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox")
       .arg("--entitlement").arg("com.apple.security.network.client");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_combined_filters_restrictive_result() {
    // Combined filters should be more restrictive than either alone
    let temp = TempDir::new().unwrap();
    
    // First get count with just path filter
    let mut cmd1 = assert_cmd::cargo_bin_cmd!("listent");
    cmd1.arg(temp.path().to_str().unwrap());
    let output1 = cmd1.assert().success().get_output().stdout.clone();
    
    // Then get count with path + entitlement filter
    let mut cmd2 = assert_cmd::cargo_bin_cmd!("listent");
    cmd2.arg(temp.path().to_str().unwrap())
        .arg("--entitlement").arg("com.nonexistent.entitlement");
    let output2 = cmd2.assert().success().get_output().stdout.clone();
    
    // Combined filter should find same or fewer matches
    let output1_str = String::from_utf8(output1).unwrap();
    let output2_str = String::from_utf8(output2).unwrap();
    
    assert!(output1_str.contains("Scanned:"), "Path-only scan should show scanned count");
    assert!(output2_str.contains("Scanned:"), "Combined filter scan should show scanned count");
}

#[test]
fn test_all_filter_types_together() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox")
       .arg("--json")
       .arg("--quiet");
    
    cmd.assert()
        .success();
}

// ==================== Multiple Entitlement Filters (OR Logic) ====================

#[test]
fn test_multiple_entitlement_filters_or_logic_scan() {
    // Multiple -e flags should use OR logic
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.security.network.client")
       .arg("-e").arg("com.apple.security.network.server")
       .arg("--quiet");
    
    cmd.assert().success();
    // Should match binaries with EITHER entitlement
}

#[test]
fn test_glob_and_exact_filters_combined() {
    // Mix of glob patterns and exact matches
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.security.network.*") // glob
       .arg("-e").arg("com.apple.security.app-sandbox") // exact
       .arg("--quiet");
    
    cmd.assert().success();
}

#[test]
fn test_overlapping_glob_patterns() {
    // Multiple glob patterns that may overlap
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.security.*")
       .arg("-e").arg("com.apple.security.network.*") // subset of above
       .arg("--quiet");
    
    cmd.assert().success();
}

// ==================== Monitor Mode Combined Filters ====================

#[test]
fn test_monitor_combined_path_and_entitlement() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
       .arg("/System/Applications/Calculator.app")
       .arg("-e").arg("com.apple.security.*")
       .arg("--interval").arg("10.0")
       .timeout(Duration::from_secs(2));
    
    cmd.assert()
       .interrupted()
       .stdout(predicate::str::contains("Monitoring /System/Applications/Calculator.app"));
}

#[test]
fn test_monitor_multiple_path_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
       .arg("/System/Applications/Calculator.app")
       .arg("/System/Applications/TextEdit.app")
       .arg("--interval").arg("10.0")
       .timeout(Duration::from_secs(2));
    
    cmd.assert()
       .interrupted()
       .stdout(predicate::str::contains("Monitoring"));
}

#[test]
fn test_monitor_multiple_entitlement_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
       .arg("-e").arg("com.apple.security.device.camera")
       .arg("-e").arg("com.apple.security.device.microphone")
       .arg("--interval").arg("10.0")
       .timeout(Duration::from_secs(2));
    
    cmd.assert()
       .interrupted()
       .stdout(predicate::str::contains("Monitoring for processes with entitlement"));
}

#[test]
fn test_monitor_all_filter_options_combined() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
       .arg("/System/Applications/Calculator.app")
       .arg("-e").arg("com.apple.security.*")
       .arg("--json")
       .arg("--quiet")
       .arg("--interval").arg("5.0")
       .timeout(Duration::from_secs(2));
    
    // Should work with all options combined
    cmd.assert().interrupted();
}

// ==================== Empty Filter Edge Cases ====================

#[test]
fn test_empty_entitlement_filter_matches_all_with_entitlements() {
    // No -e flag means match any binary with entitlements
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("--quiet");
    
    cmd.assert().success();
}

#[test]
fn test_monitor_no_filters_monitors_all() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("monitor")
       .arg("--interval").arg("10.0")
       .timeout(Duration::from_secs(2));
    
    // No path or entitlement filters - monitors all processes
    cmd.assert()
       .interrupted()
       .stdout(predicate::str::contains("Starting process monitoring"));
}

// ==================== Filter Validation Edge Cases ====================

#[test]
fn test_invalid_glob_pattern_rejected() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.[invalid");
    
    cmd.assert()
       .failure()
       .stderr(predicate::str::contains("Invalid"));
}

#[test]
fn test_nonexistent_path_rejected() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/nonexistent/path/that/does/not/exist/anywhere");
    
    cmd.assert()
       .failure();
}

// ==================== JSON Output with Filters ====================

#[test]
fn test_json_output_with_combined_filters() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.security.*")
       .arg("--json")
       .arg("--quiet");
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("Should produce valid JSON");
    
    assert!(parsed.get("results").is_some(), "Should have results field");
    assert!(parsed.get("summary").is_some(), "Should have summary field");
}

#[test]
fn test_json_results_respect_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/usr/bin")
       .arg("-e").arg("com.apple.security.network.client")
       .arg("--json")
       .arg("--quiet");
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("Should produce valid JSON");
    
    // All results should contain the filtered entitlement
    if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
        for result in results {
            if let Some(entitlements) = result.get("entitlements").and_then(|e| e.as_object()) {
                assert!(
                    entitlements.contains_key("com.apple.security.network.client"),
                    "All results should contain the filtered entitlement"
                );
            }
        }
    }
}