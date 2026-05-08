use deadpool_redis::{Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{account::Account, hash::Hash, pubkey::Pubkey};
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{Mutex, OnceCell};

use crate::{
    config::Config,
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
    sanitize_error,
};

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use crate::tests::config_mock::mock_state::get_config;

const ACCOUNT_CACHE_KEY: &str = "account";
const BLOCKHASH_CACHE_KEY: &str = "kora:blockhash";
const PRICE_CACHE_KEY_PREFIX: &str = "kora:price";
/// TTL for cached blockhash in seconds. Blockhashes are valid for ~60s,
/// but we use a short TTL to keep the hash fresh.
const BLOCKHASH_TTL: u64 = 5;
/// Number of retries for the underlying price oracle on cache misses.
const PRICE_ORACLE_MAX_RETRIES: u32 = 3;
/// Base delay for the price oracle retry loop.
const PRICE_ORACLE_BASE_DELAY: Duration = Duration::from_secs(1);

/// Global cache pool instance
static CACHE_POOL: OnceCell<Option<Pool>> = OnceCell::const_new();

/// Process-wide price oracle. Held as an `Arc` so the inner `reqwest::Client`
/// (and its connection pool) is reused across all cache misses instead of
/// being rebuilt per request.
static PRICE_ORACLE: OnceCell<(PriceSource, Arc<RetryingPriceOracle>)> = OnceCell::const_new();

/// Per-mint locks used to coalesce concurrent oracle fetches for the same
/// price key (singleflight). Without this, a TTL expiry under load triggers
/// N parallel Jupiter requests for the same mint instead of one.
///
/// The map grows by one entry per distinct mint ever queried. In practice the
/// set is bounded by `allowed_tokens` / `allowed_spl_paid_tokens`, so we
/// don't bother evicting entries.
static PRICE_FETCH_LOCKS: OnceCell<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceCell::const_new();

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

        #[allow(clippy::needless_borrow)]
        let pool = if CacheUtil::is_cache_enabled(&config) {
            let redis_url = config.kora.cache.resolved_url().ok_or(KoraError::ConfigError(
                "Redis URL is required when cache is enabled. Set the url in config or the \
                 KORA_REDIS_URL environment variable."
                    .to_string(),
            ))?;

            let cfg = deadpool_redis::Config::from_url(&redis_url);
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
    fn is_cache_enabled(config: &Config) -> bool {
        config.kora.cache.enabled && config.kora.cache.resolved_url().is_some()
    }

    /// Get account from cache with optional force refresh
    pub async fn get_account(
        config: &Config,
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

    /// Get the latest blockhash, using Redis cache when available.
    ///
    /// Reduces RPC load by caching the blockhash with a short TTL (5s).
    /// If the cache is unavailable or errors occur, falls back to a direct RPC call.
    pub async fn get_or_fetch_latest_blockhash(
        config: &Config,
        rpc_client: &RpcClient,
    ) -> Result<Hash, KoraError> {
        // If cache is disabled, fetch directly from RPC
        if !CacheUtil::is_cache_enabled(config) {
            return Self::fetch_blockhash_from_rpc(rpc_client).await;
        }

        // Get cache pool - if not initialized, fallback to RPC
        let pool = match CACHE_POOL.get() {
            Some(Some(pool)) => pool,
            _ => return Self::fetch_blockhash_from_rpc(rpc_client).await,
        };

        // Try to get from cache first
        match Self::get_blockhash_from_cache(pool).await {
            Ok(Some(hash)) => return Ok(hash),
            Ok(None) => { /* cache miss, fetch from RPC */ }
            Err(e) => {
                log::warn!("Failed to get blockhash from cache, falling back to RPC: {e}");
            }
        }

        // Cache miss or error — fetch from RPC and cache it
        let hash = Self::fetch_blockhash_from_rpc(rpc_client).await?;

        if let Err(e) = Self::set_blockhash_in_cache(pool, &hash).await {
            log::warn!("Failed to cache blockhash: {e}");
            // Don't fail the request if caching fails
        }

        Ok(hash)
    }

    /// Fetch the latest blockhash directly from the Solana RPC.
    async fn fetch_blockhash_from_rpc(rpc_client: &RpcClient) -> Result<Hash, KoraError> {
        let (blockhash, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await
            .map_err(|e| KoraError::RpcError(e.to_string()))?;
        Ok(blockhash)
    }

    /// Try to read a cached blockhash from Redis.
    async fn get_blockhash_from_cache(pool: &Pool) -> Result<Option<Hash>, KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        let cached: Option<String> = conn.get(BLOCKHASH_CACHE_KEY).await.map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to get blockhash from cache: {}",
                sanitize_error!(e)
            ))
        })?;

        match cached {
            Some(s) => {
                let hash = Hash::from_str(&s).map_err(|e| {
                    KoraError::InternalServerError(format!("Failed to parse cached blockhash: {e}"))
                })?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Store a blockhash in Redis with TTL.
    async fn set_blockhash_in_cache(pool: &Pool, hash: &Hash) -> Result<(), KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        conn.set_ex::<_, _, ()>(BLOCKHASH_CACHE_KEY, hash.to_string(), BLOCKHASH_TTL)
            .await
            .map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to set blockhash in cache: {}",
                    sanitize_error!(e)
                ))
            })?;

        Ok(())
    }

    fn get_price_key(mint_address: &str) -> String {
        format!("{PRICE_CACHE_KEY_PREFIX}:{mint_address}")
    }

    /// Return the process-wide `RetryingPriceOracle`, building it on first use.
    ///
    /// Initialization is fallible (Jupiter requires `JUPITER_API_KEY`); on
    /// failure the cell is left empty so a later call can retry.
    async fn get_price_oracle_singleton(
        config: &Config,
    ) -> Result<Arc<RetryingPriceOracle>, KoraError> {
        let requested = &config.validation.price_source;
        let (initialized, oracle) = PRICE_ORACLE
            .get_or_try_init(|| async {
                let source = config.validation.price_source.clone();
                let inner = get_price_oracle(source.clone())?;
                Ok::<_, KoraError>((
                    source,
                    Arc::new(RetryingPriceOracle::new(
                        PRICE_ORACLE_MAX_RETRIES,
                        PRICE_ORACLE_BASE_DELAY,
                        inner,
                    )),
                ))
            })
            .await?;
        if initialized != requested {
            log::warn!(
                "Price oracle already initialized with {:?}; ignoring request for {:?}. \
                 The price_source is fixed for the lifetime of the process.",
                initialized,
                requested
            );
        }
        Ok(oracle.clone())
    }

    /// Get (or insert) the lock that serializes oracle fetches for `mint`.
    async fn get_price_fetch_lock(mint: &str) -> Arc<Mutex<()>> {
        let map_cell = PRICE_FETCH_LOCKS.get_or_init(|| async { Mutex::new(HashMap::new()) }).await;
        let mut map = map_cell.lock().await;
        map.entry(mint.to_string()).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
    }

    /// Read cached prices for the given mints from Redis. Mints with no entry
    /// (or whose entry fails to deserialize) are returned in `misses`.
    async fn get_prices_from_cache(
        pool: &Pool,
        mint_addresses: &[String],
    ) -> Result<(HashMap<String, TokenPrice>, Vec<String>), KoraError> {
        let mut conn = Self::get_connection(pool).await?;

        let keys: Vec<String> = mint_addresses.iter().map(|m| Self::get_price_key(m)).collect();

        // MGET returns Vec<Option<String>> aligned with the input keys.
        let raw: Vec<Option<String>> = conn.mget(&keys).await.map_err(|e| {
            KoraError::InternalServerError(format!(
                "Failed to get prices from cache: {}",
                sanitize_error!(e)
            ))
        })?;

        let mut hits = HashMap::new();
        let mut misses = Vec::new();

        for (mint, value) in mint_addresses.iter().zip(raw) {
            match value.as_deref().map(serde_json::from_str::<TokenPrice>) {
                Some(Ok(price)) => {
                    hits.insert(mint.clone(), price);
                }
                Some(Err(e)) => {
                    log::warn!("Failed to deserialize cached price for {mint}: {e}");
                    misses.push(mint.clone());
                }
                None => misses.push(mint.clone()),
            }
        }

        Ok((hits, misses))
    }

    /// Write fetched prices back to Redis with `price_ttl`.
    async fn set_prices_in_cache(
        pool: &Pool,
        prices: &HashMap<String, TokenPrice>,
    async fn set_prices_in_cache(
        pool: &Pool,
        prices: &HashMap<String, TokenPrice>,
        ttl: u64,
    ) -> Result<(), KoraError> {
        if prices.is_empty() {
            return Ok(());
        }

        let mut conn = Self::get_connection(pool).await?;
        let mut pipe = redis::pipe();

        for (mint, price) in prices {
            let serialized = serde_json::to_string(price).map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Failed to serialize price for {mint}: {}",
                    sanitize_error!(e)
                ))
            })?;
            let key = Self::get_price_key(mint);
            pipe.set_ex(&key, serialized, ttl);
        }

        if let Err(e) = pipe.query_async::<()>(&mut conn).await {
            log::warn!("Failed to cache prices in batch: {}", sanitize_error!(e));
        }

        Ok(())
    }
    /// On cache miss, fetches only the missing mints from the configured price
    /// oracle (Jupiter or Mock) and writes the results back with `price_ttl`.
    /// Falls back to a direct oracle call when caching is disabled or unreachable.
    /// Setting `price_ttl = 0` disables price caching independently of the
    /// global cache switch (so account caching can stay on).
    pub async fn get_or_fetch_token_prices(
        config: &Config,
        mint_addresses: &[String],
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        // If cache is disabled globally, pool not initialized, or price caching
        // is opted out via `price_ttl = 0`, go straight to the oracle.
        if !Self::is_cache_enabled(config) || config.kora.cache.price_ttl == 0 {
            return Self::get_price_oracle_singleton(config)
                .await?
                .get_token_prices(mint_addresses)
                .await;
        }

        let pool = match CACHE_POOL.get() {
            Some(Some(pool)) => pool,
            _ => {
                return Self::get_price_oracle_singleton(config)
                    .await?
                    .get_token_prices(mint_addresses)
                    .await;
            }
        };

        // Try cache first; on errors, fall through to a full live fetch.
        let (mut hits, misses) = match Self::get_prices_from_cache(pool, mint_addresses).await {
            Ok(result) => result,
            Err(e) => {
                log::warn!("Failed to read prices from cache, falling back to oracle: {e}");
                return Self::get_price_oracle_singleton(config)
                    .await?
                    .get_token_prices(mint_addresses)
                    .await;
            }
        };

        if misses.is_empty() {
            return Ok(hits);
        }

        let fetched = Self::fetch_misses_with_singleflight(config, pool, misses).await?;
        hits.extend(fetched);
        Ok(hits)
    }

    /// Fetch the given mints from the oracle while coalescing concurrent
    /// requests for the same key.
    ///
    /// Per-mint locks are acquired in sorted order so concurrent batch
    /// requests with overlapping mint sets queue deterministically rather
    /// than deadlocking. After the locks are held, the cache is re-read so
    /// requests that lost the race read whatever the leader just wrote.
    async fn fetch_misses_with_singleflight(
        config: &Config,
        pool: &Pool,
        misses: Vec<String>,
    ) -> Result<HashMap<String, TokenPrice>, KoraError> {
        let mut sorted_misses = misses;
        sorted_misses.sort();
        sorted_misses.dedup();

        let mut guards = Vec::with_capacity(sorted_misses.len());
        for mint in &sorted_misses {
            let lock = Self::get_price_fetch_lock(mint).await;
            guards.push(lock.lock_owned().await);
        }

        // Re-read the cache while holding the locks: prior leaders may have
        // populated some keys while we waited.
        let (mut hits, still_missing) =
            match Self::get_prices_from_cache(pool, &sorted_misses).await {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Failed to re-read prices from cache after lock acquisition: {e}");
                    (HashMap::new(), sorted_misses)
                }
            };

        if !still_missing.is_empty() {
            let oracle = Self::get_price_oracle_singleton(config).await?;
            let fetched = oracle.get_token_prices(&still_missing).await?;
            if let Err(e) =
                Self::set_prices_in_cache(pool, &fetched, config.kora.cache.price_ttl).await
            {
                log::warn!("Failed to cache fetched prices: {e}");
            }
            hits.extend(fetched);
        }

        drop(guards);
        Ok(hits)
    }

    /// Get a single token price, using Redis cache when available.
    pub async fn get_or_fetch_token_price(
        config: &Config,
        mint_address: &str,
    ) -> Result<TokenPrice, KoraError> {
        let prices = Self::get_or_fetch_token_prices(config, &[mint_address.to_string()]).await?;

        prices.get(mint_address).cloned().ok_or_else(|| {
            KoraError::InternalServerError(format!(
                "Failed to fetch token price for {mint_address}"
            ))
        })
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
    async fn test_get_price_key_format() {
        let mint = Pubkey::new_unique().to_string();
        let key = CacheUtil::get_price_key(&mint);
        assert_eq!(key, format!("kora:price:{mint}"));
    }

    #[tokio::test]
    async fn test_get_or_fetch_token_prices_empty_returns_empty() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let config = get_config().unwrap();

        let prices = CacheUtil::get_or_fetch_token_prices(&config, &[]).await.unwrap();
        assert!(prices.is_empty());
    }

    #[tokio::test]
    async fn test_get_or_fetch_token_prices_cache_disabled_falls_back_to_oracle() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();
        let config = get_config().unwrap();

        let mint = Pubkey::new_unique().to_string();
        let prices = CacheUtil::get_or_fetch_token_prices(&config, std::slice::from_ref(&mint))
            .await
            .unwrap();

        // Mock oracle (default in ConfigMockBuilder) always returns a price for any mint.
        assert!(prices.contains_key(&mint));
    }

    #[tokio::test]
    async fn test_get_or_fetch_token_prices_zero_ttl_bypasses_cache() {
        // Cache enabled globally (so account caching would still run), but
        // price_ttl = 0 should opt price lookups out of Redis entirely.
        let _m = ConfigMockBuilder::new()
            .with_cache_enabled(true)
            .with_cache_url(Some("redis://localhost:6379".to_string()))
            .build_and_setup();

        let mut config = get_config().unwrap();
        config.kora.cache.price_ttl = 0;

        let mint = Pubkey::new_unique().to_string();
        // No real Redis is needed: the zero-ttl branch must short-circuit before any pool access.
        let prices = CacheUtil::get_or_fetch_token_prices(&config, std::slice::from_ref(&mint))
            .await
            .unwrap();

        assert!(prices.contains_key(&mint));
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

    #[tokio::test]
    async fn test_get_or_fetch_blockhash_cache_disabled() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let rpc_client = RpcMockBuilder::new().with_blockhash().build();

        let config = get_config().unwrap();
        let result = CacheUtil::get_or_fetch_latest_blockhash(&config, &rpc_client).await;

        assert!(result.is_ok(), "Should successfully get blockhash with cache disabled");
        let hash = result.unwrap();
        assert_ne!(hash, Hash::default(), "Blockhash should not be the default hash");
    }

    #[tokio::test]
    async fn test_fetch_blockhash_from_rpc_success() {
        let rpc_client = RpcMockBuilder::new().with_blockhash().build();

        let result = CacheUtil::fetch_blockhash_from_rpc(&rpc_client).await;

        assert!(result.is_ok(), "Should successfully fetch blockhash from RPC");
        let hash = result.unwrap();
        assert_ne!(hash, Hash::default(), "Blockhash should not be the default hash");
    }
}
