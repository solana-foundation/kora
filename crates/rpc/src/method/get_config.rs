use serde::Serialize;
use utoipa::ToSchema;
use kora_lib::{config::ValidationConfig, get_signer, KoraError};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetConfigResponse {
    pub fee_payer: String,
    pub validation_config: ValidationConfig,
}

pub async fn get_config(validation: &ValidationConfig) -> Result<GetConfigResponse, KoraError> {
    let signer = get_signer()
        .map_err(|e| KoraError::SigningError(format!("Failed to get signer: {}", e)))?;

    Ok(GetConfigResponse {
        fee_payer: signer.solana_pubkey().to_string(),
        validation_config: validation.clone(),
    })
}
