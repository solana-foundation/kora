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

    pub async fn validate(
        usage_config: &UsageLimitConfig,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Skip validation if usage limiting is disabled
        if !usage_config.enabled {
            return Ok((errors, warnings));
        }

        // Check if cache_url is provided when enabled
        if usage_config.cache_url.is_none() {
            if !usage_config.fallback_if_unavailable {
                errors.push(
            "Usage limiting enabled without cache_url and fallback disabled - service will fail"
                .to_string(),
        );
            } else {
                warnings.push(
                    "Usage limiting enabled without cache_url - fallback mode will disable limits"
                        .to_string(),
                );
            }
        } else if let Some(cache_url) = &usage_config.cache_url {
            // Validate cache_url format
            if !cache_url.starts_with("redis://") && !cache_url.starts_with("rediss://") {
                errors.push(format!(
                    "Invalid cache_url format: '{cache_url}' - must start with redis:// or rediss://"
                ));
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
                match Self::test_redis_connection(cache_url).await {
                    Ok(_) => {}
                    Err(e) => {
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
                };
            }
        }

        Ok((errors, warnings))
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

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

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

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

        assert!(errors.is_empty());
        assert!(warnings.iter().any(|w| w.contains(
            "Usage limiting enabled without cache_url - fallback mode will disable limits"
        )));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_usage_limit_enabled_no_cache_url_fallback_disabled() {
        let config = ConfigMockBuilder::new()
            .with_usage_limit_enabled(true)
            .with_usage_limit_cache_url(None)
            .with_usage_limit_fallback(false)
            .build();

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

        // Should error when no cache_url and fallback disabled
        assert!(errors.iter().any(|e| e.contains(
            "Usage limiting enabled without cache_url and fallback disabled - service will fail"
        )));
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

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

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
            .with_usage_limit_cache_url(Some("redis://localhost:6379".to_string()))
            .with_usage_limit_fallback(false)
            .build();

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

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
            .with_usage_limit_cache_url(Some("redis://localhost:6379".to_string()))
            .with_usage_limit_fallback(true)
            .build();

        let result = CacheValidator::validate(&config.kora.usage_limit).await;
        assert!(result.is_ok());
        let (errors, warnings) = result.unwrap();

        // Should get warnings because Redis connection fails (unit tests don't run Redis) but fallback is enabled
        assert!(errors.is_empty());
        assert!(warnings
            .iter()
            .any(|w| w.contains("Usage limit Redis connection failed (fallback enabled)")));
    }
}
