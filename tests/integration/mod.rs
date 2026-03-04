//! Integration tests module
//!
//! End-to-end tests that exercise the CLI with real filesystem operations

pub mod test_basic_monitoring;
pub mod test_combined_filters;
pub mod test_default_scan;
pub mod test_entitlement_filters;
pub mod test_interrupt_handling;
pub mod test_json_output;
pub mod test_monitor_entitlement_filtering;
pub mod test_monitor_output_formats;
pub mod test_monitor_path_filtering;
pub mod test_no_matches;
pub mod test_path_filters;
pub mod test_real_process_detection;
pub mod test_unreadable_files;
