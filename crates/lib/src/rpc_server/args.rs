use crate::log::LoggingFormat;
use clap::Parser;

/// RPC server arguments
#[derive(Debug, Parser)]
pub struct RpcArgs {
    /// HTTP port to listen on for RPC requests
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: u16,

    /// Output format for logs (standard or json)
    #[arg(long, default_value = "standard")]
    pub logging_format: LoggingFormat,

    // Signing Options
    /// Private key for transaction signing (base58, u8 array, or path to JSON keypair file).
    /// Required unless `skip_signer`, `turnkey_signer`, `vault_signer`, or `privy_signer` is set
    #[arg(long, env = "KORA_PRIVATE_KEY", required_unless_present_any = ["skip_signer", "turnkey_signer", "vault_signer", "privy_signer"], help_heading = "Signing Options")]
    pub private_key: Option<String>,

    /// Skip signer initialization (useful for testing or operations that don't require signing)
    #[arg(long = "no-load-signer", help_heading = "Signing Options")]
    pub skip_signer: bool,

    #[command(flatten)]
    pub auth_args: AuthArgs,

    #[command(flatten)]
    pub privy_args: PrivyArgs,

    #[command(flatten)]
    pub turnkey_args: TurnkeyArgs,

    #[command(flatten)]
    pub vault_args: VaultArgs,
}

#[derive(Debug, Parser)]
pub struct AuthArgs {
    /// API key for authenticating requests to the Kora server (optional) - can be set in `kora.toml`
    #[arg(long, env = "KORA_API_KEY", help_heading = "Authentication")]
    pub api_key: Option<String>,

    /// HMAC secret for request signature authentication (optional, provides stronger security than API key) - can be set in `kora.toml`
    #[arg(long, env = "KORA_HMAC_SECRET", help_heading = "Authentication")]
    pub hmac_secret: Option<String>,
}

#[derive(Debug, Parser)]
pub struct PrivyArgs {
    /// Use Privy remote signer for secure wallet management
    #[arg(long = "with-privy-signer", help_heading = "Privy Signer")]
    pub privy_signer: bool,

    /// Privy application ID for authentication
    #[arg(long, env = "PRIVY_APP_ID", help_heading = "Privy Signer")]
    pub privy_app_id: Option<String>,

    /// Privy application secret for authentication
    #[arg(long, env = "PRIVY_APP_SECRET", help_heading = "Privy Signer")]
    pub privy_app_secret: Option<String>,

    /// Privy wallet ID to use for signing
    #[arg(long, env = "PRIVY_WALLET_ID", help_heading = "Privy Signer")]
    pub privy_wallet_id: Option<String>,
}

#[derive(Debug, Parser)]
pub struct TurnkeyArgs {
    /// Use Turnkey remote signer for secure key management
    #[arg(long = "with-turnkey-signer", help_heading = "Turnkey Signer")]
    pub turnkey_signer: bool,

    /// Turnkey API public key for authentication
    #[arg(long, env = "TURNKEY_API_PUBLIC_KEY", help_heading = "Turnkey Signer")]
    pub turnkey_api_public_key: Option<String>,

    /// Turnkey API private key for authentication
    #[arg(long, env = "TURNKEY_API_PRIVATE_KEY", help_heading = "Turnkey Signer")]
    pub turnkey_api_private_key: Option<String>,

    /// Turnkey organization ID where keys are stored
    #[arg(long, env = "TURNKEY_ORGANIZATION_ID", help_heading = "Turnkey Signer")]
    pub turnkey_organization_id: Option<String>,

    /// Turnkey private key ID to use for signing
    #[arg(long, env = "TURNKEY_PRIVATE_KEY_ID", help_heading = "Turnkey Signer")]
    pub turnkey_private_key_id: Option<String>,

    /// Turnkey public key (base58 encoded) for verification
    #[arg(long, env = "TURNKEY_PUBLIC_KEY", help_heading = "Turnkey Signer")]
    pub turnkey_public_key: Option<String>,
}

#[derive(Debug, Parser)]
pub struct VaultArgs {
    /// Use HashiCorp Vault signer for enterprise key management
    #[arg(long, help_heading = "Vault Signer")]
    pub vault_signer: bool,

    /// HashiCorp Vault server address
    #[arg(long, env = "VAULT_ADDR", help_heading = "Vault Signer")]
    pub vault_addr: Option<String>,

    /// HashiCorp Vault authentication token
    #[arg(long, env = "VAULT_TOKEN", help_heading = "Vault Signer")]
    pub vault_token: Option<String>,

    /// Key name in Vault to use for signing
    #[arg(long, env = "VAULT_KEY_NAME", help_heading = "Vault Signer")]
    pub vault_key_name: Option<String>,

    /// Vault public key (base58 encoded) for verification
    #[arg(long, env = "VAULT_PUBKEY", help_heading = "Vault Signer")]
    pub vault_pubkey: Option<String>,
}
