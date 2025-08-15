use crate::common::*;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::{process::Command, str::FromStr};

/// Test the new CLI structure and initialize-atas command
#[tokio::test]
async fn test_cli_initialize_atas_command() {
    let rpc_client = RPCTestHelper::get_rpc_client().await;
    let paymaster_pubkey = Pubkey::from_str(TEST_PAYMENT_ADDRESS).unwrap();
    let test_usdc_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();

    let expected_ata = get_associated_token_address(&paymaster_pubkey, &test_usdc_mint);

    let sender_key_path = FEE_PAYER_DEFAULT;

    let output = Command::new("cargo")
        .current_dir("../..") // Set working directory to kora root
        .args([
            "run",
            "-p",
            "kora-cli",
            "--bin",
            "kora",
            "--",
            "--config",
            "tests/common/fixtures/paymaster-address-test.toml",
            "--rpc-url",
            rpc_client.url().as_str(),
            "rpc",
            "initialize-atas",
            "--private-key",
            sender_key_path,
        ])
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "initialize-atas command failed with exit code: {}\nSTDOUT: {}\nSTDERR: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
    }

    // Verify the ATA was created successfully
    assert!(
        rpc_client.get_account(&expected_ata).await.is_ok(),
        "ATA should exist after successful initialization"
    );
}
