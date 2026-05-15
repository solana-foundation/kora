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

    /// Increment usage count with absolute expiration (key expires at unix timestamp)
    async fn increment_with_expiry(&self, key: &str, expires_at: u64) -> Result<u32, KoraError>;

    /// Get current usage count for a key (returns 0 if not found)
    async fn get(&self, key: &str) -> Result<u32, KoraError>;

    /// Atomic check and increment: check if (current + delta) <= max, and increment if so.
    /// Returns true if allowed and incremented, false if denied.
    async fn check_and_increment(
        &self,
        key: &str,
        delta: u64,
        max: u64,
        expiry: Option<u64>,
    ) -> Result<bool, KoraError>;

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

    async fn increment_with_expiry(&self, key: &str, expires_at: u64) -> Result<u32, KoraError> {
        let mut conn = self.get_connection().await?;

        // Use Redis pipeline for atomic INCR + EXPIREAT
        // EXPIREAT sets absolute expiration timestamp, so repeated calls are idempotent
        let (count,): (u32,) = redis::pipe()
            .atomic()
            .incr(key, 1)
            .cmd("EXPIREAT")
            .arg(key)
            .arg(expires_at as i64)
            .ignore()
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                KoraError::InternalServerError(sanitize_error!(format!(
                    "Failed to increment with expiry for {}: {}",
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

    async fn check_and_increment(
        &self,
        key: &str,
        delta: u64,
        max: u64,
        expiry: Option<u64>,
    ) -> Result<bool, KoraError> {
        let mut conn = self.get_connection().await?;

        let script = redis::Script::new(
            r"
            local current = redis.call('GET', KEYS[1])
            local count = current and tonumber(current) or 0
            if count + tonumber(ARGV[1]) > tonumber(ARGV[2]) then return 0 end
            redis.call('INCRBY', KEYS[1], ARGV[1])
            if ARGV[3] ~= '0' and redis.call('TTL', KEYS[1]) < 0 then
                redis.call('EXPIREAT', KEYS[1], ARGV[3])
            end
            return 1
            ",
        );

        let allowed: i32 = script
            .key(key)
            .arg(delta)
            .arg(max)
            .arg(expiry.unwrap_or(0))
            .invoke_async(&mut conn)
            .await
            .map_err(|e| {
                KoraError::InternalServerError(sanitize_error!(format!(
                    "Failed to execute check_and_increment script: {}",
                    e
                )))
            })?;

        Ok(allowed == 1)
    }

    async fn clear(&self) -> Result<(), KoraError> {
        let mut conn = self.get_connection().await?;
        let _: () = conn.flushdb().await.map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!("Failed to clear Redis: {}", e)))
        })?;
        Ok(())
    }
}

/// Entry with count and optional expiry timestamp
struct UsageEntry {
    count: u32,
    expiry: Option<u64>, // Unix timestamp when this entry expires
}

/// In-memory implementation for testing
pub struct InMemoryUsageStore {
    data: Mutex<HashMap<String, UsageEntry>>,
}

impl InMemoryUsageStore {
    pub fn new() -> Self {
        Self { data: Mutex::new(HashMap::new()) }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
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
        let entry = data.entry(key.to_string()).or_insert(UsageEntry { count: 0, expiry: None });
        entry.count += 1;
        Ok(entry.count)
    }

    async fn increment_with_expiry(&self, key: &str, expires_at: u64) -> Result<u32, KoraError> {
        let mut data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;

        let now = Self::current_timestamp();
        let entry = data.entry(key.to_string()).or_insert(UsageEntry { count: 0, expiry: None });

        // Check if expired, reset if so
        if let Some(expiry) = entry.expiry {
            if now >= expiry {
                entry.count = 0;
            }
        }

        entry.count += 1;
        // Always set to the same absolute expiry (idempotent like EXPIREAT)
        entry.expiry = Some(expires_at);

        Ok(entry.count)
    }

    async fn get(&self, key: &str) -> Result<u32, KoraError> {
        let data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;

        if let Some(entry) = data.get(key) {
            // Check if expired
            if let Some(expiry) = entry.expiry {
                if Self::current_timestamp() >= expiry {
                    return Ok(0);
                }
            }
            Ok(entry.count)
        } else {
            Ok(0)
        }
    }

    async fn check_and_increment(
        &self,
        key: &str,
        delta: u64,
        max: u64,
        expiry: Option<u64>,
    ) -> Result<bool, KoraError> {
        let mut data = self.data.lock().map_err(|e| {
            KoraError::InternalServerError(sanitize_error!(format!(
                "Failed to lock usage store: {}",
                e
            )))
        })?;

        let now = Self::current_timestamp();
        let entry = data.entry(key.to_string()).or_insert(UsageEntry { count: 0, expiry: None });

        if let Some(e) = entry.expiry {
            if now >= e {
                entry.count = 0;
                entry.expiry = None;
            }
        }

        let new_count = entry.count as u64 + delta;
        if new_count > max || new_count > u32::MAX as u64 {
            return Ok(false);
        }
        entry.count = new_count as u32;
        if let Some(e) = expiry {
            if entry.expiry.is_none() {
                entry.expiry = Some(e);
            }
        }

        Ok(true)
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

    async fn increment_with_expiry(&self, _key: &str, _expires_at: u64) -> Result<u32, KoraError> {
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

    async fn check_and_increment(
        &self,
        _key: &str,
        _delta: u64,
        _max: u64,
        _expiry: Option<u64>,
    ) -> Result<bool, KoraError> {
        if self.should_error_increment {
            Err(KoraError::InternalServerError("Redis connection failed".to_string()))
        } else {
            Ok(true)
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
