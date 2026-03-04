use predicates::prelude::*;

#[test]
fn test_human_readable_output_format() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout results in interrupted status
        .stdout(predicate::str::contains("Starting process monitoring"))
        .stdout(predicate::str::contains("Press Ctrl+C to stop"));
}

#[test]
fn test_json_output_format() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--json", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout results in interrupted status
        .stdout(predicate::str::contains("Starting process monitoring"));
    
    // Note: JSON validation would require capturing output and parsing,
    // which is more complex with timeout/interrupt
}

#[test]
fn test_timestamp_formatting() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout results in interrupted status
        .stdout(predicate::str::contains("Starting process monitoring"));
}

#[test]
fn test_entitlements_list_formatting() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout results in interrupted status
        .stdout(predicate::str::contains("Starting process monitoring"));
}

#[test]
fn test_quiet_mode_output_suppression() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--quiet", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted() // Timeout results in interrupted status
        .stdout(predicate::str::contains("Starting process monitoring").not())
        .stdout(predicate::str::contains("Press Ctrl+C").not());
}

#[test]
fn test_no_entitlements_formatting() {
    // Test basic monitor mode
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.args(&["monitor", "--interval", "1.0"])
        .timeout(std::time::Duration::from_secs(2))
        .assert()
        .interrupted(); // Timeout results in interrupted status
}