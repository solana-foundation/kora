use base64::{engine::general_purpose::STANDARD, Engine as _};
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer as _,
    transaction::{Transaction, VersionedTransaction},
};
use solana_system_interface::instruction::transfer;

use super::{decode_b64_transaction, estimate_transaction_fee};
use crate::{
    rpc::test_utils::setup_test_rpc_client,
    token::{TokenInterface, TokenProgram, TokenType},
};

#[test]
fn test_decode_b64_transaction() {
    let keypair = Keypair::new();
    let instruction = Instruction::new_with_bytes(
        Pubkey::new_unique(),
        &[1, 2, 3],
        vec![AccountMeta::new(keypair.pubkey(), true)],
    );
    let message = Message::new(&[instruction], Some(&keypair.pubkey()));
    let legacy_tx = Transaction::new(&[&keypair], message, Hash::default());

    // Convert legacy transaction to versioned transaction for encoding
    let tx = VersionedTransaction {
        signatures: legacy_tx.signatures,
        message: VersionedMessage::Legacy(legacy_tx.message),
    };

    let encoded = STANDARD.encode(bincode::serialize(&tx).unwrap());
    let decoded = decode_b64_transaction(&encoded).unwrap();

    assert_eq!(tx, decoded);
}

#[test]
fn test_decode_b64_transaction_invalid_input() {
    let result = decode_b64_transaction("not-base64!");
    assert!(matches!(result, Err(crate::error::KoraError::InvalidTransaction(_))));

    let result = decode_b64_transaction("AQID"); // base64 of [1,2,3]
    assert!(matches!(result, Err(crate::error::KoraError::InvalidTransaction(_))));
}

#[tokio::test]
async fn test_estimate_transaction_fee_basic() {
    let rpc_client = setup_test_rpc_client();

    // Create a simple transfer transaction
    let from = Pubkey::new_unique();
    let to = Pubkey::new_unique();
    let instruction = transfer(&from, &to, 1000);
    let message = Message::new(&[instruction], Some(&from));
    let transaction = VersionedTransaction {
        signatures: vec![Default::default()],
        message: VersionedMessage::Legacy(message),
    };

    let fee = estimate_transaction_fee(&rpc_client, &transaction).await.unwrap();

    // Base fee + priority fee
    assert!(fee > 0);
    // Fee should be less than the minimum rent-exempt amount for a token account (~0.00204 SOL)
    assert!(fee < 2_039_280);
}

#[tokio::test]
async fn test_estimate_transaction_fee_invalid_transaction() {
    let rpc_client = setup_test_rpc_client();

    // Create an invalid transaction (empty message)
    let transaction = VersionedTransaction {
        signatures: vec![],
        message: VersionedMessage::Legacy(Message::default()),
    };

    let result = estimate_transaction_fee(&rpc_client, &transaction).await;
    assert!(result.is_ok()); // Fee estimation should still work for invalid transactions
}

#[tokio::test]
async fn test_estimate_transaction_fee_with_token_creation() {
    let rpc_client = setup_test_rpc_client();

    // Create a transaction that includes token account creation
    let payer = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let token_program = TokenProgram::new(TokenType::Spl);
    let _ata = token_program.get_associated_token_address(&owner, &mint);
    let create_ata_ix =
        token_program.create_associated_token_account_instruction(&payer, &owner, &mint);

    let message = Message::new(&[create_ata_ix], Some(&payer));
    let transaction = VersionedTransaction {
        signatures: vec![Default::default()],
        message: VersionedMessage::Legacy(message),
    };

    let fee = estimate_transaction_fee(&rpc_client, &transaction).await.unwrap();

    // Fee should include base fee + priority fee + rent for token account
    let min_expected_lamports = 2_039_280;
    assert!(
        fee >= min_expected_lamports,
        "Fee {} lamports is less than minimum expected {} lamports",
        fee,
        min_expected_lamports
    );
}

#[test]
fn test_token_functionality() {
    let token_program = TokenProgram::new(TokenType::Spl);
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let ata = token_program.get_associated_token_address(&owner, &mint);
    assert_ne!(ata, owner);
    assert_ne!(ata, mint);
}
