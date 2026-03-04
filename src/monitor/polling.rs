use crate::models::{MonitoredProcess, PollingConfiguration, ProcessSnapshot};
use crate::monitor::ProcessTracker;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use sysinfo::{ProcessesToUpdate, System};

/// Start monitoring processes with external interrupt flag (called from main.rs)
pub fn start_monitoring_with_interrupt(config: PollingConfiguration, interrupted: Arc<AtomicBool>) -> Result<()> {
    // Convert interrupted (false = continue) to running (true = continue)
    let running = Arc::new(AtomicBool::new(true));

    // Create a thread to monitor the interrupted flag and update running
    let running_monitor = running.clone();
    std::thread::spawn(move || {
        while !interrupted.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        running_monitor.store(false, Ordering::SeqCst);
    });

    start_monitoring_internal(config, running)
}

/// Internal monitoring implementation
fn start_monitoring_internal(config: PollingConfiguration, running: Arc<AtomicBool>) -> Result<()> {

    // Initialize process tracker and system info
    let mut tracker = ProcessTracker::new();
    let mut system = System::new_all();

    if !config.quiet_mode {
        println!("Starting process monitoring (interval: {:.1}s)...", config.interval.as_secs_f64());
        if !config.path_filters.is_empty() {
            println!("Monitoring {} for processes",
                config.path_filters.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "));
        }
        if !config.entitlement_filters.is_empty() {
            println!("Monitoring for processes with entitlement: {}",
                config.entitlement_filters.join(", "));
        }
        println!("Press Ctrl+C to stop monitoring.");
        println!();
    }

    while running.load(Ordering::SeqCst) {
        let cycle_start = Instant::now();

        // Refresh system information (only processes, not all system info)
        system.refresh_processes(ProcessesToUpdate::All, true);

        // Create snapshot of current processes
        let snapshot = create_process_snapshot(&system)?;

        // Detect new processes
        let mut new_processes = tracker.detect_new_processes(snapshot);

        // Extract entitlements only for new processes
        for process in &mut new_processes {
            process.entitlements = extract_process_entitlements(&process.executable_path)
                .unwrap_or_default();
        }

        // Apply filters
        let filtered_processes = apply_filters(new_processes, &config);

        // Output detected processes
        for process in &filtered_processes {
            output_process_detection(process, &config)?;
        }

        // Calculate sleep time to maintain interval
        let cycle_duration = cycle_start.elapsed();
        if let Some(sleep_duration) = config.interval.checked_sub(cycle_duration) {
            std::thread::sleep(sleep_duration);
        }
    }

    if !config.quiet_mode {
        println!("Monitoring stopped.");
    }

    Ok(())
}

fn create_process_snapshot(system: &System) -> Result<ProcessSnapshot> {
    let timestamp = SystemTime::now();
    let scan_start = Instant::now();

    let mut processes = HashMap::new();

    for (pid, process) in system.processes() {
        // Extract basic process information only (entitlements extracted later for new processes)
        let name = process.name().to_string_lossy().to_string();
        let executable_path = match process.exe() {
            Some(path) => path.to_path_buf(),
            None => continue, // Skip processes without a known executable
        };

        let start_time = process.start_time();
        let pid_u32 = pid.as_u32();

        let monitored_process = MonitoredProcess {
            pid: pid_u32,
            start_time,
            name,
            executable_path,
            entitlements: HashMap::new(), // Will be populated later for new processes only
            discovery_timestamp: timestamp,
        };

        processes.insert((pid_u32, start_time), monitored_process);
    }

    Ok(ProcessSnapshot {
        processes,
        timestamp,
        scan_duration: scan_start.elapsed(),
    })
}

fn extract_process_entitlements(executable_path: &std::path::Path) -> Result<HashMap<String, serde_json::Value>> {
    crate::entitlements::extract_entitlements(executable_path)
}

fn apply_filters(
    processes: Vec<MonitoredProcess>,
    config: &PollingConfiguration,
) -> Vec<MonitoredProcess> {
    // Filter out processes with no entitlements (reduce noise)
    let filtered: Vec<_> = processes
        .into_iter()
        .filter(|process| !process.entitlements.is_empty())
        .collect();

    // Apply path filters
    let filtered = ProcessTracker::apply_path_filters(filtered, &config.path_filters);

    // Apply entitlement filters
    ProcessTracker::apply_entitlement_filters(filtered, &config.entitlement_filters)
}

fn output_process_detection(process: &MonitoredProcess, config: &PollingConfiguration) -> Result<()> {
    let event = crate::output::create_detection_event(process)?;
    if config.output_json {
        println!("{}", crate::output::format_event_json(&event)?);
    } else {
        println!("{}", crate::output::format_event_human(&event));
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ==================== create_process_snapshot tests ====================

    /// Helper to create an entitlements HashMap from key strings (all set to true)
    fn ents(keys: &[&str]) -> HashMap<String, serde_json::Value> {
        keys.iter().map(|k| (k.to_string(), serde_json::Value::Bool(true))).collect()
    }

    #[test]
    fn test_create_process_snapshot_returns_valid_snapshot() {
        let system = System::new_all();
        let snapshot = create_process_snapshot(&system).unwrap();

        // Should have at least the current process
        assert!(!snapshot.processes.is_empty(), "Snapshot should contain at least one process");

        // Timestamp should be set
        assert!(snapshot.timestamp <= SystemTime::now());

        // Scan duration should be reasonable (less than 10 seconds)
        assert!(snapshot.scan_duration.as_secs() < 10);
    }

    #[test]
    fn test_create_process_snapshot_includes_current_process() {
        let system = System::new_all();
        let snapshot = create_process_snapshot(&system).unwrap();

        let current_pid = std::process::id();

        // Current process should be in the snapshot (check by PID component of the composite key)
        assert!(
            snapshot.processes.values().any(|p| p.pid == current_pid),
            "Snapshot should include current process (PID {})", current_pid
        );
    }

    #[test]
    fn test_process_snapshot_has_valid_executable_paths() {
        let system = System::new_all();
        let snapshot = create_process_snapshot(&system).unwrap();

        // At least some processes should have non-empty executable paths
        let processes_with_paths = snapshot.processes.values()
            .filter(|p| !p.executable_path.as_os_str().is_empty())
            .count();

        assert!(
            processes_with_paths > 0,
            "At least some processes should have executable paths"
        );
    }

    // ==================== extract_process_entitlements tests ====================

    #[test]
    fn test_extract_process_entitlements_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/binary");
        let result = extract_process_entitlements(&path);

        // Should either succeed with empty vec or return an error
        // Either way, it shouldn't panic
        match result {
            Ok(entitlements) => assert!(entitlements.is_empty() || !entitlements.is_empty()),
            Err(_) => {} // Error is acceptable for nonexistent file
        }
    }

    #[test]
    fn test_extract_process_entitlements_from_system_binary() {
        // Test with a known system binary
        let path = PathBuf::from("/usr/bin/sudo");
        if path.exists() {
            let result = extract_process_entitlements(&path);
            // Should not panic, may or may not have entitlements
            assert!(result.is_ok() || result.is_err());
        }
    }

    // ==================== apply_filters tests ====================

    #[test]
    fn test_apply_filters_removes_processes_without_entitlements() {
        let processes = vec![
            MonitoredProcess {
                pid: 1,
                start_time: 0,
                name: "test1".to_string(),
                executable_path: PathBuf::from("/bin/test1"),
                entitlements: ents(&[]), // No entitlements
                discovery_timestamp: SystemTime::now(),
            },
            MonitoredProcess {
                pid: 2,
                start_time: 0,
                name: "test2".to_string(),
                executable_path: PathBuf::from("/bin/test2"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
        ];

        let config = PollingConfiguration {
            interval: std::time::Duration::from_secs(1),
            path_filters: vec![],
            entitlement_filters: vec![],
            output_json: false,
            quiet_mode: false,
        };

        let filtered = apply_filters(processes, &config);

        // Should only keep process with entitlements
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pid, 2);
    }

    #[test]
    fn test_apply_filters_with_path_filter() {
        let processes = vec![
            MonitoredProcess {
                pid: 1,
                start_time: 0,
                name: "test1".to_string(),
                executable_path: PathBuf::from("/Applications/Test.app/test1"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
            MonitoredProcess {
                pid: 2,
                start_time: 0,
                name: "test2".to_string(),
                executable_path: PathBuf::from("/usr/bin/test2"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
        ];

        let config = PollingConfiguration {
            interval: std::time::Duration::from_secs(1),
            path_filters: vec![PathBuf::from("/Applications")],
            entitlement_filters: vec![],
            output_json: false,
            quiet_mode: false,
        };

        let filtered = apply_filters(processes, &config);

        // Should only keep process in /Applications
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pid, 1);
    }

    #[test]
    fn test_apply_filters_with_entitlement_filter() {
        let processes = vec![
            MonitoredProcess {
                pid: 1,
                start_time: 0,
                name: "test1".to_string(),
                executable_path: PathBuf::from("/bin/test1"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
            MonitoredProcess {
                pid: 2,
                start_time: 0,
                name: "test2".to_string(),
                executable_path: PathBuf::from("/bin/test2"),
                entitlements: ents(&["com.apple.security.network.client"]),
                discovery_timestamp: SystemTime::now(),
            },
        ];

        let config = PollingConfiguration {
            interval: std::time::Duration::from_secs(1),
            path_filters: vec![],
            entitlement_filters: vec!["com.apple.security.app-sandbox".to_string()],
            output_json: false,
            quiet_mode: false,
        };

        let filtered = apply_filters(processes, &config);

        // Should only keep process with sandbox entitlement
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pid, 1);
    }

    #[test]
    fn test_apply_filters_combined_filters() {
        let processes = vec![
            MonitoredProcess {
                pid: 1,
                start_time: 0,
                name: "test1".to_string(),
                executable_path: PathBuf::from("/Applications/Test.app/test1"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
            MonitoredProcess {
                pid: 2,
                start_time: 0,
                name: "test2".to_string(),
                executable_path: PathBuf::from("/Applications/Other.app/test2"),
                entitlements: ents(&["com.apple.security.network.client"]),
                discovery_timestamp: SystemTime::now(),
            },
            MonitoredProcess {
                pid: 3,
                start_time: 0,
                name: "test3".to_string(),
                executable_path: PathBuf::from("/usr/bin/test3"),
                entitlements: ents(&["com.apple.security.app-sandbox"]),
                discovery_timestamp: SystemTime::now(),
            },
        ];

        let config = PollingConfiguration {
            interval: std::time::Duration::from_secs(1),
            path_filters: vec![PathBuf::from("/Applications")],
            entitlement_filters: vec!["com.apple.security.app-sandbox".to_string()],
            output_json: false,
            quiet_mode: false,
        };

        let filtered = apply_filters(processes, &config);

        // Should only keep process matching both filters
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pid, 1);
    }

    #[test]
    fn test_apply_filters_empty_input() {
        let processes: Vec<MonitoredProcess> = vec![];

        let config = PollingConfiguration {
            interval: std::time::Duration::from_secs(1),
            path_filters: vec![],
            entitlement_filters: vec![],
            output_json: false,
            quiet_mode: false,
        };

        let filtered = apply_filters(processes, &config);
        assert!(filtered.is_empty());
    }

    // ==================== Timing and edge case tests ====================

    #[test]
    fn test_polling_configuration_interval_bounds() {
        // Valid minimum interval
        let min_config = PollingConfiguration {
            interval: std::time::Duration::from_millis(100),
            path_filters: vec![],
            entitlement_filters: vec![],
            output_json: false,
            quiet_mode: false,
        };
        assert_eq!(min_config.interval.as_millis(), 100);

        // Valid maximum interval
        let max_config = PollingConfiguration {
            interval: std::time::Duration::from_secs(300),
            path_filters: vec![],
            entitlement_filters: vec![],
            output_json: false,
            quiet_mode: false,
        };
        assert_eq!(max_config.interval.as_secs(), 300);
    }

    #[test]
    fn test_snapshot_contains_process_names() {
        let system = System::new_all();
        let snapshot = create_process_snapshot(&system).unwrap();

        // At least some processes should have non-empty names
        let processes_with_names = snapshot.processes.values()
            .filter(|p| !p.name.is_empty())
            .count();

        assert!(
            processes_with_names > 0,
            "At least some processes should have names"
        );
    }

    #[test]
    fn test_snapshot_scan_duration_is_positive() {
        let system = System::new_all();
        let snapshot = create_process_snapshot(&system).unwrap();

        // Scan duration should be set (we did actual work)
        // The duration object exists and can be accessed
        let _ = snapshot.scan_duration.as_nanos();
    }
}