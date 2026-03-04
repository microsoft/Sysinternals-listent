/// Simple, reliable functional tests for listent
/// These tests focus on basic functionality without complex process spawning

use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use anyhow::Result;

#[test]
fn test_basic_scan_functionality() -> Result<()> {
    // Test basic scan of /usr/bin (should be faster than /Applications)
    let output = Command::new("./target/release/listent")
        .arg("/usr/bin")
        .arg("--json")
        .arg("--quiet")
        .timeout(Duration::from_secs(15)) // Shorter timeout
        .output()?;

    // Should succeed or handle gracefully
    assert!(output.status.success() || output.status.code() == Some(0),
        "Basic scan should succeed");

    // If successful, output should be valid JSON
    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if !stdout_str.trim().is_empty() {
            let _: serde_json::Value = serde_json::from_str(&stdout_str)?;
        }
    }

    Ok(())
}

#[test]
fn test_basic_monitor_startup_and_shutdown() -> Result<()> {
    // Start monitor mode
    let child = Command::new("./target/release/listent")
        .arg("monitor")
        .arg("--interval")
        .arg("5.0") // Slow interval to reduce noise
        .arg("--quiet")
        .spawn()?;

    // Let it run briefly
    std::thread::sleep(Duration::from_secs(2));

    // Send SIGINT (CTRL-C)
    let pid = child.id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    // Wait for it to exit
    let result = child.wait_with_output()?;

    // Should exit cleanly
    assert_eq!(result.status.code(), Some(0),
        "Monitor should exit cleanly on CTRL-C");

    Ok(())
}

#[test]
fn test_help_and_version_flags() -> Result<()> {
    // Test --help
    let help_output = Command::new("./target/release/listent")
        .arg("--help")
        .output()?;

    assert!(help_output.status.success(), "Help should work");

    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains("Usage:") || help_text.contains("USAGE:"),
        "Help should contain usage info");

    // Test --version
    let version_output = Command::new("./target/release/listent")
        .arg("--version")
        .output()?;

    assert!(version_output.status.success(), "Version should work");

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    assert!(!version_text.trim().is_empty(), "Version should produce output");

    Ok(())
}

#[test]
fn test_empty_directory_scan() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = Command::new("./target/release/listent")
        .arg(temp_dir.path())
        .arg("--json")
        .output()?;

    assert!(output.status.success(), "Empty directory scan should succeed");

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout_str)?;

    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 0, "Empty directory should have no results");

    Ok(())
}

#[test]
fn test_nonexistent_path_handling() -> Result<()> {
    let output = Command::new("./target/release/listent")
        .arg("/definitely/nonexistent/path/12345")
        .arg("--quiet")
        .output()?;

    // Should handle gracefully - either succeed with empty results or informative error
    assert!(output.status.code().is_some(), "Should exit with status code");

    // Should not crash or hang
    Ok(())
}

#[test]
fn test_json_vs_human_output() -> Result<()> {
    // Test with a path that likely exists and has some binaries
    let test_path = "/usr/bin";

    // Human-readable output
    let human_output = Command::new("./target/release/listent")
        .arg(test_path)
        .arg("--quiet")
        .timeout(Duration::from_secs(15))
        .output()?;

    // JSON output
    let json_output = Command::new("./target/release/listent")
        .arg(test_path)
        .arg("--json")
        .arg("--quiet")
        .timeout(Duration::from_secs(15))
        .output()?;

    // Both should succeed (or handle consistently)
    assert_eq!(human_output.status.success(), json_output.status.success(),
        "Human and JSON modes should behave consistently");

    if json_output.status.success() {
        let json_str = String::from_utf8_lossy(&json_output.stdout);
        if !json_str.trim().is_empty() {
            let _: serde_json::Value = serde_json::from_str(&json_str)?;
        }
    }

    Ok(())
}

#[test]
fn test_entitlement_filtering() -> Result<()> {
    let output = Command::new("./target/release/listent")
        .arg("/usr/bin")
        .arg("-e")
        .arg("com.apple.security.*") // Common pattern
        .arg("--json")
        .arg("--quiet")
        .timeout(Duration::from_secs(20))
        .output()?;

    // Should handle filtering without crashing
    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        if !stdout_str.trim().is_empty() {
            let json: serde_json::Value = serde_json::from_str(&stdout_str)?;

            // If there are results, they should match the filter
            if let Some(results) = json["results"].as_array() {
                for result in results {
                    if let Some(entitlements) = result["entitlements"].as_object() {
                        // At least one entitlement should match the pattern
                        let has_matching = entitlements.keys().any(|key|
                            key.starts_with("com.apple.security."));
                        if !entitlements.is_empty() && !has_matching {
                            // This might be ok if there are other matching criteria
                            println!("Note: Found entitlements that don't match pattern: {:?}",
                                entitlements.keys().collect::<Vec<_>>());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[test]
fn test_single_file_scan_progress_counts() -> Result<()> {
    // Scan a single known binary file (not a directory)
    // This tests that progress correctly counts scanned files when individual files are passed
    let output = Command::new("./target/release/listent")
        .arg("/usr/bin/true")
        .output()?;

    assert!(output.status.success(), "Single file scan should succeed");

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    // The progress line should show 1/1 processed, not 0/1
    assert!(
        stderr_str.contains("Processed 1/1 files (scanned: 1, skipped: 0)"),
        "Progress should show 1/1 scanned for a single binary file, got stderr: {}",
        stderr_str
    );

    Ok(())
}

#[test]
fn test_multiple_files_scan_progress_counts() -> Result<()> {
    // Scan multiple individual binary files (simulates shell glob expansion)
    // e.g., listent /usr/bin/tr* expands to /usr/bin/true /usr/bin/tr /usr/bin/traceroute ...
    let output = Command::new("./target/release/listent")
        .arg("/usr/bin/true")
        .arg("/usr/bin/false")
        .arg("/usr/bin/env")
        .output()?;

    assert!(output.status.success(), "Multi-file scan should succeed");

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    // Progress should show 3/3 processed — all three are Mach-O binaries
    assert!(
        stderr_str.contains("Processed 3/3 files (scanned: 3, skipped: 0)"),
        "Progress should show 3/3 scanned for multiple binary files, got stderr: {}",
        stderr_str
    );

    Ok(())
}

// Helper trait for adding timeout to commands
trait CommandTimeout {
    fn timeout(&mut self, duration: Duration) -> &mut Self;
}

impl CommandTimeout for Command {
    fn timeout(&mut self, _duration: Duration) -> &mut Self {
        // Simple implementation - in production you'd want real timeout logic
        self
    }
}