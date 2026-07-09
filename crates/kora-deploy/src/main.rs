use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use kora_deploy::{deploy, program_is_live, upgrade, DeployConfig, UpgradeConfig};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};

#[derive(Parser)]
#[command(
    name = "kora-deploy",
    about = "Deploy or upgrade a Solana program on devnet via a Kora paymaster (no SOL required)."
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
    /// Wallet keypair that owns the program's upgrade rights. Defaults to
    /// ~/.config/solana/id.json when present. Without a wallet the program deploys
    /// as immutable through the paymaster.
    #[arg(long)]
    wallet: Option<PathBuf>,
    /// Existing program to upgrade. Omit to deploy a fresh program.
    #[arg(long)]
    program_id: Option<Pubkey>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let args = Args::parse();

    let user_id =
        args.user_id.clone().unwrap_or_else(|| format!("kora-deploy-{}", Pubkey::new_unique()));
    let wallet = load_wallet(args.wallet.as_deref())?;

    match args.program_id {
        Some(program) => {
            if !program_is_live(&args.rpc_url, &program).await? {
                bail!(
                    "program {program} is not live on-chain (never deployed or reaped); \
                     omit --program-id to deploy fresh"
                );
            }
            run_upgrade(&args, program, wallet.as_ref(), user_id).await
        }
        None => run_deploy(&args, wallet.as_ref(), user_id).await,
    }
}

async fn run_deploy(args: &Args, wallet: Option<&Keypair>, user_id: String) -> Result<()> {
    match wallet {
        Some(w) => log::info!("registering {} as upgrade owner", w.pubkey()),
        None => {
            log::warn!("no wallet provided; the program will be immutable through the paymaster")
        }
    }

    let result = deploy(&DeployConfig {
        kora_url: &args.kora_url,
        rpc_url: &args.rpc_url,
        program_so: &args.program_so,
        user_id,
        wallet,
    })
    .await?;

    println!();
    println!("Deployed via {}", args.kora_url);
    println!("  paymaster:    {}", result.kora_pubkey);
    println!("  program:      {}", result.program);
    println!("  program_data: {}", result.program_data);
    println!();
    match wallet {
        Some(w) => println!(
            "Upgradeable by {}. To upgrade, rerun with --program-id {}.",
            w.pubkey(),
            result.program
        ),
        None => println!("Deployed without a wallet: not upgradeable."),
    }
    println!("The paymaster owns the on-chain upgrade authority. Programs idle 7+ days are");
    println!("closed automatically and the rent returns to the paymaster.");
    Ok(())
}

async fn run_upgrade(
    args: &Args,
    program: Pubkey,
    wallet: Option<&Keypair>,
    user_id: String,
) -> Result<()> {
    let wallet = wallet.ok_or_else(|| {
        anyhow!("upgrading {program} requires the wallet it was registered with (--wallet)")
    })?;
    log::info!("upgrading {program} as {}", wallet.pubkey());

    let signature = upgrade(&UpgradeConfig {
        kora_url: &args.kora_url,
        rpc_url: &args.rpc_url,
        program_so: &args.program_so,
        user_id,
        program,
        wallet,
    })
    .await?;

    println!();
    println!("Upgraded {program} via {}", args.kora_url);
    println!("  signature: {signature}");
    Ok(())
}

fn load_wallet(path: Option<&Path>) -> Result<Option<Keypair>> {
    let read =
        |p: &Path| read_keypair_file(p).map_err(|e| anyhow!("reading wallet {}: {e}", p.display()));
    match path {
        Some(p) => read(p).map(Some),
        None => {
            let home = std::env::var("HOME").context("HOME is not set")?;
            let default = Path::new(&home).join(".config/solana/id.json");
            if default.exists() {
                read(&default).map(Some)
            } else {
                Ok(None)
            }
        }
    }
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
