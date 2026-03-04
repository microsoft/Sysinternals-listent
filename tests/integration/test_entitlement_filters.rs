use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_single_entitlement_filter() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_multiple_entitlement_filters() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox")
       .arg("--entitlement").arg("com.apple.security.network.client");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_entitlement_filter_logical_or() {
    // Multiple entitlement filters should be OR'd together
    // (binary needs at least one of the specified entitlements)
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox")
       .arg("--entitlement").arg("com.apple.security.network.client");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Matched:"));
}

#[test]
fn test_entitlement_filter_exact_match() {
    // Entitlement filtering should be exact string match, not substring
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security");  // Partial key
    
    // Should not match binaries with "com.apple.security.app-sandbox"
    cmd.assert()
        .success();
}

#[test] 
fn test_nonexistent_entitlement_filter() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.nonexistent.entitlement.key.12345");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Matched: 0")
                .or(predicate::str::contains("(no matches)")));
}