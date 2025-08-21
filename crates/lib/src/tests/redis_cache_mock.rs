use crate::{
    cache::CachedAccount,
};
use redis_test::MockRedisConnection;
use solana_sdk::{account::Account, pubkey::Pubkey};

/// Minimal Redis cache mock focused on testing cache.rs functions
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

/// Create mock cached account for testing cache.rs
pub fn create_mock_cached_account(pubkey: &Pubkey, lamports: u64) -> CachedAccount {
    CachedAccount {
        account: Account {
            lamports,
            data: vec![0u8; 100],
            owner: *pubkey,
            executable: false,
            rent_epoch: 0,
        },
        cached_at: chrono::Utc::now().timestamp(),
    }
}

/// Create expired cached account for TTL testing
pub fn create_expired_cached_account(pubkey: &Pubkey, lamports: u64) -> CachedAccount {
    CachedAccount {
        account: Account {
            lamports,
            data: vec![0u8; 100],
            owner: *pubkey,
            executable: false,
            rent_epoch: 0,
        },
        cached_at: chrono::Utc::now().timestamp() - 3600, // 1 hour ago
    }
}
