//! Daemon module for launchd integration and background process monitoring
//!
//! This module provides functionality to run listent as a macOS daemon:
//! - Configuration management with atomic updates
//! - LaunchD integration for system service management
//! - Enhanced Unified Logging System integration

pub mod config;
pub mod launchd;
pub mod logging;

use anyhow::{Context, Result, bail};
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::signal;
use crate::models::{PollingConfiguration, ProcessSnapshot, MonitoredProcess};
use crate::daemon::config::DaemonConfiguration;
use crate::constants::{APP_SUBSYSTEM, DAEMON_CATEGORY, DAEMON_SUBCOMMAND, DAEMON_RUN_SUBCOMMAND};
use crate::daemon::logging::{DaemonLogger, LogLevel};
use crate::monitor::process_tracker::ProcessTracker;

/// Check if a listent daemon process is already running
/// Returns true if any listent process with 'daemon run' subcommand is running
pub fn is_daemon_running() -> bool {
    !find_daemon_pids().is_empty()
}

/// Find PIDs of running listent daemon processes
/// Returns a list of PIDs matching the 'listent daemon run' pattern,
/// excluding the current process and sudo wrappers
pub fn find_daemon_pids() -> Vec<u32> {
    use sysinfo::{ProcessesToUpdate, System};

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let current_pid = std::process::id();

    system.processes()
        .iter()
        .filter_map(|(pid, process)| {
            let pid_u32 = pid.as_u32();

            // Skip current process
            if pid_u32 == current_pid {
                return None;
            }

            // Check command-line args for daemon run pattern
            let cmd = process.cmd();
            let has_listent = cmd.iter().any(|arg| arg.to_string_lossy().contains("listent"));
            let has_daemon = cmd.iter().any(|arg| arg == DAEMON_SUBCOMMAND);
            let has_run = cmd.iter().any(|arg| arg == DAEMON_RUN_SUBCOMMAND);
            let is_sudo = process.name() == "sudo";

            if has_listent && has_daemon && has_run && !is_sudo {
                Some(pid_u32)
            } else {
                None
            }
        })
        .collect()
}

/// Daemon runtime state
struct DaemonState {
    /// Current configuration
    config: Arc<Mutex<DaemonConfiguration>>,
    /// Process tracker for monitoring
    process_tracker: Arc<Mutex<ProcessTracker>>,
    /// Daemon logger
    logger: DaemonLogger,
}

impl DaemonState {
    /// Create new daemon state with configuration
    fn new(config: DaemonConfiguration) -> Result<Self> {
        let logger = DaemonLogger::new(
            APP_SUBSYSTEM.to_string(),
            DAEMON_CATEGORY.to_string(),
            LogLevel::Info,
        )?;

        let process_tracker = ProcessTracker::new();

        Ok(Self {
            config: Arc::new(Mutex::new(config)),
            process_tracker: Arc::new(Mutex::new(process_tracker)),
            logger,
        })
    }
}

/// Run daemon with specific configuration path
pub async fn run_daemon_with_config(config_path: Option<PathBuf>) -> Result<()> {
    // Check if we're running under LaunchD (no need to fork - LaunchD manages us)
    if std::env::var("XPC_SERVICE_NAME").is_ok() ||
       std::env::var("LISTENT_DAEMON_CHILD").is_ok() {
        // We're already managed by LaunchD or we're the child process - run directly
        run_daemon_process(config_path).await
    } else {
        // We're being run manually - spawn child and exit parent
        spawn_daemon_child(config_path).await
    }
}

/// Spawn daemon as detached child process and exit parent
async fn spawn_daemon_child(config_path: Option<PathBuf>) -> Result<()> {
    // Load configuration
    let config = if let Some(ref path) = config_path {
        DaemonConfiguration::load_from_file(path)?
    } else {
        DaemonConfiguration::default()
    };

    // Check if daemon is already running BEFORE spawning
    if is_daemon_running() {
        anyhow::bail!(
            "Daemon already running, please stop it first."
        );
    }

    let current_exe = std::env::current_exe()
        .context("Failed to get current executable path")?;

    let mut cmd = std::process::Command::new(current_exe);
    cmd.env("LISTENT_DAEMON_CHILD", "1");
    cmd.args([DAEMON_SUBCOMMAND, DAEMON_RUN_SUBCOMMAND]);

    if let Some(config) = config_path {
        cmd.arg("--config").arg(config);
    }

    // Pipe stdout so child can signal readiness via anonymous pipe
    cmd.stdout(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .context("Failed to spawn daemon child process")?;

    println!("🚀 listent daemon starting...");

    // Wait for child to signal readiness or detect early crash via pipe EOF.
    // The child writes "READY" to stdout after successful initialization;
    // if it crashes, the pipe closes and read_line returns Ok(0).
    let stdout = child.stdout.take()
        .context("Failed to capture child stdout")?;
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();

    // Use a timeout to avoid hanging forever if child neither writes nor exits
    let ready_result = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || reader.read_line(&mut line).map(|n| (n, line))),
    ).await;

    match ready_result {
        Ok(Ok(Ok((0, _)))) => {
            // Pipe closed — child exited before signaling ready
            let status = child.try_wait().ok().flatten();
            let exit_info = status.map_or("unknown".to_string(), |s| format!("{}", s));
            eprintln!("❌ Failed to start listent daemon");
            eprintln!("   The daemon process exited before becoming ready (exit: {})", exit_info);
            eprintln!("   Check logs: listent daemon logs");
            bail!("Daemon process exited before becoming ready")
        }
        Ok(Ok(Ok((_n, ref msg)))) if msg.trim() == "READY" => {
            println!("✅ listent daemon started successfully");
            println!("  Polling interval: {}s", config.daemon.polling_interval);
            println!("  View logs: listent daemon logs");
            println!("  Check status: listent daemon status");
            println!("  Stop daemon: listent daemon stop");
            Ok(())
        }
        Ok(Ok(Ok((_n, msg)))) => {
            eprintln!("❌ Failed to start listent daemon");
            eprintln!("   Unexpected daemon output: {}", msg.trim());
            bail!("Unexpected daemon output")
        }
        Ok(Ok(Err(e))) => {
            eprintln!("❌ Failed to start listent daemon");
            eprintln!("   Failed reading from daemon process: {}", e);
            bail!("Failed reading from daemon process: {}", e)
        }
        Ok(Err(e)) => {
            eprintln!("❌ Failed to start listent daemon");
            eprintln!("   Internal error: {}", e);
            bail!("Internal error waiting for daemon: {}", e)
        }
        Err(_) => {
            // Timeout — child is alive but didn't signal ready
            let _ = child.kill();
            eprintln!("❌ Failed to start listent daemon");
            eprintln!("   Daemon did not become ready within 10 seconds");
            eprintln!("   Check logs: listent daemon logs");
            bail!("Daemon startup timed out")
        }
    }
}

/// Run the actual daemon process (called by child after fork)
async fn run_daemon_process(config_path: Option<PathBuf>) -> Result<()> {
    // Load configuration
    let config = if let Some(ref path) = config_path {
        DaemonConfiguration::load_from_file(path)?
    } else {
        DaemonConfiguration::default()
    };

    // Create daemon state
    let daemon_state = DaemonState::new(config.clone())?;

    // Log startup
    daemon_state.logger.log_startup(
        config_path.as_deref().unwrap_or(&DaemonConfiguration::default_config_path()?),
        std::process::id(),
    )?;

    // Signal parent process that initialization is complete.
    // If launched by launchd (stdout not piped), println is a no-op to a closed fd.
    println!("READY");

    // Setup signal handling for graceful shutdown
    let shutdown_signal = setup_signal_handlers();

    // Main monitoring loop
    let monitoring_task = {
        let process_tracker = daemon_state.process_tracker.clone();
        let config = daemon_state.config.clone();
        let logger = daemon_state.logger.clone();

        tokio::spawn(async move {
            if let Err(e) = run_monitoring_loop(process_tracker, config, logger).await {
                eprintln!("❌ Monitoring loop error: {}", e);
            }
        })
    };

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_signal => {
            daemon_state.logger.log_shutdown("Received shutdown signal")?;
        }
        _ = monitoring_task => {
            daemon_state.logger.log_shutdown("Monitoring loop ended")?;
        }
    }

    Ok(())
}

/// Main monitoring loop that runs continuously
async fn run_monitoring_loop(
    process_tracker: Arc<Mutex<ProcessTracker>>,
    config: Arc<Mutex<DaemonConfiguration>>,
    logger: DaemonLogger,
) -> Result<()> {
    let mut interval = {
        let config = config.lock().await;
        tokio::time::interval(config.polling_duration())
    };

    loop {
        interval.tick().await;

        // Get current processes using polling logic
        let current_config = config.lock().await;
        let polling_config = PollingConfiguration {
            interval: current_config.polling_duration(),
            path_filters: current_config.monitoring.path_filters.clone(),
            entitlement_filters: current_config.monitoring.entitlement_filters.clone(),
            output_json: false, // ULS logging instead
            quiet_mode: false,  // Log all detections
        };
        drop(current_config);

        // Create current snapshot using polling logic
        let current_processes = match scan_current_processes(&polling_config).await {
            Ok(processes) => processes,
            Err(e) => {
                logger.log_error(&format!("Failed to scan processes: {}", e), None)?;
                continue;
            }
        };

        let current_snapshot = ProcessSnapshot {
            processes: current_processes,
            timestamp: std::time::SystemTime::now(),
            scan_duration: std::time::Duration::from_millis(0),
        };

        // Detect new processes (release lock before logging)
        let new_processes = {
            let mut tracker = process_tracker.lock().await;
            tracker.detect_new_processes(current_snapshot)
        };

        // Log any new processes with entitlements (silent operation)
        for process in new_processes {
            if !process.entitlements.is_empty() {
                match crate::output::create_detection_event(&process) {
                    Ok(event) => {
                        if let Err(e) = logger.log_process_detection(&event) {
                            eprintln!("❌ Failed to log process {}: {}", process.name, e);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to create event for process {}: {}", process.name, e);
                    }
                }
            }
        }
    }
}

/// Setup signal handlers for graceful shutdown
async fn setup_signal_handlers() {
    let _ = signal::ctrl_c().await;
}

/// Scan current processes and their entitlements
async fn scan_current_processes(config: &PollingConfiguration) -> Result<std::collections::HashMap<(u32, u64), MonitoredProcess>> {
    use sysinfo::{ProcessesToUpdate, System};

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut processes = std::collections::HashMap::new();

    // Scan all processes
    for (pid, process) in system.processes() {
        let pid_u32 = pid.as_u32();
        let process_name = process.name().to_string_lossy().to_string();

        // Get executable path
        let executable_path = match process.exe() {
            Some(path) => path.to_path_buf(),
            None => continue, // Skip processes without a known executable
        };

        // Apply path filters if specified
        if !config.path_filters.is_empty() {
            let matches_filter = config.path_filters.iter().any(|filter| {
                executable_path.starts_with(filter)
            });
            if !matches_filter {
                continue;
            }
        }

        // Extract entitlements - keep full key-value pairs
        let entitlements = match crate::entitlements::extract_entitlements(&executable_path) {
            Ok(entitlements_map) => entitlements_map,
            Err(_) => std::collections::HashMap::new(),
        };

        // Apply entitlement filters if specified using consistent pattern matching
        let entitlement_keys: Vec<String> = entitlements.keys().cloned().collect();
        if !crate::entitlements::pattern_matcher::entitlements_match_filters(&entitlement_keys, &config.entitlement_filters) {
            continue;
        }

        // Create monitored process
        let start_time = process.start_time();
        let monitored_process = MonitoredProcess {
            pid: pid_u32,
            start_time,
            name: process_name,
            executable_path,
            entitlements,
            discovery_timestamp: std::time::SystemTime::now(),
        };

        processes.insert((pid_u32, start_time), monitored_process);
    }

    Ok(processes)
}