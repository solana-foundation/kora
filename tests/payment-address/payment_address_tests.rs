use crate::common::*;
use jsonrpsee::{core::client::ClientT, rpc_params};
use kora_lib::{
    token::{TokenInterface, TokenProgram},
    transaction::{TransactionUtil, VersionedTransactionOps},
};
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use std::str::FromStr;

#[tokio::test]
async fn test_sign_transaction_if_paid_with_payment_address() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;
    let sender = SenderTestHelper::get_test_sender_keypair();
    let payment_address = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let payment_address_token_account = get_associated_token_address(&payment_address, &test_mint);
    let fee_amount = 10000;

    let token_interface = TokenProgram::new();
    let fee_payer_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &payment_address_token_account,
            &sender.pubkey(),
            fee_amount,
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[fee_payer_instruction],
        Some(&fee_payer),
        &recent_blockhash,
    ));

    // Create transaction and sign with sender
    let mut resolved_transaction =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
    let sender_position = resolved_transaction
        .find_signer_position(&sender.pubkey())
        .expect("Sender not found in account keys");
    let signature = sender.sign_message(&resolved_transaction.transaction.message.serialize());
    resolved_transaction.transaction.signatures[sender_position] = signature;

    let encoded_tx = resolved_transaction.encode_b64_transaction().unwrap();

    // Call signTransactionIfPaid endpoint - should succeed when payment goes to correct address
    let response: serde_json::Value = client
        .request("signTransactionIfPaid", rpc_params![encoded_tx])
        .await
        .expect("Failed to sign transaction");

    assert!(
        response["signed_transaction"].as_str().is_some(),
        "Expected signed_transaction in response"
    );
}

#[tokio::test]
async fn test_sign_transaction_if_paid_with_wrong_destination() {
    let client = ClientTestHelper::get_test_client().await;
    let rpc_client = RPCTestHelper::get_rpc_client().await;
    let fee_payer = FeePayerTestHelper::get_fee_payer_keypair();
    let sender = SenderTestHelper::get_test_sender_keypair();
    let wrong_destination = Keypair::new(); // Random wrong destination
    let test_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    // Create a transfer to the WRONG destination (not the payment address)
    let sender_token_account = get_associated_token_address(&sender.pubkey(), &test_mint);
    let wrong_dest_ata = get_associated_token_address(&wrong_destination.pubkey(), &test_mint);

    let create_wrong_ata_idempotent_ix = create_associated_token_account_idempotent(
        &fee_payer.pubkey(),
        &wrong_destination.pubkey(),
        &test_mint,
        &spl_token::id(),
    );

    let fee_amount = 10000;

    let token_interface = TokenProgram::new();
    let fee_payer_instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &wrong_dest_ata,
            &sender.pubkey(),
            fee_amount,
        )
        .unwrap();

    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[create_wrong_ata_idempotent_ix, fee_payer_instruction],
        Some(&fee_payer),
        &recent_blockhash,
    ));

    // Create transaction and sign with sender
    let mut resolved_transaction =
        TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

    let sender_position = resolved_transaction
        .find_signer_position(&sender.pubkey())
        .expect("Sender not found in account keys");
    let signature = sender.sign_message(&resolved_transaction.transaction.message.serialize());
    resolved_transaction.transaction.signatures[sender_position] = signature;

    let encoded_tx = resolved_transaction.encode_b64_transaction().unwrap();

    // Call signTransactionIfPaid endpoint - should fail when payment goes to wrong address
    let response: Result<serde_json::Value, _> =
        client.request("signTransactionIfPaid", rpc_params![encoded_tx]).await;

    assert!(response.is_err(), "Expected payment validation to fail for wrong destination");
}
