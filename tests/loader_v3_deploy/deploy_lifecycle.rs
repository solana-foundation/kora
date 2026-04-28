#![allow(deprecated)] // loader-v3 helpers are tagged "use loader-v4" but v4 is feature-gated.

use crate::common::*;
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use jsonrpsee::rpc_params;
use kora_lib::constant::BPF_LOADER_UPGRADEABLE_PROGRAM_ID;
use serde_json::Value;
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
use std::str::FromStr;

const WRITE_CHUNK_SIZE: usize = 900;
const TEST_PROGRAM_SO_REL: &str = "src/common/transfer-hook-example/transfer_hook_example.so";

#[tokio::test]
async fn deploy_v3_program_through_kora() -> Result<()> {
    let ctx = TestContext::new().await.expect("test context");
    let kora_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let program = Keypair::new();
    let buffer = Keypair::new();
    let attacker = Keypair::new();
    let bytes = read_test_program_so()?;
    let pdata = derive_program_data_address(&program.pubkey());

    // 1. create_buffer (Kora as authority)
    let buffer_lamports = ctx
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(bytes.len()))
        .await?;
    let create_buf = loader_v3::create_buffer(
        &kora_pubkey,
        &buffer.pubkey(),
        &kora_pubkey,
        buffer_lamports,
        bytes.len(),
    )?;
    submit_and_confirm(&ctx, &kora_pubkey, &create_buf, &[&buffer]).await?;

    // 2. Write the full program in chunks (Kora as authority)
    for (i, chunk) in bytes.chunks(WRITE_CHUNK_SIZE).enumerate() {
        let offset = (i * WRITE_CHUNK_SIZE) as u32;
        let ix = loader_v3::write(&buffer.pubkey(), &kora_pubkey, offset, chunk.to_vec());
        submit_and_confirm(&ctx, &kora_pubkey, &[ix], &[]).await?;
    }

    // 3. Deploy with Kora as upgrade authority
    let program_lamports = ctx
        .rpc_client()
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
    submit_and_confirm(&ctx, &kora_pubkey, &deploy_ixs, &[&program]).await?;

    // Loader-v3 Close rejects with InvalidArgument when clock.slot == programdata.slot
    // ("Program was deployed in this block already"); wait for the slot to advance.
    wait_for_next_slot(&ctx).await?;

    // 4. Assert on-chain: programdata.upgrade_authority == Kora
    let pdata_account = ctx.rpc_client().get_account(&pdata).await?;
    let state: UpgradeableLoaderState = bincode::deserialize(
        &pdata_account.data[..UpgradeableLoaderState::size_of_programdata_metadata()],
    )?;
    match state {
        UpgradeableLoaderState::ProgramData { upgrade_authority_address, .. } => {
            assert_eq!(
                upgrade_authority_address,
                Some(kora_pubkey),
                "expected Kora as upgrade authority"
            );
        }
        other => panic!("expected ProgramData state, got {other:?}"),
    }

    // 5. Drainage attempt: hand authority to attacker → DeployAuthority plugin rejects
    let bad_set_auth =
        loader_v3::set_upgrade_authority(&program.pubkey(), &kora_pubkey, Some(&attacker.pubkey()));
    expect_reject(&ctx, &kora_pubkey, &[bad_set_auth], &[], "DeployAuthority").await?;

    // 6. Drainage attempt: close with attacker as recipient → drainage guard rejects
    let bad_close = loader_v3::close_any(
        &pdata,
        &attacker.pubkey(),
        Some(&kora_pubkey),
        Some(&program.pubkey()),
    );
    expect_reject(&ctx, &kora_pubkey, &[bad_close], &[], "recipient").await?;

    // (Migrate-to-v4 isn't testable here — loader-v4 is feature-gated on the local
    //  validator, so simulation exhausts CU before the plugin's reject runs. Plugin
    //  unit tests cover that path.)

    // 7. Cleanup: close program (Kora as authority + recipient)
    let close_ix =
        loader_v3::close_any(&pdata, &kora_pubkey, Some(&kora_pubkey), Some(&program.pubkey()));
    submit_and_confirm(&ctx, &kora_pubkey, &[close_ix], &[]).await?;

    Ok(())
}

fn read_test_program_so() -> Result<Vec<u8>> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TEST_PROGRAM_SO_REL);
    Ok(std::fs::read(path)?)
}

fn derive_program_data_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], &BPF_LOADER_UPGRADEABLE_PROGRAM_ID).0
}

async fn wait_for_next_slot(ctx: &TestContext) -> Result<()> {
    let start = ctx.rpc_client().get_slot().await?;
    for _ in 0..40 {
        if ctx.rpc_client().get_slot().await? > start {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    anyhow::bail!("slot never advanced past {start}")
}

async fn build_b64_tx(
    ctx: &TestContext,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<String> {
    let blockhash = ctx.rpc_client().get_latest_blockhash().await?;
    let msg = Message::new_with_blockhash(ixs, Some(fee_payer), &blockhash);
    let mut tx = Transaction::new_unsigned(msg);
    if !extra_signers.is_empty() {
        tx.partial_sign(extra_signers, blockhash);
    }
    Ok(B64.encode(bincode::serialize(&tx)?))
}

async fn submit_and_confirm(
    ctx: &TestContext,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<()> {
    let tx_b64 = build_b64_tx(ctx, fee_payer, ixs, extra_signers).await?;
    let resp: Value = ctx.rpc_call("signAndSendTransaction", rpc_params![tx_b64]).await?;
    let sig = Signature::from_str(
        resp["signature"].as_str().ok_or_else(|| anyhow::anyhow!("missing signature: {resp}"))?,
    )?;
    for _ in 0..60 {
        if ctx
            .rpc_client()
            .confirm_transaction_with_commitment(&sig, CommitmentConfig::confirmed())
            .await?
            .value
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    anyhow::bail!("timed out waiting for {sig}")
}

async fn expect_reject(
    ctx: &TestContext,
    fee_payer: &Pubkey,
    ixs: &[Instruction],
    extra_signers: &[&Keypair],
    expected_substr: &str,
) -> Result<()> {
    let tx_b64 = build_b64_tx(ctx, fee_payer, ixs, extra_signers).await?;
    let result: std::result::Result<Value, _> =
        ctx.rpc_call("signTransaction", rpc_params![tx_b64]).await;
    match result {
        Err(e) => {
            let msg = format!("{e}");
            anyhow::ensure!(
                msg.contains(expected_substr),
                "rejection '{msg}' does not contain '{expected_substr}'"
            );
            Ok(())
        }
        Ok(v) => anyhow::bail!("expected rejection containing '{expected_substr}', got: {v}"),
    }
}
