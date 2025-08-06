use crate::log::LoggingFormat;
use clap::{command, Parser};

// Common arguments shared between CLI and RPC
#[derive(Debug, Parser)]
#[command(name = "kora")]
pub struct CommonArgs {
    /// Solana RPC endpoint URL
    #[arg(long, env = "RPC_URL", default_value = "http://127.0.0.1:8899")]
    pub rpc_url: String,

    /// API key for authenticating requests to the Kora server (optional) - can be set in `kora.toml`
    #[arg(long, env = "KORA_API_KEY")]
    pub api_key: Option<String>,

    /// HMAC secret for request signature authentication (optional, provides stronger security than API key) - can be set in `kora.toml`
    #[arg(long, env = "KORA_HMAC_SECRET")]
    pub hmac_secret: Option<String>,

    /// Private key for transaction signing (base58, u8 array, or path to JSON keypair file).
    /// Required unless `skip_signer`, `turnkey_signer`, `vault_signer`, or `privy_signer` is set
    #[arg(long, env = "KORA_PRIVATE_KEY", required_unless_present_any = ["skip_signer", "turnkey_signer", "vault_signer", "privy_signer"])]
    pub private_key: Option<String>,

    /// Path to Kora configuration file (TOML format)
    #[arg(long, default_value = "kora.toml")]
    pub config: String,

    /// Skip signer initialization (useful for testing or operations that don't require signing)
    #[arg(long = "no-load-signer")]
    pub skip_signer: bool,

    /// Use Turnkey remote signer for secure key management
    #[arg(long = "with-turnkey-signer")]
    pub turnkey_signer: bool,

    /// Turnkey API public key for authentication
    #[arg(long, env = "TURNKEY_API_PUBLIC_KEY")]
    pub turnkey_api_public_key: Option<String>,

    /// Turnkey API private key for authentication
    #[arg(long, env = "TURNKEY_API_PRIVATE_KEY")]
    pub turnkey_api_private_key: Option<String>,

    /// Turnkey organization ID where keys are stored
    #[arg(long, env = "TURNKEY_ORGANIZATION_ID")]
    pub turnkey_organization_id: Option<String>,

    /// Turnkey private key ID to use for signing
    #[arg(long, env = "TURNKEY_PRIVATE_KEY_ID")]
    pub turnkey_private_key_id: Option<String>,

    /// Turnkey public key (base58 encoded) for verification
    #[arg(long, env = "TURNKEY_PUBLIC_KEY")]
    pub turnkey_public_key: Option<String>,

    /// Use Privy remote signer for secure wallet management
    #[arg(long = "with-privy-signer")]
    pub privy_signer: bool,

    /// Privy application ID for authentication
    #[arg(long, env = "PRIVY_APP_ID")]
    pub privy_app_id: Option<String>,

    /// Privy application secret for authentication
    #[arg(long, env = "PRIVY_APP_SECRET")]
    pub privy_app_secret: Option<String>,

    /// Privy wallet ID to use for signing
    #[arg(long, env = "PRIVY_WALLET_ID")]
    pub privy_wallet_id: Option<String>,

    /// Use HashiCorp Vault signer for enterprise key management
    #[arg(long)]
    pub vault_signer: bool,

    /// HashiCorp Vault server address
    #[arg(long, env = "VAULT_ADDR")]
    pub vault_addr: Option<String>,

    /// HashiCorp Vault authentication token
    #[arg(long, env = "VAULT_TOKEN")]
    pub vault_token: Option<String>,

    /// Key name in Vault to use for signing
    #[arg(long, env = "VAULT_KEY_NAME")]
    pub vault_key_name: Option<String>,

    /// Vault public key (base58 encoded) for verification
    #[arg(long, env = "VAULT_PUBKEY")]
    pub vault_pubkey: Option<String>,

    /// Validate configuration file and show results (exits after validation)
    #[arg(long)]
    pub validate_config: bool,
}

// RPC-specific arguments
#[derive(Debug, Parser)]
#[command(name = "kora-rpc", version)]
pub struct RpcArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// HTTP port to listen on for RPC requests
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: Option<u16>,

    /// Output format for logs (standard or json)
    #[arg(long, default_value = "standard")]
    pub logging_format: LoggingFormat,

    /// Prometheus metrics endpoint URL for monitoring
    #[arg(long, default_value = None)]
    pub metrics_endpoint: Option<String>,
}

// CLI-specific arguments
#[derive(Debug, Parser)]
pub struct CliArgs {
    #[command(flatten)]
    pub common: CommonArgs,
}
