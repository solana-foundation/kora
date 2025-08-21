use crate::{
    error::KoraError,
    state::{get_config, get_signer_pool},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetPayerSignerResponse {
    /// The recommended signer's public key
    pub signer: String,
    /// The payment destination address (same as signer if no separate paymaster is configured)
    pub payment_destination: String,
}

pub async fn get_payer_signer() -> Result<GetPayerSignerResponse, KoraError> {
    let config = get_config()?;
    let pool = get_signer_pool()?;

    // Get the next signer according to the configured strategy
    let signer_meta = pool.get_next_signer()?;
    let signer_pubkey = signer_meta.signer.solana_pubkey().to_string();

    // Determine payment destination
    // If a payment address is configured, use it; otherwise use the signer address
    let payment_destination =
        config.kora.payment_address.as_ref().cloned().unwrap_or_else(|| signer_pubkey.clone());

    Ok(GetPayerSignerResponse { signer: signer_pubkey, payment_destination })
}
