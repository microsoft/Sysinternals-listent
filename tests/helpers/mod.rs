//! Test helpers for listent functional tests
//!
//! Provides controlled test environment and utilities

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;

pub mod reliable_runner;

/// Test helper for creating controlled test environments
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub test_binaries: Vec<TestBinary>,
}

#[derive(Debug, Clone)]
pub struct TestBinary {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    #[allow(dead_code)]
    pub expected_entitlements: Vec<String>,
}

impl TestEnvironment {
    /// Create a new controlled test environment
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let mut env = TestEnvironment {
            temp_dir,
            test_binaries: Vec::new(),
        };
        
        // Create test binaries with known entitlements
        env.create_test_binaries()?;
        
        Ok(env)
    }
    
    /// Get the path to the test directory
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
    
    /// Create test binaries with predictable entitlements
    fn create_test_binaries(&mut self) -> anyhow::Result<()> {
        // Create a simple Swift program that we can sign with specific entitlements
        let swift_source = r#"
import Foundation
print("Test binary started with PID: \(getpid())")
// Keep running for a controllable amount of time
let args = CommandLine.arguments
if args.count > 1, let seconds = Double(args[1]) {
    Thread.sleep(forTimeInterval: seconds)
} else {
    Thread.sleep(forTimeInterval: 1.0)
}
print("Test binary exiting")
"#;
        
        // Create different test binaries with different entitlements
        let test_configs = vec![
            ("test_network", vec!["com.apple.security.network.client".to_string()]),
            ("test_debug", vec!["com.apple.security.get-task-allow".to_string()]),
            ("test_multi", vec![
                "com.apple.security.network.client".to_string(),
                "com.apple.security.network.server".to_string(),
            ]),
            ("test_no_entitlements", vec![]),
        ];
        
        for (name, entitlements) in test_configs {
            let binary_path = self.create_test_binary(name, swift_source, &entitlements)?;
            self.test_binaries.push(TestBinary {
                name: name.to_string(),
                path: binary_path,
                expected_entitlements: entitlements,
            });
        }
        
        Ok(())
    }
    
    /// Create a single test binary with specified entitlements
    fn create_test_binary(&self, name: &str, source: &str, entitlements: &[String]) -> anyhow::Result<PathBuf> {
        let source_path = self.temp_dir.path().join(format!("{}.swift", name));
        let binary_path = self.temp_dir.path().join(name);
        
        // Write Swift source
        fs::write(&source_path, source)?;
        
        // Compile Swift program
        let compile_result = Command::new("swiftc")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()?;
            
        if !compile_result.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to compile test binary {}: {}", 
                name, 
                String::from_utf8_lossy(&compile_result.stderr)
            ));
        }
        
        // Create entitlements plist if needed
        if !entitlements.is_empty() {
            let entitlements_plist = self.create_entitlements_plist(entitlements)?;
            let entitlements_path = self.temp_dir.path().join(format!("{}.entitlements", name));
            fs::write(&entitlements_path, entitlements_plist)?;
            
            // Sign the binary with entitlements
            let sign_result = Command::new("codesign")
                .arg("-s")
                .arg("-") // Ad-hoc signing
                .arg("--entitlements")
                .arg(&entitlements_path)
                .arg("-f") // Force
                .arg(&binary_path)
                .output()?;
                
            if !sign_result.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to sign test binary {}: {}", 
                    name, 
                    String::from_utf8_lossy(&sign_result.stderr)
                ));
            }
        } else {
            // Sign without entitlements (ad-hoc)
            let sign_result = Command::new("codesign")
                .arg("-s")
                .arg("-") // Ad-hoc signing
                .arg("-f") // Force
                .arg(&binary_path)
                .output()?;
                
            if !sign_result.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to sign test binary {}: {}", 
                    name, 
                    String::from_utf8_lossy(&sign_result.stderr)
                ));
            }
        }
        
        Ok(binary_path)
    }
    
    /// Create an entitlements plist for the given entitlements
    fn create_entitlements_plist(&self, entitlements: &[String]) -> anyhow::Result<String> {
        let mut plist = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
"#
        );
        
        for entitlement in entitlements {
            plist.push_str(&format!("    <key>{}</key>\n    <true/>\n", entitlement));
        }
        
        plist.push_str("</dict>\n</plist>\n");
        Ok(plist)
    }
    
    /// Spawn a test process that will run for the specified duration
    #[allow(dead_code)]
    pub fn spawn_test_process(&self, binary_name: &str, duration_seconds: f64) -> anyhow::Result<std::process::Child> {
        let binary = self.test_binaries.iter()
            .find(|b| b.name == binary_name)
            .ok_or_else(|| anyhow::anyhow!("Test binary '{}' not found", binary_name))?;
            
        let child = Command::new(&binary.path)
            .arg(duration_seconds.to_string())
            .spawn()?;
            
        Ok(child)
    }
}

/// Test runner with timeout and cleanup
pub struct TestRunner {
    #[allow(dead_code)]
    timeout_seconds: u64,
}

impl TestRunner {
    #[allow(dead_code)]
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_seconds }
    }
    
    /// Run listent scan mode and capture output
    #[allow(dead_code)]
    pub fn run_scan(&self, args: &[&str]) -> anyhow::Result<TestResult> {
        use std::process::Stdio;
        use std::thread;
        use std::time::{Duration, Instant};
        
        let start = Instant::now();
        
        let mut cmd = Command::new("./target/release/listent");
        for arg in args {
            cmd.arg(arg);
        }
        
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        let timeout = Duration::from_secs(self.timeout_seconds);
        let poll_interval = Duration::from_millis(100);
        
        // Poll for completion within timeout
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Process completed
                    let output = child.wait_with_output()?;
                    return Ok(TestResult {
                        exit_code: output.status.code(),
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        duration: start.elapsed(),
                    });
                }
                Ok(None) => {
                    // Still running, check timeout
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let output = child.wait_with_output()?;
                        return Ok(TestResult {
                            exit_code: None, // Killed
                            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                            duration: start.elapsed(),
                        });
                    }
                    thread::sleep(poll_interval);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    
    /// Run listent monitor mode and test CTRL-C
    #[allow(dead_code)]
    pub fn run_monitor_with_interrupt(&self, args: &[&str], interrupt_after_seconds: f64) -> anyhow::Result<TestResult> {
        use std::process::Stdio;
        use std::time::Duration;
        
        let start = std::time::Instant::now();
        
        let mut cmd = Command::new("./target/release/listent");
        cmd.arg("monitor");
        for arg in args {
            cmd.arg(arg);
        }
        
        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
            
        // Wait for the specified duration, then send SIGINT
        std::thread::sleep(Duration::from_secs_f64(interrupt_after_seconds));
        
        // Send SIGINT (same as CTRL-C)
        unsafe {
            libc::kill(child.id() as i32, libc::SIGINT);
        }
        
        // Wait for process to exit (with timeout)
        let result = child.wait_with_output()?;
        
        Ok(TestResult {
            exit_code: result.status.code(),
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            #[allow(dead_code)]
            duration: start.elapsed(),
        })
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    #[allow(dead_code)]
    pub duration: std::time::Duration,
}

impl TestResult {
    #[allow(dead_code)]
    pub fn was_successful(&self) -> bool {
        self.exit_code == Some(0)
    }
    
    #[allow(dead_code)]
    pub fn contains_stdout(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }
    
    #[allow(dead_code)]
    pub fn contains_stderr(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }
}