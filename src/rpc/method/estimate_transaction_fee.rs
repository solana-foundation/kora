use std::sync::Arc;

use crate::common::{error::KoraError, transaction::decode_b58_transaction};

use serde::{Deserialize, Serialize};
use borsh::BorshDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::{
    id as associated_token_program_id,
    instruction::AssociatedTokenAccountInstruction,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeRequest {
    pub transaction: String, // Base58 encoded serialized transaction
    pub fee_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionFeeResponse {
    pub fee_in_lamports: u64,
}

async fn get_account_creation_fees(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> Result<u64, KoraError> {
    const ATA_ACCOUNT_SIZE: usize = 165; // Token-2022 support can be added by making this configurable

    let mut ata_count = 0u64;

    // Check each instruction in the transaction
    for instruction in &transaction.message.instructions {
        let program_id = transaction.message.account_keys[instruction.program_id_index as usize];

        // Skip if not an ATA program instruction
        if program_id != associated_token_program_id() {
            continue;
        }

        // Try to parse the instruction data
        if let Ok(ata_instruction) = AssociatedTokenAccountInstruction::try_from_slice(&instruction.data) {
            match ata_instruction {
                AssociatedTokenAccountInstruction::Create |
                AssociatedTokenAccountInstruction::CreateIdempotent => {
                    // Check if account already exists
                    let account_pubkey = transaction.message.account_keys[instruction.accounts[1] as usize];
                    match rpc_client.get_account(&account_pubkey).await {
                        Ok(account) if account.owner == associated_token_program_id() => continue,
                        _ => ata_count += 1,
                    }
                }
                _ => {}
            }
        }
    }

    if ata_count == 0 {
        return Ok(0);
    }

    // Get rent exemption once for all ATAs
    let rent = rpc_client
        .get_minimum_balance_for_rent_exemption(ATA_ACCOUNT_SIZE)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    Ok(rent * ata_count)
}

pub async fn estimate_transaction_fee(
    rpc_client: &Arc<RpcClient>,
    request: EstimateTransactionFeeRequest,
) -> Result<EstimateTransactionFeeResponse, KoraError> {
    let transaction = decode_b58_transaction(&request.transaction)?;

    let fee = rpc_client
        .get_fee_for_message(&transaction.message)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Get account creation fees
    let account_creation_fee = get_account_creation_fees(rpc_client, &transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Get priority fee from recent blocks
    let priority_stats = rpc_client
        .get_recent_prioritization_fees(&[])
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;
    let priority_fee = priority_stats.iter().map(|fee| fee.prioritization_fee).max().unwrap_or(0);

    Ok(EstimateTransactionFeeResponse {
        fee_in_lamports: fee + priority_fee + account_creation_fee,
    })
}

#[cfg(test)]
mod tests {
    use crate::rpc::method::estimate_transaction_fee::{
        estimate_transaction_fee, EstimateTransactionFeeRequest,
    };
    use serde_json::json;
    use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
    use solana_sdk::{
        message::Message, pubkey::Pubkey, system_instruction, transaction::Transaction,
    };
    use std::{collections::HashMap, sync::Arc};

    fn setup_test_rpc_client() -> Arc<RpcClient> {
        // Create a mock RPC client that returns predefined responses
        let rpc_url = "http://localhost:8899".to_string();
        let mut mocks = HashMap::new();
        // Add mock response for GetMinimumBalanceForRentExemption
        mocks.insert(
            RpcRequest::GetMinimumBalanceForRentExemption,
            json!(2_039_280)
        );
        Arc::new(RpcClient::new_mock_with_mocks(rpc_url, mocks))
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_basic() {
        let rpc_client = setup_test_rpc_client();

        // Create a simple transfer transaction
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let instruction = system_instruction::transfer(&from, &to, 1000);
        let message = Message::new(&[instruction], Some(&from));
        let transaction = Transaction { message, signatures: vec![Default::default()] };

        let serialized = bincode::serialize(&transaction).unwrap();
        let encoded = bs58::encode(serialized).into_string();

        let request =
            EstimateTransactionFeeRequest { transaction: encoded, fee_token: "SOL".to_string() };

        let result = estimate_transaction_fee(&rpc_client, request).await;
        assert!(result.is_ok());

        let fee_response = result.unwrap();
        // Base fee + priority fee
        assert!(fee_response.fee_in_lamports > 0);
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_invalid_transaction() {
        let rpc_client = setup_test_rpc_client();

        let request = EstimateTransactionFeeRequest {
            transaction: "invalid_transaction".to_string(),
            fee_token: "SOL".to_string(),
        };

        let result = estimate_transaction_fee(&rpc_client, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_with_token_creation() {
        let rpc_client = setup_test_rpc_client();

        // Create a transaction that includes token account creation
        let payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let ata = spl_associated_token_account::get_associated_token_address(&owner, &mint);
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer,
                &ata,
                &mint,
                &spl_token::id(),
            );

        let message = Message::new(&[create_ata_ix], Some(&payer));
        let transaction = Transaction { message, signatures: vec![Default::default()] };

        let serialized = bincode::serialize(&transaction).unwrap();
        let encoded = bs58::encode(serialized).into_string();

        let request =
            EstimateTransactionFeeRequest { transaction: encoded, fee_token: "SOL".to_string() };

        let result = estimate_transaction_fee(&rpc_client, request).await;
        assert!(result.is_ok());

        let fee_response = result.unwrap();
        // Fee should include base fee + priority fee + rent for token account
        // Fee should be at least the minimum rent-exempt amount for a token account (~0.00204 SOL)
        let min_expected_lamports = 2_039_280;
        assert!(
            fee_response.fee_in_lamports >= min_expected_lamports,
            "Fee {} lamports is less than minimum expected {} lamports",
            fee_response.fee_in_lamports,
            min_expected_lamports
        );
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_with_multiple_ata_creation() {
        let rpc_client = setup_test_rpc_client();

        // Create a transaction that creates multiple token accounts
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        let mint3 = Pubkey::new_unique();

        // Get ATAs for each mint
        let ata1 = spl_associated_token_account::get_associated_token_address(&owner, &mint1);
        let ata2 = spl_associated_token_account::get_associated_token_address(&owner, &mint2); 
        let ata3 = spl_associated_token_account::get_associated_token_address(&owner, &mint3);

        // Create instructions for each ATA
        let create_ata1_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer,
            &ata1,
            &mint1,
            &spl_token::id(),
        );
        let create_ata2_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer,
            &ata2, 
            &mint2,
            &spl_token::id(),
        );
        let create_ata3_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer,
            &ata3,
            &mint3, 
            &spl_token::id(),
        );

        let message = Message::new(&[create_ata1_ix, create_ata2_ix, create_ata3_ix], Some(&payer));
        let transaction = Transaction { message, signatures: vec![Default::default()] };

        let serialized = bincode::serialize(&transaction).unwrap();
        let encoded = bs58::encode(serialized).into_string();

        let request = EstimateTransactionFeeRequest {
            transaction: encoded,
            fee_token: "SOL".to_string(),
        };

        let result = estimate_transaction_fee(&rpc_client, request).await;
        assert!(result.is_ok());

        let fee_response = result.unwrap();
        // Fee should include base fee + priority fee + rent for 3 token accounts
        // Minimum rent-exempt amount for 3 token accounts (~0.00612 SOL)
        let min_expected_lamports = 3 * 2_039_280;
        assert!(
            fee_response.fee_in_lamports >= min_expected_lamports,
            "Fee {} lamports is less than minimum expected {} lamports",
            fee_response.fee_in_lamports,
            min_expected_lamports
        );
    }
}
