use tempfile::TempDir;
use serde_json::Value;

#[test]
fn test_json_output_flag() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    
    // JSON output is pretty-printed, so check structure by parsing
    let json: Value = serde_json::from_str(&json_str)
        .expect("Output should be valid JSON");
    
    assert!(json.get("results").is_some(), "Should have results field");
    assert!(json.get("summary").is_some(), "Should have summary field");
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
    
    // Parse both outputs and compare structure (ignoring duration_ms)
    let json1: Value = serde_json::from_str(&String::from_utf8(output1).unwrap()).unwrap();
    let json2: Value = serde_json::from_str(&String::from_utf8(output2).unwrap()).unwrap();
    
    // Results should be the same
    assert_eq!(json1.get("results"), json2.get("results"), "Results should be deterministic");
    
    // Summary fields (except duration) should be the same
    let summary1 = json1.get("summary").unwrap();
    let summary2 = json2.get("summary").unwrap();
    assert_eq!(summary1.get("scanned"), summary2.get("scanned"));
    assert_eq!(summary1.get("matched"), summary2.get("matched"));
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
    
    // Verify human output has summary text, JSON has structure
    let human_str = String::from_utf8(human_output).unwrap();
    let json_str = String::from_utf8(json_output).unwrap();
    
    assert!(human_str.contains("Scan Summary:"), "Human output should have Scan Summary");
    assert!(json_str.contains("\"results\""), "JSON output should have results field");
}