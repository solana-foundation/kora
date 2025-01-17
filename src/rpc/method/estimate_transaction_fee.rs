use std::sync::Arc;

use crate::common::{error::KoraError, transaction::decode_b58_transaction};

use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    message::Message, pubkey::Pubkey, system_instruction, transaction::Transaction,
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
    let mut total_creation_fee = 0u64;

    // Get rent exemption amount for a new token account
    let rent = rpc_client
        .get_minimum_balance_for_rent_exemption(165)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Check each account in the transaction
    for instruction in &transaction.message.instructions {
        let program_id = transaction.message.account_keys[instruction.program_id_index as usize];

        // Check if instruction is creating an associated token account
        if program_id == spl_associated_token_account::ID {
            // Associated token account creation instruction has 4 accounts:
            // 0. Funding account (payer)
            // 1. Associated token account address
            // 2. Wallet address
            // 3. Token mint address
            if instruction.accounts.len() == 4 {
                // Check if the token account doesn't exist yet
                let token_account =
                    transaction.message.account_keys[instruction.accounts[1] as usize];
                if let Ok(_) = rpc_client.get_account(&token_account).await {
                    continue;
                }
                total_creation_fee += rent;
            }
        }
    }

    Ok(total_creation_fee)
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
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::{
        message::Message, pubkey::Pubkey, system_instruction, transaction::Transaction,
    };
    use std::sync::Arc;

    fn setup_test_rpc_client() -> Arc<RpcClient> {
        // Create a mock RPC client that returns predefined responses
        let rpc_url = "http://localhost:8899".to_string();
        Arc::new(RpcClient::new_mock(rpc_url))
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
}
