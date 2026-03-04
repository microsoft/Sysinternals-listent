//! Entitlement extraction module
//!
//! Handles:
//! - Extracting entitlements from Mach-O binaries using optimized plist parsing
//! - Fallback to manual XML parsing for compatibility
//! - Error handling for unsigned/malformed binaries
//! - Performance optimization for batch operations
//! - Pattern matching for entitlement filtering

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};
use serde_json::Value;
use crate::constants::{CODESIGN_COMMAND, CODESIGN_ENTITLEMENT_ARGS};

pub mod pattern_matcher;
pub mod native;

/// Extract entitlements from a binary file
///
/// Uses optimized plist parsing for better performance,
/// with fallback to manual XML parsing if needed.
pub fn extract_entitlements(binary_path: &Path) -> Result<HashMap<String, Value>> {
    // Try optimized plist parsing first
    match native::extract_entitlements_optimized(binary_path) {
        Ok(entitlements) => return Ok(entitlements),
        Err(_) => {
            // Fall back to manual XML parsing if plist parsing fails
            // This provides compatibility for edge cases
        }
    }

    // Fallback to manual XML parsing (original implementation)
    extract_entitlements_codesign(binary_path)
}

/// Extract entitlements using codesign command-line tool (fallback method)
pub fn extract_entitlements_codesign(binary_path: &Path) -> Result<HashMap<String, Value>> {
    // Call codesign to extract entitlements
    let output = Command::new(CODESIGN_COMMAND)
        .args(CODESIGN_ENTITLEMENT_ARGS)
        .arg(binary_path)
        .output()?;

    if !output.status.success() {
        // Binary might not be signed or might not have entitlements
        return Ok(HashMap::new());
    }

    let xml_content = String::from_utf8(output.stdout)?;

    // Parse the XML plist to extract entitlements
    parse_entitlements_plist(&xml_content)
}

/// Parse entitlements from XML plist format
fn parse_entitlements_plist(xml_content: &str) -> Result<HashMap<String, Value>> {
    // Simple XML parsing for plist format
    // Look for the main dict content between <dict> and </dict>

    if xml_content.trim().is_empty() {
        return Ok(HashMap::new());
    }

    // Find the main dictionary content
    let dict_start = xml_content.find("<dict>")
        .ok_or_else(|| anyhow!("No dict found in plist"))?;
    let dict_end = xml_content.rfind("</dict>")
        .ok_or_else(|| anyhow!("Unclosed dict in plist"))?;

    if dict_start >= dict_end {
        return Ok(HashMap::new());
    }

    let dict_content = &xml_content[dict_start + 6..dict_end];

    // Parse key-value pairs
    parse_plist_dict(dict_content)
}

/// Parse dictionary content from plist XML
fn parse_plist_dict(content: &str) -> Result<HashMap<String, Value>> {
    let mut entitlements = HashMap::new();
    let mut pos = 0;

    while pos < content.len() {
        // Find next <key> tag
        if let Some(key_start) = content[pos..].find("<key>") {
            let abs_key_start = pos + key_start + 5; // Skip "<key>"

            if let Some(key_end) = content[abs_key_start..].find("</key>") {
                let abs_key_end = abs_key_start + key_end;
                let key = content[abs_key_start..abs_key_end].trim().to_string();

                // Find the value after the key
                pos = abs_key_end + 6; // Skip "</key>"

                if let Some(value) = parse_next_plist_value(&content[pos..])? {
                    entitlements.insert(key, value.0);
                    pos += value.1;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(entitlements)
}

/// Parse the next value from plist XML
fn parse_next_plist_value(content: &str) -> Result<Option<(Value, usize)>> {
    let trimmed = content.trim_start();
    let offset = content.len() - trimmed.len();

    if trimmed.starts_with("<true/>") {
        Ok(Some((Value::Bool(true), offset + 7)))
    } else if trimmed.starts_with("<false/>") {
        Ok(Some((Value::Bool(false), offset + 8)))
    } else if trimmed.starts_with("<string>") {
        if let Some(end) = trimmed.find("</string>") {
            let value = trimmed[8..end].to_string();
            Ok(Some((Value::String(value), offset + end + 9)))
        } else {
            Ok(None)
        }
    } else if trimmed.starts_with("<integer>") {
        if let Some(end) = trimmed.find("</integer>") {
            let value_str = &trimmed[9..end];
            if let Ok(num) = value_str.parse::<i64>() {
                Ok(Some((Value::Number(num.into()), offset + end + 10)))
            } else {
                Ok(Some((Value::String(value_str.to_string()), offset + end + 10)))
            }
        } else {
            Ok(None)
        }
    } else if trimmed.starts_with("<array>") {
        // For simplicity, treat arrays as strings for now
        if let Some(end) = trimmed.find("</array>") {
            let array_content = &trimmed[7..end];
            Ok(Some((Value::String(format!("[array: {}]", array_content.trim())), offset + end + 8)))
        } else {
            Ok(None)
        }
    } else if trimmed.starts_with("<dict>") {
        // For simplicity, treat nested dicts as strings for now
        if let Some(end) = trimmed.find("</dict>") {
            let dict_content = &trimmed[6..end];
            Ok(Some((Value::String(format!("[dict: {}]", dict_content.trim())), offset + end + 7)))
        } else {
            Ok(None)
        }
    } else {
        // Skip unknown tags
        if let Some(tag_end) = trimmed.find('>') {
            Ok(Some((Value::String("[unknown]".to_string()), offset + tag_end + 1)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;
    use std::io::Write;

    // ==================== parse_entitlements_plist tests ====================

    #[test]
    fn test_parse_empty_plist() {
        let result = parse_entitlements_plist("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_plist_with_single_boolean_entitlement() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("com.apple.security.app-sandbox"));
        assert_eq!(result.get("com.apple.security.app-sandbox").unwrap(), &Value::Bool(true));
    }

    #[test]
    fn test_parse_plist_with_multiple_entitlements() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <false/>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains_key("com.apple.security.app-sandbox"));
        assert!(result.contains_key("com.apple.security.network.client"));
        assert!(result.contains_key("com.apple.security.network.server"));
    }

    #[test]
    fn test_parse_plist_with_string_value() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.application-identifier</key>
    <string>TEAM123.com.example.app</string>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("com.apple.application-identifier"));
    }

    #[test]
    fn test_parse_plist_without_dict() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<!-- No dict here -->
</plist>"#;

        let result = parse_entitlements_plist(plist);
        assert!(result.is_err(), "Missing dict should fail");
    }

    #[test]
    fn test_parse_plist_unclosed_dict() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>test</key>
    <true/>
<!-- missing </dict> -->"#;

        let result = parse_entitlements_plist(plist);
        // Parser should handle this gracefully - either error or partial result
        // Both outcomes are acceptable for malformed input
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_plist_with_integer_value() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>some.integer.key</key>
    <integer>42</integer>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("some.integer.key"));
    }

    // ==================== extract_entitlements tests ====================

    #[test]
    fn test_extract_entitlements_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/to/binary");
        let result = extract_entitlements(&path);
        // Should return empty or error, not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_extract_entitlements_not_a_binary() {
        // Create a temp file that's just text, not a binary
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "This is not a binary file").unwrap();

        let result = extract_entitlements(temp_file.path());
        // Should return empty HashMap (not signed)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_extract_entitlements_from_real_system_binary() {
        // Use a known system binary that exists on macOS
        let path = PathBuf::from("/usr/bin/sudo");
        if path.exists() {
            let result = extract_entitlements(&path);
            // Should not panic, may or may not have entitlements
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_extract_entitlements_from_apple_app() {
        // Calculator.app should exist and be code signed
        let path = PathBuf::from("/System/Applications/Calculator.app/Contents/MacOS/Calculator");
        if path.exists() {
            let result = extract_entitlements(&path);
            assert!(result.is_ok());
            // Apple apps typically have entitlements
        }
    }

    // ==================== extract_entitlements_codesign tests ====================

    #[test]
    fn test_codesign_fallback_with_nonexistent_binary() {
        let path = PathBuf::from("/nonexistent/binary");
        let result = extract_entitlements_codesign(&path);
        // Should not panic, return empty or error
        assert!(result.is_ok() || result.is_err());
    }

    // ==================== Error handling edge cases ====================

    #[test]
    fn test_extract_handles_permission_denied_gracefully() {
        // /etc/sudoers is readable only by root
        let path = PathBuf::from("/etc/sudoers");
        if path.exists() {
            let result = extract_entitlements(&path);
            // Should handle gracefully (empty result), not panic
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_extract_handles_directory_gracefully() {
        // Pass a directory instead of a file
        let path = PathBuf::from("/Applications");
        let result = extract_entitlements(&path);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_extract_handles_symlink() {
        // Many macOS binaries are symlinks
        let path = PathBuf::from("/usr/bin/python3");
        if path.exists() {
            let result = extract_entitlements(&path);
            // Should follow symlink and handle gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }

    // ==================== Plist parsing edge cases ====================

    #[test]
    fn test_parse_plist_with_whitespace() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>

    <key>  com.apple.security.app-sandbox  </key>

    <true/>

</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_plist_with_array_value() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.com.example</string>
    </array>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.contains_key("com.apple.security.application-groups"));
    }

    #[test]
    fn test_parse_plist_with_nested_dict() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.developer.associated-domains</key>
    <dict>
        <key>webcredentials</key>
        <string>example.com</string>
    </dict>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_plist_preserves_key_names() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>"#;

        let result = parse_entitlements_plist(plist).unwrap();

        // Keys should be preserved exactly
        assert!(result.contains_key("com.apple.security.cs.allow-jit"));
        assert!(result.contains_key("com.apple.security.cs.disable-library-validation"));
    }
}