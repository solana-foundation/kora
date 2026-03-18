use std::{collections::HashSet, sync::Arc, time::SystemTime};

use super::{
    limiter::{LimiterContext, LimiterResult},
    rules::{InstructionRule, UsageRule},
    usage_store::{InMemoryUsageStore, RedisUsageStore},
    UsageStore,
};
use crate::{
    cache::CacheUtil,
    config::Config,
    error::KoraError,
    sanitize_error,
    state::get_signer_pool,
    token::token::TokenType,
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
};
use deadpool_redis::Runtime;
use redis::AsyncCommands;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::OnceCell;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

/// Global usage limiter instance
static USAGE_LIMITER: OnceCell<Option<UsageTracker>> = OnceCell::const_new();

pub struct UsageTracker {
    enabled: bool,
    store: Arc<dyn UsageStore>,
    rules: Vec<UsageRule>,
    instruction_rule_indices: Vec<usize>,
    kora_signers: HashSet<Pubkey>,
    fallback_if_unavailable: bool,
}

impl UsageTracker {
    pub fn new(
        enabled: bool,
        store: Arc<dyn UsageStore>,
        rules: Vec<UsageRule>,
        kora_signers: HashSet<Pubkey>,
        fallback_if_unavailable: bool,
    ) -> Self {
        // Pre-compute instruction rule indices at initialization
        let instruction_rule_indices: Vec<usize> =
            rules
                .iter()
                .enumerate()
                .filter_map(|(idx, rule)| {
                    if matches!(rule, UsageRule::Instruction(_)) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();

        Self {
            enabled,
            store,
            rules,
            instruction_rule_indices,
            kora_signers,
            fallback_if_unavailable,
        }
    }

    fn get_usage_limiter() -> Result<Option<&'static UsageTracker>, KoraError> {
        match USAGE_LIMITER.get() {
            Some(limiter) => Ok(limiter.as_ref()),
            None => {
                Err(KoraError::InternalServerError("Usage limiter not initialized".to_string()))
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled && !self.rules.is_empty()
    }

    fn has_instruction_rules(&self) -> bool {
        !self.instruction_rule_indices.is_empty()
    }

    async fn extract_user_from_payment_instruction(
        &self,
        transaction: &mut VersionedTransactionResolved,
        config: &Config,
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<Option<Pubkey>, KoraError> {
        let payment_destination = config.kora.get_payment_address(fee_payer)?;
        let parsed_spl_instructions = transaction.get_or_parse_spl_instructions()?;

        for instruction in parsed_spl_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                destination_address, owner, ..
            } = instruction
            {
                // Check if this is a payment to Kora by verifying the destination token account owner
                // matches the payment destination
                let destination_account =
                    match CacheUtil::get_account(config, rpc_client, destination_address, true)
                        .await
                    {
                        Ok(account) => account,
                        Err(KoraError::AccountNotFound(_)) => continue,
                        Err(e) => return Err(e),
                    };

                let token_program =
                    TokenType::get_token_program_from_owner(&destination_account.owner)?;
                let token_account =
                    token_program.unpack_token_account(&destination_account.data)?;

                // Check if this is a payment to Kora
                if token_account.owner() == payment_destination {
                    return Ok(Some(*owner));
                }
            }
        }

        Ok(None)
    }

    /// Extract kora signer from transaction signers
    fn extract_kora_signer(&self, transaction: &VersionedTransactionResolved) -> Option<Pubkey> {
        let account_keys = transaction.message.static_account_keys();
        let num_signers = transaction.message.header().num_required_signatures as usize;

        account_keys
            .iter()
            .take(num_signers)
            .find(|signer| self.kora_signers.contains(signer))
            .copied()
    }

    fn current_timestamp() -> u64 {
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    /// Check and record usage for a transaction
    /// Uses two-phase commit: check all rules first, then increment only if all pass
    async fn check_and_record(
        &self,
        ctx: &mut LimiterContext<'_>,
    ) -> Result<LimiterResult, KoraError> {
        if !self.is_enabled() {
            return Ok(LimiterResult::Allowed);
        }

        // Extract instruction rules using pre-computed indices (no per-request separation)
        let instruction_rules: Vec<&InstructionRule> = self
            .instruction_rule_indices
            .iter()
            .filter_map(|&idx| self.rules[idx].as_instruction())
            .collect();

        // Batch count instruction rules in single pass
        let instruction_counts = if !instruction_rules.is_empty() {
            InstructionRule::count_all_rules(&instruction_rules, ctx)
        } else {
            Vec::new()
        };

        // Build HashSet for O(1) lookup instead of Vec::contains O(n)
        let ix_idx_set: HashSet<usize> = self.instruction_rule_indices.iter().copied().collect();

        // Phase 1: Check all rules first (no incrementing yet)
        // Collect rule checks: (key, increment_count, window_seconds)
        let mut pending_increments: Vec<(String, u64, Option<u64>)> = Vec::new();
        let mut instruction_count_idx = 0;

        for (idx, rule) in self.rules.iter().enumerate() {
            let increment_count = if ix_idx_set.contains(&idx) {
                // Use pre-computed count for instruction rule
                let count = instruction_counts[instruction_count_idx];
                instruction_count_idx += 1;
                count
            } else {
                // Transaction rules always increment by 1
                1
            };

            if increment_count == 0 {
                continue;
            }

            let key = rule.storage_key(&ctx.user_id, ctx.timestamp);

            let current = self.store.get(&key).await?;
            let new_count = current as u64 + increment_count;

            if new_count > rule.max() {
                return Ok(LimiterResult::Denied {
                    reason: format!(
                        "User {} exceeded {} limit: {}/{}",
                        ctx.user_id,
                        rule.description(),
                        new_count,
                        rule.max()
                    ),
                });
            }

            // Queue for increment (don't increment yet)
            pending_increments.push((key, increment_count, rule.window_seconds()));

            log::debug!(
                "[rule] User {} {}: {}/{} ({})",
                ctx.user_id,
                rule.description(),
                new_count,
                rule.max(),
                rule.window_seconds().map_or("lifetime".to_string(), |w| format!("{}s window", w))
            );
        }

        for (key, increment_count, window_seconds) in pending_increments {
            if let Some(window) = window_seconds.filter(|&w| w > 0) {
                // Calculate bucket boundary: key expires at end of current bucket
                // bucket = timestamp / window, so bucket_end = (bucket + 1) * window
                let expires_at = (ctx.timestamp / window + 1) * window;
                // First increment with expiry
                self.store.increment_with_expiry(&key, expires_at).await?;
                // Subsequent increments without resetting expiry
                for _ in 1..increment_count {
                    self.store.increment(&key).await?;
                }
            } else {
                for _ in 0..increment_count {
                    self.store.increment(&key).await?;
                }
            }
        }

        Ok(LimiterResult::Allowed)
    }

    pub async fn init_usage_limiter() -> Result<(), KoraError> {
        let config = get_config()?;
        let usage_config = &config.kora.usage_limit;

        let set_limiter = |limiter| {
            USAGE_LIMITER.set(limiter).map_err(|_| {
                KoraError::InternalServerError("Usage limiter already initialized".to_string())
            })
        };

        if !usage_config.enabled {
            log::info!("Usage limiting disabled");
            return set_limiter(None);
        }

        let rules = usage_config.build_rules()?;
        if rules.is_empty() {
            log::info!("Usage limiting enabled but no rules configured - disabled");
            return set_limiter(None);
        }

        let kora_signers = get_signer_pool()?
            .get_signers_info()
            .iter()
            .filter_map(|info| info.public_key.parse().ok())
            .collect();

        let (store, backend): (Arc<dyn UsageStore>, &str) =
            if let Some(cache_url) = &usage_config.cache_url {
                let cfg = deadpool_redis::Config::from_url(cache_url);
                let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Failed to create Redis pool: {}",
                        sanitize_error!(e)
                    ))
                })?;

                let mut conn = pool.get().await.map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Failed to connect to Redis: {}",
                        sanitize_error!(e)
                    ))
                })?;

                let _: Option<String> = conn.get("__usage_limiter_test__").await.map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Redis connection test failed: {}",
                        sanitize_error!(e)
                    ))
                })?;

                (Arc::new(RedisUsageStore::new(pool)), "Redis")
            } else {
                log::warn!(
                    "Usage limiting configured with in-memory store. \
                     Limits will NOT be shared across instances and will reset on restart. \
                     Configure 'cache_url' in [kora.usage_limit] for production deployments."
                );
                (Arc::new(InMemoryUsageStore::new()), "in-memory")
            };

        log::info!("Usage limiting initialized with {} rules ({backend})", rules.len());

        set_limiter(Some(UsageTracker::new(
            usage_config.enabled,
            store,
            rules,
            kora_signers,
            usage_config.fallback_if_unavailable,
        )))
    }

    pub async fn check_transaction_usage_limit(
        config: &Config,
        transaction: &mut VersionedTransactionResolved,
        user_id: Option<&str>,
        fee_payer: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        // Validate user_id is provided when required
        if config.kora.usage_limit.enabled
            && matches!(&config.validation.price.model, crate::fee::price::PriceModel::Free)
            && user_id.is_none()
        {
            return Err(KoraError::ValidationError(
                "user_id is required when usage tracking is enabled and pricing is free"
                    .to_string(),
            ));
        }

        let Some(tracker) = Self::get_usage_limiter()? else {
            if config.kora.usage_limit.enabled && !config.kora.usage_limit.fallback_if_unavailable {
                return Err(KoraError::InternalServerError(
                    "Usage limiter unavailable and fallback disabled".to_string(),
                ));
            }
            return Ok(());
        };

        if tracker.has_instruction_rules() {
            transaction.get_or_parse_system_instructions()?;
            transaction.get_or_parse_spl_instructions()?;
        }

        // Resolve user_id for usage tracking:
        // - If user_id is provided, use it directly (works for both free and paid modes)
        // - Otherwise (paid mode), extract payer from payment instruction
        let resolved_user_id = if let Some(user_id_str) = user_id {
            user_id_str.to_string()
        } else {
            // Paid mode: extract payer from payment instruction
            tracker
                .extract_user_from_payment_instruction(transaction, config, fee_payer, rpc_client)
                .await?
                .ok_or_else(|| {
                    KoraError::ValidationError(
                        "Could not resolve user_id: no payment instruction found".to_string(),
                    )
                })?
                .to_string()
        };

        let kora_signer = tracker.extract_kora_signer(transaction);

        let mut ctx = LimiterContext {
            transaction,
            user_id: resolved_user_id,
            kora_signer,
            timestamp: Self::current_timestamp(),
        };

        match tracker.check_and_record(&mut ctx).await {
            Ok(LimiterResult::Allowed) => Ok(()),
            Ok(LimiterResult::Denied { reason }) => Err(KoraError::UsageLimitExceeded(reason)),
            Err(e)
                if tracker.fallback_if_unavailable
                    && matches!(e, KoraError::InternalServerError(_)) =>
            {
                log::warn!("Usage limiter error (fallback enabled): {e}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tests::{
            config_mock::ConfigMockBuilder, rpc_mock::RpcMockBuilder,
            transaction_mock::create_mock_resolved_transaction,
        },
        usage_limit::{InMemoryUsageStore, UsageLimitConfig, UsageLimitRuleConfig},
    };
    use std::sync::Arc;

    fn create_test_tracker(max_transactions: u64) -> UsageTracker {
        let store = Arc::new(InMemoryUsageStore::new());
        let config = UsageLimitConfig {
            enabled: true,
            cache_url: None,
            fallback_if_unavailable: false,
            rules: vec![UsageLimitRuleConfig::Transaction {
                max: max_transactions,
                window_seconds: None,
            }],
        };
        let rules = config.build_rules().unwrap();
        UsageTracker::new(true, store, rules, HashSet::new(), false)
    }

    #[tokio::test]
    async fn test_usage_limit_enforcement() {
        let tracker = create_test_tracker(2);
        let user_id = "test-user-enforcement".to_string();

        let mut tx1 = create_mock_resolved_transaction();
        let mut ctx1 = LimiterContext {
            transaction: &mut tx1,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };

        let mut tx2 = create_mock_resolved_transaction();
        let mut ctx2 = LimiterContext {
            transaction: &mut tx2,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };

        let mut tx3 = create_mock_resolved_transaction();
        let mut ctx3 = LimiterContext {
            transaction: &mut tx3,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };

        // First transaction should succeed
        assert!(matches!(
            tracker.check_and_record(&mut ctx1).await.unwrap(),
            LimiterResult::Allowed
        ));

        // Second transaction should succeed (at limit)
        assert!(matches!(
            tracker.check_and_record(&mut ctx2).await.unwrap(),
            LimiterResult::Allowed
        ));

        // Third transaction should fail (over limit)
        assert!(matches!(
            tracker.check_and_record(&mut ctx3).await.unwrap(),
            LimiterResult::Denied { .. }
        ));
    }

    #[tokio::test]
    async fn test_independent_user_limits() {
        let tracker = create_test_tracker(2);

        let user_id1 = "test-user-1".to_string();
        let user_id2 = "test-user-2".to_string();

        // Use up user1's limit
        let mut tx1a = create_mock_resolved_transaction();
        let mut ctx1a = LimiterContext {
            transaction: &mut tx1a,
            user_id: user_id1.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx1a).await.unwrap(),
            LimiterResult::Allowed
        ));
        let mut tx1b = create_mock_resolved_transaction();
        let mut ctx1b = LimiterContext {
            transaction: &mut tx1b,
            user_id: user_id1.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx1b).await.unwrap(),
            LimiterResult::Allowed
        ));
        let mut tx1c = create_mock_resolved_transaction();
        let mut ctx1c = LimiterContext {
            transaction: &mut tx1c,
            user_id: user_id1.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx1c).await.unwrap(),
            LimiterResult::Denied { .. }
        ));

        // User2 should still be able to make transactions
        let mut tx2a = create_mock_resolved_transaction();
        let mut ctx2a = LimiterContext {
            transaction: &mut tx2a,
            user_id: user_id2.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx2a).await.unwrap(),
            LimiterResult::Allowed
        ));
        let mut tx2b = create_mock_resolved_transaction();
        let mut ctx2b = LimiterContext {
            transaction: &mut tx2b,
            user_id: user_id2.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx2b).await.unwrap(),
            LimiterResult::Allowed
        ));
        let mut tx2c = create_mock_resolved_transaction();
        let mut ctx2c = LimiterContext {
            transaction: &mut tx2c,
            user_id: user_id2.clone(),
            kora_signer: None,
            timestamp: 1000000,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx2c).await.unwrap(),
            LimiterResult::Denied { .. }
        ));
    }

    #[tokio::test]
    async fn test_unlimited_usage() {
        let store = Arc::new(InMemoryUsageStore::new());
        let config = UsageLimitConfig {
            enabled: true,
            cache_url: None,
            fallback_if_unavailable: false,
            rules: vec![], // No rules = unlimited
        };
        let rules = config.build_rules().unwrap();
        let tracker = UsageTracker::new(true, store, rules, HashSet::new(), false);

        let user_id = "test-user-unlimited".to_string();

        // Should allow many transactions when no rules (limiter is not enabled)
        for _ in 0..10 {
            let mut tx = create_mock_resolved_transaction();
            let mut ctx = LimiterContext {
                transaction: &mut tx,
                user_id: user_id.clone(),
                kora_signer: None,
                timestamp: 1000000,
            };
            assert!(matches!(
                tracker.check_and_record(&mut ctx).await.unwrap(),
                LimiterResult::Allowed
            ));
        }
    }

    #[tokio::test]
    async fn test_multiple_rules() {
        let store: Arc<dyn UsageStore> = Arc::new(InMemoryUsageStore::new());

        let config = UsageLimitConfig {
            enabled: true,
            cache_url: None,
            fallback_if_unavailable: false,
            rules: vec![
                // Lifetime limit: 10 transactions
                UsageLimitRuleConfig::Transaction { max: 10, window_seconds: None },
                // Time bucket limit: 2 per 100 seconds
                UsageLimitRuleConfig::Transaction { max: 2, window_seconds: Some(100) },
            ],
        };

        let rules = config.build_rules().unwrap();
        let tracker = UsageTracker::new(true, store, rules, HashSet::new(), false);

        let user_id = "test-user-multiple-rules".to_string();
        // Use realistic timestamp (current time) so expiry calculations work correctly
        let now = UsageTracker::current_timestamp();

        // First two should pass (time bucket limit is 2)
        let mut tx1 = create_mock_resolved_transaction();
        let mut ctx1 = LimiterContext {
            transaction: &mut tx1,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: now,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx1).await.unwrap(),
            LimiterResult::Allowed
        ));
        let mut tx2 = create_mock_resolved_transaction();
        let mut ctx2 = LimiterContext {
            transaction: &mut tx2,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: now,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx2).await.unwrap(),
            LimiterResult::Allowed
        ));

        // Third should fail (time bucket limit exceeded)
        let mut tx3 = create_mock_resolved_transaction();
        let mut ctx3 = LimiterContext {
            transaction: &mut tx3,
            user_id: user_id.clone(),
            kora_signer: None,
            timestamp: now,
        };
        assert!(matches!(
            tracker.check_and_record(&mut ctx3).await.unwrap(),
            LimiterResult::Denied { .. }
        ));
    }

    #[tokio::test]
    async fn test_usage_limiter_disabled_fallback() {
        // Test that when usage limiting is disabled, transactions are allowed
        let _m = ConfigMockBuilder::new().with_usage_limit_enabled(false).build_and_setup();

        // Initialize the usage limiter - it should set to None when disabled
        let _ = UsageTracker::init_usage_limiter().await;

        let config = get_config().unwrap();
        let mut tx = create_mock_resolved_transaction();
        let rpc_client = Arc::new(RpcMockBuilder::new().build());
        let fee_payer = Pubkey::new_unique();
        let result = UsageTracker::check_transaction_usage_limit(
            &config,
            &mut tx,
            None,
            &fee_payer,
            &rpc_client,
        )
        .await;
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

        // Initialize with no cache_url - should use in-memory store but no rules = limiter disabled
        let _ = UsageTracker::init_usage_limiter().await;

        let config = get_config().unwrap();
        let mut tx = create_mock_resolved_transaction();
        let rpc_client = Arc::new(RpcMockBuilder::new().build());
        let fee_payer = Pubkey::new_unique();
        let result = UsageTracker::check_transaction_usage_limit(
            &config,
            &mut tx,
            None,
            &fee_payer,
            &rpc_client,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_usage_limiter_fallback_denied() {
        let _m = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(false)
            .build_and_setup();

        // Initialize with no cache_url and no rules - should set limiter to None
        let _ = UsageTracker::init_usage_limiter().await;

        let config = get_config().unwrap();
        let mut tx = create_mock_resolved_transaction();
        let rpc_client = Arc::new(RpcMockBuilder::new().build());
        let fee_payer = Pubkey::new_unique();
        let result = UsageTracker::check_transaction_usage_limit(
            &config,
            &mut tx,
            None,
            &fee_payer,
            &rpc_client,
        )
        .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Usage limiter unavailable and fallback disabled"));
    }
}
