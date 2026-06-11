use anyhow::Result;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_sdk::{pubkey::Pubkey, rent::Rent};

pub const VALIDATOR_BACKEND_ENV: &str = "KORA_TEST_VALIDATOR_BACKEND";
pub const SURFPOOL_BACKEND: &str = "surfpool";

pub fn is_surfpool_backend() -> bool {
    std::env::var(VALIDATOR_BACKEND_ENV).is_ok_and(|v| v == SURFPOOL_BACKEND)
}

pub async fn set_account(
    rpc_client: &RpcClient,
    address: &Pubkey,
    lamports: u64,
    data: &[u8],
    owner: &Pubkey,
    executable: bool,
) -> Result<()> {
    rpc_client
        .send::<serde_json::Value>(
            RpcRequest::Custom { method: "surfnet_setAccount" },
            serde_json::json!([
                address.to_string(),
                {
                    "lamports": lamports,
                    "data": hex::encode(data),
                    "owner": owner.to_string(),
                    "executable": executable,
                    "rentEpoch": 0,
                }
            ]),
        )
        .await?;
    Ok(())
}

pub async fn set_rent_exempt_account(
    rpc_client: &RpcClient,
    address: &Pubkey,
    data: &[u8],
    owner: &Pubkey,
) -> Result<()> {
    let lamports = Rent::default().minimum_balance(data.len());
    set_account(rpc_client, address, lamports, data, owner, false).await
}

pub async fn load_upgradeable_program(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    elf: &[u8],
) -> Result<()> {
    let loader_id = kora_lib::constant::BPF_LOADER_UPGRADEABLE_PROGRAM_ID;
    let (programdata_address, _) = Pubkey::find_program_address(&[program_id.as_ref()], &loader_id);

    let mut programdata = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        slot: 1,
        upgrade_authority_address: Some(Pubkey::default()),
    })?;
    programdata.extend_from_slice(elf);

    let program = bincode::serialize(&UpgradeableLoaderState::Program { programdata_address })?;

    let programdata_lamports = Rent::default().minimum_balance(programdata.len());
    set_account(
        rpc_client,
        &programdata_address,
        programdata_lamports,
        &programdata,
        &loader_id,
        false,
    )
    .await?;

    let program_lamports = Rent::default().minimum_balance(program.len());
    set_account(rpc_client, program_id, program_lamports, &program, &loader_id, true).await
}
