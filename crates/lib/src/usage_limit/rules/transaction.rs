use super::super::limiter::LimiterContext;

const TX_KEY_PREFIX: &str = "kora:tx";

/// Rule that limits the total number of transactions per user
///
/// Supports both lifetime limits (never resets) and time-windowed limits (resets periodically).
#[derive(Debug)]
pub struct TransactionRule {
    max: u64,
    window_seconds: Option<u64>,
}

impl TransactionRule {
    pub fn new(max: u64, window_seconds: Option<u64>) -> Self {
        Self { max, window_seconds }
    }

    pub fn storage_key(&self, user_id: &str, timestamp: u64) -> String {
        let base = format!("{TX_KEY_PREFIX}:{user_id}");
        match self.window_seconds {
            Some(window) if window > 0 => format!("{base}:{}", timestamp / window),
            _ => base,
        }
    }

    /// How many units to increment for this transaction (always 1)
    pub fn count_increment(&self, _ctx: &mut LimiterContext<'_>) -> u64 {
        1
    }

    /// Maximum allowed count within the window (or lifetime)
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Time window in seconds
    pub fn window_seconds(&self) -> Option<u64> {
        self.window_seconds
    }

    pub fn description(&self) -> String {
        let window = self.window_seconds.map_or("lifetime".to_string(), |w| format!("per {w}s"));
        format!("transaction ({window})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::transaction_mock::create_mock_resolved_transaction;

    #[test]
    fn test_transaction_rule_lifetime_key() {
        let rule = TransactionRule::new(100, None);
        let user_id = "test-user-123";

        let key = rule.storage_key(user_id, 1000000);
        assert_eq!(key, format!("kora:tx:{}", user_id));
    }

    #[test]
    fn test_transaction_rule_windowed_key() {
        let rule = TransactionRule::new(100, Some(3600));
        let user_id = "test-user-456";

        let key1 = rule.storage_key(user_id, 3600);
        let key2 = rule.storage_key(user_id, 7199);
        let key3 = rule.storage_key(user_id, 7200);

        assert_eq!(key1, format!("kora:tx:{}:1", user_id));
        assert_eq!(key2, format!("kora:tx:{}:1", user_id));
        assert_eq!(key3, format!("kora:tx:{}:2", user_id));
    }

    #[test]
    fn test_transaction_rule_count_increment() {
        let rule = TransactionRule::new(100, None);
        let tx = create_mock_resolved_transaction();
        let user_id = "test-user-789".to_string();
        let mut tx = tx;
        let mut ctx =
            LimiterContext { transaction: &mut tx, user_id, kora_signer: None, timestamp: 1000000 };

        assert_eq!(rule.count_increment(&mut ctx), 1);
    }

    #[test]
    fn test_transaction_rule_description() {
        let lifetime = TransactionRule::new(100, None);
        assert_eq!(lifetime.description(), "transaction (lifetime)");

        let windowed = TransactionRule::new(100, Some(3600));
        assert_eq!(windowed.description(), "transaction (per 3600s)");
    }
}
