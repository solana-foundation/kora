use solana_sdk::pubkey::Pubkey;

use crate::transaction::VersionedTransactionResolved;

/// Context passed to all limiters containing transaction data and resolved user identifier
pub struct LimiterContext<'a> {
    /// The resolved transaction
    pub transaction: &'a mut VersionedTransactionResolved,
    /// User identifier for usage tracking (can be any string - pubkey, UUID, etc.)
    pub user_id: String,
    /// Kora signer pubkey (if present) - used for filtering applicable instructions
    pub kora_signer: Option<Pubkey>,
    /// Unix timestamp of the request
    pub timestamp: u64,
}

/// Result of a limiter check
#[derive(Debug, Clone)]
pub enum LimiterResult {
    /// Transaction allowed
    Allowed,
    /// Transaction denied with reason
    Denied { reason: String },
}
