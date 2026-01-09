use clap::Parser;

/// Global arguments used by all subcommands
#[derive(Debug, Parser)]
#[command(name = "kora")]
pub struct GlobalArgs {
    /// Solana RPC endpoint URL
    #[arg(long, env = "RPC_URL", default_value = "http://127.0.0.1:8899")]
    pub rpc_url: String,

    /// Path to Kora configuration file (TOML format)
    #[arg(long, default_value = "kora.toml")]
    pub config: String,
}
