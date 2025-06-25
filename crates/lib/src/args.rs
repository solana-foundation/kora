use crate::log::LoggingFormat;
use clap::{command, Parser};

// Common arguments shared between CLI and RPC
#[derive(Debug, Parser)]
#[command(name = "kora")]
pub struct CommonArgs {
    /// RPC URL
    #[arg(long, env = "RPC_URL", default_value = "http://127.0.0.1:8899")]
    pub rpc_url: String,

    /// Base58-encoded private key for signing
    #[arg(long, env = "KORA_PRIVATE_KEY", required_unless_present_any = ["skip_signer", "turnkey_signer", "vault_signer", "privy_signer"])]
    pub private_key: Option<String>,

    /// Path to kora.toml config file
    #[arg(long, default_value = "kora.toml")]
    pub config: String,

    /// Skip loading the signer
    #[arg(long = "no-load-signer")]
    pub skip_signer: bool,

    /// Turnkey signer
    #[arg(long = "with-turnkey-signer")]
    pub turnkey_signer: bool,

    /// Turnkey API credentials
    #[arg(long, env = "TURNKEY_API_PUBLIC_KEY")]
    pub turnkey_api_public_key: Option<String>,

    #[arg(long, env = "TURNKEY_API_PRIVATE_KEY")]
    pub turnkey_api_private_key: Option<String>,

    #[arg(long, env = "TURNKEY_ORGANIZATION_ID")]
    pub turnkey_organization_id: Option<String>,

    #[arg(long, env = "TURNKEY_PRIVATE_KEY_ID")]
    pub turnkey_private_key_id: Option<String>,

    #[arg(long, env = "TURNKEY_PUBLIC_KEY")]
    pub turnkey_public_key: Option<String>,

    /// Privy API Credentials
    #[arg(long = "with-privy-signer")]
    pub privy_signer: bool,

    /// Privy API credentials
    #[arg(long, env = "PRIVY_APP_ID")]
    pub privy_app_id: Option<String>,

    #[arg(long, env = "PRIVY_APP_SECRET")]
    pub privy_app_secret: Option<String>,

    #[arg(long, env = "PRIVY_WALLET_ID")]
    pub privy_wallet_id: Option<String>,

    #[arg(long)]
    pub vault_signer: bool,

    #[arg(long, env = "VAULT_ADDR")]
    pub vault_addr: Option<String>,

    #[arg(long, env = "VAULT_TOKEN")]
    pub vault_token: Option<String>,

    #[arg(long, env = "VAULT_KEY_NAME")]
    pub vault_key_name: Option<String>,

    #[arg(long, env = "VAULT_PUBKEY")]
    pub vault_pubkey: Option<String>,
}

// RPC-specific arguments
#[derive(Debug, Parser)]
pub struct RpcArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Port
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: Option<u16>,

    /// Logging Format
    #[arg(long, default_value = "standard")]
    pub logging_format: LoggingFormat,

    /// Metrics
    #[arg(long, default_value = None)]
    pub metrics_endpoint: Option<String>,
}

// CLI-specific arguments
#[derive(Debug, Parser)]
pub struct CliArgs {
    #[command(flatten)]
    pub common: CommonArgs,
}
