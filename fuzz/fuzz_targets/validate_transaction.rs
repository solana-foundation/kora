#![no_main]

use std::sync::LazyLock;

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
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::instruction::Instruction;
use tokio::runtime::{Builder, Runtime};

const MINT_LEN: usize = 82;
const MINT_DECIMALS_OFFSET: usize = 44;
const MINT_IS_INITIALIZED_OFFSET: usize = 45;
const ACCOUNT_INFO_RESPONSES: usize = 64;

static RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Builder::new_current_thread().enable_all().build().unwrap());

static MINT_ACCOUNT_INFO: LazyLock<Value> = LazyLock::new(|| {
    let mut data = [0u8; MINT_LEN];
    data[MINT_DECIMALS_OFFSET] = 6;
    data[MINT_IS_INITIALIZED_OFFSET] = 1;
    json!({
        "context": { "slot": 1 },
        "value": {
            "data": [STANDARD.encode(data), "base64"],
            "executable": false,
            "lamports": 1_461_600,
            "owner": spl_token_interface::ID.to_string(),
            "rentEpoch": 0
        }
    })
});

fn mock_rpc_client() -> RpcClient {
    let mocks = (0..ACCOUNT_INFO_RESPONSES)
        .map(|_| (RpcRequest::GetAccountInfo, MINT_ACCOUNT_INFO.clone()))
        .collect();
    RpcClient::new_mock_with_mocks_map("succeeds".to_string(), mocks)
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
    let rpc_client = mock_rpc_client();
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
