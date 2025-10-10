use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use deadpool_redis::{Connection, Pool};
use redis::AsyncCommands;

use crate::{error::KoraError, sanitize_error};

/// Trait for storing and retrieving usage counts
#[async_trait]
pub trait UsageStore: Send + Sync {
    /// Increment usage count for a key and return the new value
    async fn increment(&self, key: &str) -> Result<u32, KoraError>;

    /// Get current usage count for a key (returns 0 if not found)
    async fn get(&self, key: &str) -> Result<u32, KoraError>;

    /// Clear all usage data (mainly for testing)
    async fn clear(&self) -> Result<(), KoraError>;
}

/// Redis-based implementation for production
pub struct RedisUsageStore {
    pool: Pool,
}

impl RedisUsageStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn get_connection(&self) -> Result<Connection, KoraError> {
        self.pool.get().await.map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to get Redis connection: {}",
                e
            )))
        })
    }
}

#[async_trait]
impl UsageStore for RedisUsageStore {
    async fn increment(&self, key: &str) -> Result<u32, KoraError> {
        let mut conn = self.get_connection().await?;
        let count: u32 = conn.incr(key, 1).await.map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to increment usage for {}: {}",
                key, e
            )))
        })?;
        Ok(count)
    }

    async fn get(&self, key: &str) -> Result<u32, KoraError> {
        let mut conn = self.get_connection().await?;
        let count: Option<u32> = conn.get(key).await.map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to get usage for {}: {}",
                key, e
            )))
        })?;
        Ok(count.unwrap_or(0))
    }

    async fn clear(&self) -> Result<(), KoraError> {
        let mut conn = self.get_connection().await?;
        let _: () = conn.flushdb().await.map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!("Failed to clear Redis: {}", e)))
        })?;
        Ok(())
    }
}

/// In-memory implementation for testing
pub struct InMemoryUsageStore {
    data: Mutex<HashMap<String, u32>>,
}

impl InMemoryUsageStore {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }
}

impl Default for InMemoryUsageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UsageStore for InMemoryUsageStore {
    async fn increment(&self, key: &str) -> Result<u32, KoraError> {
        let mut data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;
        let count = data.entry(key.to_string()).or_insert(0);
        *count += 1;
        Ok(*count)
    }

    async fn get(&self, key: &str) -> Result<u32, KoraError> {
        let data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;
        Ok(data.get(key).copied().unwrap_or(0))
    }

    async fn clear(&self) -> Result<(), KoraError> {
        let mut data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;
        data.clear();
        Ok(())
    }
}

/// Mock store that simulates Redis errors for testing error handling
#[cfg(test)]
pub struct ErrorUsageStore {
    should_error_get: bool,
    should_error_increment: bool,
}

#[cfg(test)]
impl ErrorUsageStore {
    pub fn new(should_error_get: bool, should_error_increment: bool) -> Self {
        Self { should_error_get, should_error_increment }
    }
}

#[cfg(test)]
#[async_trait]
impl UsageStore for ErrorUsageStore {
    async fn increment(&self, _key: &str) -> Result<u32, KoraError> {
        if self.should_error_increment {
            Err(KoraError::InternalServerError("Redis connection failed".to_string()))
        } else {
            Ok(1)
        }
    }

    async fn get(&self, _key: &str) -> Result<u32, KoraError> {
        if self.should_error_get {
            Err(KoraError::InternalServerError("Redis connection failed".to_string()))
        } else {
            Ok(0)
        }
    }

    async fn clear(&self) -> Result<(), KoraError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_usage_store() {
        let store = InMemoryUsageStore::new();

        // Initial count should be 0
        assert_eq!(store.get("wallet1").await.unwrap(), 0);

        // Increment should return 1
        assert_eq!(store.increment("wallet1").await.unwrap(), 1);
        assert_eq!(store.get("wallet1").await.unwrap(), 1);

        // Increment again should return 2
        assert_eq!(store.increment("wallet1").await.unwrap(), 2);
        assert_eq!(store.get("wallet1").await.unwrap(), 2);

        // Different key should be independent
        assert_eq!(store.increment("wallet2").await.unwrap(), 1);
        assert_eq!(store.get("wallet2").await.unwrap(), 1);
        assert_eq!(store.get("wallet1").await.unwrap(), 2);

        // Clear should reset everything
        store.clear().await.unwrap();
        assert_eq!(store.get("wallet1").await.unwrap(), 0);
        assert_eq!(store.get("wallet2").await.unwrap(), 0);
    }
}
