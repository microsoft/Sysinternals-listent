use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_no_matches_human_output() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.definitely.nonexistent.entitlement.xyz");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No binaries found").or(predicate::str::contains("Matched: 0")));
}

#[test]
fn test_no_matches_json_output() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--json")
       .arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.definitely.nonexistent.entitlement.xyz");
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let json_str = String::from_utf8(output).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    
    // Results array should be empty
    let results = json.get("results").unwrap().as_array().unwrap();
    assert_eq!(results.len(), 0, "Results should be empty array");
    
    // Summary should show 0 matches
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("matched").unwrap().as_u64().unwrap(), 0);
}

#[test]
fn test_no_matches_exit_code_success() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.definitely.nonexistent.entitlement.xyz");
    
    // Zero matches should still exit with code 0 (success)
    cmd.assert().success();
}

#[test]
fn test_no_matches_includes_summary() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.definitely.nonexistent.entitlement.xyz");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned:"))
        .stdout(predicate::str::contains("Duration:"))
        .stdout(predicate::str::contains("Matched: 0"));
}

#[test]
fn test_empty_directory_no_matches() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Scanned: 0")
                .or(predicate::str::contains("(no matches)")));
}