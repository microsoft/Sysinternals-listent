//! Global constants for listent
//!
//! Centralized location for application-wide constants

/// Application subsystem identifier for macOS Unified Logging System
/// Used for LaunchD service name, ULS logging, and daemon identification
pub const APP_SUBSYSTEM: &str = "com.microsoft.sysinternals.listent";

/// Default daemon category for ULS logging
pub const DAEMON_CATEGORY: &str = "daemon";

/// Default directories to scan when no paths are provided
pub const DEFAULT_SCAN_PATHS: &[&str] = &["/usr/bin", "/usr/sbin"];

/// LaunchD plist file name
pub const LAUNCHD_PLIST_NAME: &str = "com.microsoft.sysinternals.listent.plist";

/// LaunchD service name (same as subsystem)
pub const LAUNCHD_SERVICE_NAME: &str = APP_SUBSYSTEM;

/// LaunchDaemons directory for system-wide services
pub const LAUNCHD_DAEMONS_DIR: &str = "/Library/LaunchDaemons";

// --- Polling interval configuration ---

/// Minimum allowed polling interval in seconds
pub const POLLING_INTERVAL_MIN: f64 = 0.1;

/// Maximum allowed polling interval in seconds
pub const POLLING_INTERVAL_MAX: f64 = 300.0;

/// Default polling interval in seconds
pub const DEFAULT_POLLING_INTERVAL: f64 = 1.0;

/// Default polling interval as a string (for clap default_value)
pub const DEFAULT_POLLING_INTERVAL_STR: &str = "1.0";

// --- Codesign command ---

/// macOS codesign command name
pub const CODESIGN_COMMAND: &str = "codesign";

/// Arguments for extracting entitlements via codesign
pub const CODESIGN_ENTITLEMENT_ARGS: &[&str] = &["-d", "--entitlements", "-", "--xml"];

// --- Daemon subcommand identifiers ---

/// CLI subcommand name for daemon mode
pub const DAEMON_SUBCOMMAND: &str = "daemon";

/// CLI subcommand name for daemon run mode
pub const DAEMON_RUN_SUBCOMMAND: &str = "run";

// --- Logging ---

/// Separator between human message and structured JSON in ULS log lines
pub const LOG_JSON_SEPARATOR: &str = " | ";

/// Event type for process detection events
pub const EVENT_PROCESS_DETECTED: &str = "process_detected";

/// macOS log command name
pub const LOG_COMMAND: &str = "log";

/// Log style used for ULS queries
pub const LOG_STYLE: &str = "compact";

// --- Error detection ---

/// OS error string for permission denied
pub const OS_ERROR_PERMISSION: &str = "os error 13";

/// Permission denied error string
pub const PERMISSION_DENIED: &str = "Permission denied";

// --- Daemon filesystem paths ---

/// Working directory for the daemon process
pub const DAEMON_WORKING_DIR: &str = "/var/run/listent";

/// Log file path for daemon stdout/stderr
pub const DAEMON_LOG_FILE: &str = "/var/log/listent/daemon.log";

/// System PATH for the daemon environment
pub const DAEMON_SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";