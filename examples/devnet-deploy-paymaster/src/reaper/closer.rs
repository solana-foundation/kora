//! Build and send the close transaction for an idle program.
//!
//! Loader-v3: a single `close_any(program_data, recipient=fee_payer,
//! authority=fee_payer, program)` instruction recovers rent from both the
//! `ProgramData` account and the small `Program` pointer account.
//!
//! Loader-v4: two instructions in one transaction — `Retract` (so the program
//! is in the maintenance state and can shrink) followed by
//! `SetProgramLength(0)` with the fee payer as the rent recipient.

#![allow(deprecated)] // loader-v3 helpers are tagged "use loader-v4" but v4 isn't ubiquitous yet.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use kora_lib::{
    signer::SolanaSigner, state::select_request_signer_with_signer_key,
    transaction::TransactionUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_loader_v3_interface::instruction as loader_v3;
use solana_loader_v4_interface::instruction as loader_v4;
use solana_sdk::{
    instruction::Instruction,
    message::{Message, VersionedMessage},
    pubkey::Pubkey,
};

use super::{ClosedProgram, Loader, OwnedProgram};

pub async fn close_program(rpc: &Arc<RpcClient>, program: &OwnedProgram) -> Result<ClosedProgram> {
    let signer = select_request_signer_with_signer_key(None)
        .map_err(|e| anyhow!("selecting signer: {e}"))?;
    let fee_payer = signer.pubkey();

    let reclaimed_lamports = sum_reclaimable_lamports(rpc, program)
        .await
        .with_context(|| format!("reading lamports for {}", program.program))?;

    let instructions = match program.loader {
        Loader::V3 => build_v3_close(&fee_payer, program)?,
        Loader::V4 => build_v4_close(&fee_payer, program),
    };

    let blockhash =
        rpc.get_latest_blockhash().await.map_err(|e| anyhow!("getLatestBlockhash: {e}"))?;

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &instructions,
        Some(&fee_payer),
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

fn build_v3_close(fee_payer: &Pubkey, program: &OwnedProgram) -> Result<Vec<Instruction>> {
    let program_data = program
        .program_data
        .ok_or_else(|| anyhow!("v3 program {} missing program_data pubkey", program.program))?;
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

    fn v3_program() -> (Pubkey, OwnedProgram) {
        let fee_payer = Pubkey::new_unique();
        let program = OwnedProgram {
            loader: Loader::V3,
            program: Pubkey::new_unique(),
            program_data: Some(Pubkey::new_unique()),
            last_state_slot: 100,
        };
        (fee_payer, program)
    }

    #[test]
    fn build_v3_close_uses_fee_payer_as_authority_and_recipient() {
        let (fee_payer, p) = v3_program();
        let ixs = build_v3_close(&fee_payer, &p).expect("v3 close");

        assert_eq!(ixs.len(), 1, "v3 close is a single instruction");
        let ix = &ixs[0];
        // close_any account ordering: [program_data, recipient, authority, program]
        assert_eq!(ix.accounts[0].pubkey, p.program_data.unwrap());
        assert_eq!(ix.accounts[1].pubkey, fee_payer, "recipient must be fee payer");
        assert_eq!(ix.accounts[2].pubkey, fee_payer, "authority must be fee payer");
        assert!(ix.accounts[2].is_signer, "authority signs");
        assert_eq!(ix.accounts[3].pubkey, p.program);
    }

    #[test]
    fn build_v3_close_errors_without_program_data() {
        let fee_payer = Pubkey::new_unique();
        let bad = OwnedProgram {
            loader: Loader::V3,
            program: Pubkey::new_unique(),
            program_data: None,
            last_state_slot: 0,
        };
        assert!(build_v3_close(&fee_payer, &bad).is_err());
    }

    #[test]
    fn build_v4_close_emits_retract_then_set_program_length_zero() {
        let fee_payer = Pubkey::new_unique();
        let p = OwnedProgram {
            loader: Loader::V4,
            program: Pubkey::new_unique(),
            program_data: None,
            last_state_slot: 0,
        };
        let ixs = build_v4_close(&fee_payer, &p);

        assert_eq!(ixs.len(), 2, "v4 close is retract + set_program_length");

        let retract = &ixs[0];
        assert_eq!(retract.accounts[0].pubkey, p.program);
        assert_eq!(retract.accounts[1].pubkey, fee_payer, "retract authority must be fee payer");
        assert!(retract.accounts[1].is_signer);

        let spl = &ixs[1];
        assert_eq!(spl.accounts[0].pubkey, p.program);
        assert_eq!(spl.accounts[1].pubkey, fee_payer, "set_program_length authority is fee payer");
        assert!(spl.accounts[1].is_signer);
        assert_eq!(spl.accounts[2].pubkey, fee_payer, "rent recipient must be fee payer");
    }
}
