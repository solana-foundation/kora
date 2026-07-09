#![allow(deprecated)]

use std::{path::Path, str::FromStr, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_loader_v3_interface::{instruction as loader_v3, state::UpgradeableLoaderState};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

const WRITE_CHUNK_SIZE: usize = 900;
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
}

pub struct UpgradeConfig<'a> {
    pub kora_url: &'a str,
    pub rpc_url: &'a str,
    pub program_so: &'a Path,
    pub user_id: String,
    pub program: Pubkey,
    pub wallet: &'a Keypair,
}

pub struct DeployResult {
    pub kora_pubkey: Pubkey,
    pub program: Pubkey,
    pub program_data: Pubkey,
    pub buffer: Pubkey,
}

pub async fn deploy(cfg: &DeployConfig<'_>) -> Result<DeployResult> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        cfg.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    ));

    let kora_pubkey = fetch_kora_pubkey(&http, cfg.kora_url).await?;
    let program = Keypair::new();
    let buffer = Keypair::new();
    let bytes = std::fs::read(cfg.program_so)
        .with_context(|| format!("reading {}", cfg.program_so.display()))?;
    let program_data = derive_program_data_address(&program.pubkey());

    let buffer_lamports = rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(bytes.len()))
        .await?;
    let create_buf = loader_v3::create_buffer(
        &kora_pubkey,
        &buffer.pubkey(),
        &kora_pubkey,
        buffer_lamports,
        bytes.len(),
    )?;
    submit(&http, cfg.kora_url, &cfg.user_id, &rpc, &kora_pubkey, &create_buf, &[&buffer]).await?;

    let chunk_count = bytes.len().div_ceil(WRITE_CHUNK_SIZE);
    for (i, chunk) in bytes.chunks(WRITE_CHUNK_SIZE).enumerate() {
        let offset = (i * WRITE_CHUNK_SIZE) as u32;
        let ix = loader_v3::write(&buffer.pubkey(), &kora_pubkey, offset, chunk.to_vec());
        submit(&http, cfg.kora_url, &cfg.user_id, &rpc, &kora_pubkey, &[ix], &[]).await?;
        if (i + 1) % 25 == 0 || i + 1 == chunk_count {
            log::info!("wrote chunk {}/{}", i + 1, chunk_count);
        }
    }

    let program_lamports = rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
        .await?;
    let mut deploy_ixs = loader_v3::deploy_with_max_program_len(
        &kora_pubkey,
        &program.pubkey(),
        &buffer.pubkey(),
        &kora_pubkey,
        program_lamports,
        bytes.len(),
    )?;
    let mut deploy_signers: Vec<&Keypair> = vec![&program];
    if let Some(wallet) = cfg.wallet {
        deploy_ixs.push(register_ix(
            &DEFAULT_REGISTRY_PROGRAM,
            &kora_pubkey,
            &program.pubkey(),
            &wallet.pubkey(),
        ));
        deploy_signers.push(wallet);
    }
    submit(&http, cfg.kora_url, &cfg.user_id, &rpc, &kora_pubkey, &deploy_ixs, &deploy_signers)
        .await?;

    Ok(DeployResult {
        kora_pubkey,
        program: program.pubkey(),
        program_data,
        buffer: buffer.pubkey(),
    })
}

pub async fn upgrade(cfg: &UpgradeConfig<'_>) -> Result<Signature> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        cfg.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
    ));

    let kora_pubkey = fetch_kora_pubkey(&http, cfg.kora_url).await?;
    assert_registered_owner(&rpc, &DEFAULT_REGISTRY_PROGRAM, &cfg.program, &cfg.wallet.pubkey())
        .await?;
    let bytes = std::fs::read(cfg.program_so)
        .with_context(|| format!("reading {}", cfg.program_so.display()))?;

    let buffer = Keypair::new();
    let buffer_lamports = rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(bytes.len()))
        .await?;
    let create_buf = loader_v3::create_buffer(
        &kora_pubkey,
        &buffer.pubkey(),
        &kora_pubkey,
        buffer_lamports,
        bytes.len(),
    )?;
    submit(&http, cfg.kora_url, &cfg.user_id, &rpc, &kora_pubkey, &create_buf, &[&buffer]).await?;

    let chunk_count = bytes.len().div_ceil(WRITE_CHUNK_SIZE);
    for (i, chunk) in bytes.chunks(WRITE_CHUNK_SIZE).enumerate() {
        let offset = (i * WRITE_CHUNK_SIZE) as u32;
        let ix = loader_v3::write(&buffer.pubkey(), &kora_pubkey, offset, chunk.to_vec());
        submit(&http, cfg.kora_url, &cfg.user_id, &rpc, &kora_pubkey, &[ix], &[]).await?;
        if (i + 1) % 25 == 0 || i + 1 == chunk_count {
            log::info!("wrote chunk {}/{}", i + 1, chunk_count);
        }
    }

    let mut upgrade_ix =
        loader_v3::upgrade(&cfg.program, &buffer.pubkey(), &kora_pubkey, &kora_pubkey);
    upgrade_ix.accounts.push(AccountMeta::new_readonly(cfg.wallet.pubkey(), true));
    submit_returning_signature(
        &http,
        cfg.kora_url,
        &cfg.user_id,
        &rpc,
        &kora_pubkey,
        &[upgrade_ix],
        &[cfg.wallet],
    )
    .await
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
    let state: UpgradeableLoaderState = bincode::deserialize(
        &pdata_account.data[..UpgradeableLoaderState::size_of_programdata_metadata()],
    )?;
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
    let msg = Message::new_with_blockhash(ixs, Some(fee_payer), &blockhash);
    let mut tx = Transaction::new_unsigned(msg);
    if !extra_signers.is_empty() {
        tx.partial_sign(extra_signers, blockhash);
    }
    Ok(B64.encode(bincode::serialize(&tx)?))
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
        if rpc.confirm_transaction_with_commitment(sig, CommitmentConfig::confirmed()).await?.value
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("timed out waiting for {sig}")
}

async fn wait_for_next_slot(rpc: &RpcClient) -> Result<()> {
    let start = rpc.get_slot().await?;
    for _ in 0..40 {
        if rpc.get_slot().await? > start {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    bail!("slot never advanced past {start}")
}
