use crate::models::{MonitoredProcess, ProcessSnapshot};

/// Manages process state tracking between polling cycles
pub struct ProcessTracker {
    current_snapshot: Option<ProcessSnapshot>,
}

impl ProcessTracker {
    pub fn new() -> Self {
        Self {
            current_snapshot: None,
        }
    }

    /// Detect new processes by comparing current snapshot with previous
    pub fn detect_new_processes(
        &mut self,
        new_snapshot: ProcessSnapshot,
    ) -> Vec<MonitoredProcess> {
        let new_processes = match &self.current_snapshot {
            None => {
                // First snapshot - all processes are "new" but we ignore them
                // to avoid flooding output on startup
                Vec::new()
            }
            Some(previous) => new_snapshot.new_processes(previous),
        };

        self.current_snapshot = Some(new_snapshot);
        new_processes
    }

    /// Apply path filters to processes (reusing existing scan logic)
    pub fn apply_path_filters(
        processes: Vec<MonitoredProcess>,
        path_filters: &[std::path::PathBuf],
    ) -> Vec<MonitoredProcess> {
        if path_filters.is_empty() {
            return processes;
        }

        processes
            .into_iter()
            .filter(|process| {
                path_filters.iter().any(|filter_path| {
                    process.executable_path.starts_with(filter_path)
                })
            })
            .collect()
    }

    /// Apply entitlement filters to processes using consistent pattern matching
    pub fn apply_entitlement_filters(
        processes: Vec<MonitoredProcess>,
        entitlement_filters: &[String],
    ) -> Vec<MonitoredProcess> {
        use crate::entitlements::pattern_matcher;

        processes
            .into_iter()
            .filter(|process| {
                let keys: Vec<String> = process.entitlements.keys().cloned().collect();
                pattern_matcher::entitlements_match_filters(&keys, entitlement_filters)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// Helper function to create a test MonitoredProcess
    fn create_test_process(pid: u32, name: &str, path: &str, entitlements: Vec<&str>) -> MonitoredProcess {
        MonitoredProcess {
            pid,
            start_time: pid as u64 * 1000, // Deterministic start_time derived from PID for testing
            name: name.to_string(),
            executable_path: PathBuf::from(path),
            entitlements: entitlements.into_iter()
                .map(|e| (e.to_string(), serde_json::Value::Bool(true)))
                .collect(),
            discovery_timestamp: SystemTime::now(),
        }
    }

    /// Helper function to create a ProcessSnapshot from a list of processes
    fn create_snapshot(processes: Vec<MonitoredProcess>) -> ProcessSnapshot {
        let mut map = HashMap::new();
        for p in processes {
            map.insert((p.pid, p.start_time), p);
        }
        ProcessSnapshot {
            processes: map,
            timestamp: SystemTime::now(),
            scan_duration: Duration::from_millis(10),
        }
    }

    // ==================== ProcessTracker::new() tests ====================

    #[test]
    fn test_new_tracker_has_no_snapshot() {
        let tracker = ProcessTracker::new();
        assert!(tracker.current_snapshot.is_none());
    }

    // ==================== detect_new_processes() tests ====================

    #[test]
    fn test_first_snapshot_returns_no_new_processes() {
        let mut tracker = ProcessTracker::new();

        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app/Contents/MacOS/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Finder", "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder", vec!["com.apple.security.files.all"]),
        ];
        let snapshot = create_snapshot(processes);

        // First snapshot should return empty (no "new" processes to avoid flooding on startup)
        let new_procs = tracker.detect_new_processes(snapshot);
        assert!(new_procs.is_empty(), "First snapshot should not report any new processes");
    }

    #[test]
    fn test_new_process_detected_in_second_snapshot() {
        let mut tracker = ProcessTracker::new();

        // First snapshot with one process
        let initial_processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app/Contents/MacOS/Safari", vec!["com.apple.security.network.client"]),
        ];
        tracker.detect_new_processes(create_snapshot(initial_processes));

        // Second snapshot with an additional process
        let updated_processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app/Contents/MacOS/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(102, "TextEdit", "/Applications/TextEdit.app/Contents/MacOS/TextEdit", vec!["com.apple.security.app-sandbox"]),
        ];
        let new_procs = tracker.detect_new_processes(create_snapshot(updated_processes));

        assert_eq!(new_procs.len(), 1, "Should detect exactly one new process");
        assert_eq!(new_procs[0].pid, 102);
        assert_eq!(new_procs[0].name, "TextEdit");
    }

    #[test]
    fn test_multiple_new_processes_detected() {
        let mut tracker = ProcessTracker::new();

        // First snapshot - empty
        tracker.detect_new_processes(create_snapshot(vec![]));

        // Second snapshot with three new processes
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec![]),
            create_test_process(101, "Finder", "/System/Finder.app", vec![]),
            create_test_process(102, "TextEdit", "/Applications/TextEdit.app", vec![]),
        ];
        let new_procs = tracker.detect_new_processes(create_snapshot(processes));

        assert_eq!(new_procs.len(), 3, "Should detect all three new processes");
    }

    #[test]
    fn test_terminated_process_not_reported_as_new() {
        let mut tracker = ProcessTracker::new();

        // First snapshot with two processes
        let initial = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec![]),
            create_test_process(101, "Finder", "/System/Finder.app", vec![]),
        ];
        tracker.detect_new_processes(create_snapshot(initial));

        // Second snapshot - Safari terminated, only Finder remains
        let updated = vec![
            create_test_process(101, "Finder", "/System/Finder.app", vec![]),
        ];
        let new_procs = tracker.detect_new_processes(create_snapshot(updated));

        assert!(new_procs.is_empty(), "No new processes should be detected when one terminates");
    }

    #[test]
    fn test_pid_reuse_detected_as_new_process() {
        let mut tracker = ProcessTracker::new();

        // First snapshot with PID 100 running Safari (started at time 1000)
        let mut safari = create_test_process(100, "Safari", "/Applications/Safari.app", vec!["com.apple.security.network.client"]);
        safari.start_time = 1000;
        tracker.detect_new_processes(create_snapshot(vec![safari]));

        // PID 100 reused by Chrome with a different start_time (started at time 2000).
        // Even though the PID is the same, the (pid, start_time) composite key
        // ensures this is detected as a new process.
        let mut chrome = create_test_process(100, "Chrome", "/Applications/Google Chrome.app", vec!["com.apple.security.network.server"]);
        chrome.start_time = 2000;
        let new_procs = tracker.detect_new_processes(create_snapshot(vec![chrome]));

        assert_eq!(new_procs.len(), 1, "Reused PID with different start_time should be detected as new process");
        assert_eq!(new_procs[0].name, "Chrome");
    }

    #[test]
    fn test_snapshot_state_is_updated() {
        let mut tracker = ProcessTracker::new();

        let snapshot1 = create_snapshot(vec![create_test_process(100, "Safari", "/Apps/Safari", vec![])]);
        tracker.detect_new_processes(snapshot1);

        assert!(tracker.current_snapshot.is_some(), "Tracker should store the snapshot");
        assert_eq!(tracker.current_snapshot.as_ref().unwrap().processes.len(), 1);

        let snapshot2 = create_snapshot(vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec![]),
            create_test_process(101, "Finder", "/Apps/Finder", vec![]),
        ]);
        tracker.detect_new_processes(snapshot2);

        assert_eq!(tracker.current_snapshot.as_ref().unwrap().processes.len(), 2);
    }

    // ==================== apply_path_filters() tests ====================

    #[test]
    fn test_empty_path_filter_returns_all_processes() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec![]),
            create_test_process(101, "ls", "/usr/bin/ls", vec![]),
            create_test_process(102, "Finder", "/System/Library/CoreServices/Finder.app", vec![]),
        ];

        let filtered = ProcessTracker::apply_path_filters(processes.clone(), &[]);
        assert_eq!(filtered.len(), 3, "Empty filter should return all processes");
    }

    #[test]
    fn test_path_filter_single_directory() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app/Contents/MacOS/Safari", vec![]),
            create_test_process(101, "ls", "/usr/bin/ls", vec![]),
            create_test_process(102, "TextEdit", "/Applications/TextEdit.app/Contents/MacOS/TextEdit", vec![]),
        ];

        let filters = vec![PathBuf::from("/Applications")];
        let filtered = ProcessTracker::apply_path_filters(processes, &filters);

        assert_eq!(filtered.len(), 2, "Should only include /Applications processes");
        assert!(filtered.iter().all(|p| p.executable_path.starts_with("/Applications")));
    }

    #[test]
    fn test_path_filter_multiple_directories() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec![]),
            create_test_process(101, "ls", "/usr/bin/ls", vec![]),
            create_test_process(102, "Finder", "/System/Library/CoreServices/Finder.app", vec![]),
            create_test_process(103, "custom", "/opt/local/bin/custom", vec![]),
        ];

        let filters = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/usr/bin"),
        ];
        let filtered = ProcessTracker::apply_path_filters(processes, &filters);

        assert_eq!(filtered.len(), 2, "Should match /Applications and /usr/bin");
        let pids: Vec<u32> = filtered.iter().map(|p| p.pid).collect();
        assert!(pids.contains(&100));
        assert!(pids.contains(&101));
    }

    #[test]
    fn test_path_filter_no_matches() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec![]),
            create_test_process(101, "ls", "/usr/bin/ls", vec![]),
        ];

        let filters = vec![PathBuf::from("/nonexistent/path")];
        let filtered = ProcessTracker::apply_path_filters(processes, &filters);

        assert!(filtered.is_empty(), "No processes should match nonexistent path");
    }

    #[test]
    fn test_path_filter_nested_paths() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app/Contents/MacOS/Safari", vec![]),
            create_test_process(101, "Plugin", "/Applications/Safari.app/Contents/PlugIns/plugin", vec![]),
        ];

        // Filter for the deeper path
        let filters = vec![PathBuf::from("/Applications/Safari.app/Contents/MacOS")];
        let filtered = ProcessTracker::apply_path_filters(processes, &filters);

        assert_eq!(filtered.len(), 1, "Should only match exact nested path");
        assert_eq!(filtered[0].pid, 100);
    }

    // ==================== apply_entitlement_filters() tests ====================

    #[test]
    fn test_empty_entitlement_filter_returns_processes_with_entitlements() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "NoEnt", "/Apps/NoEnt", vec![]),
            create_test_process(102, "TextEdit", "/Apps/TextEdit", vec!["com.apple.security.app-sandbox"]),
        ];

        // Empty filter means "match processes that have ANY entitlements"
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &[]);

        assert_eq!(filtered.len(), 2, "Should return only processes with entitlements");
        let pids: Vec<u32> = filtered.iter().map(|p| p.pid).collect();
        assert!(pids.contains(&100));
        assert!(pids.contains(&102));
    }

    #[test]
    fn test_exact_entitlement_filter_match() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Finder", "/Apps/Finder", vec!["com.apple.security.files.all"]),
            create_test_process(102, "TextEdit", "/Apps/TextEdit", vec!["com.apple.security.app-sandbox"]),
        ];

        let filters = vec!["com.apple.security.network.client".to_string()];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 1, "Should match exactly one process");
        assert_eq!(filtered[0].pid, 100);
    }

    #[test]
    fn test_glob_entitlement_filter_match() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Server", "/Apps/Server", vec!["com.apple.security.network.server"]),
            create_test_process(102, "TextEdit", "/Apps/TextEdit", vec!["com.apple.security.app-sandbox"]),
        ];

        let filters = vec!["com.apple.security.network.*".to_string()];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 2, "Glob should match both network entitlements");
        let pids: Vec<u32> = filtered.iter().map(|p| p.pid).collect();
        assert!(pids.contains(&100));
        assert!(pids.contains(&101));
    }

    #[test]
    fn test_multiple_entitlement_filters_or_logic() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Camera", "/Apps/Camera", vec!["com.apple.security.device.camera"]),
            create_test_process(102, "TextEdit", "/Apps/TextEdit", vec!["com.apple.security.app-sandbox"]),
        ];

        // Multiple filters use OR logic
        let filters = vec![
            "com.apple.security.network.client".to_string(),
            "com.apple.security.device.camera".to_string(),
        ];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 2, "Should match processes with either entitlement");
        let pids: Vec<u32> = filtered.iter().map(|p| p.pid).collect();
        assert!(pids.contains(&100));
        assert!(pids.contains(&101));
    }

    #[test]
    fn test_process_with_multiple_entitlements_matches_single_filter() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec![
                "com.apple.security.network.client",
                "com.apple.security.app-sandbox",
                "com.apple.security.files.user-selected.read-write",
            ]),
        ];

        let filters = vec!["com.apple.security.app-sandbox".to_string()];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 1, "Process with multiple entitlements should match");
    }

    #[test]
    fn test_entitlement_filter_no_matches() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Finder", "/Apps/Finder", vec!["com.apple.security.files.all"]),
        ];

        let filters = vec!["com.apple.security.device.camera".to_string()];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert!(filtered.is_empty(), "No processes should match non-existent entitlement");
    }

    #[test]
    fn test_entitlement_filter_process_with_empty_entitlements() {
        let processes = vec![
            create_test_process(100, "NoSign", "/Apps/NoSign", vec![]), // No entitlements
            create_test_process(101, "Signed", "/Apps/Signed", vec!["com.apple.security.app-sandbox"]),
        ];

        let filters = vec!["com.apple.security.app-sandbox".to_string()];
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 1, "Process with empty entitlements should not match");
        assert_eq!(filtered[0].pid, 101);
    }

    #[test]
    fn test_wildcard_any_entitlement() {
        let processes = vec![
            create_test_process(100, "Safari", "/Apps/Safari", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Custom", "/Apps/Custom", vec!["com.example.custom"]),
        ];

        let filters = vec!["*".to_string()]; // Match any entitlement
        let filtered = ProcessTracker::apply_entitlement_filters(processes, &filters);

        assert_eq!(filtered.len(), 2, "Wildcard * should match all entitlements");
    }

    // ==================== Combined filter tests ====================

    #[test]
    fn test_combined_path_and_entitlement_filters() {
        let processes = vec![
            create_test_process(100, "Safari", "/Applications/Safari.app", vec!["com.apple.security.network.client"]),
            create_test_process(101, "Calculator", "/Applications/Calculator.app", vec!["com.apple.security.app-sandbox"]),
            create_test_process(102, "ls", "/usr/bin/ls", vec!["com.apple.security.network.client"]),
        ];

        // Apply path filter first
        let path_filters = vec![PathBuf::from("/Applications")];
        let after_path = ProcessTracker::apply_path_filters(processes, &path_filters);
        assert_eq!(after_path.len(), 2); // Safari and Calculator

        // Then apply entitlement filter
        let ent_filters = vec!["com.apple.security.network.client".to_string()];
        let final_result = ProcessTracker::apply_entitlement_filters(after_path, &ent_filters);

        assert_eq!(final_result.len(), 1, "Combined filters should narrow results");
        assert_eq!(final_result[0].pid, 100);
        assert_eq!(final_result[0].name, "Safari");
    }
}