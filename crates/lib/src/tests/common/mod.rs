use crate::{
    get_signer,
    signer::{KoraSigner, SolanaMemorySigner},
    state::init_signer,
};
use base64::{self, Engine};
use serde_json::json;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_program::program_pack::Pack;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::state::Mint;
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_LOCAL_RPC_URL: &str = "http://localhost:8899";

pub fn setup_or_get_test_signer() -> Pubkey {
    if let Ok(signer) = get_signer() {
        return signer.solana_pubkey();
    }

    let test_keypair = Keypair::new();

    let signer = SolanaMemorySigner::new(test_keypair.insecure_clone());

    match init_signer(KoraSigner::Memory(signer)) {
        Ok(_) => test_keypair.pubkey(),
        Err(_) => {
            // Signer already initialized, get it
            get_signer().expect("Signer should be available").solana_pubkey()
        }
    }
}

pub fn get_mock_rpc_client(account: &Account) -> Arc<RpcClient> {
    let mut mocks = HashMap::new();
    let encoded_data = base64::engine::general_purpose::STANDARD.encode(&account.data);
    mocks.insert(
        RpcRequest::GetAccountInfo,
        json!({
            "context": {
                "slot": 1
            },
            "value": {
                "data": [encoded_data, "base64"],
                "executable": account.executable,
                "lamports": account.lamports,
                "owner": account.owner.to_string(),
                "rentEpoch": account.rent_epoch
            }
        }),
    );
    Arc::new(RpcClient::new_mock_with_mocks(DEFAULT_LOCAL_RPC_URL.to_string(), mocks))
}

pub fn get_mock_rpc_client_with_mint(mint_decimals: u8) -> Arc<RpcClient> {
    // Create a mock mint account
    let mut mint_data = vec![0u8; Mint::LEN];
    let mint = Mint {
        mint_authority: Some(Pubkey::new_unique()).into(),
        supply: 1000000,
        decimals: mint_decimals,
        is_initialized: true,
        freeze_authority: None.into(),
    };
    Mint::pack(mint, &mut mint_data).unwrap();

    let mint_account = Account {
        lamports: 1000000,
        data: mint_data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    };

    get_mock_rpc_client(&mint_account)
}
