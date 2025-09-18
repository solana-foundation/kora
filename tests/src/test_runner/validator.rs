use crate::{
    common::constants::DEFAULT_RPC_URL,
    test_runner::accounts::{get_account_address_from_file, AccountFile},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::path::Path;
use tokio::process::Child;

const TRANSFER_HOOK_PROGRAM_ID: &str = "Bcdikjss8HWzKEuj6gEQoFq9TCnGnk6v3kUnRU1gb6hA";
const TRANSFER_HOOK_PROGRAM_PATH: &str =
    "tests/src/common/transfer-hook-example/transfer_hook_example.so";

pub async fn check_test_validator(rpc_url: &str) -> bool {
    let client = RpcClient::new(rpc_url.to_string());
    client.get_health().await.is_ok()
}

pub async fn start_test_validator(
    load_accounts: bool,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("solana-test-validator");
    cmd.arg("--reset").arg("--quiet");

    if Path::new(TRANSFER_HOOK_PROGRAM_PATH).exists() {
        cmd.arg("--bpf-program").arg(TRANSFER_HOOK_PROGRAM_ID).arg(TRANSFER_HOOK_PROGRAM_PATH);
    } else {
        println!("⚠️  Transfer hook program not found at: {TRANSFER_HOOK_PROGRAM_PATH}");
        println!("   Starting validator without transfer hook program");
        println!("   Run 'make build-transfer-hook' to build it if needed");
    }

    if load_accounts {
        for account_file in AccountFile::required_test_accounts() {
            let account_path = account_file.test_account_path();
            if account_path.exists() {
                if let Ok(account_address) = get_account_address_from_file(&account_path) {
                    cmd.arg("--account").arg(&account_address).arg(&account_path);
                    println!(
                        "Loading account: {} from {}",
                        account_address,
                        account_path.display()
                    );
                }
            }
        }
    }

    let validator_pid =
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;

    let mut attempts = 0;
    while !check_test_validator(DEFAULT_RPC_URL).await {
        attempts += 1;
        if attempts > 30 {
            return Err("Solana test validator failed to start within 60 seconds".into());
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    println!("Solana test validator started successfully");
    Ok(validator_pid)
}
