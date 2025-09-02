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
    // Token 2022 fields
    pub usdc_mint_2022_pubkey: Pubkey,
    pub sender_token_2022_account: Pubkey,
    pub recipient_token_2022_account: Pubkey,
    pub fee_payer_token_2022_account: Pubkey,
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

pub struct USDCMint2022TestHelper;

impl USDCMint2022TestHelper {
    pub fn get_test_usdc_mint_2022_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let mint_keypair = match std::env::var("TEST_USDC_MINT_2022_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(USDC_MINT_2022_KEYPAIR_PATH)
                .expect("Failed to read USDC mint 2022 private key file"),
        };
        parse_private_key_string(&mint_keypair)
            .expect("Failed to parse test USDC mint 2022 private key")
    }

    pub fn get_test_usdc_mint_2022_pubkey() -> Pubkey {
        Self::get_test_usdc_mint_2022_keypair().pubkey()
    }

    pub fn get_test_interest_bearing_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let mint_keypair = match std::env::var("TEST_INTEREST_BEARING_MINT_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(INTEREST_BEARING_MINT_KEYPAIR_PATH)
                .expect("Failed to read interest bearing mint private key file"),
        };
        parse_private_key_string(&mint_keypair)
            .expect("Failed to parse test interest bearing mint private key")
    }

    pub fn get_test_interest_bearing_mint_pubkey() -> Pubkey {
        Self::get_test_interest_bearing_mint_keypair().pubkey()
    }

    pub fn get_test_transfer_hook_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let mint_keypair = match std::env::var("TEST_TRANSFER_HOOK_MINT_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(TRANSFER_HOOK_MINT_KEYPAIR_PATH)
                .expect("Failed to read transfer hook mint private key file"),
        };
        parse_private_key_string(&mint_keypair)
            .expect("Failed to parse test transfer hook mint private key")
    }

    pub fn get_test_transfer_hook_mint_pubkey() -> Pubkey {
        Self::get_test_transfer_hook_mint_keypair().pubkey()
    }
}
