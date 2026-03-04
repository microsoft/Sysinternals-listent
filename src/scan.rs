//! Filesystem scanning and binary discovery module
//!
//! Responsible for:
//! - Traversing directory trees to find executable binaries
//! - Detecting Mach-O binaries by magic bytes
//! - Checking file executable permissions
//! - Fast file counting for progress tracking

use std::fs;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use anyhow::Result;

/// Represents a discovered binary file
#[derive(Debug, Clone)]
pub struct DiscoveredBinary {
    pub path: PathBuf,
}

/// Fast file counting (like find) - only uses filesystem metadata
fn count_files_in_directory_with_interrupt(path: &Path, interrupted: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<usize> {
    let mut count = 0;

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Ok(0), // Skip unreadable directories silently
    };

    for entry in entries {
        // Check for interruption frequently
        if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(count); // Return partial count on interrupt
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Skip unreadable entries
        };
        let entry_path = entry.path();

        // Count files and symlinks that point to files (consistent with processing logic)
        if entry_path.is_file() {
            count += 1;
        } else if entry_path.is_dir() {
            count += count_files_in_directory_with_interrupt(&entry_path, interrupted)?;
        }
    }

    Ok(count)
}

/// Fast counting of total files in all scan paths with interrupt support
pub fn count_total_files_with_interrupt(scan_paths: &[String], interrupted: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<usize> {
    let mut total = 0;

    for path_str in scan_paths {
        // Check for interruption between directories
        if interrupted.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(total); // Return partial count on interrupt
        }

        let path = Path::new(path_str);
        if path.exists() {
            if path.is_file() {
                total += 1;
            } else if path.is_dir() {
                total += count_files_in_directory_with_interrupt(path, interrupted)?;
            }
        }
    }

    Ok(total)
}

/// Check a single file to see if it's a binary
pub fn check_single_file(path: &Path) -> Option<DiscoveredBinary> {
    check_file(path)
}

/// Check if a file is a binary we should examine
fn check_file(path: &Path) -> Option<DiscoveredBinary> {
    // Check if file is executable
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return None,
    };

    let is_executable = metadata.permissions().mode() & 0o111 != 0;

    // Quick Mach-O detection by reading file header
    let is_mach_o = is_likely_mach_o(path);

    // Include if it's executable or appears to be a Mach-O binary
    if is_executable || is_mach_o {
        Some(DiscoveredBinary {
            path: path.to_path_buf(),
        })
    } else {
        None
    }
}

/// Mach-O magic byte constants
const MH_MAGIC: [u8; 4] = [0xfe, 0xed, 0xfa, 0xce];       // 32-bit big endian
const MH_CIGAM: [u8; 4] = [0xce, 0xfa, 0xed, 0xfe];       // 32-bit little endian
const MH_MAGIC_64: [u8; 4] = [0xfe, 0xed, 0xfa, 0xcf];    // 64-bit big endian
const MH_CIGAM_64: [u8; 4] = [0xcf, 0xfa, 0xed, 0xfe];    // 64-bit little endian
const FAT_MAGIC: [u8; 4] = [0xca, 0xfe, 0xba, 0xbe];      // Universal binary
const FAT_CIGAM: [u8; 4] = [0xbe, 0xba, 0xfe, 0xca];      // Universal binary, swapped
const FAT_MAGIC_64: [u8; 4] = [0xca, 0xfe, 0xba, 0xbf];   // 64-bit universal binary
const FAT_CIGAM_64: [u8; 4] = [0xbf, 0xba, 0xfe, 0xca];   // 64-bit universal binary, swapped

/// Quick check if file might be a Mach-O binary by reading magic bytes
fn is_likely_mach_o(path: &Path) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    use std::io::Read;
    let mut buffer = [0u8; 4];
    if file.read_exact(&mut buffer).is_err() {
        return false;
    }

    const MACH_O_MAGICS: [[u8; 4]; 8] = [
        MH_MAGIC, MH_CIGAM,
        MH_MAGIC_64, MH_CIGAM_64,
        FAT_MAGIC, FAT_CIGAM,
        FAT_MAGIC_64, FAT_CIGAM_64,
    ];

    MACH_O_MAGICS.contains(&buffer)
}