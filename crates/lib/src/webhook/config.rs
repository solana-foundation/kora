use serde::{Deserialize, Serialize};

fn default_timeout() -> u64 {
    5000
}

fn default_retries() -> u32 {
    3
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct WebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    pub url: Option<String>,
    pub secret: Option<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_retries")]
    pub retry_attempts: u32,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            secret: None,
            events: vec![],
            timeout_ms: default_timeout(),
            retry_attempts: default_retries(),
        }
    }
}

impl WebhookConfig {
    pub fn is_event_enabled(&self, event_type: &str) -> bool {
        self.enabled && (self.events.is_empty() || self.events.contains(&event_type.to_string()))
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.url.is_none() {
            return Err("Webhook URL is required when webhooks are enabled".to_string());
        }

        if self.secret.is_none() {
            return Err("Webhook secret is required when webhooks are enabled".to_string());
        }

        if let Some(url) = &self.url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("Webhook URL must start with http:// or https://".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebhookConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.retry_attempts, 3);
    }

    #[test]
    fn test_validate_disabled() {
        let config = WebhookConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_url() {
        let config = WebhookConfig {
            enabled: true,
            url: None,
            secret: Some("secret".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_missing_secret() {
        let config = WebhookConfig {
            enabled: true,
            url: Some("https://example.com".to_string()),
            secret: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_url() {
        let config = WebhookConfig {
            enabled: true,
            url: Some("invalid-url".to_string()),
            secret: Some("secret".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_is_event_enabled() {
        let config = WebhookConfig {
            enabled: true,
            url: Some("https://example.com".to_string()),
            secret: Some("secret".to_string()),
            events: vec!["transaction.signed".to_string()],
            ..Default::default()
        };
        assert!(config.is_event_enabled("transaction.signed"));
        assert!(!config.is_event_enabled("transaction.failed"));
    }

    #[test]
    fn test_is_event_enabled_empty_events_list() {
        let config = WebhookConfig {
            enabled: true,
            url: Some("https://example.com".to_string()),
            secret: Some("secret".to_string()),
            events: vec![],
            ..Default::default()
        };
        // Empty events list means all events are enabled
        assert!(config.is_event_enabled("transaction.signed"));
        assert!(config.is_event_enabled("any.event"));
    }
}