//! Output formatting module
//!
//! Handles:
//! - Human-readable output formatting per contracts/output-human-format.md
//! - JSON output conforming to contracts/output-json-schema.json
//! - Summary statistics generation
//! - Quiet/verbose mode behavior
//! - Progress indicators for long-running operations

use anyhow::Result;
use crate::constants::EVENT_PROCESS_DETECTED;
use crate::models::{EntitlementScanOutput, MonitoredProcess, ProcessDetectionEvent};

pub mod progress;

/// Create a ProcessDetectionEvent from a MonitoredProcess.
/// This is the canonical way to build an event for output — ensures
/// consistent field names and structure across all code paths.
pub fn create_detection_event(process: &MonitoredProcess) -> Result<ProcessDetectionEvent> {
    use time::OffsetDateTime;

    let timestamp = OffsetDateTime::from(process.discovery_timestamp);
    let timestamp_str = timestamp.format(&time::format_description::well_known::Iso8601::DEFAULT)?;

    let mut entitlement_keys: Vec<String> = process.entitlements.keys().cloned().collect();
    entitlement_keys.sort();

    Ok(ProcessDetectionEvent {
        timestamp: timestamp_str,
        event_type: EVENT_PROCESS_DETECTED.to_string(),
        pid: process.pid,
        name: process.name.clone(),
        path: process.executable_path.display().to_string(),
        entitlement_count: entitlement_keys.len(),
        entitlements: entitlement_keys,
    })
}

/// Format a process detection event as human-readable text.
/// Used by both monitor stdout and daemon log viewer for consistent output.
pub fn format_event_human(event: &ProcessDetectionEvent) -> String {
    let ent_list = if event.entitlements.is_empty() {
        "(none)".to_string()
    } else {
        event.entitlements.join(", ")
    };

    format!(
        "[{}] New process detected: {} (PID: {})\n  Path: {}\n  Entitlements: {}",
        event.timestamp, event.name, event.pid, event.path, ent_list
    )
}

/// Format a process detection event as JSON string.
pub fn format_event_json(event: &ProcessDetectionEvent) -> Result<String> {
    Ok(serde_json::to_string(event)?)
}

/// Format output in human-readable format
pub fn format_human(output: &EntitlementScanOutput) -> Result<()> {
    if output.results.is_empty() {
        println!("No binaries found with entitlements.");
    } else {
        // Group results by entitlement types for better readability
        let total_entitlements: usize = output.results.iter()
            .map(|r| r.entitlement_count)
            .sum();

        println!("Found {} binaries with {} total entitlements:\n",
                output.results.len(), total_entitlements);

        for result in &output.results {
            println!("{}:", result.path);

            // Sort entitlements for consistent output
            let mut sorted_entitlements: Vec<_> = result.entitlements.iter().collect();
            sorted_entitlements.sort_by_key(|(k, _)| *k);

            for (key, value) in sorted_entitlements {
                match value {
                    serde_json::Value::Bool(b) => println!("  {}: {}", key, b),
                    serde_json::Value::String(s) => println!("  {}: {}", key, s),
                    serde_json::Value::Number(n) => println!("  {}: {}", key, n),
                    _ => println!("  {}: {}", key, value),
                }
            }
            println!();
        }
    }

    // Print summary
    let summary = &output.summary;
    println!("Scan Summary:");
    println!("  Scanned: {} files", summary.scanned);
    println!("  Matched: {} files", summary.matched);

    if summary.skipped_unreadable > 0 {
        println!("  Skipped (unreadable): {} files", summary.skipped_unreadable);
    }

    // Format duration nicely
    let duration_sec = summary.duration_ms as f64 / 1000.0;
    if duration_sec < 1.0 {
        println!("  Duration: {}ms", summary.duration_ms);
    } else {
        println!("  Duration: {:.2}s", duration_sec);
    }

    if let Some(true) = summary.interrupted {
        println!("  Status: Interrupted by user");
    }

    Ok(())
}