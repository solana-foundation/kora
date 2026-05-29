use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use kora_deploy::{close, deploy, verify_upgrade_authority, DeployConfig};
use solana_sdk::pubkey::Pubkey;

const DEFAULT_PROGRAM_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/src/common/transfer-hook-example/transfer_hook_example.so"
);

#[derive(Parser)]
#[command(
    about = "End-to-end smoke test: deploy, verify authority, close, against a live Kora paymaster."
)]
struct Args {
    #[arg(long, default_value = "https://kora-devnet-paymaster-kysurhpjxq-uc.a.run.app")]
    kora_url: String,
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    #[arg(long)]
    program_so: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let args = Args::parse();
    let program_so = PathBuf::from(args.program_so.as_deref().unwrap_or(DEFAULT_PROGRAM_SO));
    let user_id = format!("kora-smoke-{}", Pubkey::new_unique());

    println!("Kora paymaster: {}", args.kora_url);
    println!("Solana RPC:     {}", args.rpc_url);

    let result = deploy(&DeployConfig {
        kora_url: &args.kora_url,
        rpc_url: &args.rpc_url,
        program_so: &program_so,
        user_id: user_id.clone(),
    })
    .await?;
    println!("Deployed program {}", result.program);

    verify_upgrade_authority(&args.rpc_url, &result.program_data, &result.kora_pubkey).await?;
    println!("Verified upgrade_authority == {}", result.kora_pubkey);

    let sig = close(
        &args.rpc_url,
        &args.kora_url,
        &user_id,
        &result.kora_pubkey,
        &result.program,
        &result.program_data,
    )
    .await?;
    println!("Closed program (sig {sig})");

    println!();
    println!("OK — full deploy lifecycle succeeded against {}", args.kora_url);
    Ok(())
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
