use anyhow::Error as AnyhowError;
use jsonrpsee::core::Error as RpcError;
use serde_json::Value;

/// Trait for common RPC response assertions
pub trait RpcAssertions {
    fn assert_success(&self);
    fn assert_has_field(&self, field: &str);
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
    fn assert_valid_blockhash(&self);
}

impl TransactionAssertions for Value {
    fn assert_valid_blockhash(&self) {
        let blockhash = self
            .get_field("blockhash")
            .and_then(|b| b.as_str())
            .expect("Response missing blockhash field");

        // Solana blockhashes are typically 44 chars in base58
        assert!(
            blockhash.len() >= 43 && blockhash.len() <= 44,
            "Invalid blockhash format: {blockhash}"
        );
    }
}

/// Trait for RPC error assertions
pub trait RpcErrorAssertions {
    fn assert_contains_message(&self, expected_message: &str);
    fn assert_error_type_and_message(&self, expected_type: &str, expected_message: &str);
}

impl RpcErrorAssertions for RpcError {
    fn assert_contains_message(&self, expected_message: &str) {
        let error_str = self.to_string();
        assert!(
            error_str.contains(expected_message),
            "Expected error to contain '{expected_message}', got: {error_str}",
        );
    }

    fn assert_error_type_and_message(&self, expected_type: &str, expected_message: &str) {
        let error_str = self.to_string();
        assert!(
            error_str.contains(expected_type),
            "Expected error type '{expected_type}', got: {error_str}",
        );
        assert!(
            error_str.contains(expected_message),
            "Expected error to contain '{expected_message}', got: {error_str}",
        );
    }
}

impl RpcErrorAssertions for AnyhowError {
    fn assert_contains_message(&self, expected_message: &str) {
        let error_str = self.to_string();
        assert!(
            error_str.contains(expected_message),
            "Expected error to contain '{expected_message}', got: {error_str}",
        );
    }

    fn assert_error_type_and_message(&self, expected_type: &str, expected_message: &str) {
        let error_str = self.to_string();
        assert!(
            error_str.contains(expected_type),
            "Expected error type '{expected_type}', got: {error_str}",
        );
        assert!(
            error_str.contains(expected_message),
            "Expected error to contain '{expected_message}', got: {error_str}",
        );
    }
}
