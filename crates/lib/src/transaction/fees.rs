use serde::{Deserialize, Serialize};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::VersionedMessage,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    rent::Rent,
    transaction::{Transaction, VersionedTransaction},
};
use spl_associated_token_account::get_associated_token_address;
use std::time::Duration;
use utoipa::ToSchema;

use crate::{
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle},
    token::{TokenInterface, TokenProgram, TokenType},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPriceInfo {
    pub price: f64,
}

/// Trait for transaction fee estimation
pub trait TransactionFeeEstimator {
    fn estimate_fee(
        &self,
        rpc_client: &RpcClient,
    ) -> impl std::future::Future<Output = Result<u64, KoraError>> + Send;
}

impl TransactionFeeEstimator for Transaction {
    async fn estimate_fee(&self, rpc_client: &RpcClient) -> Result<u64, KoraError> {
        estimate_transaction_fee(rpc_client, self).await
    }
}

impl TransactionFeeEstimator for VersionedTransaction {
    async fn estimate_fee(&self, rpc_client: &RpcClient) -> Result<u64, KoraError> {
        match &self.message {
            VersionedMessage::Legacy(legacy_message) => {
                // For legacy messages, use the same approach as regular transactions
                let base_fee = rpc_client
                    .get_fee_for_message(legacy_message)
                    .await
                    .map_err(|e| KoraError::RpcError(e.to_string()))?;

                // Get priority fee from recent blocks
                let priority_stats = rpc_client
                    .get_recent_prioritization_fees(&[])
                    .await
                    .map_err(|e| KoraError::RpcError(e.to_string()))?;
                let priority_fee =
                    priority_stats.iter().map(|fee| fee.prioritization_fee).max().unwrap_or(0);

                Ok(base_fee + priority_fee)
            }
            VersionedMessage::V0(v0_message) => {
                // Simulate the transaction to get the compute units consumed
                let simulation_result = rpc_client
                    .simulate_transaction_with_config(
                        self,
                        RpcSimulateTransactionConfig {
                            sig_verify: false,
                            replace_recent_blockhash: true,
                            commitment: Some(CommitmentConfig::processed()),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(|e| {
                        KoraError::RpcError(format!("Transaction simulation failed: {}", e))
                    })?;

                if let Some(units_consumed) = simulation_result.value.units_consumed {
                    // Calculate fee components
                    let blockhash_response = rpc_client
                        .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
                        .await
                        .map_err(|e| {
                            KoraError::RpcError(format!("Failed to get blockhash: {}", e))
                        })?;

                    let lamports_per_signature = blockhash_response.1;
                    let num_signatures = self.signatures.len() as u64;
                    let num_account_keys = v0_message.account_keys.len() as u64;
                    let num_lookups = v0_message.address_table_lookups.len() as u64;

                    // Calculate base fee (signatures) and additional fee (compute units + account keys + lookups)
                    let base_fee = lamports_per_signature * num_signatures;
                    let additional_fee =
                        units_consumed as u64 + (num_account_keys * 10) + (num_lookups * 20);

                    // Get priority fee from recent blocks
                    let priority_stats = rpc_client
                        .get_recent_prioritization_fees(&[])
                        .await
                        .map_err(|e| KoraError::RpcError(e.to_string()))?;
                    let priority_fee =
                        priority_stats.iter().map(|fee| fee.prioritization_fee).max().unwrap_or(0);

                    Ok(base_fee + additional_fee + priority_fee)
                } else {
                    Err(KoraError::InvalidTransaction(
                        "Failed to simulate transaction for fee estimation".to_string(),
                    ))
                }
            }
        }
    }
}

pub async fn estimate_transaction_fee(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> Result<u64, KoraError> {
    // Get base transaction fee
    let base_fee = rpc_client
        .get_fee_for_message(&transaction.message)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

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

pub async fn estimate_versioned_transaction_fee(
    rpc_client: &RpcClient,
    transaction: &VersionedTransaction,
) -> Result<u64, KoraError> {
    transaction.estimate_fee(rpc_client).await
}

async fn get_associated_token_account_creation_fees(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> Result<u64, KoraError> {
    const ATA_ACCOUNT_SIZE: usize = 165; // Standard ATA size
    let mut ata_count = 0u64;

    // Check each instruction in the transaction for ATA creation
    for instruction in &transaction.message.instructions {
        let program_id = transaction.message.account_keys[instruction.program_id_index as usize];

        // Skip if not an ATA program instruction
        if program_id != spl_associated_token_account::id() {
            continue;
        }

        let ata = transaction.message.account_keys[instruction.accounts[1] as usize];
        let owner = transaction.message.account_keys[instruction.accounts[2] as usize];
        let mint = transaction.message.account_keys[instruction.accounts[3] as usize];

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

pub async fn calculate_token_value_in_lamports(
    amount: u64,
    mint: &Pubkey,
    price_source: PriceSource,
    rpc_client: &RpcClient,
) -> Result<u64, KoraError> {
    // Fetch mint account data to determine token decimals
    let mint_account =
        rpc_client.get_account(mint).await.map_err(|e| KoraError::RpcError(e.to_string()))?;

    let token_program = TokenProgram::new(TokenType::Spl);
    let decimals = token_program.get_mint_decimals(&mint_account.data)?;

    // Initialize price oracle with retries for reliability
    let oracle =
        RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source));

    // Get token price in SOL directly
    let token_price = oracle
        .get_token_price(&mint.to_string())
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {}", e)))?;

    // Convert token amount to its real value based on decimals and multiply by SOL price
    let token_amount = amount as f64 / 10f64.powi(decimals as i32);
    let sol_amount = token_amount * token_price.price;

    // Convert SOL to lamports and round down
    let lamports = (sol_amount * LAMPORTS_PER_SOL as f64).floor() as u64;

    Ok(lamports)
}
