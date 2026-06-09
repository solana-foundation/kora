//! Security-focused logging and error message sanitization
//!
//! This module provides utilities to automatically redact sensitive information
//! from error messages and logs, including:
//! - URLs with embedded credentials (any protocol: redis://, postgres://, http://, etc.)
//! - HTTP(S) URL paths and query strings (api-key / path-token RPC providers)
//! - Long hex strings (potential private keys)

use regex::Regex;
use std::sync::LazyLock;

/// Regex patterns for detecting sensitive data
static URL_WITH_CREDENTIALS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Generic URL pattern with embedded credentials: protocol://user:password@host
    // Matches any protocol (redis, http, https, postgres, mysql, mongodb, etc.)
    Regex::new(r"[a-z][a-z0-9+.-]*://[^:@\s]+:[^@\s]+@[^\s]+")
        .expect("Failed to create url regex pattern")
});

static HTTP_URL_PATH_QUERY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // HTTP(S) URLs where the secret lives in the path or query string rather than
    // userinfo (e.g. Helius `?api-key=`, QuickNode/Alchemy path tokens). Keeps the
    // scheme+host for debugging context and redacts everything after it.
    Regex::new(r#"(https?://[^/?\s]+)[/?][^\s)'".,;!\]]*"#)
        .expect("Failed to create http url path/query regex pattern")
});

static HEX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Long hex strings (likely keys/hashes) - 32+ chars, with optional 0x prefix
    Regex::new(r"(?:0x)?[0-9a-fA-F]{32,}").expect("Failed to create hex regex pattern")
});

/// Sanitizes a message by redacting sensitive information
pub fn sanitize_message(message: &str) -> String {
    let mut result = message.to_string();

    result = URL_WITH_CREDENTIALS_PATTERN.replace_all(&result, "[REDACTED_URL]").to_string();

    result = HTTP_URL_PATH_QUERY_PATTERN.replace_all(&result, "${1}[REDACTED_PATH]").to_string();

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
        // Ensure the error message context remains
        assert!(sanitized.contains("Failed to connect to"));
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
    fn test_sanitize_url_api_key_query() {
        let msg =
            "error sending request for url (https://devnet.helius-rpc.com/?api-key=abc-123-secret)";
        let sanitized = sanitize_message(msg);
        assert!(!sanitized.contains("api-key"));
        assert!(!sanitized.contains("abc-123-secret"));
        assert!(sanitized.contains("https://devnet.helius-rpc.com[REDACTED_PATH]"));
    }

    #[test]
    fn test_sanitize_url_path_token() {
        let msg = "Request failed: https://name.solana-devnet.quiknode.pro/SECRETTOKEN/";
        let sanitized = sanitize_message(msg);
        assert!(!sanitized.contains("SECRETTOKEN"));
        assert!(sanitized.contains("https://name.solana-devnet.quiknode.pro[REDACTED_PATH]"));
    }

    #[test]
    fn test_sanitize_url_alchemy_path_key() {
        let msg = "transport error: https://solana-devnet.g.alchemy.com/v2/myalchemykey";
        let sanitized = sanitize_message(msg);
        assert!(!sanitized.contains("myalchemykey"));
        assert!(sanitized.contains("https://solana-devnet.g.alchemy.com[REDACTED_PATH]"));
    }

    #[test]
    fn test_sanitize_preserves_public_url_without_path() {
        let msg = "failed to reach https://api.devnet.solana.com";
        let sanitized = sanitize_message(msg);
        assert_eq!(sanitized, "failed to reach https://api.devnet.solana.com");
    }

    #[test]
    fn test_sanitize_preserves_trailing_prose_punctuation() {
        let msg = "failed to send request to https://node.example.com/secret, please retry";
        let sanitized = sanitize_message(msg);
        assert!(!sanitized.contains("/secret"));
        assert!(sanitized.contains("https://node.example.com[REDACTED_PATH], please retry"));
    }

    #[test]
    fn test_sanitize_hex_string() {
        let msg = "Key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = sanitize_message(msg);
        assert!(sanitized.contains("[REDACTED_HEX]"));
    }
}
