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
