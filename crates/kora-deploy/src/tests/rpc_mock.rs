use base64::{self, engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_LOCAL_RPC_URL: &str = "http://localhost:8899";

pub struct DeployRpcMockBuilder {
    mocks: HashMap<RpcRequest, Value>,
}

impl Default for DeployRpcMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DeployRpcMockBuilder {
    pub fn new() -> Self {
        Self { mocks: HashMap::new() }
    }

    pub fn with_blockhash(mut self) -> Self {
        self.mocks.insert(
            RpcRequest::GetLatestBlockhash,
            json!({ "context": { "slot": 1 }, "value": { "blockhash": Pubkey::new_unique().to_string(), "lastValidBlockHeight": 1000 } }),
        );
        self
    }

    pub fn with_minimum_balance_for_rent_exemption(mut self, lamports: u64) -> Self {
        self.mocks.insert(RpcRequest::GetMinimumBalanceForRentExemption, json!(lamports));
        self
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

    pub fn with_signature_status(mut self, confirmations: usize) -> Self {
        self.mocks.insert(
            RpcRequest::GetSignatureStatuses,
            json!({
                "context": { "slot": 1 },
                "value": [
                    {
                        "slot": 1,
                        "confirmations": confirmations,
                        "err": null,
                        "status": { "Ok": null },
                        "confirmation_status": "finalized"
                    }
                ]
            }),
        );
        self
    }

    pub fn with_slot(mut self, slot: u64) -> Self {
        self.mocks.insert(RpcRequest::GetSlot, json!(slot));
        self
    }

    pub fn build(self) -> Arc<RpcClient> {
        Arc::new(RpcClient::new_mock_with_mocks(DEFAULT_LOCAL_RPC_URL.to_string(), self.mocks))
    }
}
