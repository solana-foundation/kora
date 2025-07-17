use crate::error::KoraError;
use serde_json;
use solana_sdk::signature::Keypair;
use std::fs;

/// Utility functions for parsing private keys in multiple formats
pub struct KeypairUtil;

impl KeypairUtil {
    /// Creates a new keypair from a private key string that can be in multiple formats:
    /// - Base58 encoded string (current format)
    /// - U8Array format: "[0, 1, 2, ...]"
    /// - File path to a JSON keypair file
    pub fn from_private_key_string(private_key: &str) -> Result<Keypair, KoraError> {
        // Try to parse as a file path first
        if let Ok(file_content) = fs::read_to_string(private_key) {
            return Self::from_json_keypair(&file_content);
        }

        // Try to parse as U8Array format
        if private_key.trim().starts_with('[') && private_key.trim().ends_with(']') {
            return Self::from_u8_array_string(private_key);
        }

        // Default to base58 format (with proper error handling)
        Self::from_base58_safe(private_key)
    }

    /// Creates a new keypair from a base58-encoded private key string with proper error handling
    pub fn from_base58_safe(private_key: &str) -> Result<Keypair, KoraError> {
        // Try to decode as base58 first
        let decoded = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| KoraError::SigningError(format!("Invalid base58 string: {e}")))?;

        if decoded.len() != 64 {
            return Err(KoraError::SigningError(format!(
                "Invalid private key length: expected 64 bytes, got {}",
                decoded.len()
            )));
        }

        let keypair = Keypair::try_from(&decoded[..])
            .map_err(|e| KoraError::SigningError(format!("Invalid private key bytes: {e}")))?;

        Ok(keypair)
    }

    /// Creates a new keypair from a U8Array format string like "[0, 1, 2, ...]"
    pub fn from_u8_array_string(array_str: &str) -> Result<Keypair, KoraError> {
        let trimmed = array_str.trim();

        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Err(KoraError::SigningError(
                "U8Array string must start with '[' and end with ']'".to_string(),
            ));
        }

        let inner = &trimmed[1..trimmed.len() - 1];

        if inner.trim().is_empty() {
            return Err(KoraError::SigningError("U8Array string cannot be empty".to_string()));
        }

        let bytes: Result<Vec<u8>, _> = inner.split(',').map(|s| s.trim().parse::<u8>()).collect();

        match bytes {
            Ok(byte_array) => {
                if byte_array.len() != 64 {
                    return Err(KoraError::SigningError(format!(
                        "Private key must be exactly 64 bytes, got {}",
                        byte_array.len()
                    )));
                }
                Keypair::try_from(&byte_array[..])
                    .map_err(|e| KoraError::SigningError(format!("Invalid private key bytes: {e}")))
            }
            Err(e) => Err(KoraError::SigningError(format!("Failed to parse U8Array: {e}"))),
        }
    }

    /// Creates a new keypair from a JSON keypair file content
    pub fn from_json_keypair(json_content: &str) -> Result<Keypair, KoraError> {
        // Try to parse as a simple JSON array first
        if let Ok(byte_array) = serde_json::from_str::<Vec<u8>>(json_content) {
            if byte_array.len() != 64 {
                return Err(KoraError::SigningError(format!(
                    "JSON keypair must be exactly 64 bytes, got {}",
                    byte_array.len()
                )));
            }
            return Keypair::try_from(&byte_array[..])
                .map_err(|e| KoraError::SigningError(format!("Invalid private key bytes: {e}")));
        }

        Err(KoraError::SigningError(
            "Invalid JSON keypair format. Expected either a JSON array of 64 bytes or an object with a 'keypair' field".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{signature::Keypair, signer::Signer};
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_base58_format() {
        let keypair = Keypair::new();
        let base58_key = bs58::encode(keypair.to_bytes()).into_string();

        let parsed_keypair = KeypairUtil::from_private_key_string(&base58_key).unwrap();
        assert_eq!(parsed_keypair.pubkey(), keypair.pubkey());
    }

    #[test]
    fn test_from_u8_array_format() {
        let keypair = Keypair::new();
        let bytes = keypair.to_bytes();

        let u8_array_str =
            format!("[{}]", bytes.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "));

        let parsed_keypair = KeypairUtil::from_private_key_string(&u8_array_str).unwrap();
        assert_eq!(parsed_keypair.pubkey(), keypair.pubkey());
    }

    #[test]
    fn test_from_json_file_path() {
        let keypair = Keypair::new();
        let bytes = keypair.to_bytes();

        let temp_file = NamedTempFile::new().unwrap();
        let json_str = serde_json::to_string(&bytes.to_vec()).unwrap();
        fs::write(temp_file.path(), json_str).unwrap();

        let parsed_keypair =
            KeypairUtil::from_private_key_string(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(parsed_keypair.pubkey(), keypair.pubkey());
    }

    #[test]
    fn test_invalid_formats() {
        // Test invalid U8Array
        let result = KeypairUtil::from_private_key_string("[1, 2, 3]");
        assert!(result.is_err());

        // Test invalid JSON
        let result = KeypairUtil::from_private_key_string("{invalid json}");
        assert!(result.is_err());

        // Test nonexistent file
        let result = KeypairUtil::from_private_key_string("/nonexistent/file.json");
        assert!(result.is_err());
    }
}
