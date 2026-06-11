use crate::{
    common::{
        constants::{
            DEFAULT_RPC_URL, LIGHTHOUSE_PROGRAM_ID, LIGHTHOUSE_PROGRAM_PATH,
            TRANSFER_HOOK_PROGRAM_ID, TRANSFER_HOOK_PROGRAM_PATH,
        },
        surfnet::SURFPOOL_BACKEND,
    },
    test_runner::accounts::{get_account_address_from_file, AccountFile},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::path::Path;
use tokio::process::Child;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ValidatorBackend {
    Surfpool,
    Agave,
}

impl ValidatorBackend {
    pub fn env_value(&self) -> &'static str {
        match self {
            Self::Surfpool => SURFPOOL_BACKEND,
            Self::Agave => "agave",
        }
    }
}

pub async fn check_test_validator(rpc_url: &str) -> bool {
    let client = RpcClient::new_with_commitment(
        rpc_url.to_string(),
        solana_commitment_config::CommitmentConfig::confirmed(),
    );
    client.get_health().await.is_ok()
}

pub async fn start_test_validator(
    backend: ValidatorBackend,
    load_accounts: bool,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    if check_test_validator(DEFAULT_RPC_URL).await {
        return Err(format!(
            "A validator is already running on {DEFAULT_RPC_URL}; the new one would fail to \
            bind and tests would silently run against stale state. Stop it first \
            (pkill -f solana-test-validator; pkill -f 'surfpool start')."
        )
        .into());
    }

    let validator_pid = match backend {
        ValidatorBackend::Surfpool => spawn_surfpool().await?,
        ValidatorBackend::Agave => spawn_agave(load_accounts).await?,
    };

    let mut attempts = 0;
    let mut delay = std::time::Duration::from_millis(100);
    let max_delay = std::time::Duration::from_secs(2);
    let max_attempts = 15;

    while !check_test_validator(DEFAULT_RPC_URL).await {
        attempts += 1;
        if attempts > max_attempts {
            return Err(format!(
                "{backend:?} test validator failed to start within {max_attempts} attempts"
            )
            .into());
        }

        tokio::time::sleep(delay).await;
        delay = std::cmp::min(delay * 2, max_delay);
    }

    println!("{backend:?} test validator started successfully");
    Ok(validator_pid)
}

async fn spawn_surfpool() -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let workdir = std::env::temp_dir().join("kora-surfpool");
    tokio::fs::create_dir_all(&workdir).await?;

    let mut cmd = tokio::process::Command::new("surfpool");
    cmd.args([
        "start",
        "--no-tui",
        "--no-deploy",
        "--no-studio",
        "--offline",
        "-y",
        "--port",
        "8899",
        "--slot-time",
        "100",
    ])
    .current_dir(&workdir)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());

    cmd.spawn().map_err(|e| {
        format!(
            "Failed to spawn surfpool ({e}). Install it (https://docs.surfpool.run) \
            or run with --backend agave."
        )
        .into()
    })
}

async fn spawn_agave(
    load_accounts: bool,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("solana-test-validator");
    // 32 ticks/slot = ~200ms slots: halves confirmation latency while keeping
    // blockhash wall-clock expiry (~30s) safe for concurrent test phases
    cmd.arg("--reset").arg("--quiet").arg("--ticks-per-slot").arg("32");

    if Path::new(TRANSFER_HOOK_PROGRAM_PATH).exists() {
        cmd.arg("--bpf-program").arg(TRANSFER_HOOK_PROGRAM_ID).arg(TRANSFER_HOOK_PROGRAM_PATH);
    } else {
        println!("⚠️  Transfer hook program not found at: {TRANSFER_HOOK_PROGRAM_PATH}");
        println!("   Starting validator without transfer hook program");
    }

    if Path::new(LIGHTHOUSE_PROGRAM_PATH).exists() {
        cmd.arg("--bpf-program").arg(LIGHTHOUSE_PROGRAM_ID).arg(LIGHTHOUSE_PROGRAM_PATH);
    } else {
        println!("⚠️  Lighthouse program not found at: {LIGHTHOUSE_PROGRAM_PATH}");
        println!("   Starting validator without lighthouse program");
    }

    if load_accounts {
        for account_file in AccountFile::required_test_accounts() {
            let account_path = account_file.test_account_path();
            if account_path.exists() {
                if let Ok(account_address) = get_account_address_from_file(&account_path).await {
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

    Ok(cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?)
}
