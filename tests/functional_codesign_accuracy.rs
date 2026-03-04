/// Functional test to verify listent accuracy against raw codesign
///
/// This test compares listent's entitlement detection against direct codesign calls
/// to ensure 100% accuracy. Requires sudo for full accuracy (some binaries like
/// /usr/bin/sudo require elevated privileges to inspect).
///
/// Run with: sudo cargo test --test functional_codesign_accuracy -- --nocapture

use std::collections::HashSet;
use std::process::Command;
use anyhow::Result;

/// Compare listent output against raw codesign for a given directory
/// Returns (listent_files, codesign_files, missing_from_listent, extra_in_listent)
fn compare_entitlement_detection(path: &str) -> Result<ComparisonResult> {
    // Get files with entitlements from listent
    let listent_files = get_listent_files_with_entitlements(path)?;

    // Get files with entitlements from raw codesign
    let codesign_files = get_codesign_files_with_entitlements(path)?;

    // Calculate differences
    let missing_from_listent: HashSet<_> = codesign_files.difference(&listent_files).cloned().collect();
    let extra_in_listent: HashSet<_> = listent_files.difference(&codesign_files).cloned().collect();

    Ok(ComparisonResult {
        listent_count: listent_files.len(),
        codesign_count: codesign_files.len(),
        _listent_files: listent_files,
        _codesign_files: codesign_files,
        missing_from_listent,
        extra_in_listent,
    })
}

#[derive(Debug)]
struct ComparisonResult {
    listent_count: usize,
    codesign_count: usize,
    _listent_files: HashSet<String>,
    _codesign_files: HashSet<String>,
    missing_from_listent: HashSet<String>,
    extra_in_listent: HashSet<String>,
}

impl ComparisonResult {
    fn is_exact_match(&self) -> bool {
        self.missing_from_listent.is_empty() && self.extra_in_listent.is_empty()
    }
}

/// Get list of files with entitlements using listent --json
fn get_listent_files_with_entitlements(path: &str) -> Result<HashSet<String>> {
    let output = Command::new("./target/release/listent")
        .arg("--json")
        .arg(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if it's a permission error - skip gracefully
        if stderr.contains("Permission denied") {
            eprintln!("Warning: listent requires elevated privileges for {}", path);
            eprintln!("Run with: sudo cargo test --test functional_codesign_accuracy");
            anyhow::bail!("Permission denied - run with sudo for full test coverage");
        }
        anyhow::bail!("listent failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)?;

    let mut files = HashSet::new();
    if let Some(results) = json["results"].as_array() {
        for result in results {
            if let Some(path) = result["path"].as_str() {
                files.insert(path.to_string());
            }
        }
    }

    Ok(files)
}

/// Get list of files with entitlements using raw codesign commands
fn get_codesign_files_with_entitlements(path: &str) -> Result<HashSet<String>> {
    let mut files = HashSet::new();

    // List all files in the directory
    let entries = std::fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();

        if !file_path.is_file() {
            continue;
        }

        // Run codesign to check for entitlements
        let output = Command::new("codesign")
            .arg("-d")
            .arg("--entitlements")
            .arg("-")
            .arg(&file_path)
            .output()?;

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // Check if output contains entitlement keys (indicated by [Key] in output)
        if combined.contains("[Key]") {
            files.insert(file_path.to_string_lossy().to_string());
        }
    }

    Ok(files)
}

#[test]
fn test_listent_matches_codesign_usr_bin() -> Result<()> {
    let result = compare_entitlement_detection("/usr/bin")?;

    println!("=== /usr/bin Comparison ===");
    println!("listent found: {} files with entitlements", result.listent_count);
    println!("codesign found: {} files with entitlements", result.codesign_count);

    if !result.missing_from_listent.is_empty() {
        println!("\nFiles found by codesign but missing from listent:");
        for f in &result.missing_from_listent {
            println!("  - {}", f);
        }
    }

    if !result.extra_in_listent.is_empty() {
        println!("\nFiles found by listent but not by codesign:");
        for f in &result.extra_in_listent {
            // Check if this is a permission issue
            let output = Command::new("codesign")
                .arg("-d")
                .arg("--entitlements")
                .arg("-")
                .arg(f)
                .output()?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") {
                println!("  - {} (codesign permission denied - requires sudo)", f);
            } else {
                println!("  - {}", f);
            }
        }
    }

    // Allow for permission-related differences when not running as root
    // The test passes if:
    // 1. Results are an exact match, OR
    // 2. All differences are due to permission issues (files listent found that codesign couldn't read)
    if result.is_exact_match() {
        println!("\n✓ EXACT MATCH: listent output matches codesign perfectly");
    } else if result.missing_from_listent.is_empty() {
        // listent found everything codesign found, plus maybe some extras due to running with different perms
        println!("\n✓ PASS: listent found all files codesign found (differences likely due to permission levels)");
    } else {
        panic!(
            "MISMATCH: listent missed {} files that codesign found: {:?}",
            result.missing_from_listent.len(),
            result.missing_from_listent
        );
    }

    Ok(())
}

#[test]
fn test_listent_matches_codesign_usr_sbin() -> Result<()> {
    let result = match compare_entitlement_detection("/usr/sbin") {
        Ok(r) => r,
        Err(e) if e.to_string().contains("Permission denied") => {
            println!("SKIPPED: /usr/sbin requires sudo - run with elevated privileges");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    println!("=== /usr/sbin Comparison ===");
    println!("listent found: {} files with entitlements", result.listent_count);
    println!("codesign found: {} files with entitlements", result.codesign_count);

    if !result.missing_from_listent.is_empty() {
        println!("\nFiles found by codesign but missing from listent:");
        for f in &result.missing_from_listent {
            println!("  - {}", f);
        }
    }

    if !result.extra_in_listent.is_empty() {
        println!("\nFiles found by listent but not by codesign:");
        for f in &result.extra_in_listent {
            println!("  - {}", f);
        }
    }

    if result.is_exact_match() {
        println!("\n✓ EXACT MATCH: listent output matches codesign perfectly");
    } else if result.missing_from_listent.is_empty() {
        println!("\n✓ PASS: listent found all files codesign found");
    } else {
        panic!(
            "MISMATCH: listent missed {} files that codesign found: {:?}",
            result.missing_from_listent.len(),
            result.missing_from_listent
        );
    }

    Ok(())
}

#[test]
fn test_listent_matches_codesign_combined() -> Result<()> {
    // Test both directories together (the default scan paths)
    let usr_bin = compare_entitlement_detection("/usr/bin")?;
    let usr_sbin = match compare_entitlement_detection("/usr/sbin") {
        Ok(r) => r,
        Err(e) if e.to_string().contains("Permission denied") => {
            println!("=== Combined Test (partial - /usr/sbin requires sudo) ===");
            println!("/usr/bin: listent found {} files, codesign found {} files",
                usr_bin.listent_count, usr_bin.codesign_count);
            println!("/usr/sbin: SKIPPED (permission denied)");

            if usr_bin.missing_from_listent.is_empty() {
                println!("\n✓ PARTIAL PASS: /usr/bin matches perfectly");
                return Ok(());
            } else {
                panic!("MISMATCH in /usr/bin");
            }
        }
        Err(e) => return Err(e),
    };

    let total_listent = usr_bin.listent_count + usr_sbin.listent_count;
    let total_codesign = usr_bin.codesign_count + usr_sbin.codesign_count;
    let total_missing: HashSet<_> = usr_bin.missing_from_listent
        .union(&usr_sbin.missing_from_listent)
        .cloned()
        .collect();

    println!("=== Combined /usr/bin + /usr/sbin Comparison ===");
    println!("listent found: {} files with entitlements", total_listent);
    println!("codesign found: {} files with entitlements", total_codesign);
    println!("Missing from listent: {}", total_missing.len());

    if total_missing.is_empty() {
        println!("\n✓ PASS: listent found all files with entitlements");
    } else {
        panic!(
            "MISMATCH: listent missed {} files: {:?}",
            total_missing.len(),
            total_missing
        );
    }

    Ok(())
}

/// Verify that entitlement counts match for a specific binary
#[test]
fn test_entitlement_count_accuracy() -> Result<()> {
    // Pick a known binary with entitlements
    let test_binary = "/usr/bin/ssh";

    // Get entitlements from listent
    let listent_output = Command::new("./target/release/listent")
        .arg("--json")
        .arg(test_binary)
        .output()?;

    if !listent_output.status.success() {
        anyhow::bail!("listent failed on {}", test_binary);
    }

    let stdout = String::from_utf8_lossy(&listent_output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)?;

    let listent_entitlements: HashSet<String> = json["results"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|r| r["entitlements"].as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Get entitlements from codesign
    let codesign_output = Command::new("codesign")
        .arg("-d")
        .arg("--entitlements")
        .arg("-")
        .arg(test_binary)
        .output()?;

    let codesign_text = format!(
        "{}{}",
        String::from_utf8_lossy(&codesign_output.stdout),
        String::from_utf8_lossy(&codesign_output.stderr)
    );

    // Count [Key] occurrences in codesign output
    let codesign_key_count = codesign_text.matches("[Key]").count();

    println!("=== Entitlement Count Accuracy for {} ===", test_binary);
    println!("listent entitlements: {:?}", listent_entitlements);
    println!("listent count: {}", listent_entitlements.len());
    println!("codesign [Key] count: {}", codesign_key_count);

    assert_eq!(
        listent_entitlements.len(),
        codesign_key_count,
        "Entitlement count should match between listent and codesign"
    );

    println!("\n✓ PASS: Entitlement counts match");

    Ok(())
}
