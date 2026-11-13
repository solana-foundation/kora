#![allow(deprecated)]

pub mod state;
#[cfg(test)]
pub mod tests;

use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget::compute_budget_limits::{
    DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT, MAX_COMPUTE_UNIT_LIMIT,
    MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES,
};
use solana_loader_v3_interface::{instruction as loader_v3, state::UpgradeableLoaderState};
use solana_message::{v1, VersionedMessage};
use solana_packet::PACKET_DATA_SIZE;
use solana_sdk::{
    account::Account,
    hash::{hash, Hash},
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use solana_transaction::versioned::VersionedTransaction;

use crate::state::{ChunkWriterState, DeployState, StateLockGuard, UpgradeState};

// Write instruction data is a u32 enum tag, a u32 offset and a u64 length prefix before the bytes.
const WRITE_INSTRUCTION_HEADER_LEN: usize = 16;
/// The loader rejects instruction data over PACKET_DATA_SIZE regardless of transaction version.
pub const WRITE_CHUNK_SIZE: usize = PACKET_DATA_SIZE - WRITE_INSTRUCTION_HEADER_LEN;
pub const WRITES_PER_TX: usize = 3;
const BPF_LOADER_UPGRADEABLE: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
const SYSTEM_PROGRAM: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

pub const DEFAULT_REGISTRY_PROGRAM: Pubkey =
    solana_sdk::pubkey!("CPoBCCbvawmR2S6joHjXgfFkh9pGqzSixBe2BaBwbVkx");
const REGISTRY_IX_REGISTER: u8 = 0;
const REGISTRY_IX_CLOSE_ENTRY: u8 = 1;
const REGISTRY_ENTRY_LEN: usize = 65;

pub struct DeployConfig<'a> {
    pub kora_url: &'a str,
    pub rpc_url: &'a str,
    pub program_so: &'a Path,
    pub user_id: String,
    /// When set, the deploy transaction registers this wallet as the program's owner in the
    /// deploy registry, allowing future upgrades signed by it. Without a wallet the program
    /// is immutable through the paymaster.
    pub wallet: Option<&'a Keypair>,
    pub resume: bool,
    pub cleanup_on_failure: bool,
    pub state_path: PathBuf,
}

pub struct UpgradeConfig<'a> {
    pub kora_url: &'a str,
    pub rpc_url: &'a str,
    pub program_so: &'a Path,
    pub user_id: String,
    pub program: Pubkey,
    pub wallet: &'a Keypair,
    pub resume: bool,
    pub cleanup_on_failure: bool,
    pub state_path: PathBuf,
}

#[derive(Debug)]
pub struct DeployResult {
    pub kora_pubkey: Pubkey,
    pub program: Pubkey,
    pub program_data: Pubkey,
    pub buffer: Pubkey,
}

struct DeployCtx<'a> {
    http: &'a reqwest::Client,
    rpc: &'a Arc<RpcClient>,
    kora_url: &'a str,
    user_id: &'a str,
    state_path: &'a Path,
    cleanup_on_failure: bool,
    resume: bool,
}

impl<'a> DeployCtx<'a> {
    fn from_config(
        cfg: &'a DeployConfig<'_>,
        http: &'a reqwest::Client,
        rpc: &'a Arc<RpcClient>,
    ) -> Self {
        Self {
            http,
            rpc,
            kora_url: cfg.kora_url,
            user_id: &cfg.user_id,
            state_path: cfg.state_path.as_path(),
            cleanup_on_failure: cfg.cleanup_on_failure,
            resume: cfg.resume,
        }
    }

    fn from_upgrade_config(
        cfg: &'a UpgradeConfig<'_>,
        http: &'a reqwest::Client,
        rpc: &'a Arc<RpcClient>,
    ) -> Self {
        Self {
            http,
            rpc,
            kora_url: cfg.kora_url,
            user_id: &cfg.user_id,
            state_path: cfg.state_path.as_path(),
            cleanup_on_failure: cfg.cleanup_on_failure,
            resume: cfg.resume,
        }
    }
}

macro_rules! cleanup_buffer {
    ($force:expr, $msg:expr, $err_log:expr, $ctx:expr, $buffer:expr, $kora_pubkey:expr) => {
        if $force || $ctx.cleanup_on_failure {
            log::warn!("{}, attempting to close buffer for cleanup...", $msg);
            let close_ix = loader_v3::close_any(
                &$buffer.pubkey(),
                $kora_pubkey,
                Some($kora_pubkey),
                None,
            );
            match submit_returning_signature(
                $ctx.http,
                $ctx.kora_url,
                $ctx.user_id,
                $ctx.rpc,
                $kora_pubkey,
                &[close_ix],
                &[],
            )
            .await
            {
                Ok(_) => {
                    let state_path = $ctx.state_path;
                    if state_path.exists() {
                        if let Err(err) = fs::remove_file(state_path) {
                            log::warn!(
                                "Buffer closed successfully, but failed to delete state file: {}. Please manually delete {} to start a fresh deployment.",
                                err,
                                state_path.display()
                            );
                        }
                    }
                }
                Err(e) => {
                    $err_log(e);
                }
            }
        }
    };
}

fn read_program_binary(path: &Path) -> Result<(Vec<u8>, String)> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.is_empty() {
        bail!("program binary {} is empty", path.display());
    }
    let current_program_hash = hash(&bytes).to_string();
    Ok((bytes, current_program_hash))
}

/// Deploys a program via the paymaster, writing chunks to an intermediate buffer.
pub async fn deploy(cfg: &DeployConfig<'_>) -> Result<DeployResult> {
    let _guard = StateLockGuard::acquire(cfg.state_path.as_path())?;

    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        cfg.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    ));
    let ctx = DeployCtx::from_config(cfg, &http, &rpc);

    deploy_inner(&ctx, cfg).await
}

async fn deploy_inner(ctx: &DeployCtx<'_>, cfg: &DeployConfig<'_>) -> Result<DeployResult> {
    let (bytes, current_program_hash) = read_program_binary(cfg.program_so)?;

    let (program, buffer, program_data, kora_pubkey, mut state) =
        load_or_init_deploy_state(ctx, &bytes, &current_program_hash, cfg.wallet).await?;
    let chunk_size = resume_chunk_size(&state)
        .with_context(|| format!("deploy state at {}", cfg.state_path.display()))?;
    let chunk_count = bytes.len().div_ceil(chunk_size);

    validate_deploy_state(ctx, &buffer, &kora_pubkey, &state, &current_program_hash, chunk_count)
        .await?;

    if ctx.resume {
        let is_already_live = ctx
            .rpc
            .get_account_with_commitment(&program_data, CommitmentConfig::confirmed())
            .await?
            .value
            .is_some();
        if is_already_live {
            log::info!("Program {} is already live, skipping deployment.", program.pubkey());
            remove_state_file_on_success(ctx.state_path, "Deploy");
            return Ok(DeployResult {
                kora_pubkey,
                program: program.pubkey(),
                program_data,
                buffer: buffer.pubkey(),
            });
        }
        let buffer_account = ctx
            .rpc
            .get_account_with_commitment(&buffer.pubkey(), CommitmentConfig::confirmed())
            .await?
            .value;
        if buffer_account.is_none() {
            bail!(
                "Buffer {} is no longer live on-chain. Cannot resume. \
                 Please delete {} to start over.",
                buffer.pubkey(),
                ctx.state_path.display()
            );
        }
    }

    write_chunks(ctx, &bytes, &buffer, &kora_pubkey, &mut state, chunk_size, chunk_count).await?;

    finalize_deploy(ctx, &bytes, &program, &buffer, &kora_pubkey, cfg.wallet).await?;

    Ok(DeployResult {
        kora_pubkey,
        program: program.pubkey(),
        program_data,
        buffer: buffer.pubkey(),
    })
}

async fn create_buffer_account(
    ctx: &DeployCtx<'_>,
    kora_pubkey: &Pubkey,
    buffer: &Keypair,
    program_len: usize,
) -> Result<()> {
    let buffer_lamports = ctx
        .rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(program_len))
        .await?;
    let create_buf = loader_v3::create_buffer(
        kora_pubkey,
        &buffer.pubkey(),
        kora_pubkey,
        buffer_lamports,
        program_len,
    )?;
    submit(ctx.http, ctx.kora_url, ctx.user_id, ctx.rpc, kora_pubkey, &create_buf, &[buffer])
        .await?;
    Ok(())
}

fn load_or_preflight_state<T, F>(
    state_path: &Path,
    resume: bool,
    action: &str,
    load_fn: F,
) -> Result<Option<T>>
where
    F: FnOnce(&Path) -> Result<Option<T>>,
{
    if resume {
        let state = load_fn(state_path)?.ok_or_else(|| {
            anyhow::anyhow!(
                "--resume was requested, but no state file was found at {}. Cannot resume.",
                state_path.display()
            )
        })?;
        Ok(Some(state))
    } else {
        if state_path.exists() {
            anyhow::bail!(
                "State file exists at {}. A previous {action} failed.\n\
                Run with `--resume` to continue, or delete the file to start over (WARNING: deleting it orphans the on-chain buffer and leaks SOL).",
                state_path.display()
            );
        }
        Ok(None)
    }
}

async fn load_or_init_deploy_state(
    ctx: &DeployCtx<'_>,
    bytes: &[u8],
    current_program_hash: &str,
    wallet: Option<&Keypair>,
) -> Result<(Keypair, Keypair, Pubkey, Pubkey, DeployState)> {
    let state_path = ctx.state_path;
    let state = load_or_preflight_state(state_path, ctx.resume, "deploy", DeployState::load)?;

    if let Some(st) = state {
        let program = Keypair::try_from(st.program_keypair.as_slice())
            .context("invalid program keypair in state")?;
        let buffer = Keypair::try_from(st.buffer_keypair.as_slice())
            .context("invalid buffer keypair in state")?;
        let program_data =
            Pubkey::from_str(&st.program_data).context("invalid program data pubkey in state")?;
        let kora_pubkey =
            Pubkey::from_str(&st.kora_pubkey).context("invalid kora pubkey in state")?;

        match &st.wallet_pubkey {
            None => {}
            Some(None) => {
                if wallet.is_some() {
                    bail!("State file was created without a wallet (immutable deployment). Cannot resume with a wallet.");
                }
            }
            Some(Some(expected_wallet)) => {
                let actual = wallet.map(|w| w.pubkey().to_string());
                if actual.as_deref() != Some(expected_wallet.as_str()) {
                    bail!(
                        "State file was created with wallet {expected_wallet}, but provided wallet is {:?}. \
                         Sign with the registered wallet to resume.",
                        actual
                    );
                }
            }
        }

        log::info!("resuming deployment from state file, skipping {} chunks", st.written_chunks);
        Ok((program, buffer, program_data, kora_pubkey, st))
    } else {
        let kora_pubkey = fetch_kora_pubkey(ctx.http, ctx.kora_url).await?;
        let program = Keypair::new();
        let buffer = Keypair::new();

        create_buffer_account(ctx, &kora_pubkey, &buffer, bytes.len()).await?;

        let program_data = derive_program_data_address(&program.pubkey());
        let new_state = DeployState {
            program_keypair: program.to_bytes().to_vec(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            program_data: program_data.to_string(),
            written_chunks: 0,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash.to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            wallet_pubkey: Some(wallet.map(|w| w.pubkey().to_string())),
        };
        if let Err(e) = new_state.save(state_path) {
            // Without the state file, the buffer keypair cannot be recovered on failure.
            cleanup_buffer!(
                true,
                "failed to save initial state",
                |err| log::warn!(
                    "failed to close buffer: {}; please manually close the buffer with pubkey {}",
                    err,
                    buffer.pubkey()
                ),
                ctx,
                &buffer,
                &kora_pubkey
            );
            return Err(e.context("failed to save initial deploy state"));
        }

        Ok((program, buffer, program_data, kora_pubkey, new_state))
    }
}

fn validate_chunk_size(chunk_size: usize) -> Result<usize> {
    if chunk_size == 0 || chunk_size > WRITE_CHUNK_SIZE {
        bail!("chunk_size {chunk_size} is outside 1..={WRITE_CHUNK_SIZE}; delete it to start over");
    }
    Ok(chunk_size)
}

fn resume_chunk_size<S: ChunkWriterState>(state: &S) -> Result<usize> {
    validate_chunk_size(state.chunk_size())
}

enum StateInvariantViolation {
    HashMismatch,
    ChunkOverflow { written: usize, total: usize },
}

fn check_state_invariants<S: ChunkWriterState>(
    state: &S,
    current_program_hash: &str,
    chunk_count: usize,
) -> Option<StateInvariantViolation> {
    if state.program_hash() != current_program_hash {
        return Some(StateInvariantViolation::HashMismatch);
    }
    if state.written_chunks() > chunk_count {
        return Some(StateInvariantViolation::ChunkOverflow {
            written: state.written_chunks(),
            total: chunk_count,
        });
    }
    None
}

async fn handle_state_invariant_violation(
    ctx: &DeployCtx<'_>,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    violation: StateInvariantViolation,
) -> Result<()> {
    match violation {
        StateInvariantViolation::HashMismatch => {
            // Preserve the buffer and state file so the user can restore the original binary.
            let buffer_pubkey = buffer.pubkey();
            anyhow::bail!(
                "The .so file has changed (hash mismatch). Cannot resume.\n\
                 Buffer account: {buffer_pubkey}\n\
                 Either restore the original .so file, or delete {} to start over.\n\
                 If you choose to start over, you must close the buffer account first, otherwise the rent stays locked.",
                ctx.state_path.display()
            );
        }
        StateInvariantViolation::ChunkOverflow { written, total } => {
            cleanup_buffer!(
                false,
                "written_chunks exceeds chunk_count (corrupted state or smaller .so file)",
                |e| log::warn!(
                    "failed to close buffer: {}; keeping state file for manual recovery",
                    e
                ),
                ctx,
                buffer,
                kora_pubkey
            );

            let cleanup_status = if ctx.cleanup_on_failure {
                "Buffer cleanup was attempted."
            } else {
                "Buffer cleanup skipped (--no-cleanup-on-failure was set)."
            };
            anyhow::bail!(
                "State file is corrupted or .so file is smaller: written_chunks ({written}) > total chunk count ({total}). \
                 Cannot resume. {cleanup_status}"
            );
        }
    }
}

async fn validate_state_invariants<S: ChunkWriterState>(
    ctx: &DeployCtx<'_>,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    state: &S,
    current_program_hash: &str,
    chunk_count: usize,
) -> Result<()> {
    if let Some(violation) = check_state_invariants(state, current_program_hash, chunk_count) {
        handle_state_invariant_violation(ctx, buffer, kora_pubkey, violation).await?;
    }
    Ok(())
}

async fn validate_deploy_state(
    ctx: &DeployCtx<'_>,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    state: &DeployState,
    current_program_hash: &str,
    chunk_count: usize,
) -> Result<()> {
    validate_state_invariants(ctx, buffer, kora_pubkey, state, current_program_hash, chunk_count)
        .await
}

async fn validate_upgrade_state(
    ctx: &DeployCtx<'_>,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    state: &UpgradeState,
    program: &Pubkey,
    current_program_hash: &str,
    chunk_count: usize,
) -> Result<()> {
    if state.program != program.to_string() {
        anyhow::bail!(
            "State file is for program {}, but current target is {}. Cannot resume.",
            state.program,
            program
        );
    }
    validate_state_invariants(ctx, buffer, kora_pubkey, state, current_program_hash, chunk_count)
        .await
}

async fn load_or_init_upgrade_state(
    ctx: &DeployCtx<'_>,
    bytes: &[u8],
    current_program_hash: &str,
    program: &Pubkey,
) -> Result<Option<(Keypair, Pubkey, UpgradeState)>> {
    let state_path = ctx.state_path;
    let state = load_or_preflight_state(state_path, ctx.resume, "upgrade", UpgradeState::load)?;

    if let Some(st) = state {
        let _ = Pubkey::from_str(&st.program).context("invalid program pubkey in state")?;
        let buffer = Keypair::try_from(st.buffer_keypair.as_slice())
            .context("invalid buffer keypair in state")?;
        let kora_pubkey =
            Pubkey::from_str(&st.kora_pubkey).context("invalid kora pubkey in state")?;
        log::info!("resuming upgrade from state file, skipping {} chunks", st.written_chunks);
        Ok(Some((buffer, kora_pubkey, st)))
    } else {
        let kora_pubkey = fetch_kora_pubkey(ctx.http, ctx.kora_url).await?;
        let program_data = derive_program_data_address(program);
        // Snapshot slot before upgrade to verify advancement on resume after timeout.
        let pdata_account = ctx
            .rpc
            .get_account(&program_data)
            .await
            .with_context(|| format!("fetching ProgramData account {program_data}"))?;
        let meta_len = UpgradeableLoaderState::size_of_programdata_metadata();
        if pdata_account.data.len() < meta_len {
            bail!("ProgramData account {program_data} data is too short");
        }
        let loader_state: UpgradeableLoaderState =
            bincode::deserialize(&pdata_account.data[..meta_len])
                .context("deserializing ProgramData header")?;
        let program_data_slot_at_start = match loader_state {
            UpgradeableLoaderState::ProgramData { slot, .. } => slot,
            other => bail!("expected ProgramData, got {other:?}"),
        };

        // Skip redundant buffer allocation if target binary is already active on-chain.
        if pdata_account.data.len() >= meta_len + bytes.len() {
            let onchain_hash =
                hash(&pdata_account.data[meta_len..meta_len + bytes.len()]).to_string();
            if onchain_hash == current_program_hash {
                log::info!(
                    "Program {program} is already up to date with this binary, skipping upgrade."
                );
                remove_state_file_on_success(state_path, "Upgrade");
                return Ok(None);
            }
        }

        let buffer = Keypair::new();
        create_buffer_account(ctx, &kora_pubkey, &buffer, bytes.len()).await?;

        let new_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 0,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash.to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start,
        };
        if let Err(e) = new_state.save(state_path) {
            // Without the state file, the buffer keypair cannot be recovered on failure.
            cleanup_buffer!(
                true,
                "failed to save initial state",
                |err| log::warn!(
                    "failed to close buffer: {}; please manually close the buffer with pubkey {}",
                    err,
                    buffer.pubkey()
                ),
                ctx,
                &buffer,
                &kora_pubkey
            );
            return Err(e.context("failed to save initial upgrade state"));
        }

        Ok(Some((buffer, kora_pubkey, new_state)))
    }
}

async fn write_chunks<S: ChunkWriterState>(
    ctx: &DeployCtx<'_>,
    bytes: &[u8],
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    state: &mut S,
    chunk_size: usize,
    chunk_count: usize,
) -> Result<()> {
    let state_path = ctx.state_path;
    let mut written_chunks = state.written_chunks();
    let batches =
        write_instruction_batches(&buffer.pubkey(), kora_pubkey, bytes, chunk_size, written_chunks);
    for ixs in batches {
        match submit(ctx.http, ctx.kora_url, ctx.user_id, ctx.rpc, kora_pubkey, &ixs, &[]).await {
            Ok(_) => {
                written_chunks += ixs.len();
                state.set_written_chunks(written_chunks);
                if let Err(e) = state.save(state_path) {
                    cleanup_buffer!(
                        false,
                        "failed to save chunk state",
                        |err| {
                            log::warn!("failed to close buffer: {}; keeping state file for manual recovery", err)
                        },
                        ctx,
                        buffer,
                        kora_pubkey
                    );
                    return Err(e.context("failed to save state after chunk write"));
                }
                if written_chunks % 25 < ixs.len() || written_chunks == chunk_count {
                    log::info!("wrote chunk {}/{}", written_chunks, chunk_count);
                }
            }
            Err(e) => {
                cleanup_buffer!(
                    false,
                    "chunk write failed",
                    |err| log::warn!(
                        "failed to close buffer: {}; keeping state file for manual recovery",
                        err
                    ),
                    ctx,
                    buffer,
                    kora_pubkey
                );
                return Err(e.context("failed to write chunk"));
            }
        }
    }
    Ok(())
}

fn remove_state_file_on_success(state_path: &Path, action: &str) {
    if state_path.exists() {
        if let Err(err) = fs::remove_file(state_path) {
            log::warn!(
                "{action} succeeded, but failed to delete state file: {err}. Please manually delete {} to start a fresh deployment.",
                state_path.display()
            );
        }
    }
}

async fn finalize_deploy(
    ctx: &DeployCtx<'_>,
    bytes: &[u8],
    program: &Keypair,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    wallet: Option<&Keypair>,
) -> Result<()> {
    let state_path = ctx.state_path;
    let program_lamports = ctx
        .rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
        .await?;
    let mut deploy_ixs = loader_v3::deploy_with_max_program_len(
        kora_pubkey,
        &program.pubkey(),
        &buffer.pubkey(),
        kora_pubkey,
        program_lamports,
        bytes.len(),
    )?;
    let mut deploy_signers: Vec<&Keypair> = vec![program];
    if let Some(wallet) = wallet {
        deploy_ixs.push(register_ix(
            &DEFAULT_REGISTRY_PROGRAM,
            kora_pubkey,
            &program.pubkey(),
            &wallet.pubkey(),
        ));
        deploy_signers.push(wallet);
    }

    // Program stub survives reaping; ProgramData presence confirms a timed-out deploy landed.
    let program_data = derive_program_data_address(&program.pubkey());
    let is_already_live = if ctx.resume {
        ctx.rpc
            .get_account_with_commitment(&program_data, CommitmentConfig::confirmed())
            .await?
            .value
            .is_some()
    } else {
        false
    };

    if is_already_live {
        log::info!("Program {} is already live, skipping deployment.", program.pubkey());
        remove_state_file_on_success(state_path, "Deploy");
    } else {
        match submit(
            ctx.http,
            ctx.kora_url,
            ctx.user_id,
            ctx.rpc,
            kora_pubkey,
            &deploy_ixs,
            &deploy_signers,
        )
        .await
        {
            Ok(_) => {
                remove_state_file_on_success(state_path, "Deploy");
            }
            Err(e) => {
                cleanup_buffer!(
                    false,
                    "deploy transaction failed",
                    |err| log::warn!(
                        "failed to close buffer: {}; keeping state file for manual recovery",
                        err
                    ),
                    ctx,
                    buffer,
                    kora_pubkey
                );
                return Err(anyhow::anyhow!(
                    "Failed to submit deploy transaction for {}: {}. \n\
                    Verify status: `solana program show {}`",
                    program.pubkey(),
                    e,
                    program.pubkey()
                ));
            }
        }
    }
    Ok(())
}

async fn check_upgrade_already_landed(
    ctx: &DeployCtx<'_>,
    bytes: &[u8],
    program: &Pubkey,
    buffer: &Keypair,
    state: &UpgradeState,
) -> Result<bool> {
    let buffer_account = ctx
        .rpc
        .get_account_with_commitment(&buffer.pubkey(), CommitmentConfig::confirmed())
        .await?
        .value;

    if buffer_account.is_some() {
        return Ok(false);
    }

    // A landed upgrade drains and closes the buffer; if it is gone, verify ProgramData
    // to detect success after a client timeout.
    let program_data = derive_program_data_address(program);
    let pdata_opt = ctx
        .rpc
        .get_account_with_commitment(&program_data, CommitmentConfig::confirmed())
        .await?
        .value;

    let meta_len = UpgradeableLoaderState::size_of_programdata_metadata();
    let is_landed = pdata_opt.as_ref().is_some_and(|pdata| {
        if pdata.data.len() < meta_len + bytes.len() {
            return false;
        }
        let Ok(UpgradeableLoaderState::ProgramData { slot, .. }) =
            bincode::deserialize(&pdata.data[..meta_len])
        else {
            return false;
        };
        // Upgrades to smaller binaries leave trailing bytes, so hash only the active slice.
        let onchain_hash = hash(&pdata.data[meta_len..meta_len + bytes.len()]).to_string();
        slot > state.program_data_slot_at_start && onchain_hash == state.program_hash
    });

    if is_landed {
        log::info!("Program {program} is already upgraded, skipping upgrade transaction.");
        remove_state_file_on_success(ctx.state_path, "Upgrade");
        Ok(true)
    } else {
        bail!(
            "Buffer {} is gone, but ProgramData {program_data} does not contain the expected binary. \
            Keeping state file {} for manual investigation.",
            buffer.pubkey(),
            ctx.state_path.display()
        );
    }
}

async fn finalize_upgrade(
    ctx: &DeployCtx<'_>,
    program: &Pubkey,
    buffer: &Keypair,
    kora_pubkey: &Pubkey,
    wallet: &Keypair,
) -> Result<Option<Signature>> {
    let state_path = ctx.state_path;

    let mut upgrade_ix = loader_v3::upgrade(program, &buffer.pubkey(), kora_pubkey, kora_pubkey);
    upgrade_ix.accounts.push(AccountMeta::new_readonly(wallet.pubkey(), true));

    match submit_returning_signature(
        ctx.http,
        ctx.kora_url,
        ctx.user_id,
        ctx.rpc,
        kora_pubkey,
        &[upgrade_ix],
        &[wallet],
    )
    .await
    {
        Ok(sig) => {
            remove_state_file_on_success(state_path, "Upgrade");
            Ok(Some(sig))
        }
        Err(e) => {
            cleanup_buffer!(
                false,
                "upgrade transaction failed",
                |err| log::warn!(
                    "failed to close buffer: {}; keeping state file for manual recovery",
                    err
                ),
                ctx,
                buffer,
                kora_pubkey
            );
            Err(anyhow::anyhow!(
                "Failed to submit upgrade transaction for {program}: {e}. \n\
                Verify status: `solana program show {program}`"
            ))
        }
    }
}

/// Upgrades a program via the paymaster. Returns Ok(None) if resuming and
/// the target binary has already landed on-chain.
pub async fn upgrade(cfg: &UpgradeConfig<'_>) -> Result<Option<Signature>> {
    let _guard = StateLockGuard::acquire(cfg.state_path.as_path())?;

    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        cfg.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    ));
    let ctx = DeployCtx::from_upgrade_config(cfg, &http, &rpc);

    upgrade_inner(&ctx, cfg).await
}

async fn upgrade_inner(ctx: &DeployCtx<'_>, cfg: &UpgradeConfig<'_>) -> Result<Option<Signature>> {
    assert_registered_owner(ctx.rpc, &DEFAULT_REGISTRY_PROGRAM, &cfg.program, &cfg.wallet.pubkey())
        .await?;
    let (bytes, current_program_hash) = read_program_binary(cfg.program_so)?;

    let Some((buffer, kora_pubkey, mut state)) =
        load_or_init_upgrade_state(ctx, &bytes, &current_program_hash, &cfg.program).await?
    else {
        return Ok(None);
    };
    let chunk_size = resume_chunk_size(&state)
        .with_context(|| format!("upgrade state at {}", cfg.state_path.display()))?;
    let chunk_count = bytes.len().div_ceil(chunk_size);

    validate_upgrade_state(
        ctx,
        &buffer,
        &kora_pubkey,
        &state,
        &cfg.program,
        &current_program_hash,
        chunk_count,
    )
    .await?;

    if ctx.resume
        && check_upgrade_already_landed(ctx, &bytes, &cfg.program, &buffer, &state).await?
    {
        return Ok(None);
    }

    write_chunks(ctx, &bytes, &buffer, &kora_pubkey, &mut state, chunk_size, chunk_count).await?;

    finalize_upgrade(ctx, &cfg.program, &buffer, &kora_pubkey, cfg.wallet).await
}

/// A program is live when its programdata account exists; the reaper's close removes it
/// while the 36-byte program account survives forever.
pub async fn program_is_live(rpc_url: &str, program: &Pubkey) -> Result<bool> {
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let program_data = derive_program_data_address(program);
    Ok(rpc
        .get_account_with_commitment(&program_data, CommitmentConfig::confirmed())
        .await?
        .value
        .is_some())
}

/// Buffer writes from chunk `first_chunk` on, grouped `WRITES_PER_TX` to a transaction.
pub fn write_instruction_batches(
    buffer: &Pubkey,
    authority: &Pubkey,
    bytes: &[u8],
    chunk_size: usize,
    first_chunk: usize,
) -> Vec<Vec<Instruction>> {
    assert!(chunk_size > 0, "chunk_size must be greater than zero");
    let total_chunks = bytes.len().div_ceil(chunk_size);
    let remaining = total_chunks.saturating_sub(first_chunk);
    let mut batches = Vec::with_capacity(remaining.div_ceil(WRITES_PER_TX));
    let mut current_batch = Vec::with_capacity(WRITES_PER_TX);

    for (i, chunk) in bytes.chunks(chunk_size).enumerate().skip(first_chunk) {
        current_batch.push(loader_v3::write(
            buffer,
            authority,
            (i * chunk_size) as u32,
            chunk.to_vec(),
        ));
        if current_batch.len() == WRITES_PER_TX {
            batches.push(std::mem::replace(&mut current_batch, Vec::with_capacity(WRITES_PER_TX)));
        }
    }
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }
    batches
}

pub fn registry_entry_address(registry_program: &Pubkey, program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], registry_program).0
}

pub fn register_ix(
    registry_program: &Pubkey,
    payer: &Pubkey,
    program: &Pubkey,
    owner: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        *registry_program,
        &[REGISTRY_IX_REGISTER],
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*program, true),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(registry_entry_address(registry_program, program), false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
    )
}

/// Permissionless once the program's programdata account is gone; rent returns to the
/// stored payer, which must be passed as `recipient`.
pub fn close_entry_ix(
    registry_program: &Pubkey,
    program: &Pubkey,
    recipient: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        *registry_program,
        &[REGISTRY_IX_CLOSE_ENTRY],
        vec![
            AccountMeta::new(registry_entry_address(registry_program, program), false),
            AccountMeta::new_readonly(*program, false),
            AccountMeta::new_readonly(derive_program_data_address(program), false),
            AccountMeta::new(*recipient, false),
        ],
    )
}

async fn assert_registered_owner(
    rpc: &RpcClient,
    registry_program: &Pubkey,
    program: &Pubkey,
    wallet: &Pubkey,
) -> Result<()> {
    let entry_address = registry_entry_address(registry_program, program);
    let entry = rpc
        .get_account_with_commitment(&entry_address, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or_else(|| {
            anyhow!(
                "program {program} has no deploy-registry entry (deployed without a wallet?); \
                 it cannot be upgraded through the paymaster"
            )
        })?;
    if entry.owner != *registry_program || entry.data.len() != REGISTRY_ENTRY_LEN {
        bail!("registry entry {entry_address} is malformed");
    }
    let owner = Pubkey::try_from(&entry.data[..32])
        .map_err(|_| anyhow!("registry entry {entry_address} has an invalid owner"))?;
    if owner != *wallet {
        bail!(
            "program {program} is registered to {owner}, but the provided wallet is {wallet}; \
             sign with the registered wallet to upgrade"
        );
    }
    Ok(())
}

pub async fn verify_upgrade_authority(
    rpc_url: &str,
    program_data: &Pubkey,
    expected: &Pubkey,
) -> Result<()> {
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    let pdata_account = rpc.get_account(program_data).await?;
    verify_upgrade_authority_account(&pdata_account, program_data, expected)
}

fn verify_upgrade_authority_account(
    pdata_account: &Account,
    program_data: &Pubkey,
    expected: &Pubkey,
) -> Result<()> {
    let meta_len = UpgradeableLoaderState::size_of_programdata_metadata();
    if pdata_account.data.len() < meta_len {
        bail!("ProgramData account {program_data} data is too short");
    }
    let state: UpgradeableLoaderState = bincode::deserialize(&pdata_account.data[..meta_len])?;
    match state {
        UpgradeableLoaderState::ProgramData { upgrade_authority_address, .. } => {
            if upgrade_authority_address != Some(*expected) {
                bail!("upgrade_authority is {upgrade_authority_address:?}, expected {expected}");
            }
        }
        other => bail!("expected ProgramData, got {other:?}"),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn close(
    rpc_url: &str,
    kora_url: &str,
    user_id: &str,
    kora_pubkey: &Pubkey,
    program: &Pubkey,
    program_data: &Pubkey,
    wallet: Option<&Keypair>,
) -> Result<Signature> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    ));
    wait_for_next_slot(&rpc).await?;
    let mut close_ix =
        loader_v3::close_any(program_data, kora_pubkey, Some(kora_pubkey), Some(program));
    let mut signers: Vec<&Keypair> = Vec::new();
    if let Some(wallet) = wallet {
        close_ix.accounts.push(AccountMeta::new_readonly(wallet.pubkey(), true));
        signers.push(wallet);
    }
    submit_returning_signature(&http, kora_url, user_id, &rpc, kora_pubkey, &[close_ix], &signers)
        .await
}

async fn fetch_kora_pubkey(http: &reqwest::Client, url: &str) -> Result<Pubkey> {
    let resp: Value = http
        .post(url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"getPayerSigner","params":[]}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Pubkey::from_str(
        resp["result"]["signer_address"]
            .as_str()
            .ok_or_else(|| anyhow!("getPayerSigner missing signer_address: {resp}"))?,
    )
    .context("parsing kora pubkey")
}

fn derive_program_data_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], &BPF_LOADER_UPGRADEABLE).0
}

async fn build_b64_tx(
    rpc: &RpcClient,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<String> {
    let blockhash = rpc.get_latest_blockhash().await?;
    encode_tx(fee_payer, ixs, extra_signers, blockhash, resource_config(ixs.len()))
}

pub fn encode_tx(
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
    blockhash: Hash,
    config: v1::TransactionConfig,
) -> Result<String> {
    let tx = build_tx(fee_payer, ixs, extra_signers, blockhash, config)?;
    Ok(B64.encode(wincode::serialize(&tx)?))
}

/// V1 budgets zero for any limit the config leaves unset, so both are set to what legacy
/// transactions got by default: 200k CU per instruction and the maximum loaded data size.
pub fn resource_config(instruction_count: usize) -> v1::TransactionConfig {
    let compute_unit_limit = u32::try_from(instruction_count)
        .unwrap_or(u32::MAX)
        .saturating_mul(DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT)
        .min(MAX_COMPUTE_UNIT_LIMIT);
    v1::TransactionConfig::empty()
        .with_compute_unit_limit(compute_unit_limit)
        .with_loaded_accounts_data_size_limit(MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get())
}

/// Leaves the fee payer's signature slot at its default for Kora to fill.
fn build_tx(
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
    blockhash: Hash,
    config: v1::TransactionConfig,
) -> Result<VersionedTransaction> {
    let message = VersionedMessage::V1(v1::Message::try_compile_with_config(
        fee_payer, ixs, blockhash, config,
    )?);
    let signer_count = usize::from(message.header().num_required_signatures);
    let mut signatures = vec![Signature::default(); signer_count];
    if !extra_signers.is_empty() {
        let message_bytes = message.serialize();
        for signer in extra_signers {
            let index = message.static_account_keys()[..signer_count]
                .iter()
                .position(|key| *key == signer.pubkey())
                .ok_or_else(|| anyhow!("{} is not a required signer", signer.pubkey()))?;
            signatures[index] = signer.try_sign_message(&message_bytes)?;
        }
    }
    Ok(VersionedTransaction { signatures, message })
}

async fn submit(
    http: &reqwest::Client,
    kora_url: &str,
    user_id: &str,
    rpc: &RpcClient,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<()> {
    submit_returning_signature(http, kora_url, user_id, rpc, fee_payer, ixs, extra_signers)
        .await
        .map(|_| ())
}

async fn submit_returning_signature(
    http: &reqwest::Client,
    kora_url: &str,
    user_id: &str,
    rpc: &RpcClient,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<Signature> {
    let tx_b64 = build_b64_tx(rpc, fee_payer, ixs, extra_signers).await?;
    let resp: Value = http
        .post(kora_url)
        .json(&json!({
            "jsonrpc":"2.0","id":1,
            "method":"signAndSendTransaction",
            "params":{"transaction": tx_b64, "user_id": user_id}
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        bail!("Kora rejected: {err}");
    }
    let sig_str = resp["result"]["signature"]
        .as_str()
        .ok_or_else(|| anyhow!("response missing signature: {resp}"))?;
    let sig = Signature::from_str(sig_str)?;
    await_tx(rpc, &sig).await?;
    Ok(sig)
}

async fn await_tx(rpc: &RpcClient, sig: &Signature) -> Result<()> {
    for _ in 0..120 {
        match rpc.confirm_transaction_with_commitment(sig, CommitmentConfig::confirmed()).await {
            Ok(resp) if resp.value => return Ok(()),
            Ok(_) => {}
            Err(err) => {
                log::debug!("transient error confirming {sig}: {err}; retrying...");
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("timed out waiting for {sig}")
}

async fn wait_for_next_slot(rpc: &RpcClient) -> Result<()> {
    let mut start = None;
    for _ in 0..40 {
        match rpc.get_slot().await {
            Ok(slot) => match start {
                None => start = Some(slot),
                Some(s) if slot > s => return Ok(()),
                _ => {}
            },
            Err(err) => {
                log::debug!("transient error fetching slot: {err}; retrying...");
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    match start {
        Some(s) => bail!("slot never advanced past {s}"),
        None => bail!("timed out fetching slot from RPC"),
    }
}

#[cfg(test)]
mod inline_tests {
    use super::*;
    use crate::tests::{
        kora_mock::{mock_get_payer_signer, mock_sign_and_send_error, mock_sign_and_send_success},
        rpc_mock::DeployRpcMockBuilder,
    };
    use solana_sdk::{account::Account, signature::Signature};

    fn make_state(program_hash: &str, written_chunks: usize) -> DeployState {
        DeployState {
            program_keypair: vec![0u8; 64],
            buffer_keypair: vec![0u8; 64],
            program_data: "11111111111111111111111111111111".to_string(),
            written_chunks,
            kora_pubkey: "11111111111111111111111111111111".to_string(),
            program_hash: program_hash.to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            wallet_pubkey: None,
        }
    }

    #[test]
    fn full_write_chunk_fills_but_does_not_exceed_the_loader_instruction_limit() {
        let ix = loader_v3::write(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            0,
            vec![0xAB; WRITE_CHUNK_SIZE],
        );
        assert_eq!(ix.data.len(), PACKET_DATA_SIZE);
    }

    #[test]
    fn full_write_batch_fits_in_a_v1_transaction() {
        let kora = Keypair::new();
        let buffer = Keypair::new();
        let bytes = vec![0xAB; WRITE_CHUNK_SIZE * WRITES_PER_TX];
        let batches = write_instruction_batches(
            &buffer.pubkey(),
            &kora.pubkey(),
            &bytes,
            WRITE_CHUNK_SIZE,
            0,
        );
        assert_eq!(batches.len(), 1);
        let ixs = &batches[0];
        let tx = build_tx(&kora.pubkey(), ixs, &[], Hash::new_unique(), resource_config(ixs.len()))
            .unwrap();

        let serialized = wincode::serialize(&tx).unwrap();
        assert!(
            serialized.len() <= v1::MAX_TRANSACTION_SIZE,
            "{} bytes exceeds the v1 limit",
            serialized.len()
        );
    }

    #[test]
    #[should_panic(expected = "chunk_size must be greater than zero")]
    fn test_write_instruction_batches_chunk_size_zero_panics() {
        let buffer = Pubkey::new_unique();
        let kora = Pubkey::new_unique();
        let bytes = vec![1, 2, 3];
        write_instruction_batches(&buffer, &kora, &bytes, 0, 0);
    }

    #[test]
    fn build_tx_signs_extra_signers_and_leaves_fee_payer_slot_for_kora() {
        let kora = Keypair::new();
        let program = Keypair::new();
        let wallet = Keypair::new();
        let ixs = vec![register_ix(
            &DEFAULT_REGISTRY_PROGRAM,
            &kora.pubkey(),
            &program.pubkey(),
            &wallet.pubkey(),
        )];
        let tx = build_tx(
            &kora.pubkey(),
            &ixs,
            &[&program, &wallet],
            Hash::new_unique(),
            resource_config(1),
        )
        .unwrap();

        let decoded: VersionedTransaction =
            wincode::deserialize(&wincode::serialize(&tx).unwrap()).unwrap();
        let VersionedMessage::V1(message) = &decoded.message else {
            panic!("expected a v1 message");
        };
        assert_eq!(message.config.compute_unit_limit, Some(DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT));
        assert_eq!(
            message.config.loaded_accounts_data_size_limit,
            Some(MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get())
        );

        let keys = decoded.message.static_account_keys();
        let message_bytes = decoded.message.serialize();
        assert_eq!(keys[0], kora.pubkey());
        assert_eq!(decoded.signatures[0], Signature::default());
        for signer in [&program, &wallet] {
            let index = keys.iter().position(|key| *key == signer.pubkey()).unwrap();
            assert!(decoded.signatures[index].verify(signer.pubkey().as_ref(), &message_bytes));
        }
    }

    #[test]
    fn build_tx_rejects_a_signer_the_message_does_not_require() {
        let kora = Keypair::new();
        let buffer = Keypair::new();
        let ix = loader_v3::write(&buffer.pubkey(), &kora.pubkey(), 0, vec![1]);
        assert!(build_tx(
            &kora.pubkey(),
            &[ix],
            &[&buffer],
            Hash::new_unique(),
            resource_config(1)
        )
        .is_err());
    }

    #[test]
    fn write_batches_resume_at_the_chunk_offset_they_stopped_at() {
        let buffer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let chunk_size = 900;
        let bytes = vec![7u8; chunk_size * 7 + 1];
        let batches = write_instruction_batches(&buffer, &authority, &bytes, chunk_size, 2);

        assert_eq!(batches.iter().map(Vec::len).collect::<Vec<_>>(), vec![3, 3]);
        let offsets: Vec<u32> = batches
            .iter()
            .flatten()
            .map(|ix| u32::from_le_bytes(ix.data[4..8].try_into().unwrap()))
            .collect();
        let expected: Vec<u32> = (2..8).map(|i| (i * chunk_size) as u32).collect();
        assert_eq!(offsets, expected);
    }

    #[test]
    fn resume_chunk_size_rejects_sizes_the_loader_cannot_write() {
        assert_eq!(resume_chunk_size(&make_state("h", 0)).unwrap(), WRITE_CHUNK_SIZE);
        for chunk_size in [0, WRITE_CHUNK_SIZE + 1, 3800] {
            let state = DeployState { chunk_size, ..make_state("h", 0) };
            assert!(resume_chunk_size(&state).is_err(), "{chunk_size} accepted");
        }
    }

    #[test]
    fn resource_config_caps_compute_at_the_runtime_maximum() {
        assert_eq!(resource_config(3).compute_unit_limit, Some(600_000));
        assert_eq!(resource_config(8).compute_unit_limit, Some(MAX_COMPUTE_UNIT_LIMIT));
    }

    #[test]
    fn check_state_invariants_passes_when_valid() {
        let st = make_state("abc123", 5);
        assert!(check_state_invariants(&st, "abc123", 10).is_none());
    }

    #[test]
    fn check_state_invariants_passes_at_exact_chunk_count() {
        // written_chunks == chunk_count is valid (all chunks written, ready to finalize)
        let st = make_state("abc123", 10);
        assert!(check_state_invariants(&st, "abc123", 10).is_none());
    }

    #[test]
    fn check_state_invariants_detects_hash_mismatch() {
        let st = make_state("old_hash", 5);
        let violation = check_state_invariants(&st, "new_hash", 10);
        assert!(matches!(violation, Some(StateInvariantViolation::HashMismatch)));
    }

    #[test]
    fn check_state_invariants_hash_mismatch_takes_priority() {
        // Both conditions true — hash mismatch is checked first.
        let st = make_state("old_hash", 99);
        let violation = check_state_invariants(&st, "new_hash", 5);
        assert!(matches!(violation, Some(StateInvariantViolation::HashMismatch)));
    }

    #[test]
    fn check_state_invariants_detects_chunk_overflow() {
        let st = make_state("abc123", 11);
        let violation = check_state_invariants(&st, "abc123", 10);
        assert!(matches!(
            violation,
            Some(StateInvariantViolation::ChunkOverflow { written: 11, total: 10 })
        ));
    }

    #[tokio::test]
    async fn test_load_or_init_deploy_state_fresh_creates_buffer_and_saves_state() {
        let mut kora_server = mockito::Server::new_async().await;
        let kora_url = kora_server.url();
        let payer_mock = mock_get_payer_signer(&mut kora_server, &Pubkey::new_unique().to_string())
            .await
            .expect(1);
        let _ = mock_sign_and_send_success(&mut kora_server, &Signature::new_unique().to_string())
            .await;

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let cfg = DeployConfig {
            kora_url: &kora_url,
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_deploy_state(&ctx, &[1, 2, 3, 4, 5], "dummy-hash", cfg.wallet).await;
        assert!(res.is_ok());
        let (_, _, _, _, state) = res.unwrap();
        assert_eq!(state.program_hash, "dummy-hash");
        assert_eq!(state.written_chunks, 0);

        assert!(state_path.exists());
        let loaded_state = DeployState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded_state.program_hash, "dummy-hash");
        assert_eq!(loaded_state.written_chunks, 0);

        payer_mock.assert_async().await;
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_load_or_init_deploy_state_save_failure_triggers_forced_cleanup() {
        let mut kora_server = mockito::Server::new_async().await;
        let kora_url = kora_server.url();
        let payer_mock = mock_get_payer_signer(&mut kora_server, &Pubkey::new_unique().to_string())
            .await
            .expect(1);
        let sign_mock =
            mock_sign_and_send_success(&mut kora_server, &Signature::new_unique().to_string())
                .await
                .expect(2);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-nonexistent-dir-{}", Pubkey::new_unique()))
            .join("state.json");

        let cfg = DeployConfig {
            kora_url: &kora_url,
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_deploy_state(&ctx, &[1, 2, 3, 4, 5], "dummy-hash", cfg.wallet).await;
        match res {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("failed to save initial deploy state"));
            }
            Ok(_) => panic!("Expected an error"),
        }
        payer_mock.assert_async().await;
        sign_mock.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
        if let Some(parent) = state_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[tokio::test]
    async fn test_load_or_init_deploy_state_fresh_with_existing_state_bails() {
        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-conflict-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mock_rpc = DeployRpcMockBuilder::new().build();
        let cfg = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let Err(err) = load_or_init_deploy_state(&ctx, &[1, 2, 3], "hash", None).await else {
            panic!("expected error");
        };
        assert!(err.to_string().contains("A previous deploy failed"));

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_load_or_init_deploy_state_resume_without_state_file_errors() {
        let state_path =
            std::env::temp_dir().join(format!("kora-deploy-missing-{}.json", Pubkey::new_unique()));
        let mock_rpc = DeployRpcMockBuilder::new().build();
        let cfg = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test".to_string(),
            wallet: None,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let Err(err) = load_or_init_deploy_state(&ctx, &[1, 2, 3], "hash", None).await else {
            panic!("expected error");
        };
        assert!(err.to_string().contains("--resume was requested, but no state file was found"));
    }

    #[tokio::test]
    async fn test_load_or_init_deploy_state_resume_wallet_validation() {
        let wallet_a = Keypair::new();
        let wallet_b = Keypair::new();
        let program = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let program_data = derive_program_data_address(&program.pubkey());

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-wallet-test-{}.json", Pubkey::new_unique()));

        let state_with_wallet = DeployState {
            program_keypair: program.to_bytes().to_vec(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            program_data: program_data.to_string(),
            written_chunks: 2,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: "hash".to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            wallet_pubkey: Some(Some(wallet_a.pubkey().to_string())),
        };
        state_with_wallet.save(&state_path).unwrap();

        let mock_rpc = DeployRpcMockBuilder::new().build();
        let http = reqwest::Client::new();

        // 1. Explicit wallet mismatch errors
        let cfg_wrong = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: Some(&wallet_b),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let ctx_wrong = DeployCtx::from_config(&cfg_wrong, &http, &mock_rpc);
        let Err(err) =
            load_or_init_deploy_state(&ctx_wrong, &[1, 2], "hash", cfg_wrong.wallet).await
        else {
            panic!("expected error");
        };
        assert!(err.to_string().contains("State file was created with wallet"));

        let cfg_none = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let ctx_none = DeployCtx::from_config(&cfg_none, &http, &mock_rpc);
        let Err(err) = load_or_init_deploy_state(&ctx_none, &[1, 2], "hash", cfg_none.wallet).await
        else {
            panic!("expected error");
        };
        assert!(err.to_string().contains("State file was created with wallet"));

        // 2. Explicit match succeeds
        let cfg_match = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: Some(&wallet_a),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let ctx_match = DeployCtx::from_config(&cfg_match, &http, &mock_rpc);
        let res = load_or_init_deploy_state(&ctx_match, &[1, 2], "hash", cfg_match.wallet).await;
        assert!(res.is_ok());

        // 3. Explicit immutable state plus a wallet errors
        let immutable_state =
            DeployState { wallet_pubkey: Some(None), ..state_with_wallet.clone() };
        immutable_state.save(&state_path).unwrap();
        let Err(err) =
            load_or_init_deploy_state(&ctx_match, &[1, 2], "hash", cfg_match.wallet).await
        else {
            panic!("expected error");
        };
        assert!(err.to_string().contains("immutable deployment"));

        // 4. Old-format JSON without wallet_pubkey field plus a wallet resumes successfully
        let old_json = format!(
            r#"{{
                "program_keypair": {:?},
                "buffer_keypair": {:?},
                "program_data": "{}",
                "written_chunks": 2,
                "kora_pubkey": "{}",
                "program_hash": "hash",
                "chunk_size": {}
            }}"#,
            program.to_bytes().to_vec(),
            buffer.to_bytes().to_vec(),
            program_data,
            kora_pubkey,
            WRITE_CHUNK_SIZE
        );
        std::fs::write(&state_path, old_json).unwrap();
        let res = load_or_init_deploy_state(&ctx_match, &[1, 2], "hash", cfg_match.wallet).await;
        assert!(
            res.is_ok(),
            "old-format JSON without wallet_pubkey must resume successfully with a wallet"
        );
        let res_no_wallet =
            load_or_init_deploy_state(&ctx_none, &[1, 2], "hash", cfg_none.wallet).await;
        assert!(
            res_no_wallet.is_ok(),
            "old-format JSON without wallet_pubkey must resume successfully without a wallet"
        );

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_write_chunks_happy_path_writes_all_and_saves_state() {
        let bytes = vec![7u8; WRITE_CHUNK_SIZE * 2 + 100];
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let mut state = make_state("dummy-hash", 0);

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res =
            write_chunks(&ctx, &bytes, &buffer, &kora_pubkey, &mut state, WRITE_CHUNK_SIZE, 3)
                .await;
        assert!(res.is_ok());
        assert_eq!(state.written_chunks, 3);

        assert!(state_path.exists());
        let loaded_state = DeployState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded_state.written_chunks, 3);

        sign_mock.assert_async().await;
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_write_chunks_failure_at_second_chunk_triggers_cleanup_and_stops() {
        let bytes = vec![7u8; WRITE_CHUNK_SIZE * 3 + 100];
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let mut state = make_state("dummy-hash", 0);

        let mut server = mockito::Server::new_async().await;

        let m1 = mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
            .await
            .expect(1);
        let m2 = mock_sign_and_send_error(&mut server, 1, "simulated chunk write failure")
            .await
            .expect(1);
        let m3 = mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
            .await
            .expect(1);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res =
            write_chunks(&ctx, &bytes, &buffer, &kora_pubkey, &mut state, WRITE_CHUNK_SIZE, 4)
                .await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("failed to write chunk"));

        assert_eq!(state.written_chunks, 3);

        assert!(!state_path.exists());

        m1.assert_async().await;
        m2.assert_async().await;
        m3.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_write_chunks_failure_with_no_cleanup_preserves_state_file() {
        let bytes = vec![7u8; WRITE_CHUNK_SIZE * 3 + 100];
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-noclean-{}.json", Pubkey::new_unique()));
        let mut state = make_state("dummy-hash", 0);

        let mut server = mockito::Server::new_async().await;
        let m1 = mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
            .await
            .expect(1);
        let m2 =
            mock_sign_and_send_error(&mut server, 1, "simulated chunk failure").await.expect(1);
        let no_close =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();
        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: false,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res =
            write_chunks(&ctx, &bytes, &buffer, &kora_pubkey, &mut state, WRITE_CHUNK_SIZE, 4)
                .await;
        assert!(res.is_err());
        assert!(
            state_path.exists(),
            "State file must be retained on failure when cleanup_on_failure is false"
        );
        let loaded = DeployState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.written_chunks, 3);

        m1.assert_async().await;
        m2.assert_async().await;
        no_close.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_finalize_deploy_already_live_skips_deploy_and_removes_state() {
        let program = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let bytes = vec![1u8, 2, 3];

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_account_info(&Account {
                lamports: 1,
                data: vec![],
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            })
            .build();

        let cfg = DeployConfig {
            kora_url: "http://127.0.0.1:1",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = finalize_deploy(&ctx, &bytes, &program, &buffer, &kora_pubkey, cfg.wallet).await;
        assert!(res.is_ok());
        assert!(!state_path.exists());

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_finalize_deploy_submits_and_removes_state_on_success() {
        let program = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let bytes = vec![1u8, 2, 3];

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = finalize_deploy(&ctx, &bytes, &program, &buffer, &kora_pubkey, cfg.wallet).await;
        assert!(res.is_ok());
        assert!(!state_path.exists());

        sign_mock.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_finalize_deploy_submit_failure_triggers_cleanup_and_returns_error() {
        let program = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let bytes = vec![1u8, 2, 3];

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let m1 =
            mock_sign_and_send_error(&mut server, 1, "simulated deploy failure").await.expect(1);
        let m2 = mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
            .await
            .expect(1);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = finalize_deploy(&ctx, &bytes, &program, &buffer, &kora_pubkey, cfg.wallet).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to submit deploy transaction for"));
        assert!(err_msg.contains("Verify status: `solana program show"));

        assert!(!state_path.exists());

        m1.assert_async().await;
        m2.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_validate_state_hash_mismatch_preserves_buffer_and_state() {
        let state = make_state("old-hash", 0);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = validate_deploy_state(&ctx, &buffer, &kora_pubkey, &state, "new-hash", 5).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("The .so file has changed (hash mismatch). Cannot resume."));
        assert!(err_msg.contains(&buffer.pubkey().to_string()));
        assert!(err_msg.contains("close the buffer account"));
        assert!(err_msg.contains("otherwise the rent stays locked"));
        assert!(state_path.exists(), "State file must be preserved on hash mismatch");

        sign_mock.assert_async().await;
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_validate_state_chunk_overflow_with_cleanup_enabled_closes_buffer() {
        let state = make_state("same-hash", 11);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = validate_deploy_state(&ctx, &buffer, &kora_pubkey, &state, "same-hash", 10).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("written_chunks"));
        assert!(err_msg.contains("Buffer cleanup was attempted."));
        assert!(!state_path.exists(), "State file should be removed after buffer close");

        sign_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_validate_state_chunk_overflow_with_cleanup_disabled_skips_close() {
        let state = make_state("same-hash", 11);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-test-state-{}.json", Pubkey::new_unique()));

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: false,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = validate_deploy_state(&ctx, &buffer, &kora_pubkey, &state, "same-hash", 10).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("written_chunks"));
        assert!(err_msg.contains("Buffer cleanup skipped (--no-cleanup-on-failure was set)."));

        no_call_mock.assert_async().await;
    }

    fn make_upgrade_state(
        program: &Pubkey,
        program_hash: &str,
        written_chunks: usize,
    ) -> UpgradeState {
        UpgradeState {
            program: program.to_string(),
            buffer_keypair: vec![0u8; 64],
            written_chunks,
            kora_pubkey: "11111111111111111111111111111111".to_string(),
            program_hash: program_hash.to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        }
    }

    #[test]
    fn resume_upgrade_chunk_size_rejects_sizes_the_loader_cannot_write() {
        let program = Pubkey::new_unique();
        assert_eq!(
            resume_chunk_size(&make_upgrade_state(&program, "h", 0)).unwrap(),
            WRITE_CHUNK_SIZE
        );
        for chunk_size in [0, WRITE_CHUNK_SIZE + 1, 3800] {
            let state = UpgradeState { chunk_size, ..make_upgrade_state(&program, "h", 0) };
            assert!(resume_chunk_size(&state).is_err(), "{chunk_size} accepted");
        }
    }

    #[test]
    fn check_state_invariants_upgrade_state() {
        let program = Pubkey::new_unique();
        let st = make_upgrade_state(&program, "abc123", 5);
        assert!(check_state_invariants(&st, "abc123", 10).is_none());

        let st_exact = make_upgrade_state(&program, "abc123", 10);
        assert!(check_state_invariants(&st_exact, "abc123", 10).is_none());

        let st_hash_mismatch = make_upgrade_state(&program, "old_hash", 5);
        assert!(matches!(
            check_state_invariants(&st_hash_mismatch, "new_hash", 10),
            Some(StateInvariantViolation::HashMismatch)
        ));

        let st_overflow = make_upgrade_state(&program, "abc123", 11);
        assert!(matches!(
            check_state_invariants(&st_overflow, "abc123", 10),
            Some(StateInvariantViolation::ChunkOverflow { written: 11, total: 10 })
        ));
    }

    #[test]
    fn test_verify_upgrade_authority_short_data_fails() {
        let program_data = Pubkey::new_unique();
        let expected = Pubkey::new_unique();
        let short_account = Account {
            lamports: 100,
            data: vec![0u8; 10],
            owner: BPF_LOADER_UPGRADEABLE,
            executable: false,
            rent_epoch: 0,
        };
        let err =
            verify_upgrade_authority_account(&short_account, &program_data, &expected).unwrap_err();
        assert!(err.to_string().contains("data is too short"));
    }

    #[test]
    fn test_verify_upgrade_authority_wrong_authority_fails() {
        let program_data = Pubkey::new_unique();
        let expected = Pubkey::new_unique();
        let actual_authority = Pubkey::new_unique();
        let account = crate::tests::rpc_mock::make_program_data_account(
            1,
            Some(&actual_authority),
            &[1, 2, 3],
        );
        let err = verify_upgrade_authority_account(&account, &program_data, &expected).unwrap_err();
        assert!(err.to_string().contains("upgrade_authority is"));
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_already_up_to_date_skips_buffer_creation_and_returns_none(
    ) {
        let program = Pubkey::new_unique();
        let upgrade_authority = Pubkey::new_unique();
        let elf_bytes = vec![0xCA; 200];
        let elf_hash = hash(&elf_bytes).to_string();

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_program_data_account(42, Some(&upgrade_authority), &elf_bytes)
            .build();

        let mut server = mockito::Server::new_async().await;
        let payer_mock =
            mock_get_payer_signer(&mut server, &upgrade_authority.to_string()).await.expect(1);
        let no_sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-uptodate-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &elf_bytes, &elf_hash, &program).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none(), "should skip buffer creation and return None");
        assert!(!state_path.exists(), "State file should be cleaned up");

        payer_mock.assert_async().await;
        no_sign_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_fresh_creates_buffer_and_stores_slot() {
        let mut kora_server = mockito::Server::new_async().await;
        let kora_url = kora_server.url();
        let payer_mock = mock_get_payer_signer(&mut kora_server, &Pubkey::new_unique().to_string())
            .await
            .expect(1);
        let sign_mock =
            mock_sign_and_send_success(&mut kora_server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let program = Pubkey::new_unique();
        let upgrade_authority = Pubkey::new_unique();
        let mock_rpc = DeployRpcMockBuilder::new()
            .with_program_data_account(77, Some(&upgrade_authority), &[1, 2, 3])
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let cfg = UpgradeConfig {
            kora_url: &kora_url,
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &[1, 2, 3, 4, 5], "dummy-hash", &program).await;
        assert!(res.is_ok());
        let (buffer, _, state) = res.unwrap().expect("expected Some");

        assert_eq!(state.program, program.to_string());
        assert_eq!(state.buffer_keypair, buffer.to_bytes().to_vec());
        assert_eq!(state.program_hash, "dummy-hash");
        assert_eq!(state.written_chunks, 0);
        assert_eq!(state.chunk_size, WRITE_CHUNK_SIZE);
        assert_eq!(state.program_data_slot_at_start, 77);

        assert!(state_path.exists());
        let loaded = UpgradeState::load(&state_path).unwrap().unwrap();
        assert_eq!(loaded.program, program.to_string());
        assert_eq!(loaded.written_chunks, 0);
        assert_eq!(loaded.program_data_slot_at_start, 77);

        payer_mock.assert_async().await;
        sign_mock.assert_async().await;
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_fresh_with_existing_state_bails() {
        let kora_server = mockito::Server::new_async().await;
        let kora_url = kora_server.url();
        let program = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = UpgradeConfig {
            kora_url: &kora_url,
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &[1, 2, 3], "dummy-hash", &program).await;
        let Err(err) = res else {
            panic!("expected an error");
        };
        let err_msg = err.to_string();
        assert!(err_msg.contains("State file exists at"));
        assert!(err_msg.contains("A previous upgrade failed."));
        assert!(err_msg.contains("Run with `--resume` to continue"));

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_resume_without_state_file_errors() {
        let program = Pubkey::new_unique();
        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        if state_path.exists() {
            let _ = std::fs::remove_file(&state_path);
        }

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = UpgradeConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &[1, 2, 3], "dummy-hash", &program).await;
        let Err(err) = res else {
            panic!("expected an error");
        };
        let err_msg = err.to_string();
        assert!(err_msg.contains("--resume was requested, but no state file was found at"));
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_resume_loads_existing_state() {
        let program = Pubkey::new_unique();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 5,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: "dummy-hash".to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 99,
        };
        initial_state.save(&state_path).unwrap();

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = UpgradeConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &[1, 2, 3], "dummy-hash", &program).await;
        assert!(res.is_ok());
        let (loaded_buffer, loaded_kora, state) = res.unwrap().expect("expected Some");
        assert_eq!(loaded_buffer.pubkey(), buffer.pubkey());
        assert_eq!(loaded_kora, kora_pubkey);
        assert_eq!(state.written_chunks, 5);

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_load_or_init_upgrade_state_save_failure_triggers_forced_cleanup() {
        let mut kora_server = mockito::Server::new_async().await;
        let kora_url = kora_server.url();
        let payer_mock = mock_get_payer_signer(&mut kora_server, &Pubkey::new_unique().to_string())
            .await
            .expect(1);
        let sign_mock =
            mock_sign_and_send_success(&mut kora_server, &Signature::new_unique().to_string())
                .await
                .expect(2);

        let program = Pubkey::new_unique();
        let upgrade_authority = Pubkey::new_unique();
        let mock_rpc = DeployRpcMockBuilder::new()
            .with_program_data_account(77, Some(&upgrade_authority), &[1, 2, 3])
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-nonexistent-dir-{}", Pubkey::new_unique()))
            .join("state.json");

        let cfg = UpgradeConfig {
            kora_url: &kora_url,
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = load_or_init_upgrade_state(&ctx, &[1, 2, 3, 4, 5], "dummy-hash", &program).await;
        match res {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("failed to save initial upgrade state"));
            }
            Ok(_) => panic!("Expected an error"),
        }
        payer_mock.assert_async().await;
        sign_mock.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
        if let Some(parent) = state_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    #[tokio::test]
    async fn test_validate_upgrade_state_program_mismatch_bails_without_cleanup() {
        let program = Pubkey::new_unique();
        let different_program = Pubkey::new_unique();
        let state = make_upgrade_state(&different_program, "hash", 0);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res =
            validate_upgrade_state(&ctx, &buffer, &kora_pubkey, &state, &program, "hash", 5).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("State file is for program"));
        assert!(err_msg.contains("Cannot resume."));

        assert!(state_path.exists(), "State file must not be removed on program mismatch");
        no_call_mock.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_validate_upgrade_state_hash_mismatch_preserves_buffer_and_state() {
        let program = Pubkey::new_unique();
        let state = make_upgrade_state(&program, "old-hash", 0);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res =
            validate_upgrade_state(&ctx, &buffer, &kora_pubkey, &state, &program, "new-hash", 5)
                .await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("The .so file has changed (hash mismatch). Cannot resume."));
        assert!(err_msg.contains(&buffer.pubkey().to_string()));
        assert!(err_msg.contains("close the buffer account"));
        assert!(err_msg.contains("otherwise the rent stays locked"));

        assert!(state_path.exists(), "State file should be preserved on hash mismatch");
        sign_mock.assert_async().await;
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_validate_upgrade_state_chunk_overflow_with_cleanup_closes_buffer() {
        let program = Pubkey::new_unique();
        let state = make_upgrade_state(&program, "same-hash", 15);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let mock_rpc =
            DeployRpcMockBuilder::new().with_blockhash().with_signature_status(10).build();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res =
            validate_upgrade_state(&ctx, &buffer, &kora_pubkey, &state, &program, "same-hash", 10)
                .await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("written_chunks"));
        assert!(err_msg.contains("Buffer cleanup was attempted."));

        assert!(!state_path.exists(), "State file should be cleaned up after buffer close");
        sign_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_validate_upgrade_state_chunk_overflow_without_cleanup_skips_close() {
        let program = Pubkey::new_unique();
        let state = make_upgrade_state(&program, "same-hash", 15);
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-test-state-{}.json", Pubkey::new_unique()));
        std::fs::write(&state_path, b"dummy").unwrap();

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc = DeployRpcMockBuilder::new().build();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program,
            wallet: &Keypair::new(),
            resume: true,
            cleanup_on_failure: false,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res =
            validate_upgrade_state(&ctx, &buffer, &kora_pubkey, &state, &program, "same-hash", 10)
                .await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("written_chunks"));
        assert!(err_msg.contains("Buffer cleanup skipped (--no-cleanup-on-failure was set)."));

        assert!(state_path.exists(), "State file must be retained when cleanup is disabled");
        no_call_mock.assert_async().await;

        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_fresh_end_to_end_succeeds_and_removes_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let mut server = mockito::Server::new_async().await;
        let payer_mock =
            mock_get_payer_signer(&mut server, &kora_pubkey.to_string()).await.expect(1);
        let sign_mocks =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(4);

        let old_bytes = vec![0xBA; WRITE_CHUNK_SIZE * 3 + 10];
        let new_bytes = vec![0xCA; WRITE_CHUNK_SIZE * 3 + 10];
        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_program_data_account(42, Some(&kora_pubkey), &old_bytes)
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-fresh-state-{}.json", Pubkey::new_unique()));
        let program_so =
            std::env::temp_dir().join(format!("kora-upgrade-fresh-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &new_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_ok());
        let sig = res.unwrap();
        assert!(sig.is_some());
        assert!(!state_path.exists(), "State file should be removed on upgrade success");

        payer_mock.assert_async().await;
        sign_mocks.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_resume_midway_skips_written_chunks_and_succeeds() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-resume-mid-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; WRITE_CHUNK_SIZE * 4 + 10];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let sign_mocks =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(3);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_info(&Account {
                lamports: 1_000_000,
                data: vec![1],
                owner: crate::BPF_LOADER_UPGRADEABLE,
                executable: false,
                rent_epoch: 0,
            })
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-resume-mid-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_ok());
        let sig = res.unwrap();
        assert!(sig.is_some());
        assert!(!state_path.exists(), "State file should be removed on upgrade success");

        sign_mocks.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_resume_buffer_gone_program_data_matches_already_upgraded() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-landed-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_not_found()
            .with_program_data_account(50, Some(&kora_pubkey), &elf_bytes)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-landed-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
        assert!(!state_path.exists(), "State file should be removed on already-landed upgrade");

        no_call_mock.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_resume_buffer_gone_program_data_mismatch_bails_and_keeps_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-mismatch-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let different_elf_bytes = vec![0xFE; 100];
        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_not_found()
            .with_program_data_account(50, Some(&kora_pubkey), &different_elf_bytes)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-mismatch-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains(&format!("Buffer {} is gone", buffer.pubkey())));
        assert!(err_msg.contains("does not contain the expected binary"));
        assert!(err_msg.contains("Keeping state file"));
        assert!(state_path.exists(), "State file must be kept for manual investigation");

        no_call_mock.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_resume_buffer_gone_program_data_trailing_padding_matches() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-padding-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let no_call_mock =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(0);

        let mut onchain_pdata_bytes = elf_bytes.clone();
        onchain_pdata_bytes.extend_from_slice(&[0x00; 128]);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_not_found()
            .with_program_data_account(50, Some(&kora_pubkey), &onchain_pdata_bytes)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-padding-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), None);
        assert!(!state_path.exists(), "State file should be removed on already-landed upgrade");

        no_call_mock.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_finalize_fails_with_cleanup_enabled_cleans_buffer_and_removes_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-clean-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let m_upgrade_fail =
            mock_sign_and_send_error(&mut server, 1, "simulated upgrade transaction failure")
                .await
                .expect(1);
        let m_close_success =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(1);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_info(&Account {
                lamports: 1_000_000,
                data: vec![1],
                owner: crate::BPF_LOADER_UPGRADEABLE,
                executable: false,
                rent_epoch: 0,
            })
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-clean-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains(&format!("Failed to submit upgrade transaction for {program}")));
        assert!(err_msg.contains(&format!("solana program show {program}")));
        assert!(!state_path.exists(), "State file must be removed when cleanup succeeds");

        m_upgrade_fail.assert_async().await;
        m_close_success.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_finalize_fails_with_cleanup_disabled_skips_cleanup_and_keeps_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-noclean-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let m_upgrade_fail =
            mock_sign_and_send_error(&mut server, 1, "simulated upgrade transaction failure")
                .await
                .expect(1);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_info(&Account {
                lamports: 1_000_000,
                data: vec![1],
                owner: crate::BPF_LOADER_UPGRADEABLE,
                executable: false,
                rent_epoch: 0,
            })
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-noclean-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: false,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains(&format!("Failed to submit upgrade transaction for {program}")));
        assert!(err_msg.contains(&format!("solana program show {program}")));
        assert!(state_path.exists(), "State file must be retained when cleanup is disabled");

        m_upgrade_fail.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_finalize_fails_and_close_fails_keeps_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-timedout-state-{}.json", Pubkey::new_unique()));

        let elf_bytes = vec![0xCA; 100];
        let current_program_hash = hash(&elf_bytes).to_string();

        let initial_state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: current_program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 42,
        };
        initial_state.save(&state_path).unwrap();

        let mut server = mockito::Server::new_async().await;
        let m_upgrade_fail =
            mock_sign_and_send_error(&mut server, 1, "client timed out waiting for tx")
                .await
                .expect(1);
        let m_close_fail =
            mock_sign_and_send_error(&mut server, 2, "buffer account not found or already closed")
                .await
                .expect(1);

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_info(&Account {
                lamports: 1_000_000,
                data: vec![1],
                owner: crate::BPF_LOADER_UPGRADEABLE,
                executable: false,
                rent_epoch: 0,
            })
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let program_so = std::env::temp_dir()
            .join(format!("kora-upgrade-fail-timedout-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = UpgradeConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains(&format!("Failed to submit upgrade transaction for {program}")));
        assert!(err_msg.contains(&format!("solana program show {program}")));
        assert!(
            state_path.exists(),
            "State file must be retained when buffer close fails (e.g. landed-but-timed-out)"
        );

        m_upgrade_fail.assert_async().await;
        m_close_fail.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_fails_fast_when_state_lock_held() {
        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-lock-test-{}.json", Pubkey::new_unique()));
        let _guard = StateLockGuard::acquire(&state_path).unwrap();

        let cfg = UpgradeConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            program: Pubkey::new_unique(),
            wallet: &Keypair::new(),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let res = upgrade(&cfg).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Another kora-deploy process is using state file"));

        let lock_path = StateLockGuard::lock_path(&state_path);
        drop(_guard);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[tokio::test]
    async fn test_deploy_fails_fast_when_state_lock_held() {
        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-lock-test-{}.json", Pubkey::new_unique()));
        let _guard = StateLockGuard::acquire(&state_path).unwrap();

        let cfg = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: Path::new("unused.so"),
            user_id: "test-user".to_string(),
            wallet: None,
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let res = deploy(&cfg).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Another kora-deploy process is using state file"));

        let lock_path = StateLockGuard::lock_path(&state_path);
        drop(_guard);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[tokio::test]
    async fn test_deploy_resume_buffer_not_found_bails() {
        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-resume-nobuf-{}.json", Pubkey::new_unique()));
        let program_so = std::env::temp_dir()
            .join(format!("kora-deploy-resume-nobuf-{}.so", Pubkey::new_unique()));
        let bytes = vec![1u8, 2, 3, 4, 5];
        std::fs::write(&program_so, &bytes).unwrap();
        let program_hash = hash(&bytes).to_string();

        let program = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let program_data = derive_program_data_address(&program.pubkey());

        let state = DeployState {
            program_keypair: program.to_bytes().to_vec(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            program_data: program_data.to_string(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash,
            chunk_size: WRITE_CHUNK_SIZE,
            wallet_pubkey: None,
        };
        state.save(&state_path).unwrap();

        let mock_rpc =
            DeployRpcMockBuilder::new().with_account_not_found().with_account_not_found().build();

        let cfg = DeployConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            wallet: None,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = deploy_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("is no longer live on-chain. Cannot resume"));

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_deploy_fresh_end_to_end_succeeds_and_removes_state() {
        let wallet = Keypair::new();
        let kora_keypair = Keypair::new();
        let kora_pubkey = kora_keypair.pubkey();

        let mut server = mockito::Server::new_async().await;
        let payer_mock =
            mock_get_payer_signer(&mut server, &kora_pubkey.to_string()).await.expect(1);
        let sign_mocks =
            mock_sign_and_send_success(&mut server, &Signature::new_unique().to_string())
                .await
                .expect(4);

        let elf_bytes = vec![0xCA; WRITE_CHUNK_SIZE * 3 + 10];
        let mock_rpc = DeployRpcMockBuilder::new()
            .with_minimum_balance_for_rent_exemption(1_000_000)
            .with_blockhash()
            .with_signature_status(10)
            .build();

        let state_path = std::env::temp_dir()
            .join(format!("kora-deploy-fresh-state-{}.json", Pubkey::new_unique()));
        let program_so =
            std::env::temp_dir().join(format!("kora-deploy-fresh-so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();

        let cfg = DeployConfig {
            kora_url: &server.url(),
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test-user".to_string(),
            wallet: Some(&wallet),
            resume: false,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };

        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_config(&cfg, &http, &mock_rpc);

        let res = deploy_inner(&ctx, &cfg).await;
        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result.kora_pubkey, kora_pubkey);
        assert!(!state_path.exists(), "State file should be removed on deploy success");

        payer_mock.assert_async().await;
        sign_mocks.assert_async().await;

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }

    #[tokio::test]
    async fn test_upgrade_resume_buffer_gone_slot_not_advanced_bails_and_keeps_state() {
        let program = Pubkey::new_unique();
        let wallet = Keypair::new();
        let buffer = Keypair::new();
        let kora_pubkey = Pubkey::new_unique();
        let state_path = std::env::temp_dir()
            .join(format!("kora-upgrade-slot-test-{}.json", Pubkey::new_unique()));
        let elf_bytes = vec![0xCA; 100];
        let state = UpgradeState {
            program: program.to_string(),
            buffer_keypair: buffer.to_bytes().to_vec(),
            written_chunks: 1,
            kora_pubkey: kora_pubkey.to_string(),
            program_hash: hash(&elf_bytes).to_string(),
            chunk_size: WRITE_CHUNK_SIZE,
            program_data_slot_at_start: 100,
        };
        state.save(&state_path).unwrap();

        let mock_rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&DEFAULT_REGISTRY_PROGRAM, &wallet.pubkey())
            .with_account_not_found()
            .with_program_data_account(100, Some(&kora_pubkey), &elf_bytes)
            .build();

        let program_so = std::env::temp_dir().join(format!("so-{}.so", Pubkey::new_unique()));
        std::fs::write(&program_so, &elf_bytes).unwrap();
        let cfg = UpgradeConfig {
            kora_url: "unused",
            rpc_url: "unused",
            program_so: &program_so,
            user_id: "test".to_string(),
            program,
            wallet: &wallet,
            resume: true,
            cleanup_on_failure: true,
            state_path: state_path.clone(),
        };
        let http = reqwest::Client::new();
        let ctx = DeployCtx::from_upgrade_config(&cfg, &http, &mock_rpc);

        let res = upgrade_inner(&ctx, &cfg).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("does not contain the expected binary"));
        assert!(state_path.exists());

        let _ = std::fs::remove_file(&program_so);
        let _ = std::fs::remove_file(&state_path);
    }
}
