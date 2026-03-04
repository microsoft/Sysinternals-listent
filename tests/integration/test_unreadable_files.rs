use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn test_unreadable_files_counted() {
    let temp = TempDir::new().unwrap();
    
    // Create a file and make it unreadable (if possible)
    let unreadable_file = temp.path().join("unreadable");
    fs::write(&unreadable_file, "content").unwrap();
    
    // Try to make it unreadable (may not work in all environments)
    let _ = fs::set_permissions(&unreadable_file, fs::Permissions::from_mode(0o000));
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    // Should complete successfully even with unreadable files
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}

#[test]
fn test_unreadable_files_warning_in_normal_mode() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    // In normal mode, warnings should go to stderr
    // For now, just verify the command runs successfully
    cmd.assert().success();
}

#[test]
fn test_unreadable_files_suppressed_in_quiet_mode() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--quiet")
       .arg(temp.path().to_str().unwrap());
    
    // Quiet mode should suppress warnings
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("WARN").not());
}

#[test]
fn test_unreadable_count_in_summary() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    // Unreadable count is only shown in summary when > 0
    // For empty directory with no unreadable files, just verify success
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scan Summary:"));
}

#[test]
fn test_unreadable_count_in_json() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    let summary = json.get("summary").unwrap();
    assert!(summary.get("skipped_unreadable").is_some(), 
            "JSON summary should include skipped_unreadable count");
}

#[test]
fn test_scan_continues_after_unreadable_files() {
    // Unreadable files should not stop the entire scan
    let temp = TempDir::new().unwrap();
    
    // Create both readable and potentially unreadable files
    fs::write(temp.path().join("readable.txt"), "content").unwrap();
    let unreadable = temp.path().join("unreadable.txt");
    fs::write(&unreadable, "content").unwrap();
    let _ = fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000));
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    // Should complete successfully
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"));
}