use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::bundle::jito::constant::JITO_DEFAULT_BLOCK_ENGINE_URL;

/// Jito-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JitoConfig {
    /// Jito block engine URL
    #[serde(default = "default_jito_block_engine_url")]
    pub block_engine_url: String,
}

fn default_jito_block_engine_url() -> String {
    JITO_DEFAULT_BLOCK_ENGINE_URL.to_string()
}

impl Default for JitoConfig {
    fn default() -> Self {
        Self { block_engine_url: JITO_DEFAULT_BLOCK_ENGINE_URL.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jito_config_default() {
        let config = JitoConfig::default();
        assert_eq!(config.block_engine_url, JITO_DEFAULT_BLOCK_ENGINE_URL);
    }

    #[test]
    fn test_jito_config_serde() {
        let toml = r#"
            block_engine_url = "https://custom.jito.wtf"
        "#;
        let config: JitoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.block_engine_url, "https://custom.jito.wtf");
    }

    #[test]
    fn test_jito_config_empty_uses_defaults() {
        let toml = "";
        let config: JitoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.block_engine_url, JITO_DEFAULT_BLOCK_ENGINE_URL);
    }

    #[test]
    fn test_jito_config_mock_url() {
        use crate::bundle::jito::constant::JITO_MOCK_BLOCK_ENGINE_URL;

        let toml = r#"block_engine_url = "mock""#;
        let config: JitoConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.block_engine_url, JITO_MOCK_BLOCK_ENGINE_URL);
        assert_eq!(config.block_engine_url, "mock");
    }
}
