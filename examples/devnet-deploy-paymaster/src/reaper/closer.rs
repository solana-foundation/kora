#![allow(deprecated)] // loader-v3 helpers marked deprecated upstream; v4 isn't ubiquitous yet.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use kora_lib::{signer::SolanaSigner, transaction::TransactionUtil};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_loader_v3_interface::instruction as loader_v3;
use solana_loader_v4_interface::instruction as loader_v4;
use solana_sdk::{
    instruction::Instruction,
    message::{Message, VersionedMessage},
    pubkey::Pubkey,
};

use super::{AccountKind, ClosedProgram, Loader, OwnedProgram};

pub async fn close_program(
    rpc: &Arc<RpcClient>,
    program: &OwnedProgram,
    signer: &solana_keychain::Signer,
    fee_payer: &Pubkey,
) -> Result<ClosedProgram> {
    let reclaimed_lamports = sum_reclaimable_lamports(rpc, program)
        .await
        .with_context(|| format!("reading lamports for {}", program.program))?;

    let mut instructions = match program.loader {
        Loader::V3 => build_v3_close(fee_payer, program)?,
        Loader::V4 => build_v4_close(fee_payer, program),
    };
    if let Some(entry_close) = registry_entry_close(rpc, fee_payer, &program.program).await {
        instructions.push(entry_close);
    }

    let blockhash =
        rpc.get_latest_blockhash().await.map_err(|e| anyhow!("getLatestBlockhash: {e}"))?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(fee_payer),
        &blockhash,
    ));
    let mut tx = TransactionUtil::new_unsigned_versioned_transaction(message);
    let signature = signer
        .sign_message(&tx.message.serialize())
        .await
        .map_err(|e| anyhow!("sign_message: {e}"))?;
    tx.signatures = vec![signature];

    let sig = rpc
        .send_and_confirm_transaction_with_spinner(&tx)
        .await
        .map_err(|e| anyhow!("send_and_confirm: {e}"))?;

    Ok(ClosedProgram {
        program: program.program,
        loader: program.loader,
        signature: sig.to_string(),
        reclaimed_lamports,
    })
}

/// Registry entries only exist for programs deployed with a wallet; the close is appended to
/// the same transaction so it runs after the programdata drain it requires. Returns None when
/// there is no entry (deployed without a wallet), so the program close isn't bundled with an
/// instruction that would revert.
async fn registry_entry_close(
    rpc: &Arc<RpcClient>,
    fee_payer: &Pubkey,
    program: &Pubkey,
) -> Option<Instruction> {
    let registry = kora_deploy::DEFAULT_REGISTRY_PROGRAM;
    let entry_address = kora_deploy::registry_entry_address(&registry, program);
    let entry = rpc.get_account(&entry_address).await.ok()?;
    if entry.owner != registry {
        return None;
    }
    Some(kora_deploy::close_entry_ix(&registry, program, fee_payer))
}

fn build_v3_close(fee_payer: &Pubkey, program: &OwnedProgram) -> Result<Vec<Instruction>> {
    if program.kind == AccountKind::Buffer {
        return Ok(vec![loader_v3::close_any(&program.program, fee_payer, Some(fee_payer), None)]);
    }
    let program_data = program
        .program_data
        .ok_or_else(|| anyhow!("v3 program {} missing program_data", program.program))?;
    Ok(vec![loader_v3::close_any(
        &program_data,
        fee_payer,
        Some(fee_payer),
        Some(&program.program),
    )])
}

fn build_v4_close(fee_payer: &Pubkey, program: &OwnedProgram) -> Vec<Instruction> {
    vec![
        loader_v4::retract(&program.program, fee_payer),
        loader_v4::set_program_length(&program.program, fee_payer, 0, fee_payer),
    ]
}

async fn sum_reclaimable_lamports(rpc: &Arc<RpcClient>, program: &OwnedProgram) -> Result<u64> {
    let mut keys = vec![program.program];
    if let Some(pdata) = program.program_data {
        keys.push(pdata);
    }
    let accounts =
        rpc.get_multiple_accounts(&keys).await.map_err(|e| anyhow!("getMultipleAccounts: {e}"))?;
    Ok(accounts.into_iter().flatten().map(|a| a.lamports).sum())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn build_v3_close_uses_fee_payer_as_authority_and_recipient() {
        let fee_payer = Pubkey::new_unique();
        let p = OwnedProgram {
            loader: Loader::V3,
            kind: AccountKind::Program,
            program: Pubkey::new_unique(),
            program_data: Some(Pubkey::new_unique()),
            last_state_slot: 100,
        };
        let ixs = build_v3_close(&fee_payer, &p).unwrap();

        assert_eq!(ixs.len(), 1);
        let ix = &ixs[0];
        // close_any: [program_data, recipient, authority, program]
        assert_eq!(ix.accounts[0].pubkey, p.program_data.unwrap());
        assert_eq!(ix.accounts[1].pubkey, fee_payer);
        assert_eq!(ix.accounts[2].pubkey, fee_payer);
        assert!(ix.accounts[2].is_signer);
        assert_eq!(ix.accounts[3].pubkey, p.program);
    }

    #[test]
    fn build_v3_close_buffer_targets_buffer_with_no_program_account() {
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let b = OwnedProgram {
            loader: Loader::V3,
            kind: AccountKind::Buffer,
            program: buffer,
            program_data: None,
            last_state_slot: 0,
        };
        let ixs = build_v3_close(&fee_payer, &b).unwrap();

        assert_eq!(ixs.len(), 1);
        let ix = &ixs[0];
        // close_any: [buffer, recipient, authority]
        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.accounts[0].pubkey, buffer);
        assert_eq!(ix.accounts[1].pubkey, fee_payer);
        assert_eq!(ix.accounts[2].pubkey, fee_payer);
        assert!(ix.accounts[2].is_signer);
    }

    #[test]
    fn build_v3_close_errors_without_program_data() {
        let bad = OwnedProgram {
            loader: Loader::V3,
            kind: AccountKind::Program,
            program: Pubkey::new_unique(),
            program_data: None,
            last_state_slot: 0,
        };
        assert!(build_v3_close(&Pubkey::new_unique(), &bad).is_err());
    }

    #[test]
    fn build_v4_close_emits_retract_then_set_program_length_zero() {
        let fee_payer = Pubkey::new_unique();
        let p = OwnedProgram {
            loader: Loader::V4,
            kind: AccountKind::Program,
            program: Pubkey::new_unique(),
            program_data: None,
            last_state_slot: 0,
        };
        let ixs = build_v4_close(&fee_payer, &p);

        assert_eq!(ixs.len(), 2);
        assert_eq!(ixs[0].accounts[0].pubkey, p.program);
        assert_eq!(ixs[0].accounts[1].pubkey, fee_payer);
        assert!(ixs[0].accounts[1].is_signer);

        assert_eq!(ixs[1].accounts[0].pubkey, p.program);
        assert_eq!(ixs[1].accounts[1].pubkey, fee_payer);
        assert!(ixs[1].accounts[1].is_signer);
        assert_eq!(ixs[1].accounts[2].pubkey, fee_payer);
    }
}
