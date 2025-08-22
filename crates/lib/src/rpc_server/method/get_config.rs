use crate::{
    config::{EnabledMethods, ValidationConfig},
    signer::SelectionStrategy,
    state::{self, get_signer_pool},
    KoraError,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignerPoolInfo {
    pub strategy: SelectionStrategy,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetConfigResponse {
    pub fee_payers: Vec<String>,
    pub validation_config: ValidationConfig,
    pub enabled_methods: EnabledMethods,
}

pub async fn get_config() -> Result<GetConfigResponse, KoraError> {
    let config = state::get_config()?;

    // Get signer pool information (required in multi-signer mode)
    let pool = get_signer_pool()
        .map_err(|e| KoraError::InternalServerError(format!("Signer pool not initialized: {e}")))?;

    // Get all fee payer public keys from the signer pool
    let fee_payers: Vec<String> =
        pool.get_signers_info().iter().map(|signer| signer.public_key.clone()).collect();

    Ok(GetConfigResponse {
        fee_payers,
        validation_config: config.validation.clone(),
        enabled_methods: config.kora.enabled_methods.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{common::setup_or_get_test_signer, config_mock::ConfigMockBuilder};

    #[tokio::test]
    async fn test_get_config_success() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let _ = setup_or_get_test_signer();

        let result = get_config().await;

        assert!(result.is_ok(), "Should successfully get config");
        let response = result.unwrap();
        assert!(!response.fee_payers.is_empty(), "Should have at least one fee payer");
        assert!(!response.fee_payers[0].is_empty(), "Fee payer pubkey should not be empty");
        assert!(
            response.validation_config.max_allowed_lamports > 0,
            "Should have max_allowed_lamports set"
        );
    }
}
