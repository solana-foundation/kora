use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Context, Result};
use kora_lib::constant::{BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LOADER_V4_PROGRAM_ID};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_commitment_config::CommitmentConfig;
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_loader_v4_interface::state::LoaderV4State;
use solana_sdk::{account::Account, pubkey::Pubkey};

use super::{Loader, OwnedProgram};

// v3 ProgramData bincode: [discriminator 4][slot 8][Option tag 1][authority 32].
const V3_PROGRAMDATA_DISCRIMINATOR: [u8; 4] = [3, 0, 0, 0];
const V3_PROGRAMDATA_SLOT_OFFSET: usize = 4;
const V3_PROGRAMDATA_AUTH_OFFSET: usize = 13;

// v3 Program: [discriminator 4][programdata_address 32].
const V3_PROGRAM_DISCRIMINATOR: [u8; 4] = [2, 0, 0, 0];
const V3_PROGRAM_PDATA_OFFSET: usize = 4;
const V3_PROGRAM_ACCOUNT_SIZE: u64 = UpgradeableLoaderState::size_of_program() as u64;

const V4_SLOT_OFFSET: usize = std::mem::offset_of!(LoaderV4State, slot);
const V4_AUTH_OFFSET: usize =
    std::mem::offset_of!(LoaderV4State, authority_address_or_next_version);

pub async fn discover_owned_programs(
    rpc: &Arc<RpcClient>,
    fee_payer: &Pubkey,
) -> Result<Vec<OwnedProgram>> {
    let mut out = Vec::new();
    out.extend(discover_v3(rpc, fee_payer).await.context("discover v3")?);
    out.extend(discover_v4(rpc, fee_payer).await.context("discover v4")?);
    Ok(out)
}

async fn discover_v3(rpc: &Arc<RpcClient>, fee_payer: &Pubkey) -> Result<Vec<OwnedProgram>> {
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, V3_PROGRAMDATA_DISCRIMINATOR.to_vec())),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            V3_PROGRAMDATA_AUTH_OFFSET,
            fee_payer.as_ref().to_vec(),
        )),
    ];

    let programdata_accounts = rpc
        .get_program_accounts_with_config(
            &BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: minimal_account_config(),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v3 ProgramData): {e}"))?;

    if programdata_accounts.is_empty() {
        return Ok(Vec::new());
    }

    let pdata_to_program = build_v3_program_index(rpc).await?;

    let mut out = Vec::with_capacity(programdata_accounts.len());
    for (pdata_pubkey, pdata_account) in programdata_accounts {
        let last_state_slot = parse_v3_programdata_slot(&pdata_account).unwrap_or(0);
        let Some(program) = pdata_to_program.get(&pdata_pubkey).copied() else {
            log::warn!("skipping orphan v3 ProgramData {pdata_pubkey}");
            continue;
        };
        out.push(OwnedProgram {
            loader: Loader::V3,
            program,
            program_data: Some(pdata_pubkey),
            last_state_slot,
        });
    }
    Ok(out)
}

async fn build_v3_program_index(rpc: &Arc<RpcClient>) -> Result<HashMap<Pubkey, Pubkey>> {
    let filters = vec![
        RpcFilterType::DataSize(V3_PROGRAM_ACCOUNT_SIZE),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, V3_PROGRAM_DISCRIMINATOR.to_vec())),
    ];

    let accounts = rpc
        .get_program_accounts_with_config(
            &BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: minimal_account_config(),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v3 Program): {e}"))?;

    let mut map = HashMap::with_capacity(accounts.len());
    for (program_pubkey, account) in accounts {
        if account.data.len() as u64 != V3_PROGRAM_ACCOUNT_SIZE {
            continue;
        }
        let pdata =
            Pubkey::try_from(&account.data[V3_PROGRAM_PDATA_OFFSET..V3_PROGRAM_PDATA_OFFSET + 32])
                .map_err(|e| anyhow!("malformed Program account {program_pubkey}: {e}"))?;
        map.insert(pdata, program_pubkey);
    }
    Ok(map)
}

fn parse_v3_programdata_slot(account: &Account) -> Option<u64> {
    let bytes = account.data.get(V3_PROGRAMDATA_SLOT_OFFSET..V3_PROGRAMDATA_SLOT_OFFSET + 8)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

async fn discover_v4(rpc: &Arc<RpcClient>, fee_payer: &Pubkey) -> Result<Vec<OwnedProgram>> {
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        V4_AUTH_OFFSET,
        fee_payer.as_ref().to_vec(),
    ))];

    let accounts = rpc
        .get_program_accounts_with_config(
            &LOADER_V4_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: minimal_account_config(),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v4): {e}"))?;

    Ok(accounts
        .into_iter()
        .map(|(program_pubkey, account)| OwnedProgram {
            loader: Loader::V4,
            program: program_pubkey,
            program_data: None,
            last_state_slot: parse_v4_slot(&account).unwrap_or(0),
        })
        .collect())
}

fn parse_v4_slot(account: &Account) -> Option<u64> {
    let bytes = account.data.get(V4_SLOT_OFFSET..V4_SLOT_OFFSET + 8)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

fn minimal_account_config() -> RpcAccountInfoConfig {
    RpcAccountInfoConfig {
        encoding: None,
        data_slice: None,
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v3_programdata_slot_reads_le_u64() {
        let mut data = vec![0u8; 45];
        data[V3_PROGRAMDATA_SLOT_OFFSET..V3_PROGRAMDATA_SLOT_OFFSET + 8]
            .copy_from_slice(&0x0123_4567_89AB_CDEFu64.to_le_bytes());
        let account = Account {
            lamports: 0,
            data,
            owner: Pubkey::default(),
            executable: false,
            rent_epoch: 0,
        };
        assert_eq!(parse_v3_programdata_slot(&account), Some(0x0123_4567_89AB_CDEF));
    }

    #[test]
    fn parse_v3_programdata_slot_returns_none_when_truncated() {
        let account = Account {
            lamports: 0,
            data: vec![0u8; 4],
            owner: Pubkey::default(),
            executable: false,
            rent_epoch: 0,
        };
        assert_eq!(parse_v3_programdata_slot(&account), None);
    }
}
