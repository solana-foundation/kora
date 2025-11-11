use deadpool_redis::{Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::sync::OnceCell;

use crate::{error::KoraError, sanitize_error};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

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

        let pool = if CacheUtil::is_cache_enabled(&config) {
            let redis_url = config.kora.cache.url.as_ref().ok_or(KoraError::ConfigError)?;

            let cfg = deadpool_redis::Config::from_url(redis_url);
            let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to create cache pool: {}",
                    sanitize_error!(e)
                ))
            })?;

            // Test connection
            let mut conn = pool.get().await.map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to connect to cache: {}",
                    sanitize_error!(e)
                ))
            })?;

            // Simple connection test - try to get a non-existent key
            let _: Option<String> = conn.get("__connection_test__").await.map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Cache connection test failed: {}",
                    sanitize_error!(e)
                ))
            })?;

            log::info!("Cache initialized successfully");

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
            KoraError::InternalServerError(format!(
                "Failed to get cache connection: {}",
                sanitize_error!(e)
            ))
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
                let kora_error = e.into();
                match kora_error {
                    KoraError::AccountNotFound(_) => {
                        Err(KoraError::AccountNotFound(pubkey.to_string()))
                    }
                    other_error => Err(other_error),
                }
            }
        }
    }

    /// Get data from cache
    async fn get_from_cache(pool: &Pool, key: &str) -> Result<Option<CachedAccount>, KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        let cached_data: Option<String> = conn.get(key).await.map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to get from cache: {}",
                sanitize_error!(e)
            ))
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

    /// Get account from RPC and cache it
    async fn get_account_from_rpc_and_cache(
        rpc_client: &RpcClient,
        pubkey: &Pubkey,
        pool: &Pool,
        ttl: u64,
    ) -> Result<Account, KoraError> {
        let account = Self::get_account_from_rpc(rpc_client, pubkey).await?;

        let cache_key = Self::get_account_key(pubkey);
        let cached_account =
            CachedAccount { account: account.clone(), cached_at: chrono::Utc::now().timestamp() };

        if let Err(e) = Self::set_in_cache(pool, &cache_key, &cached_account, ttl).await {
            log::warn!("Failed to cache account {pubkey}: {e}");
            // Don't fail the request if caching fails
        }

        Ok(account)
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
            KoraError::InternalServerError(format!(
                "Failed to serialize cache data: {}",
                sanitize_error!(e)
            ))
        })?;

        conn.set_ex::<_, _, ()>(key, serialized, ttl_seconds).await.map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to set cache data: {}",
                sanitize_error!(e)
            ))
        })?;

        Ok(())
    }

    /// Check if cache is enabled and available
    fn is_cache_enabled(config: &crate::config::Config) -> bool {
        config.kora.cache.enabled && config.kora.cache.url.is_some()
    }

    /// Get account from cache with optional force refresh
    pub async fn get_account(
        config: &crate::config::Config,
        rpc_client: &RpcClient,
        pubkey: &Pubkey,
        force_refresh: bool,
    ) -> Result<Account, KoraError> {
        // If cache is disabled or force refresh is requested, go directly to RPC
        if !CacheUtil::is_cache_enabled(config) {
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

        if force_refresh {
            return Self::get_account_from_rpc_and_cache(
                rpc_client,
                pubkey,
                pool,
                config.kora.cache.account_ttl,
            )
            .await;
        }

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
        let account = Self::get_account_from_rpc_and_cache(
            rpc_client,
            pubkey,
            pool,
            config.kora.cache.account_ttl,
        )
        .await?;

        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        common::{create_mock_token_account, RpcMockBuilder},
        config_mock::ConfigMockBuilder,
    };

    #[tokio::test]
    async fn test_is_cache_enabled_disabled() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let config = get_config().unwrap();
        assert!(!CacheUtil::is_cache_enabled(&config));
    }

    #[tokio::test]
    async fn test_is_cache_enabled_no_url() {
        let _m = ConfigMockBuilder::new()
            .with_cache_enabled(true)
            .with_cache_url(None) // Explicitly set no URL
            .build_and_setup();

        // Without URL, cache should be disabled
        let config = get_config().unwrap();
        assert!(!CacheUtil::is_cache_enabled(&config));
    }

    #[tokio::test]
    async fn test_is_cache_enabled_with_url() {
        let _m = ConfigMockBuilder::new()
            .with_cache_enabled(true)
            .with_cache_url(Some("redis://localhost:6379".to_string()))
            .build_and_setup();

        // Give time for config to be set up
        let config = get_config().unwrap();
        assert!(CacheUtil::is_cache_enabled(&config));
    }

    #[tokio::test]
    async fn test_get_account_key_format() {
        let pubkey = Pubkey::new_unique();
        let key = CacheUtil::get_account_key(&pubkey);
        assert_eq!(key, format!("account:{pubkey}"));
    }

    #[tokio::test]
    async fn test_get_account_from_rpc_success() {
        let pubkey = Pubkey::new_unique();
        let expected_account = create_mock_token_account(&pubkey, &Pubkey::new_unique());

        let rpc_client = RpcMockBuilder::new().with_account_info(&expected_account).build();

        let result = CacheUtil::get_account_from_rpc(&rpc_client, &pubkey).await;

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.lamports, expected_account.lamports);
        assert_eq!(account.owner, expected_account.owner);
    }

    #[tokio::test]
    async fn test_get_account_from_rpc_error() {
        let pubkey = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().with_account_not_found().build();

        let result = CacheUtil::get_account_from_rpc(&rpc_client, &pubkey).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            KoraError::AccountNotFound(account_key) => {
                assert_eq!(account_key, pubkey.to_string());
            }
            _ => panic!("Expected AccountNotFound for account not found error"),
        }
    }

    #[tokio::test]
    async fn test_get_account_cache_disabled_fallback_to_rpc() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let pubkey = Pubkey::new_unique();
        let expected_account = create_mock_token_account(&pubkey, &Pubkey::new_unique());

        let rpc_client = RpcMockBuilder::new().with_account_info(&expected_account).build();

        let config = get_config().unwrap();
        let result = CacheUtil::get_account(&config, &rpc_client, &pubkey, false).await;

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.lamports, expected_account.lamports);
    }

    #[tokio::test]
    async fn test_get_account_force_refresh_bypasses_cache() {
        let _m = ConfigMockBuilder::new()
            .with_cache_enabled(false) // Force RPC fallback for simplicity
            .build_and_setup();

        let pubkey = Pubkey::new_unique();
        let expected_account = create_mock_token_account(&pubkey, &Pubkey::new_unique());

        let rpc_client = RpcMockBuilder::new().with_account_info(&expected_account).build();

        // force_refresh = true should always go to RPC
        let config = get_config().unwrap();
        let result = CacheUtil::get_account(&config, &rpc_client, &pubkey, true).await;

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.lamports, expected_account.lamports);
    }
}
