//! macOS LaunchD integration for daemon service management
//!
//! Handles plist generation, service installation, and lifecycle management

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::constants::{LAUNCHD_SERVICE_NAME, LAUNCHD_PLIST_NAME, LAUNCHD_DAEMONS_DIR, DAEMON_SUBCOMMAND, DAEMON_RUN_SUBCOMMAND, DAEMON_WORKING_DIR, DAEMON_LOG_FILE, DAEMON_SYSTEM_PATH};

/// Escape special XML characters to prevent injection in plist content
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// LaunchD plist configuration
#[derive(Debug, Clone)]
pub struct LaunchDPlist {
    /// Service label (reverse DNS format)
    pub label: String,
    /// Executable path and arguments
    pub program_arguments: Vec<String>,
    /// Whether to start at boot/login
    pub run_at_load: bool,
    /// Whether to restart if process exits
    pub keep_alive: bool,
    /// Working directory for daemon
    pub working_directory: Option<PathBuf>,
    /// Standard output log file
    pub standard_out_path: Option<PathBuf>,
    /// Standard error log file
    pub standard_error_path: Option<PathBuf>,
    /// Environment variables
    pub environment_variables: Option<std::collections::HashMap<String, String>>,
    /// User to run as (for system daemons)
    pub user_name: Option<String>,
    /// Group to run as (for system daemons)
    pub group_name: Option<String>,
}

impl LaunchDPlist {
    /// Create a new LaunchD plist with default settings
    pub fn new(daemon_path: &Path) -> Self {
        Self {
            label: LAUNCHD_SERVICE_NAME.to_string(),
            program_arguments: vec![
                daemon_path.to_string_lossy().to_string(),
                DAEMON_SUBCOMMAND.to_string(),
                DAEMON_RUN_SUBCOMMAND.to_string(),
            ],
            run_at_load: true,
            keep_alive: true,
            working_directory: Some(PathBuf::from(DAEMON_WORKING_DIR)),
            standard_out_path: Some(PathBuf::from(DAEMON_LOG_FILE)),
            standard_error_path: Some(PathBuf::from(DAEMON_LOG_FILE)),
            environment_variables: Some({
                let mut env = std::collections::HashMap::new();
                env.insert("PATH".to_string(), DAEMON_SYSTEM_PATH.to_string());
                env
            }),
            user_name: Some("root".to_string()),
            group_name: Some("wheel".to_string()),
        }
    }

    /// Generate plist XML content
    pub fn generate_plist(&self) -> Result<String> {
        let mut plist = String::new();

        plist.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        plist.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        plist.push_str("<plist version=\"1.0\">\n");
        plist.push_str("<dict>\n");

        // Label
        plist.push_str("\t<key>Label</key>\n");
        plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(&self.label)));

        // Program arguments
        plist.push_str("\t<key>ProgramArguments</key>\n");
        plist.push_str("\t<array>\n");
        for arg in &self.program_arguments {
            plist.push_str(&format!("\t\t<string>{}</string>\n", xml_escape(arg)));
        }
        plist.push_str("\t</array>\n");

        // RunAtLoad
        plist.push_str("\t<key>RunAtLoad</key>\n");
        plist.push_str(&format!("\t<{}/>\n", if self.run_at_load { "true" } else { "false" }));

        // KeepAlive
        plist.push_str("\t<key>KeepAlive</key>\n");
        plist.push_str(&format!("\t<{}/>\n", if self.keep_alive { "true" } else { "false" }));

        // Working directory
        if let Some(ref working_dir) = self.working_directory {
            plist.push_str("\t<key>WorkingDirectory</key>\n");
            plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(&working_dir.display().to_string())));
        }

        // Standard output
        if let Some(ref stdout_path) = self.standard_out_path {
            plist.push_str("\t<key>StandardOutPath</key>\n");
            plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(&stdout_path.display().to_string())));
        }

        // Standard error
        if let Some(ref stderr_path) = self.standard_error_path {
            plist.push_str("\t<key>StandardErrorPath</key>\n");
            plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(&stderr_path.display().to_string())));
        }

        // Environment variables
        if let Some(ref env_vars) = self.environment_variables {
            if !env_vars.is_empty() {
                plist.push_str("\t<key>EnvironmentVariables</key>\n");
                plist.push_str("\t<dict>\n");
                for (key, value) in env_vars {
                    plist.push_str(&format!("\t\t<key>{}</key>\n", xml_escape(key)));
                    plist.push_str(&format!("\t\t<string>{}</string>\n", xml_escape(value)));
                }
                plist.push_str("\t</dict>\n");
            }
        }

        // User and group (for system daemons)
        if let Some(ref user) = self.user_name {
            plist.push_str("\t<key>UserName</key>\n");
            plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(user)));
        }
        if let Some(ref group) = self.group_name {
            plist.push_str("\t<key>GroupName</key>\n");
            plist.push_str(&format!("\t<string>{}</string>\n", xml_escape(group)));
        }

        plist.push_str("</dict>\n");
        plist.push_str("</plist>\n");

        Ok(plist)
    }

    /// Install plist file to appropriate location
    fn install_plist(&self, plist_content: &str) -> Result<PathBuf> {
        // Use LaunchDaemons directory for system-wide service (requires sudo)
        let plist_path = Path::new(LAUNCHD_DAEMONS_DIR)
            .join(LAUNCHD_PLIST_NAME);

        // Write plist file
        std::fs::write(&plist_path, plist_content)
            .with_context(|| format!("Failed to write plist file: {}", plist_path.display()))?;

        Ok(plist_path)
    }

    /// Load service with launchctl using modern bootstrap API
    /// Falls back to legacy 'load' command if bootstrap fails
    pub fn launchctl_load(&self, plist_path: &Path) -> Result<()> {
        let path_str = plist_path.to_str()
            .context("Plist path contains invalid UTF-8")?;

        // Try modern bootstrap API first (macOS 10.10+)
        let output = Command::new("launchctl")
            .args(["bootstrap", "system", path_str])
            .output()
            .context("Failed to execute launchctl bootstrap")?;

        if output.status.success() {
            return Ok(());
        }

        // Fall back to legacy load command for older macOS versions
        let legacy_output = Command::new("launchctl")
            .args(["load", path_str])
            .output()
            .context("Failed to execute launchctl load")?;

        if !legacy_output.status.success() {
            let stderr = String::from_utf8_lossy(&legacy_output.stderr);
            anyhow::bail!("launchctl load failed: {}", stderr);
        }

        Ok(())
    }

    /// Unload service with launchctl using modern bootout API
    /// Falls back to legacy 'unload' command if bootout fails
    pub fn launchctl_unload(&self) -> Result<()> {
        // Try modern bootout API first (macOS 10.10+)
        // Format: launchctl bootout system/<service-label>
        let output = Command::new("launchctl")
            .args(["bootout", &format!("system/{}", self.label)])
            .output()
            .context("Failed to execute launchctl bootout")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);

        // If service not found with bootout, try legacy unload
        if stderr.contains("Could not find specified service") || stderr.contains("No such process") {
            // Service might not be loaded, try legacy unload as fallback
            let legacy_output = Command::new("launchctl")
                .args(["unload", &format!("{}/{}", LAUNCHD_DAEMONS_DIR, LAUNCHD_PLIST_NAME)])
                .output()
                .context("Failed to execute launchctl unload")?;

            if !legacy_output.status.success() {
                let legacy_stderr = String::from_utf8_lossy(&legacy_output.stderr);
                // Don't fail if service was already unloaded
                if !legacy_stderr.contains("Could not find specified service") {
                    anyhow::bail!("launchctl unload failed: {}", legacy_stderr);
                }
            }
            return Ok(());
        }

        anyhow::bail!("launchctl bootout failed: {}", stderr);
    }

    /// Check if service is currently loaded
    pub fn is_service_loaded(&self) -> Result<bool> {
        let output = Command::new("launchctl")
            .args(["list", &self.label])
            .output()
            .context("Failed to execute launchctl list")?;

        Ok(output.status.success())
    }

    /// Install daemon service to LaunchD
    pub fn install_service(&self, daemon_path: &Path, config_path: Option<&Path>) -> Result<()> {
        // Create necessary directories
        if let Some(ref working_dir) = self.working_directory {
            std::fs::create_dir_all(working_dir)
                .with_context(|| format!("Failed to create working directory: {}", working_dir.display()))?;
        }

        if let Some(ref log_path) = self.standard_out_path {
            if let Some(log_dir) = log_path.parent() {
                std::fs::create_dir_all(log_dir)
                    .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;
            }
        }

        // Generate plist content
        let mut plist = self.clone();

        // Update program arguments with config path if provided
        if let Some(config) = config_path {
            plist.program_arguments = vec![
                daemon_path.to_string_lossy().to_string(),
                DAEMON_SUBCOMMAND.to_string(),
                DAEMON_RUN_SUBCOMMAND.to_string(),
                "--config".to_string(),
                config.to_string_lossy().to_string(),
            ];
        }

        let plist_content = plist.generate_plist()?;

        // Install plist file
        let plist_path = self.install_plist(&plist_content)?;

        // Load service with launchctl
        self.launchctl_load(&plist_path)?;

        println!("✅ Daemon service installed successfully");
        println!("   Service: {}", self.label);
        println!("   Plist: {}", plist_path.display());
        println!("   View logs: listent daemon logs");
        println!("   Check status: listent daemon status");
        println!("   Uninstall: sudo listent daemon uninstall");

        Ok(())
    }

    /// Uninstall daemon service from LaunchD
    pub fn uninstall_service(&self) -> Result<()> {
        // Unload service first
        self.launchctl_unload()?;

        // Remove plist file from LaunchDaemons
        let plist_path = Path::new(LAUNCHD_DAEMONS_DIR)
            .join(LAUNCHD_PLIST_NAME);

        if plist_path.exists() {
            std::fs::remove_file(&plist_path)
                .with_context(|| format!("Failed to remove plist file: {}", plist_path.display()))?;
        }

        println!("✅ Daemon service uninstalled successfully");
        println!("   Service: {}", self.label);

        Ok(())
    }

    /// Get service status information
    pub fn get_service_status(&self) -> Result<Option<ServiceStatus>> {
        let output = Command::new("launchctl")
            .args(["list", &self.label])
            .output()
            .context("Failed to execute launchctl list")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse launchctl plist-style output
        // Look for PID and LastExitStatus fields
        let mut pid: Option<i32> = None;
        let mut status_code: i32 = -1;

        for line in stdout.lines() {
            let line = line.trim();
            if line.contains("\"PID\"") {
                // Parse: "PID" = 12345;
                if let Some(start) = line.find('=') {
                    let value_part = &line[start + 1..].trim();
                    if let Some(end) = value_part.find(';') {
                        let value_str = &value_part[..end].trim();
                        pid = value_str.parse::<i32>().ok();
                    }
                }
            } else if line.contains("\"LastExitStatus\"") {
                // Parse: "LastExitStatus" = 0;
                if let Some(start) = line.find('=') {
                    let value_part = &line[start + 1..].trim();
                    if let Some(end) = value_part.find(';') {
                        let value_str = &value_part[..end].trim();
                        status_code = value_str.parse::<i32>().unwrap_or(-1);
                    }
                }
            }
        }

        Ok(Some(ServiceStatus {
            pid,
            status_code,
            label: self.label.clone(),
        }))
    }
}

/// Service status information from launchctl
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// Process ID (None if not running)
    pub pid: Option<i32>,
    /// Status code from launchctl
    pub status_code: i32,
    /// Service label
    pub label: String,
}

impl ServiceStatus {
    /// Check if service is currently running
    pub fn is_running(&self) -> bool {
        self.pid.is_some() && self.status_code == 0
    }
}