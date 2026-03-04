//! CLI argument parsing and validation module
//!
//! Handles command-line interface using clap, including:
//! - Scan mode (default): scan files/directories for entitlements
//! - Monitor subcommand: real-time process monitoring
//! - Daemon subcommand: background daemon operations

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Result, anyhow, Context};
use crate::constants::{DEFAULT_SCAN_PATHS, DEFAULT_POLLING_INTERVAL_STR, POLLING_INTERVAL_MIN, POLLING_INTERVAL_MAX};
use crate::models::{ScanConfig, ScanFilters, PollingConfiguration, MonitorError};
use std::time::Duration;

/// Command line arguments for listent
#[derive(Parser, Debug)]
#[command(author, version = env!("LISTENT_VERSION"), about = "Sysinternals tool to discover and list code signing entitlements for macOS binaries.\nBy default, scans /usr/bin and /usr/sbin. Use subcommands for monitoring or daemon mode.", disable_help_subcommand = true)]
#[command(after_help = "Examples:
  listent                                      Scan default paths (/usr/bin, /usr/sbin)
  listent -e \"*network*\"                       Scan with entitlement filter
  listent monitor                              Monitor all new processes
  listent monitor -e \"com.apple.security.*\"    Monitor with entitlement filter
  listent daemon install                       Install as background service")]
pub struct Args {
    /// Subcommands (monitor, daemon)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Directory or file paths to scan (default: /usr/bin, /usr/sbin)
    #[arg(value_name = "PATH")]
    pub path: Vec<PathBuf>,

    /// Filter by entitlement key (exact or glob pattern)
    #[arg(short, long, value_name = "PATTERN", value_delimiter = ',')]
    pub entitlement: Vec<String>,

    /// Output in JSON format
    #[arg(short, long)]
    pub json: bool,

    /// Suppress warnings about unreadable files
    #[arg(short, long)]
    pub quiet: bool,
}

/// Top-level subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Real-time process monitoring for entitlements
    #[command(about = "Monitor new processes for entitlements in real-time")]
    Monitor {
        /// Directory or file paths to filter monitored processes
        #[arg(value_name = "PATH")]
        path: Vec<PathBuf>,

        /// Filter by entitlement key (exact match or glob pattern)
        #[arg(short, long, value_name = "KEY", value_delimiter = ',')]
        entitlement: Vec<String>,

        /// Polling interval in seconds (0.1 - 300.0)
        #[arg(short, long, default_value = DEFAULT_POLLING_INTERVAL_STR, value_name = "SECONDS")]
        interval: f64,

        /// Output in JSON format
        #[arg(short, long)]
        json: bool,

        /// Suppress warnings
        #[arg(short, long)]
        quiet: bool,
    },

    /// Daemon management commands
    #[command(about = "Background daemon operations")]
    #[command(after_help = "Examples:
  listent daemon install                 Install and start daemon service
  listent daemon install --config FILE   Install with custom config
  listent daemon status                  Check if daemon is running
  listent daemon logs                    View logs from last hour
  listent daemon logs --since 24h        View logs from last 24 hours
  listent daemon logs -f                 Follow logs in real-time
  listent daemon stop                    Stop daemon process
  listent daemon uninstall               Remove daemon service")]
    Daemon {
        #[command(subcommand)]
        action: DaemonCommands,
    },
}

/// Daemon subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum DaemonCommands {
    /// Run daemon in foreground (for testing or manual operation)
    #[command(about = "Run daemon in foreground")]
    Run {
        /// Path to configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },

    /// Install daemon as LaunchD service
    #[command(about = "Install daemon as LaunchD service (requires sudo)")]
    Install {
        /// Path to configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,
    },

    /// Uninstall daemon service from LaunchD
    #[command(about = "Uninstall daemon service (requires sudo)")]
    Uninstall,

    /// Check daemon service status
    #[command(about = "Show daemon service status")]
    Status,

    /// Stop running daemon process
    #[command(about = "Stop running daemon process")]
    Stop,

    /// View daemon logs
    #[command(about = "View daemon logs from macOS Unified Logging System")]
    Logs {
        /// Follow log output continuously
        #[arg(short, long)]
        follow: bool,

        /// Show logs since specific time (e.g., "1h", "30m", "2023-01-01 10:00")
        #[arg(long, value_name = "TIME")]
        since: Option<String>,

        /// Output format (json, human)
        #[arg(long, default_value = "human")]
        format: String,
    },
}

/// Parse command line arguments and return scan configuration
pub fn parse_args_from(args: Args) -> Result<ScanConfig> {
    // This function is only for scan mode (no subcommand)
    if args.command.is_some() {
        return Err(anyhow!("Internal error: parse_args called with subcommand"));
    }

    // Validate paths if provided
    let mut scan_paths = Vec::new();
    if !args.path.is_empty() {
        for path in &args.path {
            if !path.exists() {
                return Err(anyhow!("Path does not exist: {}", path.display()));
            }
            scan_paths.push(path.display().to_string());
        }
    } else {
        // Use default paths
        scan_paths.extend(DEFAULT_SCAN_PATHS.iter().map(|s| s.to_string()));
    }

    // Validate entitlement filters if provided
    if !args.entitlement.is_empty() {
        crate::entitlements::pattern_matcher::validate_entitlement_filters(&args.entitlement)
            .context("Invalid entitlement filter")?;
    }

    let filters = ScanFilters {
        entitlements: args.entitlement,
    };

    Ok(ScanConfig {
        scan_paths,
        filters,
        json_output: args.json,
        quiet_mode: args.quiet,
    })
}

/// Parse command line arguments and return monitor configuration
pub fn parse_monitor_config(
    path: Vec<PathBuf>,
    entitlement: Vec<String>,
    interval: f64,
    json: bool,
    quiet: bool,
) -> Result<PollingConfiguration> {
    // Validate interval range
    if interval < POLLING_INTERVAL_MIN || interval > POLLING_INTERVAL_MAX {
        return Err(MonitorError::InvalidInterval(interval).into());
    }

    // Validate entitlement filters if provided
    if !entitlement.is_empty() {
        crate::entitlements::pattern_matcher::validate_entitlement_filters(&entitlement)
            .context("Invalid entitlement filter")?;
    }

    // Validate paths if provided
    let mut path_filters = Vec::new();
    for p in &path {
        if !p.exists() {
            return Err(anyhow!("Path does not exist: {}", p.display()));
        }
        path_filters.push(p.clone());
    }

    Ok(PollingConfiguration {
        interval: Duration::from_secs_f64(interval),
        path_filters,
        entitlement_filters: entitlement,
        output_json: json,
        quiet_mode: quiet,
    })
}

/// Get execution mode based on CLI arguments
pub fn get_execution_mode() -> Result<ExecutionMode> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Monitor { path, entitlement, interval, json, quiet }) => {
            Ok(ExecutionMode::Monitor { path, entitlement, interval, json, quiet })
        }
        Some(Commands::Daemon { action }) => {
            Ok(ExecutionMode::Daemon(action))
        }
        None => {
            // Default: scan mode — pass parsed args to avoid re-parsing
            Ok(ExecutionMode::Scan(args))
        }
    }
}

/// Execution modes for the application
#[derive(Debug)]
pub enum ExecutionMode {
    /// Scan mode with pre-parsed CLI args
    Scan(Args),
    Monitor {
        path: Vec<PathBuf>,
        entitlement: Vec<String>,
        interval: f64,
        json: bool,
        quiet: bool,
    },
    Daemon(DaemonCommands),
}

/// Validate time format for log filtering
pub fn validate_time_format(time_str: &str) -> Result<()> {
    // Simple validation for common time formats
    if time_str.ends_with('h') || time_str.ends_with('m') || time_str.ends_with('s') {
        let number_part = &time_str[..time_str.len()-1];
        number_part.parse::<u32>()
            .map_err(|_| anyhow!("Invalid time format: {}", time_str))?;
        Ok(())
    } else if time_str.contains('-') && time_str.contains(':') {
        // Basic datetime format validation (could be more robust)
        Ok(())
    } else {
        Err(anyhow!("Invalid time format: {}. Use formats like '1h', '30m', or '2023-01-01 10:00'", time_str))
    }
}
