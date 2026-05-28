//! Find programs owned by the paymaster's fee payer.
//!
//! Strategy:
//! - **Loader-v3**: query `getProgramAccounts(BPFLoaderUpgradeable, ...)` with a
//!   memcmp filter on the `ProgramData` account at byte offset 12 — that's the
//!   `Option<Pubkey>` tag byte (`1`) followed by 32 bytes of upgrade authority.
//!   Then for each matching `ProgramData`, look up its sibling `Program`
//!   account (36 bytes, discriminator `2`, points back at the `ProgramData`).
//! - **Loader-v4**: query `getProgramAccounts(LoaderV4, ...)` with a memcmp
//!   filter on the `authority_address_or_next_version` field at byte offset 8
//!   inside `LoaderV4State`. The same account holds the program code, so no
//!   second lookup is needed.

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

/// v3 `ProgramData` layout (bincode):
///   - bytes 0..4   enum discriminator (`ProgramData == 3`)
///   - bytes 4..12  slot (u64 LE)
///   - byte  12     `Option<Pubkey>` tag (`1` for `Some`)
///   - bytes 13..45 upgrade_authority_address
pub(crate) const V3_PROGRAMDATA_DISCRIMINATOR: [u8; 4] = [3, 0, 0, 0];
pub(crate) const V3_PROGRAMDATA_AUTH_TAG_OFFSET: usize = 12;
pub(crate) const V3_PROGRAM_DISCRIMINATOR: [u8; 4] = [2, 0, 0, 0];
pub(crate) const V3_PROGRAM_ACCOUNT_SIZE: u64 = 36;

/// v4 `LoaderV4State` layout (`#[repr(C)]`):
///   - bytes 0..8   slot (u64)
///   - bytes 8..40  authority_address_or_next_version
///   - bytes 40..48 status (u64)
pub(crate) const V4_AUTHORITY_OFFSET: usize = 8;

/// Find every program whose upgrade authority is `fee_payer`.
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
    auth_blob.push(1); // Option::Some tag
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
        let program = pdata_to_program.get(&pdata_pubkey).copied();
        // Skip orphan ProgramData (no Program pointer) — closing those is a
        // separate cleanup; the typical reap path needs both accounts so
        // `close_any` can recover the Program rent too.
        let Some(program) = program else {
            log::warn!(
                "skipping orphan v3 ProgramData {pdata_pubkey} — no matching Program account found"
            );
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

/// Pull every loader-v3 `Program` account (36 bytes, discriminator `2`) and
/// index it by the `ProgramData` it points at. Devnet returns a few thousand
/// of these; the response is small because each account is only 36 bytes.
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
        // bytes 0..4 = discriminator (already filtered), bytes 4..36 = programdata_address
        let pdata = Pubkey::try_from(&account.data[4..36])
            .map_err(|e| anyhow!("Program account {program_pubkey} had malformed body: {e}"))?;
        map.insert(pdata, program_pubkey);
    }
    Ok(map)
}

fn parse_v3_programdata_slot(account: &Account) -> Option<u64> {
    let bytes = account.data.get(4..12)?;
    let arr: [u8; 8] = bytes.try_into().ok()?;
    Some(u64::from_le_bytes(arr))
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

    let mut out = Vec::with_capacity(accounts.len());
    for (program_pubkey, account) in accounts {
        let last_state_slot = parse_v4_slot(&account).unwrap_or(0);
        out.push(OwnedProgram {
            loader: Loader::V4,
            program: program_pubkey,
            program_data: None,
            last_state_slot,
        });
    }
    Ok(out)
}

fn parse_v4_slot(account: &Account) -> Option<u64> {
    let bytes = account.data.get(0..8)?;
    let arr: [u8; 8] = bytes.try_into().ok()?;
    Some(u64::from_le_bytes(arr))
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

    /// If this fails the loader-v3 wire format changed under us — the memcmp
    /// filter in [`discover_v3`] would silently match nothing on devnet, so
    /// catch it at compile time rather than in production.
    #[test]
    fn v3_programdata_offsets_match_bincode_layout() {
        let authority = Pubkey::new_unique();
        let state = UpgradeableLoaderState::ProgramData {
            slot: 0xABCD_EF01_2345_6789,
            upgrade_authority_address: Some(authority),
        };
        let bytes = bincode::serialize(&state).expect("serialize programdata");

        assert_eq!(&bytes[0..4], &V3_PROGRAMDATA_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(bytes[4..12].try_into().unwrap()), 0xABCD_EF01_2345_6789,);
        assert_eq!(bytes[V3_PROGRAMDATA_AUTH_TAG_OFFSET], 1, "Option::Some tag");
        assert_eq!(
            &bytes[V3_PROGRAMDATA_AUTH_TAG_OFFSET + 1..V3_PROGRAMDATA_AUTH_TAG_OFFSET + 33],
            authority.as_ref(),
        );
    }

    #[test]
    fn v3_program_account_layout_matches_filters() {
        let pdata = Pubkey::new_unique();
        let state = UpgradeableLoaderState::Program { programdata_address: pdata };
        let bytes = bincode::serialize(&state).expect("serialize program");

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
        // LoaderV4State is `#[repr(C)]`, so a byte view of the in-memory
        // layout is what the on-chain account holds.
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
