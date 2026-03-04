use std::process::{Command, Child, Stdio};
use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::thread;
use anyhow::Result;

/// Test harness that ensures reliable cleanup and timeout handling
pub struct ReliableTestRunner {
    timeout: Duration,
    cleanup_handles: Vec<CleanupHandle>,
}

enum CleanupHandle {
    Process(u32), // PID to kill
}

impl ReliableTestRunner {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_seconds),
            cleanup_handles: Vec::new(),
        }
    }
    
    /// Run a command with automatic timeout and cleanup
    pub fn run_command_with_timeout(&mut self, mut cmd: Command) -> Result<TestOutput> {
        let start = Instant::now();
        // Capture stdout and stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        
        // Register for cleanup
        self.cleanup_handles.push(CleanupHandle::Process(child.id()));
        
        // Set up timeout mechanism
        let (tx, rx) = mpsc::channel();
        let timeout = self.timeout;
        
        // Spawn timeout thread
        thread::spawn(move || {
            thread::sleep(timeout);
            tx.send(()).ok();
        });
        
        // Wait for either completion or timeout
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process already finished
                let output = child.wait_with_output()?;
                Ok(TestOutput::from_output(output, start.elapsed()))
            },
            Ok(None) => {
                // Process still running, wait with timeout
                self.wait_with_timeout(child, rx, timeout)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to check child process: {}", e)),
        }
    }
    
    /// Run listent in monitor mode with controlled interruption
    #[allow(dead_code)]
    pub fn run_monitor_with_interrupt(&mut self, args: &[&str], interrupt_after: Duration) -> Result<TestOutput> {
        let _start = Instant::now();
        
        let mut cmd = Command::new("./target/release/listent");
        cmd.arg("monitor");
        for arg in args {
            cmd.arg(arg);
        }
        // Capture stdout and stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let child = cmd.spawn()?;
        self.cleanup_handles.push(CleanupHandle::Process(child.id()));
        
        // Wait for specified duration
        thread::sleep(interrupt_after);
        
        // Send SIGINT
        self.send_sigint(child.id())?;
        
        // Wait for graceful shutdown (with timeout)
        let shutdown_timeout = Duration::from_secs(5);
        let (tx, rx) = mpsc::channel();
        
        thread::spawn(move || {
            thread::sleep(shutdown_timeout);
            tx.send(()).ok();
        });
        
        let result = self.wait_with_timeout(child, rx, shutdown_timeout)?;
        Ok(result)
    }
    
    /// Wait for child with timeout using channel signaling
    fn wait_with_timeout(&self, mut child: Child, timeout_rx: mpsc::Receiver<()>, _timeout: Duration) -> Result<TestOutput> {
        let start = Instant::now();
        
        // Use a more reliable approach with try_wait in a loop
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Process finished, get output
                    break;
                },
                Ok(None) => {
                    // Still running, check timeout
                    if timeout_rx.try_recv().is_ok() {
                        // Timeout reached, kill process
                        let _ = child.kill();
                        let output = child.wait_with_output()?;
                        return Ok(TestOutput::from_output_timeout(output, start.elapsed()));
                    }
                    // Sleep briefly before checking again
                    thread::sleep(Duration::from_millis(100));
                },
                Err(e) => return Err(anyhow::anyhow!("Error waiting for child: {}", e)),
            }
        }
        
        // Process finished, collect output
        let output = child.wait_with_output()?;
        Ok(TestOutput::from_output(output, start.elapsed()))
    }
    
    /// Send SIGINT to a process
    fn send_sigint(&self, pid: u32) -> Result<()> {
        unsafe {
            let result = libc::kill(pid as i32, libc::SIGINT);
            if result != 0 {
                return Err(anyhow::anyhow!("Failed to send SIGINT to PID {}", pid));
            }
        }
        Ok(())
    }
    
    /// Kill a process forcefully
    fn kill_process(&self, pid: u32) -> Result<()> {
        unsafe {
            let result = libc::kill(pid as i32, libc::SIGKILL);
            if result != 0 {
                // Process might already be dead, which is fine
                return Ok(());
            }
        }
        Ok(())
    }
}

impl Drop for ReliableTestRunner {
    fn drop(&mut self) {
        // Clean up all registered resources
        for handle in &self.cleanup_handles {
            match handle {
                CleanupHandle::Process(pid) => {
                    // Try graceful shutdown first, then force kill
                    let _ = self.send_sigint(*pid);
                    thread::sleep(Duration::from_millis(500));
                    let _ = self.kill_process(*pid);
                },
            }
        }
    }
}

#[derive(Debug)]
pub struct TestOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub duration: Duration,
    pub timed_out: bool,
}

impl TestOutput {
    fn from_output(output: std::process::Output, duration: Duration) -> Self {
        Self {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            timed_out: false,
        }
    }
    
    fn from_output_timeout(output: std::process::Output, duration: Duration) -> Self {
        Self {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            timed_out: true,
        }
    }
    
    pub fn was_successful(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }
    
    pub fn contains_stdout(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }
    
    #[allow(dead_code)]
    pub fn contains_stderr(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }
    
    #[allow(dead_code)]
    pub fn contains_output(&self, text: &str) -> bool {
        self.stdout.contains(text) || self.stderr.contains(text)
    }
}

/// Spawn a test process that can be easily controlled and cleaned up
struct ControlledTestProcess {
    child: Child,
}

impl ControlledTestProcess {
    fn spawn(_name: &str, binary_path: &std::path::Path, duration_seconds: f64, _expected_entitlements: Vec<String>) -> Result<Self> {
        let child = Command::new(binary_path)
            .arg(duration_seconds.to_string())
            .spawn()?;
            
        Ok(Self {
            child,
        })
    }
    
    fn pid(&self) -> u32 {
        self.child.id()
    }
    
    fn terminate(&mut self) -> Result<()> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }
}

impl Drop for ControlledTestProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

/// Test scenario builder for complex integration tests
#[allow(dead_code)]
pub struct TestScenario {
    #[allow(dead_code)]
    name: String,
    processes: Vec<ControlledTestProcess>,
    runner: ReliableTestRunner,
}

impl TestScenario {
    #[allow(dead_code)]
    pub fn new(name: &str, timeout_seconds: u64) -> Self {
        Self {
            name: name.to_string(),
            processes: Vec::new(),
            runner: ReliableTestRunner::new(timeout_seconds),
        }
    }
    
    #[allow(dead_code)]
    pub fn spawn_process(&mut self, name: &str, binary_path: &std::path::Path, duration: f64, entitlements: Vec<String>) -> Result<()> {
        let process = ControlledTestProcess::spawn(name, binary_path, duration, entitlements)?;
        self.processes.push(process);
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn run_monitor_test(&mut self, monitor_args: &[&str], test_duration: Duration) -> Result<TestOutput> {
        // Start monitor
        let result = self.runner.run_monitor_with_interrupt(monitor_args, test_duration)?;
        
        // Clean up any remaining processes
        for process in &mut self.processes {
            let _ = process.terminate();
        }
        
        Ok(result)
    }
    
    #[allow(dead_code)]
    pub fn get_process_pids(&self) -> Vec<u32> {
        self.processes.iter().map(|p| p.pid()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reliable_runner_timeout() -> Result<()> {
        let mut runner = ReliableTestRunner::new(2); // 2 second timeout
        
        // Run a command that should timeout (sleep longer than timeout)
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let result = runner.run_command_with_timeout(cmd)?;
        
        assert!(result.timed_out, "Should have timed out");
        assert!(result.duration >= Duration::from_secs(2), "Should respect timeout");
        
        Ok(())
    }
    
    #[test]
    fn test_reliable_runner_success() -> Result<()> {
        let mut runner = ReliableTestRunner::new(5);
        
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let result = runner.run_command_with_timeout(cmd)?;
        
        assert!(!result.timed_out, "Should not timeout");
        assert!(result.was_successful(), "Should succeed");
        assert!(result.contains_stdout("hello"), "Should contain expected output");
        
        Ok(())
    }
}