use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{rent::Rent, transaction::VersionedTransaction};
use spl_associated_token_account::get_associated_token_address;
use utoipa::ToSchema;

use crate::{error::KoraError, transaction::get_estimate_fee};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPriceInfo {
    pub price: f64,
}

pub async fn estimate_transaction_fee(
    rpc_client: &RpcClient,
    transaction: &VersionedTransaction,
) -> Result<u64, KoraError> {
    // Get base transaction fee
    let base_fee = get_estimate_fee(rpc_client, &transaction.message).await?;

    // Get account creation fees (for ATA creation)
    let account_creation_fee = get_associated_token_account_creation_fees(rpc_client, transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Get priority fee from recent blocks
    let priority_stats = rpc_client
        .get_recent_prioritization_fees(&[])
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;
    let priority_fee = priority_stats.iter().map(|fee| fee.prioritization_fee).max().unwrap_or(0);

    Ok(base_fee + priority_fee + account_creation_fee)
}

async fn get_associated_token_account_creation_fees(
    rpc_client: &RpcClient,
    transaction: &VersionedTransaction,
) -> Result<u64, KoraError> {
    const ATA_ACCOUNT_SIZE: usize = 165; // Standard ATA size
    let mut ata_count = 0u64;

    // Check each instruction in the transaction for ATA creation
    for instruction in transaction.message.instructions() {
        let account_keys = transaction.message.static_account_keys();
        let program_id = account_keys[instruction.program_id_index as usize];

        // Skip if not an ATA program instruction
        if program_id != spl_associated_token_account::id() {
            continue;
        }

        let ata = account_keys[instruction.accounts[1] as usize];
        let owner = account_keys[instruction.accounts[2] as usize];
        let mint = account_keys[instruction.accounts[3] as usize];

        let expected_ata = get_associated_token_address(&owner, &mint);

        if ata == expected_ata && rpc_client.get_account(&ata).await.is_err() {
            ata_count += 1;
        }
    }

    // Get rent cost in lamports for ATA creation
    let rent = Rent::default();
    let exempt_min = rent.minimum_balance(ATA_ACCOUNT_SIZE);

    Ok(exempt_min * ata_count)
}
