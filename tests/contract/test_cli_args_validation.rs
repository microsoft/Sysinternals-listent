use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_invalid_path_returns_error() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("/nonexistent/path/12345");
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("path"))
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("does not exist")));
}

#[test]
fn test_path_not_directory_returns_error() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("not_a_directory.txt");
    fs::write(&file_path, "test content").unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(file_path.to_str().unwrap());
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("directory"))
        .stderr(predicate::str::contains("not").or(predicate::str::contains("invalid")));
}

#[test]
fn test_quiet_and_verbose_conflict() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--quiet").arg("--verbose");
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("conflict")
                .or(predicate::str::contains("cannot be used"))
                .or(predicate::str::contains("mutually exclusive")));
}

#[test]
fn test_duplicate_entitlements_accepted() {
    // Duplicate entitlements should be accepted and internally deduplicated
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox")
       .arg("--entitlement").arg("com.apple.security.app-sandbox");
    
    // Should not fail due to duplicate entitlements
    cmd.assert()
        .success();
}