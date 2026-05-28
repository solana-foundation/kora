//! Decide whether a discovered program is "idle enough" to close.
//!
//! Primary signal: `getSignaturesForAddress(program, limit=1)` and compare
//! `block_time` to `now - threshold`. If the RPC returned no signatures (the
//! program has never been called, or history was pruned), fall back to the
//! program's last-state slot read during discovery — convert it to a wall
//! time via `getBlockTime(slot)`. If even that fails, treat the program as
//! idle (safe default for a devnet faucet — we'd rather reap an inactive
//! program than leak rent forever).

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
    /// Last activity is within the threshold — leave alone.
    Recent { last_seen_unix: i64 },
    /// Last activity is older than the threshold — eligible for close.
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
            return Ok(if block_time >= cutoff {
                ActivityVerdict::Recent { last_seen_unix: block_time }
            } else {
                ActivityVerdict::Idle { last_seen_unix: Some(block_time) }
            });
        }
        // Sig exists but block_time missing — RPC dropped it, treat as recent
        // to be safe (don't close something we can't measure).
        return Ok(ActivityVerdict::Recent { last_seen_unix: now });
    }

    // Fallback: program has no recorded sigs in the RPC window. Use the
    // last-state slot from discovery as a coarse signal.
    if program.last_state_slot == 0 {
        return Ok(ActivityVerdict::Idle { last_seen_unix: None });
    }

    match rpc.get_block_time(program.last_state_slot).await {
        Ok(block_time) => Ok(if block_time >= cutoff {
            ActivityVerdict::Recent { last_seen_unix: block_time }
        } else {
            ActivityVerdict::Idle { last_seen_unix: Some(block_time) }
        }),
        Err(e) => {
            log::debug!(
                "getBlockTime({}) failed for {}: {e}; treating as idle",
                program.last_state_slot,
                program.program
            );
            Ok(ActivityVerdict::Idle { last_seen_unix: None })
        }
    }
}
