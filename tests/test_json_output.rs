use predicates::prelude::*;
use tempfile::TempDir;
use serde_json::Value;

#[test]
fn test_json_output_flag() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn test_json_output_is_valid_json() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    
    // Should parse as valid JSON
    let _: Value = serde_json::from_str(&json_str)
        .expect("JSON output should be valid JSON");
}

#[test]
fn test_json_output_deterministic_ordering() {
    let temp = TempDir::new().unwrap();
    
    // Run the same command twice
    let mut cmd1 = assert_cmd::cargo_bin_cmd!("listent");
    cmd1.arg("--json").arg(temp.path().to_str().unwrap());
    let output1 = cmd1.assert().success().get_output().stdout.clone();
    
    let mut cmd2 = assert_cmd::cargo_bin_cmd!("listent");
    cmd2.arg("--json").arg(temp.path().to_str().unwrap());
    let output2 = cmd2.assert().success().get_output().stdout.clone();
    
    // Parse JSON and compare non-timing fields
    let json1: serde_json::Value = serde_json::from_slice(&output1).unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&output2).unwrap();
    
    // Check that structure is the same (ignoring duration_ms which can vary)
    assert_eq!(json1["results"], json2["results"], "Results should be deterministic");
    assert_eq!(json1["summary"]["scanned"], json2["summary"]["scanned"]);
    assert_eq!(json1["summary"]["matched"], json2["summary"]["matched"]);
    assert_eq!(json1["summary"]["skipped_unreadable"], json2["summary"]["skipped_unreadable"]);
}

#[test]
fn test_json_output_with_filters() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.apple.security.app-sandbox");
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: Value = serde_json::from_str(&json_str).unwrap();
    
    // Should still have valid structure with filters
    assert!(json.get("results").unwrap().is_array());
    assert!(json.get("summary").unwrap().is_object());
}

#[test]
fn test_json_vs_human_output_different() {
    let temp = TempDir::new().unwrap();
    
    // Human output
    let mut cmd1 = assert_cmd::cargo_bin_cmd!("listent");
    cmd1.arg(temp.path().to_str().unwrap());
    let human_output = cmd1.assert().success().get_output().stdout.clone();
    
    // JSON output  
    let mut cmd2 = assert_cmd::cargo_bin_cmd!("listent");
    cmd2.arg("--json").arg(temp.path().to_str().unwrap());
    let json_output = cmd2.assert().success().get_output().stdout.clone();
    
    // Should be different formats
    assert_ne!(human_output, json_output, "JSON and human output should differ");
    
    // Human should contain "Scan Summary:" section, JSON should not
    let human_str = String::from_utf8(human_output).unwrap();
    let json_str = String::from_utf8(json_output).unwrap();
    
    assert!(human_str.contains("Scan Summary:"), "Human output should have summary separator");
    assert!(!json_str.contains("Scan Summary:"), "JSON output should not have summary separator");
}