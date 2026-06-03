use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;
use devnet_deploy_paymaster::suite;

const DEFAULT_PROGRAM_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/src/common/transfer-hook-example/transfer_hook_example.so"
);

#[derive(Parser)]
#[command(
    about = "Devnet deployer verification: happy-path deploy lifecycle plus adversarial drain/grief probes against a live Kora paymaster."
)]
struct Args {
    #[arg(long, default_value = "https://deployer.devnet.solana.com")]
    kora_url: String,
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    #[arg(long)]
    program_so: Option<String>,
    #[arg(long)]
    happy_only: bool,
    #[arg(long)]
    adversarial_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let args = Args::parse();
    let program_so = PathBuf::from(args.program_so.as_deref().unwrap_or(DEFAULT_PROGRAM_SO));

    if args.happy_only && args.adversarial_only {
        bail!("--happy-only and --adversarial-only are mutually exclusive");
    }

    println!("Kora paymaster: {}", args.kora_url);
    println!();

    let ok = suite::run_full(
        &args.kora_url,
        &args.rpc_url,
        &program_so,
        !args.adversarial_only,
        !args.happy_only,
    )
    .await?;

    if !ok {
        bail!("verification failed: a probe was signed");
    }
    Ok(())
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
