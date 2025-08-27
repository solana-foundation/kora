use anyhow::Result;
use kora_lib::signer::KeypairUtil;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

use crate::common::constants::*;

/// Test account information for outputting to the user
#[derive(Debug)]
pub struct TestAccountInfo {
    pub fee_payer_pubkey: Pubkey,
    pub sender_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint_pubkey: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipient_token_account: Pubkey,
    pub fee_payer_token_account: Pubkey,
}

/// Helper function to parse a private key string in multiple formats.
pub fn parse_private_key_string(private_key: &str) -> Result<Keypair, String> {
    KeypairUtil::from_private_key_string(private_key).map_err(|e| e.to_string())
}

pub struct FeePayerTestHelper;

impl FeePayerTestHelper {
    pub fn get_fee_payer_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let private_key = match std::env::var("KORA_PRIVATE_KEY") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(FEE_PAYER_KEYPAIR_PATH)
                .expect("Failed to read fee payer private key file"),
        };
        parse_private_key_string(&private_key).expect("Failed to parse fee payer private key")
    }

    pub fn get_fee_payer_pubkey() -> Pubkey {
        Self::get_fee_payer_keypair().pubkey()
    }
}

pub struct SenderTestHelper;

impl SenderTestHelper {
    pub fn get_test_sender_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let private_key = match std::env::var("TEST_SENDER_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(SENDER_KEYPAIR_PATH)
                .expect("Failed to read sender private key file"),
        };
        parse_private_key_string(&private_key).expect("Failed to parse test sender private key")
    }
}

pub struct RecipientTestHelper;

impl RecipientTestHelper {
    pub fn get_recipient_pubkey() -> Pubkey {
        dotenv::dotenv().ok();
        let recipient_str =
            std::env::var("TEST_RECIPIENT_PUBKEY").unwrap_or_else(|_| RECIPIENT_PUBKEY.to_string());
        Pubkey::from_str(&recipient_str).expect("Invalid recipient pubkey")
    }
}

pub struct USDCMintTestHelper;

impl USDCMintTestHelper {
    pub fn get_test_usdc_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let mint_keypair = match std::env::var("TEST_USDC_MINT_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(USDC_MINT_KEYPAIR_PATH)
                .expect("Failed to read USDC mint private key file"),
        };
        parse_private_key_string(&mint_keypair).expect("Failed to parse test USDC mint private key")
    }

    pub fn get_test_usdc_mint_pubkey() -> Pubkey {
        Self::get_test_usdc_mint_keypair().pubkey()
    }

    pub fn get_test_usdc_mint_decimals() -> u8 {
        dotenv::dotenv().ok();
        std::env::var("TEST_USDC_MINT_DECIMALS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(TEST_USDC_MINT_DECIMALS)
    }
}
