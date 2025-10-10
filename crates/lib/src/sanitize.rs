//! Security-focused logging and error message sanitization
//!
//! This module provides utilities to automatically redact sensitive information
//! from error messages and logs, including:
//! - URLs with embedded credentials (any protocol: redis://, postgres://, http://, etc.)
//! - Long hex strings (potential private keys)

use regex::Regex;
use std::sync::LazyLock;

/// Regex patterns for detecting sensitive data
static URL_WITH_CREDENTIALS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Generic URL pattern with embedded credentials: protocol://user:password@host
    // Matches any protocol (redis, http, https, postgres, mysql, mongodb, etc.)
    Regex::new(r"[a-z][a-z0-9+.-]*://[^:@\s]+:[^@\s]+@[^\s]+").unwrap()
});

static HEX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Long hex strings (likely keys/hashes) - 32+ chars, with optional 0x prefix
    Regex::new(r"(?:0x)?[0-9a-fA-F]{32,}").unwrap()
});

/// Sanitizes a message by redacting sensitive information
pub fn sanitize_message(message: &str) -> String {
    let mut result = message.to_string();

    result = URL_WITH_CREDENTIALS_PATTERN.replace_all(&result, "[REDACTED_URL]").to_string();

    result = HEX_PATTERN.replace_all(&result, "[REDACTED_HEX]").to_string();

    result
}

/// Sanitizes an error message based on the `unsafe-debug` feature flag
///
/// - With `unsafe-debug`: Returns the original error message
/// - Without `unsafe-debug`: Returns a sanitized version with sensitive data redacted
#[macro_export]
macro_rules! sanitize_error {
    ($error:expr) => {{
        #[cfg(feature = "unsafe-debug")]
        {
            format!("{}", $error)
        }
        #[cfg(not(feature = "unsafe-debug"))]
        {
            $crate::sanitize::sanitize_message(&format!("{}", $error))
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url_with_credentials_redis() {
        let msg = "Failed to connect to redis://user:password@localhost:6379";
        let sanitized = sanitize_message(msg);
        assert!(sanitized.contains("[REDACTED_URL]"));
        assert!(!sanitized.contains("password"));
        assert!(!sanitized.contains("redis://user:"));
    }

    #[test]
    fn test_sanitize_url_with_credentials_http() {
        let msg = "Request failed: https://user:token@api.example.com/endpoint";
        let sanitized = sanitize_message(msg);
        assert!(sanitized.contains("[REDACTED_URL]"));
        assert!(!sanitized.contains("token"));
        assert!(!sanitized.contains("https://user:"));
    }

    #[test]
    fn test_sanitize_url_with_credentials_postgres() {
        let msg = "DB error: postgres://admin:secret123@db.internal:5432/mydb";
        let sanitized = sanitize_message(msg);
        assert!(sanitized.contains("[REDACTED_URL]"));
        assert!(!sanitized.contains("admin"));
        assert!(!sanitized.contains("secret123"));
    }

    #[test]
    fn test_sanitize_hex_string() {
        let msg = "Key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = sanitize_message(msg);
        assert!(sanitized.contains("[REDACTED_HEX]"));
    }
}
