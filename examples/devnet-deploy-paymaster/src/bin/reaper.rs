use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use devnet_deploy_paymaster::reaper::{self, LoaderFilter, ReaperConfig};
use kora_lib::{
    rpc::get_rpc_client,
    signer::{SignerPool, SignerPoolConfig, SolanaSigner},
    state::{get_signer_pool, init_config, init_signer_pool},
    Config,
};

#[derive(Parser)]
#[command(
    name = "devnet_deploy_reaper",
    about = "Close devnet programs the paymaster owns that have been idle past a threshold."
)]
struct Args {
    #[arg(long, default_value = "kora.toml")]
    config: PathBuf,
    #[arg(long)]
    signers_config: PathBuf,
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,
    /// Idle threshold (humantime: `7d`, `48h`).
    #[arg(long, default_value = "7d", value_parser = parse_duration)]
    threshold: Duration,
    /// Log what would close, change nothing on-chain.
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    max_closes: Option<usize>,
    #[arg(long, value_enum, default_value_t = LoaderFilterArg::Both)]
    loader: LoaderFilterArg,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum LoaderFilterArg {
    V3,
    V4,
    Both,
}

impl From<LoaderFilterArg> for LoaderFilter {
    fn from(arg: LoaderFilterArg) -> Self {
        match arg {
            LoaderFilterArg::V3 => LoaderFilter::V3Only,
            LoaderFilterArg::V4 => LoaderFilter::V4Only,
            LoaderFilterArg::Both => LoaderFilter::Both,
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    init_logging();

    let config = Config::load_config(&args.config)
        .with_context(|| format!("loading kora config at {}", args.config.display()))?;
    init_config(config).map_err(|e| anyhow!("init_config: {e}"))?;

    let pool_config = SignerPoolConfig::load_config(&args.signers_config)
        .with_context(|| format!("loading signers config at {}", args.signers_config.display()))?;
    let pool = SignerPool::from_config(pool_config)
        .await
        .map_err(|e| anyhow!("building signer pool: {e}"))?;
    init_signer_pool(pool).map_err(|e| anyhow!("init_signer_pool: {e}"))?;

    let signer = get_signer_pool()
        .map_err(|e| anyhow!("get_signer_pool: {e}"))?
        .select_next_signer()
        .map_err(|e| anyhow!("selecting fee payer: {e}"))?;
    let fee_payer = signer.pubkey();

    let rpc: Arc<_> = get_rpc_client(&args.rpc_url);

    let cfg = ReaperConfig {
        fee_payer,
        threshold: args.threshold,
        dry_run: args.dry_run,
        max_closes: args.max_closes,
        loader_filter: args.loader.into(),
    };

    log::info!(
        "reaper start: fee_payer={fee_payer} threshold={:?} dry_run={} max_closes={:?} loader={:?}",
        cfg.threshold,
        cfg.dry_run,
        cfg.max_closes,
        cfg.loader_filter,
    );

    let report = reaper::run(rpc, cfg).await?;

    log::info!(
        "reaper done: discovered={} skipped_recent={} closed={} failed={}",
        report.discovered,
        report.skipped_recent,
        report.closed.len(),
        report.failed.len(),
    );
    for c in &report.closed {
        log::info!(
            "closed program={} loader={:?} sig={} reclaimed_lamports={}",
            c.program,
            c.loader,
            c.signature,
            c.reclaimed_lamports
        );
    }
    for f in &report.failed {
        log::warn!("failed program={} loader={:?} error={}", f.program, f.loader, f.error);
    }

    if !report.failed.is_empty() {
        std::process::exit(2);
    }
    Ok(())
}

fn init_logging() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
