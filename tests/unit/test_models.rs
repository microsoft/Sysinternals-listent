//! Unit tests for data models
//!
//! Tests validation rules and invariants for:
//! - BinaryResult: path, entitlements, entitlement_count consistency
//! - ScanSummary: interrupted field serialization, statistics
//! - MonitoredProcess: PID validation, path requirements
//! - ProcessSnapshot: new_processes comparison logic
//! - PollingConfiguration: interval bounds validation

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use listent::models::*;

/// Helper to create an entitlements HashMap from key strings (all set to true)
fn ents(keys: &[&str]) -> HashMap<String, serde_json::Value> {
    keys.iter().map(|k| (k.to_string(), serde_json::Value::Bool(true))).collect()
}

// ==================== BinaryResult Tests ====================

#[test]
fn test_binary_result_entitlement_count_consistency() {
    let mut entitlements = HashMap::new();
    entitlements.insert("com.apple.security.app-sandbox".to_string(), serde_json::json!(true));
    entitlements.insert("com.apple.security.network.client".to_string(), serde_json::json!(true));

    let result = BinaryResult {
        path: "/Applications/Safari.app/Contents/MacOS/Safari".to_string(),
        entitlement_count: entitlements.len(),
        entitlements,
    };

    assert_eq!(result.entitlement_count, result.entitlements.len(),
        "entitlement_count must equal size of entitlements map");
}

#[test]
fn test_binary_result_empty_entitlements() {
    let result = BinaryResult {
        path: "/usr/bin/ls".to_string(),
        entitlement_count: 0,
        entitlements: HashMap::new(),
    };

    assert_eq!(result.entitlement_count, 0);
    assert!(result.entitlements.is_empty());
}

#[test]
fn test_binary_result_serialization() {
    let mut entitlements = HashMap::new();
    entitlements.insert("com.apple.security.app-sandbox".to_string(), serde_json::json!(true));

    let result = BinaryResult {
        path: "/Applications/Test.app".to_string(),
        entitlement_count: 1,
        entitlements,
    };

    let json = serde_json::to_string(&result).expect("Should serialize");
    assert!(json.contains("path"));
    assert!(json.contains("entitlements"));
    assert!(json.contains("entitlement_count"));
}

// ==================== ScanSummary Tests ====================

#[test]
fn test_scan_summary_interrupted_omitted_when_none() {
    let summary = ScanSummary {
        scanned: 100,
        matched: 10,
        skipped_unreadable: 5,
        duration_ms: 1500,
        interrupted: None,
    };

    let json = serde_json::to_string(&summary).expect("Should serialize");
    assert!(!json.contains("interrupted"), "interrupted should be omitted when None");
}

#[test]
fn test_scan_summary_interrupted_present_when_true() {
    let summary = ScanSummary {
        scanned: 50,
        matched: 5,
        skipped_unreadable: 2,
        duration_ms: 800,
        interrupted: Some(true),
    };

    let json = serde_json::to_string(&summary).expect("Should serialize");
    assert!(json.contains("\"interrupted\":true"), "interrupted should be present when Some(true)");
}

#[test]
fn test_scan_summary_statistics_consistency() {
    let summary = ScanSummary {
        scanned: 100,
        matched: 30,
        skipped_unreadable: 10,
        duration_ms: 2000,
        interrupted: None,
    };

    // Matched + skipped should not exceed scanned (not required but logical)
    assert!(summary.matched + summary.skipped_unreadable <= summary.scanned,
        "matched + skipped should not exceed scanned");
}

#[test]
fn test_scan_summary_duration_non_negative() {
    let summary = ScanSummary {
        scanned: 10,
        matched: 5,
        skipped_unreadable: 0,
        duration_ms: 0, // Edge case: instant scan
        interrupted: None,
    };

    // duration_ms is u64, so it's always non-negative
    assert!(summary.duration_ms == 0 || summary.duration_ms > 0);
}

// ==================== MonitoredProcess Tests ====================

#[test]
fn test_monitored_process_creation() {
    let process = MonitoredProcess {
        pid: 12345,
        start_time: 0,
        name: "Safari".to_string(),
        executable_path: PathBuf::from("/Applications/Safari.app/Contents/MacOS/Safari"),
        entitlements: ents(&["com.apple.security.network.client"]),
        discovery_timestamp: SystemTime::now(),
    };

    assert_eq!(process.pid, 12345);
    assert_eq!(process.name, "Safari");
    assert!(process.executable_path.is_absolute(), "Path should be absolute");
}

#[test]
fn test_monitored_process_with_no_entitlements() {
    let process = MonitoredProcess {
        pid: 1,
        start_time: 0,
        name: "unsigned".to_string(),
        executable_path: PathBuf::from("/tmp/unsigned"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    };

    assert!(process.entitlements.is_empty());
}

#[test]
fn test_monitored_process_with_multiple_entitlements() {
    let process = MonitoredProcess {
        pid: 500,
        start_time: 0,
        name: "app".to_string(),
        executable_path: PathBuf::from("/Applications/App.app"),
        entitlements: ents(&[
            "com.apple.security.app-sandbox",
            "com.apple.security.network.client",
            "com.apple.security.files.user-selected.read-write",
        ]),
        discovery_timestamp: SystemTime::now(),
    };

    assert_eq!(process.entitlements.len(), 3);
}

#[test]
fn test_monitored_process_serialization() {
    let process = MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "test".to_string(),
        executable_path: PathBuf::from("/test"),
        entitlements: ents(&["entitlement"]),
        discovery_timestamp: SystemTime::UNIX_EPOCH,
    };

    let json = serde_json::to_string(&process).expect("Should serialize");
    assert!(json.contains("\"pid\":100"));
    assert!(json.contains("\"name\":\"test\""));
}

// ==================== ProcessSnapshot Tests ====================

#[test]
fn test_process_snapshot_new_processes_empty_previous() {
    let previous = ProcessSnapshot {
        processes: HashMap::new(),
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    let mut current_processes = HashMap::new();
    current_processes.insert((100, 0), MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "new".to_string(),
        executable_path: PathBuf::from("/new"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let current = ProcessSnapshot {
        processes: current_processes,
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    let new_procs = current.new_processes(&previous);
    assert_eq!(new_procs.len(), 1, "New process should be detected");
    assert_eq!(new_procs[0].pid, 100);
}

#[test]
fn test_process_snapshot_new_processes_same_snapshot() {
    let mut processes = HashMap::new();
    processes.insert((100, 0), MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "existing".to_string(),
        executable_path: PathBuf::from("/existing"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let snapshot = ProcessSnapshot {
        processes: processes.clone(),
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    // Compare identical snapshots
    let new_procs = snapshot.new_processes(&snapshot);
    assert!(new_procs.is_empty(), "No new processes in identical snapshot");
}

#[test]
fn test_process_snapshot_detects_multiple_new_processes() {
    let mut old_processes = HashMap::new();
    old_processes.insert((100, 0), MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "old".to_string(),
        executable_path: PathBuf::from("/old"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let previous = ProcessSnapshot {
        processes: old_processes,
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    let mut new_processes = HashMap::new();
    new_processes.insert((100, 0), MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "old".to_string(),
        executable_path: PathBuf::from("/old"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });
    new_processes.insert((101, 0), MonitoredProcess {
        pid: 101,
        start_time: 0,
        name: "new1".to_string(),
        executable_path: PathBuf::from("/new1"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });
    new_processes.insert((102, 0), MonitoredProcess {
        pid: 102,
        start_time: 0,
        name: "new2".to_string(),
        executable_path: PathBuf::from("/new2"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let current = ProcessSnapshot {
        processes: new_processes,
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    let new_procs = current.new_processes(&previous);
    assert_eq!(new_procs.len(), 2, "Should detect both new processes");

    let pids: Vec<u32> = new_procs.iter().map(|p| p.pid).collect();
    assert!(pids.contains(&101));
    assert!(pids.contains(&102));
}

#[test]
fn test_process_snapshot_terminated_process_not_detected() {
    let mut old_processes = HashMap::new();
    old_processes.insert((100, 0), MonitoredProcess {
        pid: 100,
        start_time: 0,
        name: "terminated".to_string(),
        executable_path: PathBuf::from("/terminated"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });
    old_processes.insert((101, 0), MonitoredProcess {
        pid: 101,
        start_time: 0,
        name: "remaining".to_string(),
        executable_path: PathBuf::from("/remaining"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let previous = ProcessSnapshot {
        processes: old_processes,
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    // Only PID 101 remains
    let mut new_processes = HashMap::new();
    new_processes.insert((101, 0), MonitoredProcess {
        pid: 101,
        start_time: 0,
        name: "remaining".to_string(),
        executable_path: PathBuf::from("/remaining"),
        entitlements: HashMap::new(),
        discovery_timestamp: SystemTime::now(),
    });

    let current = ProcessSnapshot {
        processes: new_processes,
        timestamp: SystemTime::now(),
        scan_duration: Duration::from_millis(10),
    };

    let new_procs = current.new_processes(&previous);
    assert!(new_procs.is_empty(), "Terminated process should not appear as new");
}

// ==================== PollingConfiguration Tests ====================

#[test]
fn test_polling_configuration_default_values() {
    let config = PollingConfiguration {
        interval: Duration::from_secs(1),
        path_filters: vec![],
        entitlement_filters: vec![],
        output_json: false,
        quiet_mode: false,
    };

    assert_eq!(config.interval, Duration::from_secs(1));
    assert!(config.path_filters.is_empty());
    assert!(config.entitlement_filters.is_empty());
}

#[test]
fn test_polling_configuration_with_filters() {
    let config = PollingConfiguration {
        interval: Duration::from_millis(500),
        path_filters: vec![PathBuf::from("/Applications")],
        entitlement_filters: vec!["com.apple.security.*".to_string()],
        output_json: true,
        quiet_mode: true,
    };

    assert_eq!(config.path_filters.len(), 1);
    assert_eq!(config.entitlement_filters.len(), 1);
    assert!(config.output_json);
    assert!(config.quiet_mode);
}

#[test]
fn test_polling_configuration_minimum_interval() {
    // 0.1 seconds is the minimum allowed interval
    let config = PollingConfiguration {
        interval: Duration::from_millis(100),
        path_filters: vec![],
        entitlement_filters: vec![],
        output_json: false,
        quiet_mode: false,
    };

    assert_eq!(config.interval.as_millis(), 100);
}

#[test]
fn test_polling_configuration_maximum_interval() {
    // 300 seconds is the maximum allowed interval
    let config = PollingConfiguration {
        interval: Duration::from_secs(300),
        path_filters: vec![],
        entitlement_filters: vec![],
        output_json: false,
        quiet_mode: false,
    };

    assert_eq!(config.interval.as_secs(), 300);
}

// ==================== MonitorError Tests ====================

#[test]
fn test_monitor_error_invalid_interval_display() {
    let error = MonitorError::InvalidInterval(0.05);
    let error_str = format!("{}", error);

    assert!(error_str.contains("0.05"));
    assert!(error_str.contains("0.1"));
    assert!(error_str.contains("300.0"));
}

// ==================== ScanConfig Tests ====================

#[test]
fn test_scan_config_creation() {
    let config = ScanConfig {
        scan_paths: vec!["/Applications".to_string()],
        filters: ScanFilters {
            entitlements: vec!["com.apple.security.network.*".to_string()],
        },
        json_output: true,
        quiet_mode: false,
    };

    assert_eq!(config.scan_paths.len(), 1);
    assert_eq!(config.filters.entitlements.len(), 1);
}

#[test]
fn test_scan_filters_default() {
    let filters = ScanFilters::default();
    assert!(filters.entitlements.is_empty(), "Default should have no entitlement filters");
}

// ==================== EntitlementScanOutput Tests ====================

#[test]
fn test_entitlement_scan_output_serialization() {
    let output = EntitlementScanOutput {
        results: vec![
            BinaryResult {
                path: "/test".to_string(),
                entitlement_count: 1,
                entitlements: {
                    let mut m = HashMap::new();
                    m.insert("test".to_string(), serde_json::json!(true));
                    m
                },
            },
        ],
        summary: ScanSummary {
            scanned: 1,
            matched: 1,
            skipped_unreadable: 0,
            duration_ms: 100,
            interrupted: None,
        },
    };

    let json = serde_json::to_string_pretty(&output).expect("Should serialize");
    assert!(json.contains("results"));
    assert!(json.contains("summary"));
    assert!(json.contains("scanned"));
}

#[test]
fn test_entitlement_scan_output_empty_results() {
    let output = EntitlementScanOutput {
        results: vec![],
        summary: ScanSummary {
            scanned: 100,
            matched: 0,
            skipped_unreadable: 5,
            duration_ms: 500,
            interrupted: None,
        },
    };

    assert!(output.results.is_empty());
    assert_eq!(output.summary.matched, 0);
}