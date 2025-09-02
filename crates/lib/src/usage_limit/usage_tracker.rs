use std::{collections::HashSet, sync::Arc};

use deadpool_redis::Runtime;
use redis::AsyncCommands;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use tokio::sync::OnceCell;

use super::usage_store::{RedisUsageStore, UsageStore};
use crate::{error::KoraError, get_all_signers};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Global usage limiter instance
static USAGE_LIMITER: OnceCell<Option<UsageTracker>> = OnceCell::const_new();

pub struct UsageTracker {
    store: Arc<dyn UsageStore>,
    default_max_transactions: u64,
    kora_signers: HashSet<Pubkey>,
}

impl UsageTracker {
    pub fn new(
        store: Arc<dyn UsageStore>,
        default_max_transactions: u64,
        kora_signers: HashSet<Pubkey>,
    ) -> Self {
        Self { store, default_max_transactions, kora_signers }
    }

    fn get_usage_key(&self, wallet: &Pubkey) -> String {
        format!("kora:usage_limit:{wallet}")
    }

    async fn increment_usage(&self, wallet: &Pubkey) -> Result<u32, KoraError> {
        let key = self.get_usage_key(wallet);
        self.store.increment(&key).await
    }

    async fn check_usage_limit(&self, wallet: &Pubkey) -> Result<(), KoraError> {
        // Skip check if unlimited (0)
        if self.default_max_transactions == 0 {
            return Ok(());
        }

        let current_count = self.increment_usage(wallet).await?;

        if current_count > self.default_max_transactions as u32 {
            return Err(KoraError::UsageLimitExceeded(format!(
                "Wallet {wallet} exceeded limit: {current_count}/{}",
                self.default_max_transactions
            )));
        }

        log::debug!(
            "Usage check passed for {wallet}: {current_count}/{}",
            self.default_max_transactions
        );

        Ok(())
    }

    fn get_usage_limiter() -> Result<Option<&'static UsageTracker>, KoraError> {
        match USAGE_LIMITER.get() {
            Some(limiter) => Ok(limiter.as_ref()),
            None => {
                Err(KoraError::InternalServerError("Usage limiter not initialized".to_string()))
            }
        }
    }

    /// Extract sender from transaction
    fn extract_transaction_sender(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Pubkey, KoraError> {
        let account_keys = transaction.message.static_account_keys();

        if account_keys.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction has no account keys".to_string(),
            ));
        }

        for signer in
            account_keys.iter().take(transaction.message.header().num_required_signatures as usize)
        {
            if !self.kora_signers.contains(signer) {
                return Ok(*signer);
            }
        }

        Err(KoraError::InvalidTransaction(
            "No user signers found (all signers are Kora fee payers)".to_string(),
        ))
    }

    /// Initialize the global usage limiter
    pub async fn init_usage_limiter() -> Result<(), KoraError> {
        let config = get_config()?;

        if !config.kora.usage_limit.enabled {
            log::info!("Usage limiting disabled");
            USAGE_LIMITER.set(None).map_err(|_| {
                KoraError::InternalServerError("Usage limiter already initialized".to_string())
            })?;
            return Ok(());
        }

        let usage_limiter = if let Some(cache_url) = &config.kora.usage_limit.cache_url {
            let cfg = deadpool_redis::Config::from_url(cache_url);
            let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
                KoraError::InternalServerError(format!("Failed to create Redis pool: {e}"))
            })?;

            // Test Redis connection
            let mut conn = pool.get().await.map_err(|e| {
                KoraError::InternalServerError(format!("Failed to connect to Redis: {e}"))
            })?;

            // Simple connection test
            let _: Option<String> = conn.get("__usage_limiter_test__").await.map_err(|e| {
                KoraError::InternalServerError(format!("Redis connection test failed: {e}"))
            })?;

            log::info!(
                "Usage limiter initialized with Redis at {} (max: {} transactions)",
                cache_url,
                config.kora.usage_limit.default_max_transactions
            );

            let kora_signers =
                get_all_signers()?.iter().map(|signer| signer.signer.solana_pubkey()).collect();

            let store = Arc::new(RedisUsageStore::new(pool));
            Some(UsageTracker::new(
                store,
                config.kora.usage_limit.default_max_transactions,
                kora_signers,
            ))
        } else {
            log::info!("Usage limiting enabled but no cache_url configured - disabled");
            None
        };

        USAGE_LIMITER.set(usage_limiter).map_err(|_| {
            KoraError::InternalServerError("Usage limiter already initialized".to_string())
        })?;

        Ok(())
    }

    /// Check usage limit for transaction sender
    pub async fn check_transaction_usage_limit(
        transaction: &VersionedTransaction,
    ) -> Result<(), KoraError> {
        let config = get_config()?;

        if let Some(limiter) = Self::get_usage_limiter()? {
            let sender = limiter.extract_transaction_sender(transaction)?;
            limiter.check_usage_limit(&sender).await?;
            Ok(())
        } else if config.kora.usage_limit.enabled
            && !config.kora.usage_limit.fallback_if_unavailable
        {
            // Usage limiting enabled but limiter unavailable and fallback disabled
            Err(KoraError::InternalServerError(
                "Usage limiter unavailable and fallback disabled".to_string(),
            ))
        } else {
            // Usage limiting disabled or fallback allowed
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::{config_mock::ConfigMockBuilder, transaction_mock::create_mock_transaction},
        usage_limit::InMemoryUsageStore,
    };

    #[tokio::test]
    async fn test_get_usage_key_format() {
        let wallet = Pubkey::new_unique();
        let expected_key = format!("kora:usage_limit:{wallet}");

        assert_eq!(expected_key, format!("kora:usage_limit:{wallet}"));
    }

    #[tokio::test]
    async fn test_usage_limit_enforcement() {
        let store = Arc::new(InMemoryUsageStore::new());
        let kora_signers = HashSet::new();
        let tracker = UsageTracker::new(store, 2, kora_signers);

        let wallet = Pubkey::new_unique();

        // First transaction should succeed
        assert!(tracker.check_usage_limit(&wallet).await.is_ok());

        // Second transaction should succeed (at limit)
        assert!(tracker.check_usage_limit(&wallet).await.is_ok());

        // Third transaction should fail (over limit)
        let result = tracker.check_usage_limit(&wallet).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeded limit"));
    }

    #[tokio::test]
    async fn test_independent_wallet_limits() {
        let store = Arc::new(InMemoryUsageStore::new());
        let kora_signers = HashSet::new();
        let tracker = UsageTracker::new(store, 2, kora_signers);

        let wallet1 = Pubkey::new_unique();
        let wallet2 = Pubkey::new_unique();

        // Use up wallet1's limit
        assert!(tracker.check_usage_limit(&wallet1).await.is_ok());
        assert!(tracker.check_usage_limit(&wallet1).await.is_ok());
        assert!(tracker.check_usage_limit(&wallet1).await.is_err());

        // Wallet2 should still be able to make transactions
        assert!(tracker.check_usage_limit(&wallet2).await.is_ok());
        assert!(tracker.check_usage_limit(&wallet2).await.is_ok());
        assert!(tracker.check_usage_limit(&wallet2).await.is_err());
    }

    #[tokio::test]
    async fn test_unlimited_usage() {
        let store = Arc::new(InMemoryUsageStore::new());
        let kora_signers = HashSet::new();
        let tracker = UsageTracker::new(store, 0, kora_signers); // 0 = unlimited

        let wallet = Pubkey::new_unique();

        // Should allow many transactions when unlimited
        for _ in 0..10 {
            assert!(tracker.check_usage_limit(&wallet).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_usage_limiter_disabled_fallback() {
        // Test that when usage limiting is disabled, transactions are allowed
        let _m = ConfigMockBuilder::new().with_usage_limit_enabled(false).build_and_setup();

        // Initialize the usage limiter - it should set to None when disabled
        let _ = UsageTracker::init_usage_limiter().await;

        let result = UsageTracker::check_transaction_usage_limit(&create_mock_transaction()).await;
        match &result {
            Ok(_) => {}
            Err(e) => println!("Test failed with error: {e}"),
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_usage_limiter_fallback_allowed() {
        let _m = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(true)
            .build_and_setup();

        // Initialize with no cache_url - should set limiter to None
        let _ = UsageTracker::init_usage_limiter().await;

        let result = UsageTracker::check_transaction_usage_limit(&create_mock_transaction()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_usage_limiter_fallback_denied() {
        let _m = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(false)
            .build_and_setup();

        // Initialize with no cache_url - should set limiter to None
        let _ = UsageTracker::init_usage_limiter().await;

        let result = UsageTracker::check_transaction_usage_limit(&create_mock_transaction()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Usage limiter unavailable and fallback disabled"));
    }
}
