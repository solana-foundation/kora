//! Closes programs that the devnet deploy-paymaster owns and that have been
//! idle past a threshold. Intended to run as a Cloud Run Job on a daily cron.
//!
//! Flow per invocation:
//! 1. `discovery` scans the chain for programs whose upgrade authority is the
//!    paymaster's fee payer.
//! 2. `activity` checks each candidate's most recent on-chain signature; ones
//!    newer than the threshold are skipped.
//! 3. `closer` builds and sends the close transaction for the rest, signing
//!    locally via the same `SignerPool` the RPC uses.

pub mod activity;
pub mod closer;
pub mod discovery;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

/// Which loader a discovered program lives under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    /// BPF Loader Upgradeable (legacy v3).
    V3,
    /// Loader-v4.
    V4,
}

/// A program owned by the paymaster, returned by [`discovery`].
#[derive(Debug, Clone)]
pub struct OwnedProgram {
    pub loader: Loader,
    /// The `Program` account (loader-v3) or the program account that also
    /// holds state for loader-v4.
    pub program: Pubkey,
    /// The `ProgramData` PDA. Only set for loader-v3 — loader-v4 stores state
    /// inline on `program`.
    pub program_data: Option<Pubkey>,
    /// Slot at which the program was last deployed / upgraded, read from the
    /// loader's on-chain state. Used as a fallback when `getSignaturesForAddress`
    /// returns no history.
    pub last_state_slot: u64,
}

/// Per-invocation knobs. Mirrors the CLI flags 1:1 so the binary stays thin.
#[derive(Debug, Clone)]
pub struct ReaperConfig {
    pub fee_payer: Pubkey,
    pub threshold: Duration,
    pub dry_run: bool,
    pub max_closes: Option<usize>,
    pub loader_filter: LoaderFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderFilter {
    V3Only,
    V4Only,
    Both,
}

impl LoaderFilter {
    pub fn includes(self, loader: Loader) -> bool {
        matches!(
            (self, loader),
            (LoaderFilter::Both, _)
                | (LoaderFilter::V3Only, Loader::V3)
                | (LoaderFilter::V4Only, Loader::V4)
        )
    }
}

/// Summary returned by [`run`] so the binary can log a structured report and
/// exit with an appropriate code.
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

/// Top-level reaper entrypoint. Discovers, filters by activity, and closes.
pub async fn run(rpc: Arc<RpcClient>, cfg: ReaperConfig) -> Result<ReaperReport> {
    let owned = discovery::discover_owned_programs(&rpc, &cfg.fee_payer).await?;
    log::info!("discovered {} programs owned by {}", owned.len(), cfg.fee_payer);

    let mut report = ReaperReport { discovered: owned.len(), ..Default::default() };

    let mut idle: Vec<OwnedProgram> = Vec::new();
    for program in owned {
        if !cfg.loader_filter.includes(program.loader) {
            continue;
        }
        match activity::classify(&rpc, &program, cfg.threshold).await {
            Ok(activity::ActivityVerdict::Recent { last_seen_unix }) => {
                report.skipped_recent += 1;
                log::debug!(
                    "skip program={} loader={:?} last_seen={last_seen_unix}",
                    program.program,
                    program.loader
                );
            }
            Ok(activity::ActivityVerdict::Idle { last_seen_unix }) => {
                log::info!(
                    "idle program={} loader={:?} last_seen={:?}",
                    program.program,
                    program.loader,
                    last_seen_unix
                );
                idle.push(program);
            }
            Err(e) => {
                log::warn!(
                    "activity check failed for {} ({:?}): {e}; skipping",
                    program.program,
                    program.loader
                );
            }
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
