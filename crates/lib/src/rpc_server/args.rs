use crate::log::LoggingFormat;
use clap::Parser;
use std::path::PathBuf;

/// RPC server arguments
#[derive(Parser)]
pub struct RpcArgs {
    /// HTTP port to listen on for RPC requests
    #[arg(short = 'p', long, default_value = "8080")]
    pub port: u16,

    /// Output format for logs (standard or json)
    #[arg(long, default_value = "standard")]
    pub logging_format: LoggingFormat,

    /// Path to multi-signer configuration file (TOML format)
    /// Required unless using --no-load-signer
    #[arg(long, required_unless_present = "skip_signer")]
    pub signers_config: Option<PathBuf>,

    /// Skip signer initialization (useful for testing or operations that don't require signing)
    #[arg(long = "no-load-signer")]
    pub skip_signer: bool,

    #[command(flatten)]
    pub auth_args: AuthArgs,
}

#[derive(Parser)]
pub struct AuthArgs {
    /// API key for authenticating requests to the Kora server (optional) - can be set in `kora.toml`
    #[arg(long, env = "KORA_API_KEY", help_heading = "Authentication")]
    pub api_key: Option<String>,

    /// HMAC secret for request signature authentication (optional, provides stronger security than API key) - can be set in `kora.toml`
    #[arg(long, env = "KORA_HMAC_SECRET", help_heading = "Authentication")]
    pub hmac_secret: Option<String>,
}
