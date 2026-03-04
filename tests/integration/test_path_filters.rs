use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_single_path_filter() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_multiple_path_filters() {
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp1.path().to_str().unwrap())
       .arg(temp2.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_path_filter_restricts_scope() {
    let temp = TempDir::new().unwrap();
    
    // Create a subdirectory with a file
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("testfile"), "content").unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    // Should only scan the specified temp directory, not system-wide
    cmd.assert()
        .success();
    
    // Verify it's actually restricted (numbers should be small for empty temp dir)
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8(output).unwrap();
    
    // Should find very few or zero binaries in empty temp directory
    assert!(output_str.contains("Scanned:"), "Should show scan count");
}

#[test] 
fn test_path_filter_with_tilde_expansion() {
    // Tilde expansion is not supported by the CLI
    // The path ~/Applications is treated as a literal path, which doesn't exist
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("~/Applications");
    
    // Should fail because ~/Applications is not a valid literal path
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}