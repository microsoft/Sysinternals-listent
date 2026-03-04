//! Pattern matching for entitlement filtering
//! 
//! Provides consistent entitlement filtering across static scan and monitor modes.
//! Supports both exact string matching (for backwards compatibility) and glob 
//! pattern matching with auto-detection based on pattern characters.

use glob::Pattern;
use anyhow::{Result, anyhow};

/// Check if a filter string contains glob pattern characters
pub fn is_glob_pattern(filter: &str) -> bool {
    filter.contains('*') || filter.contains('?') || filter.contains('[')
}

/// Match an entitlement key against a filter using either exact or glob matching
/// 
/// If the filter contains glob characters ('*', '?', '['), uses glob pattern matching.
/// Otherwise, uses exact string matching for backwards compatibility.
pub fn matches_entitlement_filter(entitlement: &str, filter: &str) -> bool {
    if is_glob_pattern(filter) {
        // Use glob pattern matching
        match Pattern::new(filter) {
            Ok(pattern) => pattern.matches(entitlement),
            Err(_) => {
                // If pattern is invalid, fall back to exact matching
                entitlement == filter
            }
        }
    } else {
        // Use exact string matching (backwards compatible)
        entitlement == filter
    }
}

/// Check if any entitlement in a list matches any of the provided filters
/// 
/// This is the main filtering function used by both scan and monitor modes.
/// Returns true if no filters are provided AND entitlements exist, or if any 
/// entitlement matches any filter.
pub fn entitlements_match_filters(entitlements: &[String], filters: &[String]) -> bool {
    // If no filters provided, only match binaries that have entitlements
    if filters.is_empty() {
        return !entitlements.is_empty();
    }
    
    // Check if any entitlement matches any filter (logical OR)
    filters.iter().any(|filter| {
        entitlements.iter().any(|entitlement| {
            matches_entitlement_filter(entitlement, filter)
        })
    })
}

/// Validate that all filters are syntactically correct glob patterns
pub fn validate_entitlement_filters(filters: &[String]) -> Result<()> {
    for filter in filters {
        if is_glob_pattern(filter) {
            Pattern::new(filter)
                .map_err(|e| anyhow!("Invalid glob pattern '{}': {}", filter, e))?;
        }
        // Exact strings are always valid
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_pattern() {
        assert!(!is_glob_pattern("exact.string"));
        assert!(!is_glob_pattern("com.apple.security.network.client"));
        assert!(is_glob_pattern("com.apple.*"));
        assert!(is_glob_pattern("*.network.*"));
        assert!(is_glob_pattern("com.apple.security.?"));
        assert!(is_glob_pattern("com.apple.[abc]"));
    }

    #[test]
    fn test_exact_matching() {
        let entitlement = "com.apple.security.network.client";
        
        // Exact match should work
        assert!(matches_entitlement_filter(entitlement, "com.apple.security.network.client"));
        
        // Partial match should NOT work (different from old monitor mode)
        assert!(!matches_entitlement_filter(entitlement, "network.client"));
        assert!(!matches_entitlement_filter(entitlement, "security"));
        
        // Case sensitive
        assert!(!matches_entitlement_filter(entitlement, "com.apple.SECURITY.network.client"));
    }

    #[test]
    fn test_glob_pattern_matching() {
        let entitlement = "com.apple.security.network.client";
        
        // Wildcard patterns
        assert!(matches_entitlement_filter(entitlement, "com.apple.*"));
        assert!(matches_entitlement_filter(entitlement, "*.network.*"));
        assert!(matches_entitlement_filter(entitlement, "*client"));
        assert!(matches_entitlement_filter(entitlement, "com.apple.security.*"));
        
        // Should not match
        assert!(!matches_entitlement_filter(entitlement, "com.microsoft.*"));
        assert!(!matches_entitlement_filter(entitlement, "*server"));
        
        // Question mark pattern
        assert!(matches_entitlement_filter("com.apple.security.a", "com.apple.security.?"));
        assert!(!matches_entitlement_filter("com.apple.security.ab", "com.apple.security.?"));
    }

    #[test]
    fn test_entitlements_match_filters() {
        let entitlements = vec![
            "com.apple.security.network.client".to_string(),
            "com.apple.security.app-sandbox".to_string(),
            "com.apple.private.something".to_string(),
        ];
        
        // No filters with non-empty entitlements - should match
        assert!(entitlements_match_filters(&entitlements, &[]));
        
        // No filters with empty entitlements - should NOT match (this is our fix)
        let empty_entitlements: Vec<String> = vec![];
        assert!(!entitlements_match_filters(&empty_entitlements, &[]));
        
        // Exact match
        let filters = vec!["com.apple.security.network.client".to_string()];
        assert!(entitlements_match_filters(&entitlements, &filters));
        
        // Glob pattern match
        let filters = vec!["com.apple.security.*".to_string()];
        assert!(entitlements_match_filters(&entitlements, &filters));
        
        // Multiple patterns (OR logic)
        let filters = vec![
            "com.apple.private.*".to_string(),
            "*.network.*".to_string(),
        ];
        assert!(entitlements_match_filters(&entitlements, &filters));
        
        // No match
        let filters = vec!["com.microsoft.*".to_string()];
        assert!(!entitlements_match_filters(&entitlements, &filters));
        
        // Empty entitlements with filters should never match
        let filters = vec!["com.apple.*".to_string()];
        assert!(!entitlements_match_filters(&empty_entitlements, &filters));
    }

    #[test]
    fn test_validate_entitlement_filters() {
        // Valid exact filters
        let filters = vec!["com.apple.security.network.client".to_string()];
        assert!(validate_entitlement_filters(&filters).is_ok());
        
        // Valid glob patterns
        let filters = vec!["com.apple.*".to_string(), "*.network.*".to_string()];
        assert!(validate_entitlement_filters(&filters).is_ok());
        
        // Invalid glob pattern
        let filters = vec!["com.apple.[".to_string()]; // Unclosed bracket
        assert!(validate_entitlement_filters(&filters).is_err());
    }

    #[test]
    fn test_backwards_compatibility() {
        // All current exact filters should continue to work identically
        let test_cases = vec![
            ("com.apple.security.network.client", "com.apple.security.network.client", true),
            ("com.apple.security.network.client", "com.apple.security.network.server", false),
            ("com.apple.security.network.client", "network.client", false), // This was inconsistent before
        ];
        
        for (entitlement, filter, expected) in test_cases {
            assert_eq!(
                matches_entitlement_filter(entitlement, filter), 
                expected,
                "Failed for entitlement='{}', filter='{}'", entitlement, filter
            );
        }
    }
}