use crate::test_runner::accounts::AccountFile;
use std::{
    collections::HashSet,
    path::Path,
    sync::{LazyLock, Mutex},
};
use tokio::{net::TcpListener, process::Child};

// Global port tracker to prevent immediate reuse
static USED_PORTS: LazyLock<Mutex<HashSet<u16>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub const KORA_BINARY_PATH: &str = "target/debug/kora";
pub const PORT_RANGE_START: u16 = 8080;
pub const PORT_RANGE_END: u16 = 8180;

pub async fn get_kora_binary_path() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !Path::new(KORA_BINARY_PATH).exists() {
        return Err(format!(
            "Pre-built Kora binary not found at '{KORA_BINARY_PATH}'. \
            Run 'cargo build --bin kora' or 'make build' first for much better performance.",
        )
        .into());
    }
    Ok(KORA_BINARY_PATH.to_string())
}

pub async fn check_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).await.is_ok()
}

pub async fn find_available_port() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    for port in PORT_RANGE_START..PORT_RANGE_END {
        // Check if port is available and not recently used
        if check_port_available(port).await {
            let mut used_ports = USED_PORTS.lock().unwrap();
            if !used_ports.contains(&port) {
                used_ports.insert(port);
                return Ok(port);
            }
        }
    }
    Err(format!("No available ports found in range {PORT_RANGE_START}-{PORT_RANGE_END}").into())
}

pub fn release_port(port: u16) {
    let mut used_ports = USED_PORTS.lock().unwrap();
    used_ports.remove(&port);
}

pub async fn is_kora_running_with_client(client: &reqwest::Client, port: &str) -> bool {
    let url = format!("http://127.0.0.1:{port}/liveness");
    client.get(&url).timeout(std::time::Duration::from_secs(5)).send().await.is_ok()
}

pub async fn start_kora_rpc_server(
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    cached_keys: &std::collections::HashMap<AccountFile, String>,
) -> Result<(Child, u16), Box<dyn std::error::Error + Send + Sync>> {
    let fee_payer_key =
        cached_keys.get(&AccountFile::FeePayer).ok_or("FeePayer key not found in cache")?;
    let signer_2 =
        cached_keys.get(&AccountFile::Signer2).ok_or("Signer2 key not found in cache")?;

    let port = find_available_port().await?;
    let kora_binary_path = get_kora_binary_path().await?;

    let kora_pid = tokio::process::Command::new(kora_binary_path)
        .args([
            "--config",
            config_file,
            "--rpc-url",
            rpc_url.as_str(),
            "rpc",
            "start",
            "--signers-config",
            signers_config,
            "--port",
            &port.to_string(),
        ])
        .env("KORA_PRIVATE_KEY", fee_payer_key.trim())
        .env("KORA_PRIVATE_KEY_2", signer_2.trim())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    Ok((kora_pid, port))
}
