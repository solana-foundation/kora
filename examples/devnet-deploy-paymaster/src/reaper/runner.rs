use std::sync::Arc;

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;

use super::{
    activity::{self, ActivityVerdict},
    closer, discovery,
    types::{FailedClose, OwnedProgram, ReaperConfig, ReaperReport},
};

pub async fn run(rpc: Arc<RpcClient>, cfg: ReaperConfig) -> Result<ReaperReport> {
    let owned = discovery::discover_owned_programs(&rpc, &cfg.fee_payer).await?;
    log::info!("discovered {} programs owned by {}", owned.len(), cfg.fee_payer);

    let mut report = ReaperReport { discovered: owned.len(), ..Default::default() };
    let mut idle: Vec<OwnedProgram> = Vec::new();

    for program in owned {
        match activity::classify(&rpc, &program, cfg.threshold).await {
            Ok(ActivityVerdict::Recent { .. }) => report.skipped_recent += 1,
            Ok(ActivityVerdict::Idle { last_seen_unix }) => {
                log::info!(
                    "idle {:?} {} loader={:?} last_seen={:?}",
                    program.kind,
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
        match closer::close_program(&rpc, &program, &cfg.signer, &cfg.fee_payer).await {
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
