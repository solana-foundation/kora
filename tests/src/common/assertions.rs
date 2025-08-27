use serde_json::Value;
use solana_sdk::signature::Signature;
use std::str::FromStr;

/// Trait for common RPC response assertions
pub trait RpcAssertions {
    /// Assert the response indicates success
    fn assert_success(&self);

    /// Assert the response contains an error with the given code
    fn assert_error(&self, expected_code: i32);

    /// Assert the response has a valid signature
    fn assert_has_signature(&self) -> Signature;

    /// Assert the response has a specific field
    fn assert_has_field(&self, field: &str);

    /// Get a field value from the response
    fn get_field(&self, field: &str) -> Option<&Value>;
}

impl RpcAssertions for Value {
    fn assert_success(&self) {
        if let Some(error) = self.get("error") {
            panic!(
                "Expected successful response, but got error: {}",
                serde_json::to_string_pretty(error).unwrap()
            );
        }

        // Success is simply the absence of an error field
        // Different endpoints return different response structures:
        // - Transaction endpoints: have "signature" field
        // - Config endpoints: return data directly (fee_payers, validation_config, etc.)
        // - Fee estimation endpoints: return fee fields directly
        // - JSON-RPC wrapped responses: have "result" field
    }

    fn assert_error(&self, expected_code: i32) {
        let error =
            self.get("error").expect("Expected error in response, but got successful result");

        let code =
            error.get("code").and_then(|c| c.as_i64()).expect("Error response missing code field");

        assert_eq!(
            code,
            expected_code as i64,
            "Expected error code {}, got {}. Full error: {}",
            expected_code,
            code,
            serde_json::to_string_pretty(error).unwrap()
        );
    }

    fn assert_has_signature(&self) -> Signature {
        // Check for signature in multiple possible locations
        let sig_str = self
            .get("signature")
            .or_else(|| self.get("result").and_then(|r| r.get("signature")))
            .and_then(|s| s.as_str())
            .expect("Response does not contain a signature");

        Signature::from_str(sig_str).expect("Invalid signature format")
    }

    fn assert_has_field(&self, field: &str) {
        let value = self.get(field).or_else(|| self.get("result").and_then(|r| r.get(field)));

        assert!(
            value.is_some(),
            "Expected field '{}' in response, got: {}",
            field,
            serde_json::to_string_pretty(self).unwrap()
        );
    }

    fn get_field(&self, field: &str) -> Option<&Value> {
        self.get(field).or_else(|| self.get("result").and_then(|r| r.get(field)))
    }
}

/// Assertions for transaction responses
pub trait TransactionAssertions {
    /// Assert the transaction blockhash is valid (44 chars base58)
    fn assert_valid_blockhash(&self);
}

impl TransactionAssertions for Value {
    fn assert_valid_blockhash(&self) {
        let blockhash = self
            .get_field("blockhash")
            .and_then(|b| b.as_str())
            .expect("Response missing blockhash field");

        // Solana blockhashes are typically 44 chars in base58
        assert_eq!(blockhash.len(), 44, "Invalid blockhash format: {blockhash}");
    }
}

/// Standard JSON-RPC error codes for reference
pub struct JsonRpcErrorCodes;

impl JsonRpcErrorCodes {
    pub const METHOD_NOT_FOUND: i32 = -32601;
}
