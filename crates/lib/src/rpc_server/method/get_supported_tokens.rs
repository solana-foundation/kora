use crate::{error::KoraError, state::get_config};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetSupportedTokensResponse {
    pub tokens: Vec<String>,
}

pub async fn get_supported_tokens() -> Result<GetSupportedTokensResponse, KoraError> {
    let config = &get_config()?;
    let tokens = &config.validation.allowed_tokens;

    if tokens.is_empty() {
        return Err(KoraError::InternalServerError("No tokens provided".to_string()));
    }

    let response = GetSupportedTokensResponse { tokens: tokens.to_vec() };

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::update_config, tests::config_mock::ConfigMockBuilder};

    #[tokio::test]
    async fn test_get_supported_tokens_empty_list() {
        let config = ConfigMockBuilder::new().with_allowed_tokens(vec![]).build();
        update_config(config).expect("Failed to update config");

        let result = get_supported_tokens().await;

        assert!(result.is_err(), "Should fail when no tokens configured");
        let error = result.unwrap_err();
        assert!(
            matches!(error, KoraError::InternalServerError(_)),
            "Should return InternalServerError"
        );
    }

    #[tokio::test]
    async fn test_get_supported_tokens_contains_expected_tokens() {
        let expected_tokens = vec![
            "11111111111111111111111111111111".to_string(),
            "22222222222222222222222222222222".to_string(),
        ];
        let config = ConfigMockBuilder::new().with_allowed_tokens(expected_tokens.clone()).build();
        update_config(config).expect("Failed to update config");

        let result = get_supported_tokens().await;

        assert!(result.is_ok(), "Should successfully get supported tokens");
        let response = result.unwrap();
        assert_eq!(response.tokens.len(), 2, "Should have exactly 2 tokens");
        assert!(
            response.tokens.contains(&"11111111111111111111111111111111".to_string()),
            "Should contain first token"
        );
        assert!(
            response.tokens.contains(&"22222222222222222222222222222222".to_string()),
            "Should contain second token"
        );
    }
}
