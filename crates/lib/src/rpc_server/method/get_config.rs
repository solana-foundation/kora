use crate::{
    config::{EnabledMethods, ValidationConfig},
    state::{self, get_request_signer},
    KoraError,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetConfigResponse {
    pub fee_payer: String,
    pub validation_config: ValidationConfig,
    pub enabled_methods: EnabledMethods,
}

pub async fn get_config() -> Result<GetConfigResponse, KoraError> {
    let signer = get_request_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {e}")))?;

    let config = state::get_config()?;

    Ok(GetConfigResponse {
        fee_payer: signer.solana_pubkey().to_string(),
        validation_config: config.validation.clone(),
        enabled_methods: config.kora.enabled_methods.clone(),
    })
}
