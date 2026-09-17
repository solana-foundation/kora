use anyhow::Result;
use kora_lib::signer::KeypairUtil;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};
use std::str::FromStr;

use crate::common::constants::*;
#[cfg(test)]
use crate::common::TestContext;

/// Default fee for a transaction with 2 signers (5000 lamports each)
/// This is used for a lot of tests that only has sender and fee payer as signers
pub fn get_fee_for_default_transaction_in_usdc() -> u64 {
    // 10_000 lamports: 2 signers x 5_000 each at 0.001 SOL/USDC pricing
    10_000
}

pub fn parse_private_key_string(private_key: &str) -> Result<Keypair, String> {
    KeypairUtil::from_private_key_string(private_key).map_err(|e| e.to_string())
}

pub struct FeePayerTestHelper;

impl FeePayerTestHelper {
    pub fn get_fee_payer_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(KORA_PRIVATE_KEY_ENV)
                .expect("KORA_PRIVATE_KEY environment variable is not set"),
        )
        .expect("Failed to parse fee payer private key")
    }

    pub fn get_fee_payer_pubkey() -> Pubkey {
        Self::get_fee_payer_keypair().pubkey()
    }
}

pub struct SenderTestHelper;

impl SenderTestHelper {
    pub fn get_test_sender_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_SENDER_KEYPAIR_ENV)
                .expect("TEST_SENDER_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test sender private key")
    }
}

pub struct RecipientTestHelper;

impl RecipientTestHelper {
    pub fn get_recipient_pubkey() -> Pubkey {
        Pubkey::from_str(RECIPIENT_PUBKEY).expect("Invalid recipient pubkey")
    }
}

pub struct USDCMintTestHelper;

impl USDCMintTestHelper {
    pub fn get_test_usdc_mint_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_USDC_MINT_KEYPAIR_ENV)
                .expect("TEST_USDC_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test USDC mint private key")
    }

    pub fn get_test_usdc_mint_pubkey() -> Pubkey {
        Self::get_test_usdc_mint_keypair().pubkey()
    }

    pub fn get_test_usdc_mint_decimals() -> u8 {
        TEST_USDC_MINT_DECIMALS
    }
}

pub struct USDCMint2022TestHelper;

impl USDCMint2022TestHelper {
    pub fn get_test_usdc_mint_2022_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_USDC_MINT_2022_KEYPAIR_ENV)
                .expect("TEST_USDC_MINT_2022_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test USDC mint 2022 private key")
    }

    pub fn get_test_usdc_mint_2022_pubkey() -> Pubkey {
        Self::get_test_usdc_mint_2022_keypair().pubkey()
    }

    pub fn get_test_interest_bearing_mint_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV)
                .expect("TEST_INTEREST_BEARING_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test interest bearing mint private key")
    }

    pub fn get_test_transfer_hook_mint_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV)
                .expect("TEST_TRANSFER_HOOK_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test transfer hook mint private key")
    }
}

pub struct FeePayerPolicyMintTestHelper;

impl FeePayerPolicyMintTestHelper {
    pub fn get_fee_payer_policy_mint_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_FEE_PAYER_POLICY_MINT_KEYPAIR_ENV)
                .expect("TEST_FEE_PAYER_POLICY_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse fee payer policy mint private key")
    }

    pub fn get_fee_payer_policy_mint_pubkey() -> Pubkey {
        Self::get_fee_payer_policy_mint_keypair().pubkey()
    }

    pub fn get_fee_payer_policy_mint_2022_keypair() -> Keypair {
        parse_private_key_string(
            &std::env::var(TEST_FEE_PAYER_POLICY_MINT_2022_KEYPAIR_ENV)
                .expect("TEST_FEE_PAYER_POLICY_MINT_2022_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse fee payer policy mint 2022 private key")
    }

    pub fn get_fee_payer_policy_mint_2022_pubkey() -> Pubkey {
        Self::get_fee_payer_policy_mint_2022_keypair().pubkey()
    }
}

/// Concurrent tests sharing static keypairs can submit byte-identical setup
/// transactions within one blockhash window; the duplicate is rejected as
/// already processed even though the intended state change landed.
pub async fn send_and_confirm_allow_duplicate(
    rpc_client: &RpcClient,
    transaction: &Transaction,
) -> Result<()> {
    match rpc_client.send_and_confirm_transaction(transaction).await {
        Ok(_) => Ok(()),
        Err(e) if is_already_processed(&e) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Surfnet reports a duplicate signature as a bare `-32002` string instead of
/// putting a `TransactionError` in the error data, so the structured check that
/// works against agave misses it.
fn is_already_processed(error: &solana_client::client_error::ClientError) -> bool {
    error.get_transaction_error() == Some(TransactionError::AlreadyProcessed)
        || error.to_string().contains("already been processed")
}

#[cfg(test)]
pub async fn create_funded_wallet(ctx: &TestContext) -> Keypair {
    let wallet = Keypair::new();
    let sig = ctx
        .rpc_client()
        .request_airdrop(&wallet.pubkey(), 1_000_000_000)
        .await
        .expect("Failed to request airdrop");
    loop {
        if ctx.rpc_client().confirm_transaction(&sig).await.unwrap_or(false) {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    wallet
}
