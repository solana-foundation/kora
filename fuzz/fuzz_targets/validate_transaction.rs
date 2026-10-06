#![no_main]

use std::{collections::HashMap, sync::LazyLock};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use kora_fuzz::{
    oracle::expected_rejection,
    scenario::{Scenario, FEE_PAYER},
};
use kora_lib::{
    transaction::VersionedTransactionResolved,
    validator::transaction_validator::TransactionValidator,
};
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};
use solana_client::{
    client_error::Result as ClientResult,
    nonblocking::rpc_client::RpcClient,
    rpc_client::RpcClientConfig,
    rpc_request::RpcRequest,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_program_pack::Pack;
use solana_rpc_client::mock_sender::MockSender;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};
use tokio::runtime::{Builder, Runtime};

const TOKEN_TRANSFER_TAG: u8 = 3;
const MINT_DECIMALS: u8 = 6;

static RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Builder::new_current_thread().enable_all().build().unwrap());

fn source_mint(program: Pubkey) -> Pubkey {
    if program == spl_token_2022_interface::ID {
        Pubkey::new_from_array([6; 32])
    } else {
        Pubkey::new_from_array([5; 32])
    }
}

fn mint_data() -> Vec<u8> {
    let mut data = vec![0; Mint::LEN];
    Mint { decimals: MINT_DECIMALS, is_initialized: true, ..Mint::default() }
        .pack_into_slice(&mut data);
    data
}

fn token_account_data(mint: Pubkey) -> Vec<u8> {
    let mut data = vec![0; TokenAccount::LEN];
    TokenAccount { mint, state: AccountState::Initialized, ..TokenAccount::default() }
        .pack_into_slice(&mut data);
    data
}

fn account_info(owner: Pubkey, data: &[u8]) -> Value {
    json!({
        "context": { "slot": 1 },
        "value": {
            "data": [STANDARD.encode(data), "base64"],
            "executable": false,
            "lamports": 1_461_600,
            "owner": owner.to_string(),
            "rentEpoch": 0
        }
    })
}

struct AccountsByAddress {
    transfer_sources: HashMap<Pubkey, Pubkey>,
    fallback: MockSender,
}

impl AccountsByAddress {
    fn new(instructions: &[Instruction]) -> Self {
        let transfer_sources = instructions
            .iter()
            .filter(|ix| {
                (ix.program_id == spl_token_interface::ID
                    || ix.program_id == spl_token_2022_interface::ID)
                    && ix.data.first() == Some(&TOKEN_TRANSFER_TAG)
            })
            .filter_map(|ix| Some((ix.accounts.first()?.pubkey, ix.program_id)))
            .collect();
        Self { transfer_sources, fallback: MockSender::new("succeeds") }
    }
}

#[async_trait]
impl RpcSender for AccountsByAddress {
    async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        if request != RpcRequest::GetAccountInfo {
            return self.fallback.send(request, params).await;
        }
        let address = params[0].as_str().and_then(|key| key.parse::<Pubkey>().ok());
        let token_2022_mint = source_mint(spl_token_2022_interface::ID);
        Ok(match address {
            Some(address) if self.transfer_sources.contains_key(&address) => {
                let program = self.transfer_sources[&address];
                account_info(program, &token_account_data(source_mint(program)))
            }
            Some(address) if address == token_2022_mint => {
                account_info(spl_token_2022_interface::ID, &mint_data())
            }
            _ => account_info(spl_token_interface::ID, &mint_data()),
        })
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }

    fn url(&self) -> String {
        "succeeds".to_string()
    }
}

fuzz_target!(|scenario: Scenario| {
    let built_ops = scenario.built_ops();
    let instructions: Vec<Instruction> = built_ops.iter().map(|(_, ix)| ix.clone()).collect();
    let Some(transaction) = scenario.transaction(&instructions) else {
        return;
    };
    let Ok(mut resolved) = VersionedTransactionResolved::from_kora_built_transaction(&transaction)
    else {
        return;
    };

    let config = scenario.config();
    let validator = TransactionValidator::new(&config, FEE_PAYER).unwrap();
    let rpc_client =
        RpcClient::new_sender(AccountsByAddress::new(&instructions), RpcClientConfig::default());
    let result =
        RUNTIME.block_on(validator.validate_transaction(&config, &mut resolved, &rpc_client));

    let policy = &config.validation.fee_payer_policy;
    if let Some(rule) = built_ops
        .iter()
        .find_map(|(op, _)| expected_rejection(op, policy, scenario.allow_durable_transactions))
    {
        assert!(result.is_err(), "validator accepted a transaction that violates `{rule}`");
    }
});
