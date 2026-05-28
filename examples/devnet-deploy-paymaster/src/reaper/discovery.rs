use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Context, Result};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{account::Account, pubkey::Pubkey};

use super::{Loader, OwnedProgram};

const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
const LOADER_V4_ID: Pubkey = solana_sdk::pubkey!("LoaderV411111111111111111111111111111111111");

// v3 `ProgramData` bincode layout: [discriminator=3; 4][slot u64 LE; 8][Some tag; 1][authority; 32]
pub(crate) const V3_PROGRAMDATA_DISCRIMINATOR: [u8; 4] = [3, 0, 0, 0];
pub(crate) const V3_PROGRAMDATA_AUTH_TAG_OFFSET: usize = 12;
pub(crate) const V3_PROGRAM_DISCRIMINATOR: [u8; 4] = [2, 0, 0, 0];
pub(crate) const V3_PROGRAM_ACCOUNT_SIZE: u64 = 36;

// v4 `#[repr(C)] LoaderV4State`: [slot u64; 8][authority Pubkey; 32][status u64; 8]
pub(crate) const V4_AUTHORITY_OFFSET: usize = 8;

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
    let mut auth_blob = Vec::with_capacity(33);
    auth_blob.push(1);
    auth_blob.extend_from_slice(fee_payer.as_ref());

    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, V3_PROGRAMDATA_DISCRIMINATOR.to_vec())),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(V3_PROGRAMDATA_AUTH_TAG_OFFSET, auth_blob)),
    ];

    let programdata_accounts = rpc
        .get_program_accounts_with_config(
            &BPF_LOADER_UPGRADEABLE_ID,
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
            &BPF_LOADER_UPGRADEABLE_ID,
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
        if account.data.len() != V3_PROGRAM_ACCOUNT_SIZE as usize {
            continue;
        }
        let pdata = Pubkey::try_from(&account.data[4..36])
            .map_err(|e| anyhow!("malformed Program account {program_pubkey}: {e}"))?;
        map.insert(pdata, program_pubkey);
    }
    Ok(map)
}

fn parse_v3_programdata_slot(account: &Account) -> Option<u64> {
    let bytes = account.data.get(4..12)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

async fn discover_v4(rpc: &Arc<RpcClient>, fee_payer: &Pubkey) -> Result<Vec<OwnedProgram>> {
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        V4_AUTHORITY_OFFSET,
        fee_payer.as_ref().to_vec(),
    ))];

    let accounts = rpc
        .get_program_accounts_with_config(
            &LOADER_V4_ID,
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
    let bytes = account.data.get(0..8)?;
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
    use solana_loader_v3_interface::state::UpgradeableLoaderState;
    use solana_loader_v4_interface::state::{LoaderV4State, LoaderV4Status};

    // Catches silent breakage if the v3 wire format ever drifts under the memcmp filter.
    #[test]
    fn v3_programdata_offsets_match_bincode_layout() {
        let authority = Pubkey::new_unique();
        let state = UpgradeableLoaderState::ProgramData {
            slot: 0xABCD_EF01_2345_6789,
            upgrade_authority_address: Some(authority),
        };
        let bytes = bincode::serialize(&state).unwrap();

        assert_eq!(&bytes[0..4], &V3_PROGRAMDATA_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(bytes[4..12].try_into().unwrap()), 0xABCD_EF01_2345_6789);
        assert_eq!(bytes[V3_PROGRAMDATA_AUTH_TAG_OFFSET], 1);
        assert_eq!(
            &bytes[V3_PROGRAMDATA_AUTH_TAG_OFFSET + 1..V3_PROGRAMDATA_AUTH_TAG_OFFSET + 33],
            authority.as_ref(),
        );
    }

    #[test]
    fn v3_program_account_layout_matches_filters() {
        let pdata = Pubkey::new_unique();
        let bytes =
            bincode::serialize(&UpgradeableLoaderState::Program { programdata_address: pdata })
                .unwrap();

        assert_eq!(bytes.len() as u64, V3_PROGRAM_ACCOUNT_SIZE);
        assert_eq!(&bytes[0..4], &V3_PROGRAM_DISCRIMINATOR);
        assert_eq!(&bytes[4..36], pdata.as_ref());
    }

    #[test]
    fn v4_authority_offset_matches_loader_v4_state() {
        let authority = Pubkey::new_unique();
        let state = LoaderV4State {
            slot: 0,
            authority_address_or_next_version: authority,
            status: LoaderV4Status::Deployed,
        };
        // repr(C), so byte-view of memory == on-chain layout
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&state as *const LoaderV4State) as *const u8,
                std::mem::size_of::<LoaderV4State>(),
            )
        };
        assert_eq!(&bytes[V4_AUTHORITY_OFFSET..V4_AUTHORITY_OFFSET + 32], authority.as_ref());
    }

    #[test]
    fn parse_v3_programdata_slot_reads_le_u64() {
        let mut data = vec![0u8; 45];
        data[0..4].copy_from_slice(&V3_PROGRAMDATA_DISCRIMINATOR);
        data[4..12].copy_from_slice(&0x0123_4567_89AB_CDEFu64.to_le_bytes());
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
