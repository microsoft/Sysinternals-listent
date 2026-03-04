use std::process::Command;
use tempfile::TempDir;
use anyhow::Result;

mod helpers;
use helpers::{TestEnvironment, TestRunner};

#[test]
fn test_static_scan_with_controlled_binaries() -> Result<()> {
    // Create controlled test environment
    let test_env = TestEnvironment::new()?;
    let runner = TestRunner::new(10); // 10 second timeout
    
    // Run scan on our controlled test directory
    let result = runner.run_scan(&[
        test_env.path().to_str().unwrap(),
        "--json"
    ])?;
    
    assert!(result.was_successful(), "Scan should succeed");
    
    // Parse JSON output
    let json: serde_json::Value = serde_json::from_str(&result.stdout)?;
    
    // Verify we found our test binaries
    let results = json["results"].as_array().unwrap();
    
    // Should find at least our test binaries with entitlements
    let binaries_with_entitlements: Vec<_> = results.iter()
        .filter(|r| r["entitlement_count"].as_u64().unwrap() > 0)
        .collect();
    
    // We created 3 binaries with entitlements (test_network, test_debug, test_multi)
    assert!(binaries_with_entitlements.len() >= 3, 
        "Should find at least 3 binaries with entitlements, found: {}", 
        binaries_with_entitlements.len());
    
    // Verify specific entitlements are found
    let stdout_text = &result.stdout;
    assert!(stdout_text.contains("com.apple.security.network.client"), 
        "Should find network client entitlement");
    assert!(stdout_text.contains("com.apple.security.get-task-allow"), 
        "Should find debug entitlement");
    
    Ok(())
}

#[test]
fn test_static_scan_with_entitlement_filter() -> Result<()> {
    let test_env = TestEnvironment::new()?;
    let runner = TestRunner::new(10);
    
    // Run scan with specific entitlement filter
    let result = runner.run_scan(&[
        test_env.path().to_str().unwrap(),
        "-e", "com.apple.security.network.client",
        "--json"
    ])?;
    
    assert!(result.was_successful(), "Filtered scan should succeed");
    
    let json: serde_json::Value = serde_json::from_str(&result.stdout)?;
    let results = json["results"].as_array().unwrap();
    
    // Should only find binaries with the network client entitlement
    for result in results {
        let entitlements = result["entitlements"].as_object().unwrap();
        assert!(entitlements.contains_key("com.apple.security.network.client"),
            "All results should contain the filtered entitlement");
    }
    
    Ok(())
}

#[test]
fn test_static_scan_human_readable_output() -> Result<()> {
    let test_env = TestEnvironment::new()?;
    let runner = TestRunner::new(10);
    
    // Run scan with human-readable output (default)
    let result = runner.run_scan(&[
        test_env.path().to_str().unwrap()
    ])?;
    
    assert!(result.was_successful(), "Scan should succeed");
    
    // Check for human-readable format indicators
    assert!(result.stdout.contains("Found"), "Should show 'Found' summary");
    assert!(result.stdout.contains("Scan Summary:"), "Should show scan summary");
    assert!(result.stdout.contains("files"), "Should mention files in summary");
    
    Ok(())
}

#[test]
fn test_static_scan_interrupt_handling() -> Result<()> {
    let test_env = TestEnvironment::new()?;
    
    // Create a large directory structure to give us time to interrupt
    create_large_test_structure(&test_env)?;
    
    let start = std::time::Instant::now();
    
    // Start scan process
    let child = Command::new("./target/release/listent")
        .arg(test_env.path())
        .arg("--quiet") // Reduce output noise
        .spawn()?;
    
    // Let it run for a short time
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Send SIGINT
    let pid = child.id() as i32;
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }
    
    // Wait for it to exit
    let result = child.wait_with_output()?;
    let duration = start.elapsed();
    
    // Should exit within reasonable time (less than 5 seconds total)
    assert!(duration < std::time::Duration::from_secs(5), 
        "Process should exit quickly after interrupt");
    
    // Exit code should be 0 (clean exit on interrupt)
    assert_eq!(result.status.code(), Some(0), 
        "Should exit cleanly on interrupt");
    
    Ok(())
}

fn create_large_test_structure(test_env: &TestEnvironment) -> Result<()> {
    use std::fs;
    
    // Create nested directories with some files
    for i in 0..10 {
        let dir_path = test_env.path().join(format!("subdir_{}", i));
        fs::create_dir_all(&dir_path)?;
        
        for j in 0..10 {
            let file_path = dir_path.join(format!("file_{}.txt", j));
            fs::write(file_path, "dummy content")?;
        }
    }
    
    Ok(())
}

#[test]
fn test_nonexistent_path_handling() -> Result<()> {
    let runner = TestRunner::new(5);
    
    let result = runner.run_scan(&[
        "/nonexistent/path/that/should/not/exist"
    ])?;
    
    // Should handle gracefully - either success with empty results or informative error
    // The exact behavior depends on implementation, but it shouldn't crash
    assert!(result.exit_code.is_some(), "Should exit with a status code");
    
    Ok(())
}

#[test]
fn test_empty_directory_scan() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let runner = TestRunner::new(5);
    
    let result = runner.run_scan(&[
        temp_dir.path().to_str().unwrap(),
        "--json"
    ])?;
    
    assert!(result.was_successful(), "Empty directory scan should succeed");
    
    let json: serde_json::Value = serde_json::from_str(&result.stdout)?;
    let results = json["results"].as_array().unwrap();
    
    assert_eq!(results.len(), 0, "Empty directory should yield no results");
    
    Ok(())
}