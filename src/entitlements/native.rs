//! Optimized entitlement extraction using the plist crate
//!
//! This module provides more reliable entitlement parsing than the fallback:
//! - Uses the plist crate for proper binary/XML plist parsing
//! - Better error handling for edge cases

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::constants::{CODESIGN_COMMAND, CODESIGN_ENTITLEMENT_ARGS};

/// Extract entitlements using optimized codesign with proper plist parsing
pub fn extract_entitlements_optimized(binary_path: &Path) -> Result<HashMap<String, Value>> {
    // Call codesign to extract entitlements in plist format
    let output = Command::new(CODESIGN_COMMAND)
        .args(CODESIGN_ENTITLEMENT_ARGS)
        .arg(binary_path)
        .output()?;

    if !output.status.success() {
        // Binary might not be signed or might not have entitlements
        return Ok(HashMap::new());
    }

    if output.stdout.is_empty() {
        return Ok(HashMap::new());
    }

    // Parse the plist XML using the plist crate for better performance and reliability
    let plist_value: plist::Value = plist::from_bytes(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse entitlements plist: {}", e))?;

    // Convert plist value to JSON-compatible HashMap
    plist_to_json_map(plist_value)
}

/// Convert plist::Value to JSON-compatible HashMap
fn plist_to_json_map(plist_value: plist::Value) -> Result<HashMap<String, Value>> {
    match plist_value {
        plist::Value::Dictionary(dict) => {
            let mut result = HashMap::new();
            for (key, value) in dict {
                let json_value = plist_value_to_json_value(value)?;
                result.insert(key, json_value);
            }
            Ok(result)
        }
        _ => {
            // If the root is not a dictionary, return empty (no entitlements)
            Ok(HashMap::new())
        }
    }
}

/// Convert plist::Value to serde_json::Value
fn plist_value_to_json_value(plist_value: plist::Value) -> Result<Value> {
    match plist_value {
        plist::Value::String(s) => Ok(Value::String(s)),
        plist::Value::Boolean(b) => Ok(Value::Bool(b)),
        plist::Value::Integer(i) => {
            // Convert plist::Integer to string and then parse to i64
            let s = i.to_string();
            if let Ok(value) = s.parse::<i64>() {
                Ok(Value::Number(value.into()))
            } else {
                // If it can't fit in i64, keep as string
                Ok(Value::String(s))
            }
        }
        plist::Value::Real(f) => {
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .ok_or_else(|| anyhow!("Invalid floating point number: {}", f))
        }
        plist::Value::Array(arr) => {
            let mut json_arr = Vec::new();
            for item in arr {
                json_arr.push(plist_value_to_json_value(item)?);
            }
            Ok(Value::Array(json_arr))
        }
        plist::Value::Dictionary(dict) => {
            let mut json_obj = serde_json::Map::new();
            for (key, value) in dict {
                json_obj.insert(key, plist_value_to_json_value(value)?);
            }
            Ok(Value::Object(json_obj))
        }
        plist::Value::Data(data) => {
            // Convert binary data to hex string for display
            Ok(Value::String(hex_encode(&data)))
        }
        plist::Value::Date(date) => {
            // Convert date to string representation
            Ok(Value::String(format!("{:?}", date)))
        }
        plist::Value::Uid(_uid) => {
            // UIDs are not commonly used in entitlements, treat as null
            Ok(Value::Null)
        }
        _ => {
            // Handle any other plist types by converting to string
            Ok(Value::String(format!("{:?}", plist_value)))
        }
    }
}

/// Encode binary data as a hex string for display
fn hex_encode(data: &[u8]) -> String {
    format!("0x{}", data.iter().map(|b| format!("{:02x}", b)).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_optimized_extraction_system_binary() {
        // Test with a known system binary that should have entitlements
        let test_binary = PathBuf::from("/usr/bin/top");
        if test_binary.exists() {
            let result = extract_entitlements_optimized(&test_binary);
            match result {
                Ok(entitlements) => {
                    if !entitlements.is_empty() {
                        println!("Found {} entitlements in /usr/bin/top", entitlements.len());
                        for (key, value) in &entitlements {
                            println!("  {}: {:?}", key, value);
                        }
                    }
                }
                Err(e) => {
                    println!("Could not extract entitlements from /usr/bin/top: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_optimized_extraction_unsigned_binary() {
        // Test with our own binary (likely unsigned in debug builds)
        let current_exe = std::env::current_exe().expect("Could not get current executable");
        let result = extract_entitlements_optimized(&current_exe);

        // Should succeed but might return empty entitlements for unsigned binaries
        assert!(result.is_ok(), "Optimized extraction should handle unsigned binaries gracefully");
    }
}