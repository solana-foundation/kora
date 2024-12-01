use clap::{command, Parser};

use crate::LoggingFormat;

#[derive(Debug, Parser)]
#[command(name = "kora")]
pub struct Args {
    /// Port
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// RPC URL
    #[arg(short, long, env = "RPC_URL", default_value = "http://127.0.0.1:8899")]
    pub rpc_url: String,

    /// Logging Format
    #[arg(short, long, default_value = "standard")]
    pub logging_format: LoggingFormat,

    /// Metrics
    #[arg(long, default_value = None)]
    pub metrics_endpoint: Option<String>,

    /// Base58-encoded private key for signing
    #[arg(long, env = "KORA_PRIVATE_KEY")]
    pub private_key: String,
}
