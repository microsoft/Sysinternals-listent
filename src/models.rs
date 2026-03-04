//! Data models module
//!
//! Defines core data structures:
//! - BinaryRecord: Discovered executable metadata
//! - EntitlementSet: Parsed entitlement key-value pairs
//! - ScanResult: Successful entitlement enumeration
//! - ScanSummary: Aggregated scan statistics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Represents a single binary file with its entitlements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryResult {
    /// Absolute path to the binary file
    pub path: String,
    /// Entitlements found in the binary (key-value pairs)
    pub entitlements: HashMap<String, serde_json::Value>,
    /// Count of entitlements for quick reference
    pub entitlement_count: usize,
}

/// Summary statistics for the scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    /// Total number of files scanned
    pub scanned: usize,
    /// Number of files that matched filters and had entitlements
    pub matched: usize,
    /// Number of files that couldn't be read due to permissions
    pub skipped_unreadable: usize,
    /// Duration of the scan in milliseconds
    pub duration_ms: u64,
    /// Whether the scan was interrupted by user signal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupted: Option<bool>,
}

/// Complete output structure for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitlementScanOutput {
    /// Array of binary results
    pub results: Vec<BinaryResult>,
    /// Summary statistics
    pub summary: ScanSummary,
}

/// Filter criteria for scanning operations
#[derive(Debug, Clone, Default)]
pub struct ScanFilters {
    /// Filter by specific entitlement keys
    pub entitlements: Vec<String>,
}

/// Configuration for the scan operation
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Base directories to scan (defaults to system app directories)
    pub scan_paths: Vec<String>,
    /// Filter criteria
    pub filters: ScanFilters,
    /// Whether to output JSON format
    pub json_output: bool,
    /// Whether to run in quiet mode (suppress warnings)
    pub quiet_mode: bool,
}

//
// Monitor-specific data structures (T012-T015)
//

/// Represents a monitored process and its entitlements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredProcess {
    /// Process ID (PID)
    pub pid: u32,
    /// Process start time as Unix timestamp (seconds since epoch).
    /// Combined with PID, this uniquely identifies a process even across PID reuse.
    pub start_time: u64,
    /// Process name (executable name)
    pub name: String,
    /// Full path to the executable
    pub executable_path: PathBuf,
    /// Entitlements found in the process executable (key-value pairs)
    pub entitlements: HashMap<String, serde_json::Value>,
    /// Timestamp when this process was first discovered
    pub discovery_timestamp: SystemTime,
}

/// Configuration for polling behavior in monitor mode
#[derive(Debug, Clone)]
pub struct PollingConfiguration {
    /// Polling interval
    pub interval: Duration,
    /// Path filters for process monitoring
    pub path_filters: Vec<PathBuf>,
    /// Entitlement filters for process monitoring
    pub entitlement_filters: Vec<String>,
    /// Whether to output JSON format
    pub output_json: bool,
    /// Whether to run in quiet mode
    pub quiet_mode: bool,
}

/// Snapshot of process state at a given moment
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    /// HashMap of (PID, start_time) -> MonitoredProcess for O(1) lookups.
    /// Using (PID, start_time) as key ensures PID reuse is detected as a new process.
    pub processes: HashMap<(u32, u64), MonitoredProcess>,
    /// Timestamp of this snapshot
    #[allow(dead_code)]
    pub timestamp: SystemTime,
    /// Duration taken to create this snapshot
    #[allow(dead_code)]
    pub scan_duration: Duration,
}

/// Custom error types for monitoring operations
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// Note: bounds must match POLLING_INTERVAL_MIN/MAX in constants.rs
    #[error("Invalid polling interval: {0}. Must be between 0.1 and 300.0 seconds")]
    InvalidInterval(f64),
}

/// Canonical event structure for process detection output.
/// Used by monitor stdout, daemon ULS logging, and daemon log viewer
/// to ensure consistent JSON schema and human-readable formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDetectionEvent {
    /// ISO 8601 timestamp of when the process was detected
    pub timestamp: String,
    /// Event type identifier
    pub event_type: String,
    /// Process ID
    pub pid: u32,
    /// Process name (executable name)
    pub name: String,
    /// Full path to the executable
    pub path: String,
    /// Entitlement count for quick reference
    pub entitlement_count: usize,
    /// Entitlements as a list of key names
    pub entitlements: Vec<String>,
}

impl ProcessSnapshot {
    /// Returns processes that are in this snapshot but not in the previous one.
    /// Comparison uses (PID, start_time) keys to handle PID reuse correctly.
    pub fn new_processes(&self, previous: &ProcessSnapshot) -> Vec<MonitoredProcess> {
        self.processes
            .iter()
            .filter(|(key, _)| !previous.processes.contains_key(key))
            .map(|(_, process)| process.clone())
            .collect()
    }
}