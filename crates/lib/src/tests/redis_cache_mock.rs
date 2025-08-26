use crate::error::KoraError;
use mockall::mock;
use redis_test::MockRedisConnection;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};

/// Redis cache mock for testing scenarios where caching behavior is tested
///
/// It provides a mockall-based replacement for the real CacheUtil to avoid Redis dependencies in tests.
///
/// Usage:
/// ```rust
/// #[cfg(test)]
/// use crate::tests::redis_cache_mock::MockCacheUtil as CacheUtil;
/// ```
pub struct RedisCacheMock {
    mock_connection: MockRedisConnection,
}

impl RedisCacheMock {
    /// Create basic mock for cache.rs testing
    pub fn new() -> Self {
        Self { mock_connection: MockRedisConnection::new(vec![]) }
    }

    /// Get mock connection for testing
    pub fn get_mock_connection(&self) -> &MockRedisConnection {
        &self.mock_connection
    }
}

impl Default for RedisCacheMock {
    fn default() -> Self {
        Self::new()
    }
}

mock! {
    pub CacheUtil {
        pub async fn init() -> Result<(), KoraError>;
        pub async fn get_account(
            rpc_client: &RpcClient,
            pubkey: &Pubkey,
            force_refresh: bool,
        ) -> Result<Account, KoraError>;
    }
}
