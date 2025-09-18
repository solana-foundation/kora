use crate::test_runner::accounts::AccountFile;
use std::fs;
use tokio::process::Child;

pub async fn is_kora_running(port: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/liveness");
    client.get(&url).send().await.is_ok()
}

pub async fn start_kora_rpc_server(
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    port: &str,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let fee_payer_key = fs::read_to_string(AccountFile::FeePayer.local_key_path())?;
    let signer_2 = fs::read_to_string(AccountFile::Signer2.local_key_path())?;

    let kora_pid = tokio::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "kora-cli",
            "--bin",
            "kora",
            "--",
            "--config",
            config_file,
            "--rpc-url",
            rpc_url.as_str(),
            "rpc",
            "start",
            "--signers-config",
            signers_config,
            "--port",
            port,
        ])
        .env("KORA_PRIVATE_KEY", fee_payer_key.trim())
        .env("KORA_PRIVATE_KEY_2", signer_2.trim())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    Ok(kora_pid)
}
