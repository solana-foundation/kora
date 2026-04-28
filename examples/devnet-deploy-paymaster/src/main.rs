#![allow(deprecated)] // loader-v3 helpers are tagged "use loader-v4" but v4 is feature-gated.

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use clap::Parser;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_loader_v3_interface::{instruction as loader_v3, state::UpgradeableLoaderState};
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};

const WRITE_CHUNK_SIZE: usize = 900;
const DEFAULT_PROGRAM_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/src/common/transfer-hook-example/transfer_hook_example.so"
);
const BPF_LOADER_UPGRADEABLE: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

#[derive(Parser)]
#[command(about = "End-to-end smoke test: deploy a program through the live Kora paymaster")]
struct Args {
    /// Kora paymaster URL.
    #[arg(long, default_value = "https://kora-devnet-paymaster-kysurhpjxq-uc.a.run.app")]
    kora_url: String,

    /// Solana RPC for reading on-chain state.
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to a `.so` program. Defaults to the transfer-hook-example bundled with the tests.
    #[arg(long)]
    program_so: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let http = reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?;
    let rpc = Arc::new(RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    ));

    // Fetch Kora's pubkey from the live paymaster (matches the KMS-backed signer).
    let kora_pubkey = fetch_kora_pubkey(&http, &args.kora_url).await?;
    println!("Kora paymaster: {}", args.kora_url);
    println!("Kora pubkey:    {}", kora_pubkey);
    println!("Solana RPC:     {}", args.rpc_url);
    println!();

    let user_id = format!("kora-smoke-{}", Pubkey::new_unique());
    let program = Keypair::new();
    let buffer = Keypair::new();
    let program_path = args.program_so.as_deref().unwrap_or(DEFAULT_PROGRAM_SO);
    let bytes = std::fs::read(program_path).with_context(|| format!("reading {program_path}"))?;
    let chunk_count = bytes.len().div_ceil(WRITE_CHUNK_SIZE);
    let pdata = derive_program_data_address(&program.pubkey());

    println!("[1/6] create_buffer (Kora as authority)");
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
    submit(&http, &args.kora_url, &user_id, &rpc, &kora_pubkey, &create_buf, &[&buffer]).await?;
    println!("      buffer = {}", buffer.pubkey());

    println!("[2/6] writing {} bytes in {} chunks", bytes.len(), chunk_count);
    for (i, chunk) in bytes.chunks(WRITE_CHUNK_SIZE).enumerate() {
        let offset = (i * WRITE_CHUNK_SIZE) as u32;
        let ix = loader_v3::write(&buffer.pubkey(), &kora_pubkey, offset, chunk.to_vec());
        submit(&http, &args.kora_url, &user_id, &rpc, &kora_pubkey, &[ix], &[]).await?;
        if (i + 1) % 25 == 0 || i + 1 == chunk_count {
            println!("      chunk {}/{}", i + 1, chunk_count);
        }
    }

    println!("[3/6] deploy_with_max_program_len (Kora as upgrade authority)");
    let program_lamports = rpc
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
        .await?;
    let deploy_ixs = loader_v3::deploy_with_max_program_len(
        &kora_pubkey,
        &program.pubkey(),
        &buffer.pubkey(),
        &kora_pubkey,
        program_lamports,
        bytes.len(),
    )?;
    submit(&http, &args.kora_url, &user_id, &rpc, &kora_pubkey, &deploy_ixs, &[&program]).await?;
    println!("      program = {}", program.pubkey());

    println!("[4/6] verifying on-chain state");
    let pdata_account = rpc.get_account(&pdata).await?;
    let state: UpgradeableLoaderState = bincode::deserialize(
        &pdata_account.data[..UpgradeableLoaderState::size_of_programdata_metadata()],
    )?;
    match state {
        UpgradeableLoaderState::ProgramData { upgrade_authority_address, .. } => {
            if upgrade_authority_address != Some(kora_pubkey) {
                bail!("upgrade_authority is {upgrade_authority_address:?}, expected {kora_pubkey}");
            }
        }
        other => bail!("expected ProgramData, got {other:?}"),
    }
    println!("      programdata.upgrade_authority == Kora");

    println!("[5/6] waiting for slot to advance (loader-v3 same-block close guard)");
    wait_for_next_slot(&rpc).await?;

    println!("[6/6] close (Kora as authority + recipient → recovers rent)");
    let close_ix =
        loader_v3::close_any(&pdata, &kora_pubkey, Some(&kora_pubkey), Some(&program.pubkey()));
    submit(&http, &args.kora_url, &user_id, &rpc, &kora_pubkey, &[close_ix], &[]).await?;

    println!();
    println!("OK — full deploy lifecycle succeeded against {}", args.kora_url);
    Ok(())
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
    await_tx(rpc, &sig).await
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
