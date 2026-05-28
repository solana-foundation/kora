pub mod activity;
pub mod closer;
pub mod discovery;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    V3,
    V4,
}

#[derive(Debug, Clone)]
pub struct OwnedProgram {
    pub loader: Loader,
    pub program: Pubkey,
    /// `None` for v4 — state and code share one account.
    pub program_data: Option<Pubkey>,
    pub last_state_slot: u64,
}

#[derive(Debug, Clone)]
pub struct ReaperConfig {
    pub fee_payer: Pubkey,
    pub threshold: Duration,
    pub dry_run: bool,
    pub max_closes: Option<usize>,
}

#[derive(Debug, Default)]
pub struct ReaperReport {
    pub discovered: usize,
    pub skipped_recent: usize,
    pub closed: Vec<ClosedProgram>,
    pub failed: Vec<FailedClose>,
}

#[derive(Debug)]
pub struct ClosedProgram {
    pub program: Pubkey,
    pub loader: Loader,
    pub signature: String,
    pub reclaimed_lamports: u64,
}

#[derive(Debug)]
pub struct FailedClose {
    pub program: Pubkey,
    pub loader: Loader,
    pub error: String,
}

pub async fn run(rpc: Arc<RpcClient>, cfg: ReaperConfig) -> Result<ReaperReport> {
    let owned = discovery::discover_owned_programs(&rpc, &cfg.fee_payer).await?;
    log::info!("discovered {} programs owned by {}", owned.len(), cfg.fee_payer);

    let mut report = ReaperReport { discovered: owned.len(), ..Default::default() };
    let mut idle: Vec<OwnedProgram> = Vec::new();

    for program in owned {
        match activity::classify(&rpc, &program, cfg.threshold).await {
            Ok(activity::ActivityVerdict::Recent { .. }) => report.skipped_recent += 1,
            Ok(activity::ActivityVerdict::Idle { last_seen_unix }) => {
                log::info!(
                    "idle program={} loader={:?} last_seen={:?}",
                    program.program,
                    program.loader,
                    last_seen_unix
                );
                idle.push(program);
            }
            Err(e) => log::warn!("activity check failed for {}: {e}", program.program),
        }
    }

    if let Some(max) = cfg.max_closes {
        if idle.len() > max {
            log::info!("capping closes at --max-closes={max} (idle={})", idle.len());
            idle.truncate(max);
        }
    }

    if cfg.dry_run {
        log::info!("dry-run: {} program(s) would be closed", idle.len());
        return Ok(report);
    }

    for program in idle {
        match closer::close_program(&rpc, &program).await {
            Ok(closed) => report.closed.push(closed),
            Err(e) => report.failed.push(FailedClose {
                program: program.program,
                loader: program.loader,
                error: e.to_string(),
            }),
        }
    }

    Ok(report)
}
