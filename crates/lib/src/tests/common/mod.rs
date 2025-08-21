use crate::{
    config::{FeePayerPolicy, KoraConfig, MetricsConfig, Token2022Config, ValidationConfig},
    fee::price::PriceConfig,
    get_request_signer_with_signer_key,
    oracle::PriceSource,
    signer::{KoraSigner, SignerPool, SignerWithMetadata, SolanaMemorySigner},
    state::{get_config, update_config, update_signer_pool},
    Config,
};
use base64::{self, Engine};
use serde_json::{json, Value};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_program::program_pack::Pack;
use solana_sdk::{account::Account, program_option::COption, pubkey::Pubkey, signature::Keypair};
use spl_token::state::Mint;
use spl_token_2022::state::Mint as Mint2022;
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_LOCAL_RPC_URL: &str = "http://localhost:8899";

/*
Signer Mocks
*/
pub fn setup_or_get_test_signer() -> Pubkey {
    if let Ok(signer) = get_request_signer_with_signer_key(None) {
        return signer.solana_pubkey();
    }

    let test_keypair = Keypair::new();

    let signer = SolanaMemorySigner::new(test_keypair.insecure_clone());

    let pool = SignerPool::new(vec![SignerWithMetadata::new(
        "test_signer".to_string(),
        KoraSigner::Memory(signer.clone()),
        1,
    )]);

    match update_signer_pool(pool) {
        Ok(_) => {}
        Err(e) => {
            panic!("Failed to update signer pool: {e}");
        }
    }

    signer.solana_pubkey()
}

/*
Config Mocks
*/
pub fn setup_or_get_test_config() -> Config {
    if let Ok(config) = get_config() {
        return config.clone();
    }

    let config = Config {
        validation: ValidationConfig {
            max_allowed_lamports: 1000000000000000000,
            max_signatures: 1000000000000000000,
            allowed_programs: vec![],
            allowed_tokens: vec![],
            allowed_spl_paid_tokens: vec![],
            disallowed_accounts: vec![],
            price_source: PriceSource::Mock,
            fee_payer_policy: FeePayerPolicy::default(),
            price: PriceConfig::default(),
            token_2022: Token2022Config::default(),
        },
        kora: KoraConfig::default(),
        metrics: MetricsConfig::default(),
    };

    match update_config(config.clone()) {
        Ok(_) => config.clone(),
        Err(e) => {
            panic!("Failed to initialize config: {e}");
        }
    }
}

/*
RPC Mocks
*/
pub fn get_fee_estimate_response_mock(fee: u64) -> (RpcRequest, Value) {
    (
        RpcRequest::GetFeeForMessage,
        json!({
            "context": { "slot": 1 },
            "value": fee
        }),
    )
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

pub fn get_mock_rpc_client_with_extra_mocks(
    account: &Account,
    extra_mocks: HashMap<RpcRequest, Value>,
) -> Arc<RpcClient> {
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
    mocks.extend(extra_mocks);
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

pub fn create_mock_rpc_client_account_not_found() -> Arc<RpcClient> {
    let mut mocks = HashMap::new();
    mocks.insert(
        RpcRequest::GetAccountInfo,
        json!({
            "context": { "slot": 1 },
            "value": null
        }),
    );

    Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
}

/*
Account Mocks
*/
pub fn create_mock_token_account(owner: &Pubkey, mint: &Pubkey) -> Account {
    let token_account = spl_token::state::Account {
        mint: *mint,
        owner: *owner,
        amount: 100,
        delegate: COption::None,
        state: spl_token::state::AccountState::Initialized,
        is_native: COption::Some(0),
        delegated_amount: 0,
        close_authority: COption::None,
    };

    let mut data = vec![0u8; spl_token::state::Account::LEN];
    token_account.pack_into_slice(&mut data);

    Account { lamports: 1000000, data, owner: spl_token::id(), executable: false, rent_epoch: 0 }
}

pub fn create_mock_program_account() -> Account {
    Account {
        lamports: 1000000,
        data: vec![0u8; 100],        // Program data
        owner: Pubkey::new_unique(), // Programs are owned by the loader
        executable: true,            // Programs are executable
        rent_epoch: 0,
    }
}

pub fn create_mock_non_executable_account() -> Account {
    Account {
        lamports: 1000000,
        data: vec![0u8; 100],
        owner: Pubkey::new_unique(),
        executable: false, // Not executable
        rent_epoch: 0,
    }
}

pub fn create_mock_spl_mint_account(decimals: u8) -> Account {
    let mint_data = Mint {
        mint_authority: COption::Some(Pubkey::new_unique()),
        supply: 1_000_000_000_000,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; Mint::LEN];
    mint_data.pack_into_slice(&mut data);

    Account { lamports: 0, data, owner: spl_token::id(), executable: false, rent_epoch: 0 }
}

pub fn create_mock_token2022_mint_account(decimals: u8) -> Account {
    let mint_data = Mint2022 {
        mint_authority: COption::Some(Pubkey::new_unique()),
        supply: 1_000_000_000_000,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; Mint2022::LEN];
    mint_data.pack_into_slice(&mut data);

    Account { lamports: 0, data, owner: spl_token_2022::id(), executable: false, rent_epoch: 0 }
}
