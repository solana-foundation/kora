use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use kora_lib::constant::{BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LOADER_V4_PROGRAM_ID};
use solana_account_decoder_client_types::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_commitment_config::CommitmentConfig;
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_loader_v4_interface::state::LoaderV4State;
use solana_sdk::pubkey::Pubkey;

use super::{AccountKind, Loader, OwnedProgram};

// UpgradeableLoaderState::ProgramData bincode layout: [discriminator 4][slot 8][Option tag 1][authority 32]
const V3_PROGRAMDATA_DISCRIMINATOR: [u8; 4] = [3, 0, 0, 0];
const V3_PROGRAMDATA_SLOT_OFFSET: usize = 4;
const V3_PROGRAMDATA_AUTH_OFFSET: usize = 13;

// UpgradeableLoaderState::Program bincode layout: [discriminator 4][programdata_address 32]
const V3_PROGRAM_DISCRIMINATOR: [u8; 4] = [2, 0, 0, 0];
const V3_PROGRAM_PDATA_OFFSET: usize = 4;
const V3_PROGRAM_ACCOUNT_SIZE: u64 = UpgradeableLoaderState::size_of_program() as u64;

// UpgradeableLoaderState::Buffer bincode layout: [discriminator 4][Option tag 1][authority 32]
const V3_BUFFER_DISCRIMINATOR: [u8; 4] = [1, 0, 0, 0];
const V3_BUFFER_AUTH_OFFSET: usize = 5;

const V4_SLOT_OFFSET: usize = std::mem::offset_of!(LoaderV4State, slot);
const V4_AUTH_OFFSET: usize =
    std::mem::offset_of!(LoaderV4State, authority_address_or_next_version);

pub async fn discover_owned_programs(
    rpc: &Arc<RpcClient>,
    fee_payer: &Pubkey,
) -> Result<Vec<OwnedProgram>> {
    let mut out = Vec::new();
    out.extend(discover_v3(rpc, fee_payer).await.context("discover v3")?);
    out.extend(discover_v3_buffers(rpc, fee_payer).await.context("discover v3 buffers")?);
    out.extend(discover_v4(rpc, fee_payer).await.context("discover v4")?);
    Ok(out)
}

async fn discover_v3_buffers(
    rpc: &Arc<RpcClient>,
    fee_payer: &Pubkey,
) -> Result<Vec<OwnedProgram>> {
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, V3_BUFFER_DISCRIMINATOR.to_vec())),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            V3_BUFFER_AUTH_OFFSET,
            fee_payer.as_ref().to_vec(),
        )),
    ];

    let accounts = rpc
        .get_program_accounts_with_config(
            &BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: account_config_slice(0, 0),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v3 Buffer): {e}"))?;

    Ok(accounts
        .into_iter()
        .map(|(buffer, _)| OwnedProgram {
            loader: Loader::V3,
            kind: AccountKind::Buffer,
            program: buffer,
            program_data: None,
            last_state_slot: 0,
        })
        .collect())
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
                account_config: account_config_slice(V3_PROGRAMDATA_SLOT_OFFSET, 8),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v3 ProgramData): {e}"))?;

    let mut out = Vec::with_capacity(programdata_accounts.len());
    for (pdata_pubkey, pdata_account) in programdata_accounts {
        let last_state_slot = parse_u64_le(&pdata_account.data).unwrap_or(0);
        let Some(program) = find_v3_program_for_pdata(rpc, &pdata_pubkey).await? else {
            log::warn!("skipping orphan v3 ProgramData {pdata_pubkey}");
            continue;
        };
        out.push(OwnedProgram {
            loader: Loader::V3,
            kind: AccountKind::Program,
            program,
            program_data: Some(pdata_pubkey),
            last_state_slot,
        });
    }
    Ok(out)
}

async fn find_v3_program_for_pdata(rpc: &Arc<RpcClient>, pdata: &Pubkey) -> Result<Option<Pubkey>> {
    let filters = vec![
        RpcFilterType::DataSize(V3_PROGRAM_ACCOUNT_SIZE),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, V3_PROGRAM_DISCRIMINATOR.to_vec())),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            V3_PROGRAM_PDATA_OFFSET,
            pdata.as_ref().to_vec(),
        )),
    ];

    let accounts = rpc
        .get_program_accounts_with_config(
            &BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: account_config_slice(0, 0),
                with_context: None,
                sort_results: None,
            },
        )
        .await
        .map_err(|e| anyhow!("getProgramAccounts(loader-v3 Program for {pdata}): {e}"))?;

    Ok(accounts.into_iter().next().map(|(pk, _)| pk))
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
                account_config: account_config_slice(V4_SLOT_OFFSET, 8),
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
            kind: AccountKind::Program,
            program: program_pubkey,
            program_data: None,
            last_state_slot: parse_u64_le(&account.data).unwrap_or(0),
        })
        .collect())
}

fn parse_u64_le(bytes: &[u8]) -> Option<u64> {
    let arr: [u8; 8] = bytes.get(..8)?.try_into().ok()?;
    Some(u64::from_le_bytes(arr))
}

fn account_config_slice(offset: usize, length: usize) -> RpcAccountInfoConfig {
    RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: Some(UiDataSliceConfig { offset, length }),
        commitment: Some(CommitmentConfig::confirmed()),
        min_context_slot: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_u64_le_reads_eight_bytes() {
        let bytes = 0x0123_4567_89AB_CDEFu64.to_le_bytes();
        assert_eq!(parse_u64_le(&bytes), Some(0x0123_4567_89AB_CDEF));
    }

    #[test]
    fn parse_u64_le_returns_none_when_truncated() {
        assert_eq!(parse_u64_le(&[0u8; 4]), None);
    }

    // DEVNET_RPC_URL=<url> cargo test -p devnet-deploy-paymaster -- --ignored
    #[tokio::test]
    #[ignore]
    async fn discover_against_devnet() {
        use std::time::Duration;
        let url = std::env::var("DEVNET_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        let rpc = Arc::new(RpcClient::new_with_timeout(url, Duration::from_secs(120)));
        let kora = Pubkey::from_str("FEvT5nka9qBSXryv65P8ScjvGtfLUE7LgPu1rzuJ38h8").unwrap();
        let programs = discover_owned_programs(&rpc, &kora).await.unwrap();
        println!("found {} owned programs", programs.len());
        for p in &programs {
            println!(
                "  {:?}: program={} pdata={:?} slot={}",
                p.loader, p.program, p.program_data, p.last_state_slot
            );
        }
    }
}
