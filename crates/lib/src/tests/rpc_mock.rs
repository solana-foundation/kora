use base64::{self, engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};

use crate::tests::account_mock::MintAccountMockBuilder;

pub const DEFAULT_LOCAL_RPC_URL: &str = "http://localhost:8899";

/// Builder for creating mock RPC clients with different responses
pub struct RpcMockBuilder {
    mocks: HashMap<RpcRequest, Value>,
}

impl Default for RpcMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcMockBuilder {
    pub fn new() -> Self {
        Self { mocks: HashMap::new() }
    }

    pub fn with_account_info(mut self, account: &Account) -> Self {
        let encoded_data = STANDARD.encode(&account.data);
        self.mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                    "rentEpoch": account.rent_epoch
                }
            }),
        );
        self
    }

    pub fn with_account_not_found(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": { "slot": 1 },
                "value": null
            }),
        );
        self
    }

    pub fn with_fee_estimate(mut self, fee: u64) -> Self {
        self.mocks.insert(
            RpcRequest::GetFeeForMessage,
            json!({
                "context": { "slot": 1 },
                "value": fee
            }),
        );
        self
    }

    pub fn with_mint_account(self, decimals: u8) -> Self {
        let mint_account = MintAccountMockBuilder::new()
            .with_decimals(decimals)
            .with_supply(1_000_000_000_000)
            .build();

        self.with_account_info(&mint_account)
    }

    pub fn with_custom_mock(mut self, request: RpcRequest, response: Value) -> Self {
        self.mocks.insert(request, response);
        self
    }

    pub fn with_custom_mocks(mut self, extra_mocks: HashMap<RpcRequest, Value>) -> Self {
        self.mocks.extend(extra_mocks);
        self
    }

    pub fn with_blockhash(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::GetLatestBlockhash,
            json!({ "context": { "slot": 1 }, "value": { "blockhash": Pubkey::new_unique().to_string(), "lastValidBlockHeight": 1000 } }),
        );
        self
    }

    pub fn with_simulation(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::SimulateTransaction,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "err": null,
                    "logs": [],
                    "accounts": null,
                    "unitsConsumed": 1000,
                    "innerInstructions": []
                }
            }),
        );
        self
    }

    pub fn with_epoch_info_mock(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::GetEpochInfo,
            json!({
                "context": { "slot": 1 },
                "value": {
                    "epoch": 100,
                    "slotIndex": 1,
                    "slotsInEpoch": 432000
                }
            }),
        );
        self
    }

    pub fn with_send_transaction(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::SendTransaction,
            json!("5j7s8Wmt6yZb8kWBBdyKVEE8Pk8z2yBV2bX4Ct4nnEzHrNmHbG8LNHKtPj8F3mJq1vE8Zk2sZf2RjNjVxNz8QdJZ")
        );
        self.mocks.insert(
            RpcRequest::GetSignatureStatuses,
            json!({
                "context": { "slot": 1 },
                "value": [
                    {
                        "slot": 1,
                        "confirmations": 0,
                        "err": null,
                        "status": { "Ok": null },
                        "confirmation_status": "finalized"
                    }
                ]
            }),
        );
        self
    }

    pub fn build_with_sequential_accounts(self, accounts: Vec<&Account>) -> Arc<RpcClient> {
        let mut entries: Vec<(RpcRequest, Value)> = self.mocks.into_iter().collect();

        for account in accounts {
            let encoded_data = STANDARD.encode(&account.data);
            entries.push((
                RpcRequest::GetAccountInfo,
                json!({
                    "context": { "slot": 1 },
                    "value": {
                        "data": [encoded_data, "base64"],
                        "executable": account.executable,
                        "lamports": account.lamports,
                        "owner": account.owner.to_string(),
                        "rentEpoch": account.rent_epoch
                    }
                }),
            ));
        }

        Arc::new(RpcClient::new_mock_with_mocks_map(
            DEFAULT_LOCAL_RPC_URL.to_string(),
            entries.into_iter().collect(),
        ))
    }

    pub fn build(self) -> Arc<RpcClient> {
        Arc::new(RpcClient::new_mock_with_mocks(DEFAULT_LOCAL_RPC_URL.to_string(), self.mocks))
    }
}

pub fn create_mock_rpc_client_with_account(account: &Account) -> Arc<RpcClient> {
    RpcMockBuilder::new().with_account_info(account).build()
}

pub fn create_mock_rpc_client_account_not_found() -> Arc<RpcClient> {
    RpcMockBuilder::new().with_account_not_found().build()
}

pub fn create_mock_rpc_client_with_mint(mint_decimals: u8) -> Arc<RpcClient> {
    RpcMockBuilder::new().with_mint_account(mint_decimals).build()
}
