use crate::{
    constant::LAMPORTS_PER_SIGNATURE,
    error::KoraError,
    get_signer,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle},
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::{get_estimate_fee, VersionedTransactionExt},
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
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

pub fn is_fee_payer_in_signers(
    transaction: &impl VersionedTransactionExt,
) -> Result<bool, KoraError> {
    let fee_payer = get_signer()
        .map(|signer| signer.solana_pubkey())
        .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer: {e}")))?;

    let all_account_keys = transaction.get_all_account_keys();
    let transaction_inner = transaction.get_transaction();

    // In messages, the first num_required_signatures accounts are signers
    Ok(match &transaction_inner.message {
        VersionedMessage::Legacy(legacy_message) => {
            let num_signers = legacy_message.header.num_required_signatures as usize;
            all_account_keys.iter().take(num_signers).any(|key| *key == fee_payer)
        }
        VersionedMessage::V0(v0_message) => {
            let num_signers = v0_message.header.num_required_signatures as usize;
            all_account_keys.iter().take(num_signers).any(|key| *key == fee_payer)
        }
    })
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

    // Priority fees are now included in the calculate done by the RPC getFeeForMessage

    // If the Kora signer is not inclded in the signers, we add another base fee, since each transaction will be 5000 lamports
    let mut kora_signature_fee = 0u64;
    if !is_fee_payer_in_signers(resolved_transaction)? {
        kora_signature_fee = LAMPORTS_PER_SIGNATURE;
    }

    Ok(base_fee + account_creation_fee + kora_signature_fee)
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
    use crate::{
        signer::{KoraSigner, SolanaMemorySigner},
        state::init_signer,
        transaction::{new_unsigned_versioned_transaction, VersionedTransactionResolved},
    };
    use solana_message::{v0, Message};
    use solana_sdk::{
        hash::Hash,
        signature::{Keypair, Signer},
    };
    use solana_system_interface::instruction::transfer;

    fn setup_or_get_test_signer() -> Pubkey {
        if let Ok(signer) = get_signer() {
            return signer.solana_pubkey();
        }

        let test_keypair = Keypair::new();
        let signer = SolanaMemorySigner::new(test_keypair.insecure_clone());
        match init_signer(KoraSigner::Memory(signer)) {
            Ok(_) => test_keypair.pubkey(),
            Err(_) => {
                // Signer already initialized, get it
                get_signer().expect("Signer should be available").solana_pubkey()
            }
        }
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_not_signer() {
        setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&sender.pubkey(), &recipient.pubkey(), 1000);

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(!is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &fee_payer,
            &[transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_not_signer() {
        setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &sender.pubkey(),
            &[transfer(&sender.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(!is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }
}
