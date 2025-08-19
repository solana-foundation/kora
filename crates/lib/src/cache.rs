use deadpool_redis::{Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::sync::OnceCell;

use crate::{error::KoraError, state::get_config};

const ACCOUNT_CACHE_KEY: &str = "account";

/// Global cache pool instance
static CACHE_POOL: OnceCell<Option<Pool>> = OnceCell::const_new();

/// Cached account data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAccount {
    pub account: Account,
    pub cached_at: i64, // Unix timestamp
}

/// Cache utility for Solana RPC calls
pub struct CacheUtil;

impl CacheUtil {
    /// Initialize the cache pool based on configuration
    pub async fn init() -> Result<(), KoraError> {
        let config = get_config()?;

        let pool = if CacheUtil::is_cache_enabled() {
            let redis_url = config.kora.cache.url.as_ref().unwrap();

            let cfg = deadpool_redis::Config::from_url(redis_url);
            let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
                KoraError::InternalServerError(format!("Failed to create cache pool: {e}"))
            })?;

            // Test connection
            let mut conn = pool.get().await.map_err(|e| {
                KoraError::InternalServerError(format!("Failed to connect to cache: {e}"))
            })?;

            // Simple connection test - try to get a non-existent key
            let _: Option<String> = conn.get("__connection_test__").await.map_err(|e| {
                KoraError::InternalServerError(format!("Cache connection test failed: {e}"))
            })?;

            log::info!("Cache initialized successfully with Redis at {redis_url}");

            Some(pool)
        } else {
            log::info!("Cache disabled or no URL configured");
            None
        };

        CACHE_POOL.set(pool).map_err(|_| {
            KoraError::InternalServerError("Cache pool already initialized".to_string())
        })?;

        Ok(())
    }

    async fn get_connection(pool: &Pool) -> Result<deadpool_redis::Connection, KoraError> {
        pool.get().await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get cache connection: {e}"))
        })
    }

    fn get_account_key(pubkey: &Pubkey) -> String {
        format!("{ACCOUNT_CACHE_KEY}:{pubkey}")
    }

    /// Get account directly from RPC (bypassing cache)
    async fn get_account_from_rpc(
        rpc_client: &RpcClient,
        pubkey: &Pubkey,
    ) -> Result<Account, KoraError> {
        match rpc_client.get_account(pubkey).await {
            Ok(account) => Ok(account),
            Err(e) => {
                Err(KoraError::InternalServerError(format!("Failed to get account {pubkey}: {e}")))
            }
        }
    }

    /// Get data from cache
    async fn get_from_cache(pool: &Pool, key: &str) -> Result<Option<CachedAccount>, KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        let cached_data: Option<String> = conn.get(key).await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get from cache: {e}"))
        })?;

        match cached_data {
            Some(data) => {
                let cached_account: CachedAccount = serde_json::from_str(&data).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Failed to deserialize cached data: {e}"
                    ))
                })?;
                Ok(Some(cached_account))
            }
            None => Ok(None),
        }
    }

    /// Set data in cache with TTL
    async fn set_in_cache(
        pool: &Pool,
        key: &str,
        data: &CachedAccount,
        ttl_seconds: u64,
    ) -> Result<(), KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        let serialized = serde_json::to_string(data).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to serialize cache data: {e}"))
        })?;

        conn.set_ex::<_, _, ()>(key, serialized, ttl_seconds).await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to set cache data: {e}"))
        })?;

        Ok(())
    }

    /// Get account from cache with optional force refresh
    pub async fn get_account_from_cache(
        rpc_client: &RpcClient,
        pubkey: &Pubkey,
        force_refresh: bool,
    ) -> Result<Account, KoraError> {
        let config = get_config()?;

        // If cache is disabled or force refresh is requested, go directly to RPC
        if !CacheUtil::is_cache_enabled() || force_refresh {
            return Self::get_account_from_rpc(rpc_client, pubkey).await;
        }

        // Get cache pool - if not initialized, fallback to RPC
        let pool = match CACHE_POOL.get() {
            Some(pool) => pool,
            None => {
                // Cache not initialized, fallback to RPC
                return Self::get_account_from_rpc(rpc_client, pubkey).await;
            }
        };

        let pool = match pool {
            Some(pool) => pool,
            None => {
                // Cache disabled, fallback to RPC
                return Self::get_account_from_rpc(rpc_client, pubkey).await;
            }
        };

        let cache_key = Self::get_account_key(pubkey);

        // Try to get from cache first
        if let Ok(Some(cached_account)) = Self::get_from_cache(pool, &cache_key).await {
            let current_time = chrono::Utc::now().timestamp();
            let cache_age = current_time - cached_account.cached_at;

            // Check if cache is still valid
            if cache_age < config.kora.cache.account_ttl as i64 {
                return Ok(cached_account.account);
            }
        }

        // Cache miss or expired, fetch from RPC
        let account = Self::get_account_from_rpc(rpc_client, pubkey).await?;

        // Cache the result if we got an account
        let cached_account =
            CachedAccount { account: account.clone(), cached_at: chrono::Utc::now().timestamp() };

        if let Err(e) =
            Self::set_in_cache(pool, &cache_key, &cached_account, config.kora.cache.account_ttl)
                .await
        {
            log::warn!("Failed to cache account {pubkey}: {e}");
            // Don't fail the request if caching fails
        }

        Ok(account)
    }

    /// Clear cache for a specific account
    pub async fn invalidate_account_cache(pubkey: &Pubkey) -> Result<(), KoraError> {
        let pool = match CACHE_POOL.get() {
            Some(pool) => pool,
            None => return Ok(()), // Cache not initialized, nothing to invalidate
        };

        let pool = match pool {
            Some(pool) => pool,
            None => return Ok(()), // Cache disabled, nothing to invalidate
        };

        let mut conn = Self::get_connection(pool).await?;

        let cache_key = Self::get_account_key(pubkey);
        let _: u32 = conn.del(&cache_key).await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to invalidate cache: {e}"))
        })?;

        Ok(())
    }

    /// Check if cache is enabled and available
    pub fn is_cache_enabled() -> bool {
        match get_config() {
            Ok(config) => config.kora.cache.enabled && config.kora.cache.url.is_some(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, KoraConfig, ValidationConfig},
        fee::price::PriceConfig,
        oracle::PriceSource,
        state::update_config,
    };
    use serial_test::serial;

    fn create_test_config(cache_enabled: bool, cache_url: Option<String>) -> Config {
        Config {
            validation: ValidationConfig {
                max_allowed_lamports: 1000000,
                max_signatures: 10,
                allowed_programs: vec![],
                allowed_tokens: vec![],
                allowed_spl_paid_tokens: vec![],
                disallowed_accounts: vec![],
                price_source: PriceSource::Mock,
                fee_payer_policy: crate::config::FeePayerPolicy::default(),
                price: PriceConfig::default(),
                token_2022: crate::config::Token2022Config::default(),
            },
            kora: KoraConfig {
                rate_limit: 100,
                enabled_methods: crate::config::EnabledMethods::default(),
                auth: crate::config::AuthConfig::default(),
                payment_address: None,
                cache: crate::config::CacheConfig {
                    url: cache_url,
                    enabled: cache_enabled,
                    default_ttl: 300,
                    account_ttl: 60,
                },
            },
            metrics: crate::config::MetricsConfig::default(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_cache_disabled() {
        let config = create_test_config(false, None);
        let _ = update_config(config);

        // Cache should report as disabled
        assert!(!CacheUtil::is_cache_enabled());
    }

    #[tokio::test]
    #[serial]
    async fn test_cache_enabled_no_url() {
        let config = create_test_config(true, None);
        let _ = update_config(config);

        // Cache should report as disabled when no URL is provided
        assert!(!CacheUtil::is_cache_enabled());
    }

    #[tokio::test]
    #[serial]
    async fn test_cache_enabled_with_url() {
        let config = create_test_config(true, Some("redis://localhost:6379".to_string()));
        let _ = update_config(config);

        // Cache should report as enabled
        assert!(CacheUtil::is_cache_enabled());
    }
}
