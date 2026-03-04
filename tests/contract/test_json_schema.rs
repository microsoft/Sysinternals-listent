use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn test_json_output_structure() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: Value = serde_json::from_str(&json_str).expect("Invalid JSON output");
    
    // Validate top-level structure
    assert!(json.is_object(), "Output should be a JSON object");
    assert!(json.get("results").is_some(), "Missing 'results' field");
    assert!(json.get("summary").is_some(), "Missing 'summary' field");
    
    // Validate results array
    let results = json.get("results").unwrap();
    assert!(results.is_array(), "'results' should be an array");
    
    // Validate summary object
    let summary = json.get("summary").unwrap();
    assert!(summary.is_object(), "'summary' should be an object");
    assert!(summary.get("scanned").is_some(), "Missing 'scanned' in summary");
    assert!(summary.get("matched").is_some(), "Missing 'matched' in summary");
    assert!(summary.get("skipped_unreadable").is_some(), "Missing 'skipped_unreadable' in summary");
    assert!(summary.get("duration_ms").is_some(), "Missing 'duration_ms' in summary");
}

#[test]
fn test_json_result_entry_structure() {
    // When we have results, they should have the correct structure
    // This test will be enhanced once we have test fixtures with actual binaries
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: Value = serde_json::from_str(&json_str).unwrap();
    
    let results = json.get("results").unwrap().as_array().unwrap();
    
    // For each result entry (if any), validate structure
    for result in results {
        assert!(result.get("path").is_some(), "Result missing 'path' field");
        assert!(result.get("entitlements").is_some(), "Result missing 'entitlements' field");
        assert!(result.get("entitlement_count").is_some(), "Result missing 'entitlement_count' field");
        
        // Validate types
        assert!(result.get("path").unwrap().is_string(), "'path' should be string");
        assert!(result.get("entitlements").unwrap().is_object(), "'entitlements' should be object");
        assert!(result.get("entitlement_count").unwrap().is_number(), "'entitlement_count' should be number");
    }
}

#[test]
fn test_json_no_extra_fields() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: Value = serde_json::from_str(&json_str).unwrap();
    
    // Top level should only have "results" and "summary"
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 2, "Top-level object should only have 'results' and 'summary'");
}