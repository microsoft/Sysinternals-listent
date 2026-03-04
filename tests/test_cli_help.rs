use predicates::prelude::*;

#[test]
fn test_help_includes_required_options() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("PATH"))  // Positional argument, not --path
        .stdout(predicate::str::contains("--entitlement"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--quiet"))
        .stdout(predicate::str::contains("--version"))
        .stdout(predicate::str::contains("--help"));
}

#[test]
fn test_help_describes_path_option() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Directory"))
        .stdout(predicate::str::contains("path"));
}

#[test]
fn test_help_describes_entitlement_filter() {
    let mut cmd = assert_cmd::cargo_bin_cmd!("listent");
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("entitlement"))
        .stdout(predicate::str::contains("Filter"));
}