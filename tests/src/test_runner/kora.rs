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
pub const KORA_BINARY_PATH_ENV: &str = "KORA_TEST_BINARY_PATH";
pub const PORT_RANGE_START: u16 = 8080;
pub const PORT_RANGE_END: u16 = 8180;

pub async fn get_kora_binary_path() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = std::env::var(KORA_BINARY_PATH_ENV).unwrap_or_else(|_| KORA_BINARY_PATH.to_string());
    if !Path::new(&path).exists() {
        return Err(format!(
            "Pre-built Kora binary not found at '{path}'. \
            Run 'cargo build --bin kora' or 'just build' first for much better performance.",
        )
        .into());
    }
    Ok(path)
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

/// SIGTERM first so an llvm-cov-instrumented Kora can flush coverage data on
/// exit; SIGKILL only if it doesn't terminate in time.
pub async fn stop_kora_gracefully(child: &mut Child) {
    if let Some(pid) = child.id() {
        let terminated = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if terminated
            && tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await.is_ok()
        {
            return;
        }
    }
    child.kill().await.ok();
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
    preferred_port: u16,
    verbose: bool,
) -> Result<(Child, u16), Box<dyn std::error::Error + Send + Sync>> {
    let fee_payer_key =
        cached_keys.get(&AccountFile::FeePayer).ok_or("FeePayer key not found in cache")?;
    let signer_2 =
        cached_keys.get(&AccountFile::Signer2).ok_or("Signer2 key not found in cache")?;

    let port = if check_port_available(preferred_port).await {
        let mut used_ports = USED_PORTS.lock().unwrap();
        used_ports.insert(preferred_port);
        preferred_port
    } else {
        find_available_port().await?
    };
    let kora_binary_path = get_kora_binary_path().await?;

    let (std_out, std_err) = if verbose {
        (std::process::Stdio::inherit(), std::process::Stdio::inherit())
    } else {
        (std::process::Stdio::null(), std::process::Stdio::null())
    };

    let mut cmd = tokio::process::Command::new(kora_binary_path);
    cmd.args([
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
    // Overrides the fixture's cache_url (usage_limit store) when set
    .env_remove("KORA_REDIS_URL")
    .stdout(std_out)
    .stderr(std_err);

    if let Ok(jupiter_key) = std::env::var("JUPITER_API_KEY") {
        cmd.env("JUPITER_API_KEY", jupiter_key);
    }

    let kora_pid = cmd.spawn()?;

    Ok((kora_pid, port))
}
