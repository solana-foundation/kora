//! Types for Jito bundle operations

use serde::{Deserialize, Serialize};
use solana_sdk::transaction::VersionedTransaction;
use utoipa::ToSchema;

/// Represents a bundle of transactions to be submitted to Jito
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The transactions in the bundle (max 5)
    pub transactions: Vec<VersionedTransaction>,
    /// Index of transaction containing the tip (if any)
    pub tip_transaction_index: Option<usize>,
    /// Index of the tip instruction within the tip transaction (if any)
    pub tip_instruction_index: Option<usize>,
    /// The tip amount in lamports
    pub tip_lamports: u64,
}

impl Bundle {
    /// Creates a new bundle from transactions
    pub fn new(transactions: Vec<VersionedTransaction>) -> Self {
        Self {
            transactions,
            tip_transaction_index: None,
            tip_instruction_index: None,
            tip_lamports: 0,
        }
    }

    /// Returns the number of transactions in the bundle
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Returns true if the bundle has no transactions
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Returns true if the bundle has a tip
    pub fn has_tip(&self) -> bool {
        self.tip_transaction_index.is_some()
    }
}

/// Result of bundle validation
#[derive(Debug, Clone)]
pub struct BundleValidationResult {
    /// Whether the bundle is valid
    pub is_valid: bool,
    /// Whether the bundle has a tip
    pub has_tip: bool,
    /// The tip amount in lamports (0 if no tip)
    pub tip_amount_lamports: u64,
    /// Total estimated fees for all transactions in lamports
    pub total_fee_lamports: u64,
    /// Validation errors (if any)
    pub errors: Vec<String>,
}

impl BundleValidationResult {
    /// Creates a valid result
    pub fn valid(has_tip: bool, tip_amount_lamports: u64, total_fee_lamports: u64) -> Self {
        Self {
            is_valid: true,
            has_tip,
            tip_amount_lamports,
            total_fee_lamports,
            errors: Vec::new(),
        }
    }

    /// Creates an invalid result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            has_tip: false,
            tip_amount_lamports: 0,
            total_fee_lamports: 0,
            errors,
        }
    }
}

/// Response from Jito sendBundle API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendBundleResponse {
    /// The bundle ID returned by Jito
    pub bundle_id: String,
}

/// Response from Jito getBundleStatuses API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleStatus {
    /// The bundle ID
    pub bundle_id: String,
    /// Status of the bundle
    pub status: BundleStatusType,
    /// Slot the bundle landed in (if landed)
    pub landed_slot: Option<u64>,
}

/// Bundle status types from Jito
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum BundleStatusType {
    /// Bundle is pending
    Pending,
    /// Bundle landed on chain
    Landed,
    /// Bundle failed to land
    Failed,
    /// Bundle was not found
    Invalid,
}

/// Information about a detected tip in a bundle
#[derive(Debug, Clone)]
pub struct TipInfo {
    /// Index of the transaction containing the tip
    pub transaction_index: usize,
    /// Index of the tip instruction within the transaction
    pub instruction_index: usize,
    /// The tip amount in lamports
    pub amount_lamports: u64,
    /// The tip recipient account
    pub tip_account: solana_sdk::pubkey::Pubkey,
}

/// Request for signBundle RPC method
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SignBundleRequest {
    /// Array of base64-encoded transactions (max 5)
    pub transactions: Vec<String>,
    /// Optional signer key for consistency
    #[serde(default)]
    pub signer_key: Option<String>,
    /// Tip amount in lamports (uses default if not provided)
    #[serde(default)]
    pub tip_lamports: Option<u64>,
    /// Whether to auto-add tip if missing (default: true)
    #[serde(default = "default_true")]
    pub auto_add_tip: bool,
}

/// Response for signBundle RPC method
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignBundleResponse {
    /// Array of base64-encoded signed transactions
    pub signed_transactions: Vec<String>,
    /// Signer public key
    pub signer_pubkey: String,
    /// Total tip amount in lamports
    pub tip_lamports: u64,
    /// Index of transaction containing tip
    pub tip_transaction_index: usize,
}

/// Request for signAndSendBundle RPC method
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SignAndSendBundleRequest {
    /// Array of base64-encoded transactions (max 5)
    pub transactions: Vec<String>,
    /// Optional signer key for consistency
    #[serde(default)]
    pub signer_key: Option<String>,
    /// Tip amount in lamports (uses default if not provided)
    #[serde(default)]
    pub tip_lamports: Option<u64>,
    /// Whether to auto-add tip if missing (default: true)
    #[serde(default = "default_true")]
    pub auto_add_tip: bool,
}

/// Response for signAndSendBundle RPC method
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignAndSendBundleResponse {
    /// Jito bundle ID for status tracking
    pub bundle_id: String,
    /// Array of transaction signatures
    pub signatures: Vec<String>,
    /// Total tip amount in lamports
    pub tip_lamports: u64,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_new() {
        let bundle = Bundle::new(vec![]);
        assert!(bundle.is_empty());
        assert!(!bundle.has_tip());
        assert_eq!(bundle.tip_lamports, 0);
    }

    #[test]
    fn test_bundle_validation_result_valid() {
        let result = BundleValidationResult::valid(true, 10000, 50000);
        assert!(result.is_valid);
        assert!(result.has_tip);
        assert_eq!(result.tip_amount_lamports, 10000);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_bundle_validation_result_invalid() {
        let result =
            BundleValidationResult::invalid(vec!["Error 1".to_string(), "Error 2".to_string()]);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 2);
    }
}
