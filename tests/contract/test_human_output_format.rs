use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_human_output_format_structure() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"---\nScanned: \d+\nMatched: \d+").unwrap());
}

#[test]
fn test_human_output_summary_format() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8(output).unwrap();
    
    // Should have summary block with required fields
    assert!(output_str.contains("Scanned:"), "Missing 'Scanned:' in summary");
    assert!(output_str.contains("Matched:"), "Missing 'Matched:' in summary");  
    assert!(output_str.contains("Skipped (unreadable):"), "Missing 'Skipped (unreadable):' in summary");
    assert!(output_str.contains("Duration:"), "Missing 'Duration:' in summary");
    assert!(output_str.contains("ms"), "Missing 'ms' duration unit");
}

#[test]
fn test_human_output_no_matches_case() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap())
       .arg("--entitlement").arg("com.nonexistent.entitlement.key");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("(no matches)"));
}

#[test]
fn test_entitlement_line_format() {
    // This test will be more meaningful once we have actual test binaries
    // For now, validate that when entitlements are found, they follow the format:
    // "  entitlement: key=value"
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg(temp.path().to_str().unwrap());
    
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8(output).unwrap();
    
    // Look for entitlement lines if any results exist
    if !output_str.contains("(no matches)") {
        // Each entitlement line should start with two spaces and contain "entitlement:"
        for line in output_str.lines() {
            if line.trim_start().starts_with("entitlement:") {
                assert!(line.starts_with("  "), "Entitlement lines should be indented with 2 spaces");
                assert!(line.contains("="), "Entitlement lines should contain '=' separator");
            }
        }
    }
}

#[test]
fn test_quiet_mode_suppresses_warnings() {
    let temp = TempDir::new().unwrap();
    
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--quiet")
       .arg(temp.path().to_str().unwrap());
    
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("WARN").not());
}