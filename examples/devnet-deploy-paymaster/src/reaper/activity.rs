use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_commitment_config::CommitmentConfig;

use super::OwnedProgram;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityVerdict {
    Recent { last_seen_unix: i64 },
    Idle { last_seen_unix: Option<i64> },
}

pub async fn classify(
    rpc: &Arc<RpcClient>,
    program: &OwnedProgram,
    threshold: Duration,
) -> Result<ActivityVerdict> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow!("system clock before epoch: {e}"))?
        .as_secs() as i64;
    let cutoff = now - threshold.as_secs() as i64;

    let signatures = rpc
        .get_signatures_for_address_with_config(
            &program.program,
            GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: None,
                limit: Some(1),
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await
        .map_err(|e| anyhow!("getSignaturesForAddress({}): {e}", program.program))?;

    if let Some(sig) = signatures.first() {
        if let Some(block_time) = sig.block_time {
            return Ok(verdict(block_time, cutoff));
        }
        // RPC has the sig but no block_time — don't close what we can't measure.
        return Ok(ActivityVerdict::Recent { last_seen_unix: now });
    }

    if program.last_state_slot == 0 {
        return Ok(ActivityVerdict::Idle { last_seen_unix: None });
    }

    match rpc.get_block_time(program.last_state_slot).await {
        Ok(block_time) => Ok(verdict(block_time, cutoff)),
        Err(e) => {
            log::debug!("getBlockTime({}) failed: {e}", program.last_state_slot);
            Ok(ActivityVerdict::Idle { last_seen_unix: None })
        }
    }
}

fn verdict(block_time: i64, cutoff: i64) -> ActivityVerdict {
    if block_time >= cutoff {
        ActivityVerdict::Recent { last_seen_unix: block_time }
    } else {
        ActivityVerdict::Idle { last_seen_unix: Some(block_time) }
    }
}
