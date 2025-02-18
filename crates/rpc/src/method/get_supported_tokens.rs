use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use kora_lib::error::KoraError;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetSupportedTokensResponse {
    pub tokens: Vec<String>,
}

pub async fn get_supported_tokens(
    tokens: &[String],
) -> Result<GetSupportedTokensResponse, KoraError> {
    if tokens.is_empty() {
        return Err(KoraError::InternalServerError("No tokens provided".to_string()));
    }

    let response = GetSupportedTokensResponse { tokens: tokens.to_vec() };

    Ok(response)
}
