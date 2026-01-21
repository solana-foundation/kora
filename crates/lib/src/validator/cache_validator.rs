use deadpool_redis::Runtime;
use redis::AsyncCommands;

use crate::config::UsageLimitConfig;

pub struct CacheValidator {}

impl CacheValidator {
    /// Test Redis connection for usage limit cache
    async fn test_redis_connection(cache_url: &str) -> Result<(), String> {
        let cfg = deadpool_redis::Config::from_url(cache_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| format!("Failed to create Redis pool: {e}"))?;

        let mut conn = pool.get().await.map_err(|e| format!("Failed to connect to Redis: {e}"))?;

        let _: Option<String> = conn
            .get("__config_validator_test__")
            .await
            .map_err(|e| format!("Redis connection test failed: {e}"))?;

        drop(conn);
        drop(pool);

        Ok(())
    }

    pub async fn validate(usage_config: &UsageLimitConfig) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Skip validation if usage limiting is disabled
        if !usage_config.enabled {
            return (errors, warnings);
        }

        // Check if cache_url is provided when enabled
        match &usage_config.cache_url {
            None => {
                // In-memory store will be used - warn about non-persistence
                warnings.push(
                    "Usage limiting enabled without cache_url - using in-memory store (not persistent across restarts)"
                        .to_string(),
                );
            }
            Some(cache_url) => {
                // Validate cache_url format
                if !cache_url.starts_with("redis://") && !cache_url.starts_with("rediss://") {
                    errors.push(format!(
                        "Invalid cache_url format: '{cache_url}' - must start with redis:// or rediss://"
                    ));
                }
            }
        }

        // Warn about fallback configuration
        if !usage_config.fallback_if_unavailable {
            warnings.push(
                "Usage limit fallback disabled - service will fail if cache becomes unavailable"
                    .to_string(),
            );
        }

        // Test Redis connection
        if let Some(cache_url) = &usage_config.cache_url {
            if cache_url.starts_with("redis://") || cache_url.starts_with("rediss://") {
                if let Err(e) = Self::test_redis_connection(cache_url).await {
                    if usage_config.fallback_if_unavailable {
                        warnings.push(format!(
                            "Usage limit Redis connection failed (fallback enabled): {e}"
                        ));
                    } else {
                        errors.push(format!(
                            "Usage limit Redis connection failed (fallback disabled): {e}"
                        ));
                    }
                }
            }
        }

        (errors, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_disabled() {
        let config = ConfigMockBuilder::new().with_usage_limit_enabled(false).build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_enabled_no_cache_url_fallback_enabled() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(true)
            .build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        assert!(errors.is_empty());
        assert!(warnings.iter().any(
            |w| w.contains("Usage limiting enabled without cache_url - using in-memory store")
        ));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_enabled_no_cache_url_fallback_disabled() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(false)
            .build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        // In-memory store works - just warn about non-persistence
        assert!(errors.is_empty());
        assert!(warnings.iter().any(
            |w| w.contains("Usage limiting enabled without cache_url - using in-memory store")
        ));
        // Fallback warning still applies
        assert!(warnings.iter().any(|w| w.contains(
            "Usage limit fallback disabled - service will fail if cache becomes unavailable"
        )));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_invalid_cache_url_format() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(Some("invalid://localhost:6379".to_string()))
            .with_usage_limit_fallback(true)
            .build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        // Should error for invalid cache_url format
        assert!(errors.iter().any(|e| e.contains("Invalid cache_url format")
            && e.contains("must start with redis:// or rediss://")));
        // No fallback warning since fallback is enabled
        assert!(!warnings.iter().any(|w| w.contains(
            "Usage limit fallback disabled - service will fail if cache becomes unavailable"
        )));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_fallback_disabled_warning() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            // We use port 54321 to ensure connection failure, as tests assert that the validator
            // catches connection errors. Using default 6379 might succeed if a local Redis is running.
            .with_usage_limit_cache_url(Some("redis://localhost:54321".to_string()))
            .with_usage_limit_fallback(false)
            .build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        // Should error about Redis connection failure with fallback disabled
        assert!(errors
            .iter()
            .any(|e| e.contains("Usage limit Redis connection failed (fallback disabled)")));
        assert!(warnings.iter().any(|w| w.contains(
            "Usage limit fallback disabled - service will fail if cache becomes unavailable"
        )));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_valid_redis_url() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            // Ensure connection failure by using a non-standard/unused port
            .with_usage_limit_cache_url(Some("redis://localhost:54321".to_string()))
            .with_usage_limit_fallback(true)
            .build();

        let (errors, warnings) = CacheValidator::validate(&config.kora.usage_limit).await;

        // Should get warnings because Redis connection fails (unit tests don't run Redis) but fallback is enabled
        assert!(errors.is_empty());
        assert!(warnings
            .iter()
            .any(|w| w.contains("Usage limit Redis connection failed (fallback enabled)")));
    }
}
