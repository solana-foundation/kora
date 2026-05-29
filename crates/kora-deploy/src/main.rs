use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use kora_deploy::{deploy, DeployConfig};
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(
    name = "kora-deploy",
    about = "Deploy a Solana program to devnet via a Kora paymaster (no SOL required)."
)]
struct Args {
    /// Paymaster URL.
    #[arg(long, default_value = "https://deployer.devnet.solana.com")]
    kora_url: String,
    /// Solana RPC for reading on-chain state.
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    /// Path to the program `.so` file.
    #[arg(long)]
    program_so: PathBuf,
    /// Arbitrary tag the paymaster buckets by for usage limits.
    /// Defaults to a per-invocation random ID so each run gets its own bucket.
    #[arg(long)]
    user_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let args = Args::parse();

    let user_id = args.user_id.unwrap_or_else(|| format!("kora-deploy-{}", Pubkey::new_unique()));

    let result = deploy(&DeployConfig {
        kora_url: &args.kora_url,
        rpc_url: &args.rpc_url,
        program_so: &args.program_so,
        user_id,
    })
    .await?;

    print_summary(&args.kora_url, &result.kora_pubkey, &result.program, &result.program_data);
    Ok(())
}

fn print_summary(kora_url: &str, kora_pubkey: &Pubkey, program: &Pubkey, program_data: &Pubkey) {
    println!();
    println!("Deployed via {kora_url}");
    println!("  paymaster:    {kora_pubkey}");
    println!("  program:      {program}");
    println!("  program_data: {program_data}");
    println!();
    println!("The paymaster owns the upgrade authority. Programs idle 7+ days are");
    println!("closed automatically and the rent returns to the paymaster.");
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
