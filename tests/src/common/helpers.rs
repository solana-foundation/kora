use anyhow::Result;
use kora_lib::signer::KeypairUtil;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

use crate::common::constants::*;
#[cfg(test)]
use crate::common::TestContext;

/// Default fee for a transaction with 2 signers (5000 lamports each)
/// This is used for a lot of tests that only has sender and fee payer as signers
pub fn get_fee_for_default_transaction_in_usdc() -> u64 {
    // 10 000 USDC priced at default 0.001 SOL / USDC (Mock pricing) (6 decimals), so 0.01 USDC
    // 10 000 lamports required (2 x 5000 for signatures) (9 decimals), so 0.00001 SOL
    //
    // Required SOL amount is 0.01 (usdc amount) * 0.001 (usdc price) = 0.00001 SOL
    // Required lamports is 0.00001 SOL * 10^9 (lamports per SOL) = 10 000 lamports
    10_000
}

/// Helper function to parse a private key string in multiple formats.
pub fn parse_private_key_string(private_key: &str) -> Result<Keypair, String> {
    KeypairUtil::from_private_key_string(private_key).map_err(|e| e.to_string())
}

pub struct FeePayerTestHelper;

impl FeePayerTestHelper {
    pub fn get_fee_payer_keypair() -> Keypair {
        dotenv::dotenv().ok();
        parse_private_key_string(
            &std::env::var(KORA_PRIVATE_KEY_ENV)
                .expect("KORA_PRIVATE_KEY environment variable is not set"),
        )
        .expect("Failed to parse fee payer private key")
    }

    pub fn get_fee_payer_pubkey() -> Pubkey {
        Self::get_fee_payer_keypair().pubkey()
    }

    pub fn get_signer_2_keypair() -> Keypair {
        dotenv::dotenv().ok();
        parse_private_key_string(
            &std::env::var(SIGNER_2_KEYPAIR_ENV)
                .expect("SIGNER_2_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse signer 2 private key")
    }

    pub fn get_signer_2_pubkey() -> Pubkey {
        Self::get_signer_2_keypair().pubkey()
    }
}

pub struct SenderTestHelper;

impl SenderTestHelper {
    pub fn get_test_sender_keypair() -> Keypair {
        dotenv::dotenv().ok();
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
        dotenv::dotenv().ok();
        let recipient_str = std::env::var(TEST_RECIPIENT_PUBKEY_ENV)
            .unwrap_or_else(|_| RECIPIENT_PUBKEY.to_string());
        Pubkey::from_str(&recipient_str).expect("Invalid recipient pubkey")
    }
}

pub struct USDCMintTestHelper;

impl USDCMintTestHelper {
    pub fn get_test_usdc_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
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
        dotenv::dotenv().ok();
        std::env::var(TEST_USDC_MINT_DECIMALS_ENV)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(TEST_USDC_MINT_DECIMALS)
    }
}

pub struct USDCMint2022TestHelper;

impl USDCMint2022TestHelper {
    pub fn get_test_usdc_mint_2022_keypair() -> Keypair {
        dotenv::dotenv().ok();

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
        dotenv::dotenv().ok();

        parse_private_key_string(
            &std::env::var(TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV)
                .expect("TEST_INTEREST_BEARING_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test interest bearing mint private key")
    }

    pub fn get_test_interest_bearing_mint_pubkey() -> Pubkey {
        Self::get_test_interest_bearing_mint_keypair().pubkey()
    }

    pub fn get_test_transfer_hook_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();

        parse_private_key_string(
            &std::env::var(TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV)
                .expect("TEST_TRANSFER_HOOK_MINT_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse test transfer hook mint private key")
    }

    pub fn get_test_transfer_hook_mint_pubkey() -> Pubkey {
        Self::get_test_transfer_hook_mint_keypair().pubkey()
    }
}

pub struct PaymentAddressTestHelper;

impl PaymentAddressTestHelper {
    pub fn get_payment_address_keypair() -> Keypair {
        dotenv::dotenv().ok();
        parse_private_key_string(
            &std::env::var(PAYMENT_ADDRESS_KEYPAIR_ENV)
                .expect("PAYMENT_ADDRESS_KEYPAIR environment variable is not set"),
        )
        .expect("Failed to parse payment address private key")
    }

    pub fn get_payment_address_pubkey() -> Pubkey {
        Self::get_payment_address_keypair().pubkey()
    }

    pub fn get_payment_test_address_pubkey() -> Pubkey {
        Pubkey::from_str(TEST_PAYMENT_ADDRESS).expect("Invalid payment test address")
    }
}

pub struct PYUSDTestHelper;

impl PYUSDTestHelper {
    pub fn get_pyusd_mint_pubkey() -> Pubkey {
        Pubkey::from_str(PYUSD_MINT).expect("Invalid PYUSD mint")
    }
}

pub struct FeePayerPolicyMintTestHelper;

impl FeePayerPolicyMintTestHelper {
    pub fn get_fee_payer_policy_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
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
        dotenv::dotenv().ok();
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
