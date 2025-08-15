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
