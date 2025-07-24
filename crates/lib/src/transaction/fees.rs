use crate::{
    constant::{BASE_COMPUTE_UNIT_LIMIT, BASE_COMPUTE_UNIT_PRICE},
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle},
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::{get_estimate_fee, VersionedTransactionExt},
};
use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_compute_budget_interface::{ComputeBudgetInstruction, ID as COMPUTE_BUDGET_ID};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, rent::Rent, transaction::VersionedTransaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPriceInfo {
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct ComputeUnitInfo {
    pub compute_unit_limit: Option<u32>,
    pub compute_unit_price: Option<u64>,
}

impl ComputeUnitInfo {
    pub fn calculate_priority_fee(&self) -> u64 {
        let limit = self.compute_unit_limit.unwrap_or(BASE_COMPUTE_UNIT_LIMIT);
        let price = self.compute_unit_price.unwrap_or(BASE_COMPUTE_UNIT_PRICE);

        // Priority fee = compute_unit_limit * compute_unit_price
        // Note: compute_unit_price is in microlamports, so we divide by 1_000_000 to get lamports
        (limit as u64).saturating_mul(price) / 1_000_000
    }

    /// Extract compute unit information from transaction instructions
    pub fn extract_compute_unit_info(loaded_transaction: &impl VersionedTransactionExt) -> Self {
        let mut compute_unit_limit = None;
        let mut compute_unit_price = None;

        let transaction = loaded_transaction.get_transaction();
        let account_keys = loaded_transaction.get_all_account_keys();

        for instruction in transaction.message.instructions() {
            let program_id = account_keys[instruction.program_id_index as usize];
            if program_id != COMPUTE_BUDGET_ID {
                continue;
            }

            if instruction.data.is_empty() {
                continue;
            }

            match ComputeBudgetInstruction::try_from_slice(&instruction.data) {
                Ok(ComputeBudgetInstruction::SetComputeUnitLimit(limit)) => {
                    compute_unit_limit = Some(limit);
                }
                Ok(ComputeBudgetInstruction::SetComputeUnitPrice(price)) => {
                    compute_unit_price = Some(price);
                }
                _ => {}
            }
        }

        ComputeUnitInfo { compute_unit_limit, compute_unit_price }
    }
}

pub async fn estimate_transaction_fee(
    rpc_client: &RpcClient,
    // Should have resolved addresses for lookup tables
    resolved_transaction: &impl VersionedTransactionExt,
) -> Result<u64, KoraError> {
    let transaction = resolved_transaction.get_transaction();

    // Get base transaction fee
    let base_fee = get_estimate_fee(rpc_client, &transaction.message).await?;

    // Get account creation fees (for ATA creation)
    let account_creation_fee = get_associated_token_account_creation_fees(rpc_client, transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Extract compute unit information from transaction instructions
    let compute_unit_info = ComputeUnitInfo::extract_compute_unit_info(resolved_transaction);
    let priority_fee = compute_unit_info.calculate_priority_fee();

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
        .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {e}")))?;

    // Convert token amount to its real value based on decimals and multiply by SOL price
    let token_amount = amount as f64 / 10f64.powi(decimals as i32);
    let sol_amount = token_amount * token_price.price;

    // Convert SOL to lamports and round down
    let lamports = (sol_amount * LAMPORTS_PER_SOL as f64).floor() as u64;

    Ok(lamports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        message::{v0, VersionedMessage},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    use solana_compute_budget_interface::ComputeBudgetInstruction;
    use solana_system_interface::instruction::transfer;

    #[test]
    fn test_extract_compute_unit_info_with_both_instructions() {
        // Create a transaction with both SetComputeUnitLimit and SetComputeUnitPrice instructions
        let payer = Keypair::new();
        let to = Pubkey::new_unique();

        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(300_000),
            ComputeBudgetInstruction::set_compute_unit_price(50_000),
            transfer(&payer.pubkey(), &to, 1_000_000),
        ];

        let message = VersionedMessage::V0(
            v0::Message::try_compile(
                &payer.pubkey(),
                &instructions,
                &[],
                solana_sdk::hash::Hash::default(),
            )
            .unwrap(),
        );

        let transaction = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message,
        };

        let compute_info = ComputeUnitInfo::extract_compute_unit_info(&transaction);

        assert_eq!(compute_info.compute_unit_limit, Some(300_000));
        assert_eq!(compute_info.compute_unit_price, Some(50_000));

        // Test priority fee calculation: 300_000 * 50_000 / 1_000_000 = 15_000 lamports
        assert_eq!(compute_info.calculate_priority_fee(), 15_000);
    }

    #[test]
    fn test_extract_compute_unit_info_with_only_limit() {
        let payer = Keypair::new();
        let to = Pubkey::new_unique();

        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(150_000),
            transfer(&payer.pubkey(), &to, 1_000_000),
        ];

        let message = VersionedMessage::V0(
            v0::Message::try_compile(
                &payer.pubkey(),
                &instructions,
                &[],
                solana_sdk::hash::Hash::default(),
            )
            .unwrap(),
        );

        let transaction = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message,
        };

        let compute_info = ComputeUnitInfo::extract_compute_unit_info(&transaction);

        assert_eq!(compute_info.compute_unit_limit, Some(150_000));
        assert_eq!(compute_info.compute_unit_price, None);

        // Test priority fee calculation: 150_000 * 0 / 1_000_000 = 0 lamports (no price set)
        assert_eq!(compute_info.calculate_priority_fee(), 0);
    }

    #[test]
    fn test_extract_compute_unit_info_with_only_price() {
        let payer = Keypair::new();
        let to = Pubkey::new_unique();

        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_price(25_000),
            transfer(&payer.pubkey(), &to, 1_000_000),
        ];

        let message = VersionedMessage::V0(
            v0::Message::try_compile(
                &payer.pubkey(),
                &instructions,
                &[],
                solana_sdk::hash::Hash::default(),
            )
            .unwrap(),
        );

        let transaction = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message,
        };

        let compute_info = ComputeUnitInfo::extract_compute_unit_info(&transaction);

        assert_eq!(compute_info.compute_unit_limit, None);
        assert_eq!(compute_info.compute_unit_price, Some(25_000));

        // Test priority fee calculation: 200_000 (default) * 25_000 / 1_000_000 = 5_000 lamports
        assert_eq!(compute_info.calculate_priority_fee(), 5_000);
    }

    #[test]
    fn test_extract_compute_unit_info_no_compute_budget_instructions() {
        let payer = Keypair::new();
        let to = Pubkey::new_unique();

        let instructions = vec![transfer(&payer.pubkey(), &to, 1_000_000)];

        let message = VersionedMessage::V0(
            v0::Message::try_compile(
                &payer.pubkey(),
                &instructions,
                &[],
                solana_sdk::hash::Hash::default(),
            )
            .unwrap(),
        );

        let transaction = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message,
        };

        let compute_info = ComputeUnitInfo::extract_compute_unit_info(&transaction);

        assert_eq!(compute_info.compute_unit_limit, None);
        assert_eq!(compute_info.compute_unit_price, None);

        // Test priority fee calculation: 200_000 (default) * 0 (default) / 1_000_000 = 0 lamports
        assert_eq!(compute_info.calculate_priority_fee(), 0);
    }

    #[test]
    fn test_compute_unit_info_calculate_priority_fee_edge_cases() {
        // Test with maximum values to check for overflow
        let compute_info = ComputeUnitInfo {
            compute_unit_limit: Some(1_400_000), // Max allowed compute units
            compute_unit_price: Some(u64::MAX),
        };

        // Should use saturating_mul to prevent overflow
        let priority_fee = compute_info.calculate_priority_fee();
        assert!(priority_fee > 0); // Should not overflow to 0

        // Test with zero values
        let compute_info_zero =
            ComputeUnitInfo { compute_unit_limit: Some(0), compute_unit_price: Some(0) };
        assert_eq!(compute_info_zero.calculate_priority_fee(), 0);

        // Test calculation correctness with known values
        let compute_info_known = ComputeUnitInfo {
            compute_unit_limit: Some(1_000_000),
            compute_unit_price: Some(1_000_000), // 1 lamport per CU
        };
        // 1_000_000 * 1_000_000 / 1_000_000 = 1_000_000 lamports
        assert_eq!(compute_info_known.calculate_priority_fee(), 1_000_000);
    }
}
