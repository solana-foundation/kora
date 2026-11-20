use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{collections::HashMap, sync::Arc};

pub const DEFAULT_LOCAL_RPC_URL: &str = "http://localhost:8899";

fn account_to_value(account: &Account) -> Value {
    let encoded_data = STANDARD.encode(&account.data);
    json!({
        "context": { "slot": 1 },
        "value": {
            "data": [encoded_data, "base64"],
            "executable": account.executable,
            "lamports": account.lamports,
            "owner": account.owner.to_string(),
            "rentEpoch": account.rent_epoch
        }
    })
}

// Registry entry layout stores owner in the first 32 bytes of a 65-byte account data slice.
pub fn make_registry_account(registry_program: &Pubkey, owner: &Pubkey) -> Account {
    let mut data = vec![0u8; crate::REGISTRY_ENTRY_LEN];
    data[..32].copy_from_slice(owner.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: *registry_program,
        executable: false,
        rent_epoch: 0,
    }
}

// Matches on-chain ProgramData account layout: 45-byte metadata header followed by ELF bytes.
pub fn make_program_data_account(
    slot: u64,
    upgrade_authority: Option<&Pubkey>,
    elf_bytes: &[u8],
) -> Account {
    let header = UpgradeableLoaderState::ProgramData {
        slot,
        upgrade_authority_address: upgrade_authority.copied(),
    };
    let mut data = vec![0u8; UpgradeableLoaderState::size_of_programdata_metadata()];
    let serialized = bincode::serialize(&header).expect("failed to serialize ProgramData header");
    data[..serialized.len()].copy_from_slice(&serialized);
    data.extend_from_slice(elf_bytes);

    Account {
        lamports: 1_000_000,
        data,
        owner: crate::BPF_LOADER_UPGRADEABLE,
        executable: false,
        rent_epoch: 0,
    }
}

pub struct DeployRpcMockBuilder {
    mocks: HashMap<RpcRequest, Value>,
    account_responses: Vec<Value>,
}

impl Default for DeployRpcMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DeployRpcMockBuilder {
    pub fn new() -> Self {
        Self { mocks: HashMap::new(), account_responses: Vec::new() }
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
        self.account_responses.push(account_to_value(account));
        self
    }

    pub fn with_account_not_found(mut self) -> Self {
        self.account_responses.push(json!({
            "context": { "slot": 1 },
            "value": null
        }));
        self
    }

    pub fn with_registry_account(self, registry_program: &Pubkey, owner: &Pubkey) -> Self {
        let account = make_registry_account(registry_program, owner);
        self.with_account_info(&account)
    }

    pub fn with_program_data_account(
        self,
        slot: u64,
        upgrade_authority: Option<&Pubkey>,
        elf_bytes: &[u8],
    ) -> Self {
        let account = make_program_data_account(slot, upgrade_authority, elf_bytes);
        self.with_account_info(&account)
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

    pub fn build(mut self) -> Arc<RpcClient> {
        if self.account_responses.len() <= 1 {
            if let Some(val) = self.account_responses.pop() {
                self.mocks.insert(RpcRequest::GetAccountInfo, val);
            }
            Arc::new(RpcClient::new_mock_with_mocks(DEFAULT_LOCAL_RPC_URL.to_string(), self.mocks))
        } else {
            let mut entries: Vec<(RpcRequest, Value)> = self.mocks.into_iter().collect();
            for val in self.account_responses {
                entries.push((RpcRequest::GetAccountInfo, val));
            }
            Arc::new(RpcClient::new_mock_with_mocks_map(
                DEFAULT_LOCAL_RPC_URL.to_string(),
                entries.into_iter().collect(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_program_data_account_round_trip() -> Result<()> {
        let slot = 98_765_432;
        let authority = Pubkey::new_unique();
        let elf_bytes = b"\x7fELF_DUMMY_PROGRAM_BYTES_FOR_TESTING";

        let rpc = DeployRpcMockBuilder::new()
            .with_program_data_account(slot, Some(&authority), elf_bytes)
            .build();

        let pdata_pubkey = Pubkey::new_unique();
        let account = rpc.get_account(&pdata_pubkey).await?;

        let metadata_len = UpgradeableLoaderState::size_of_programdata_metadata();
        assert_eq!(metadata_len, 45);
        assert!(account.data.len() >= metadata_len);

        let state: UpgradeableLoaderState = bincode::deserialize(&account.data[..metadata_len])?;
        match state {
            UpgradeableLoaderState::ProgramData {
                slot: parsed_slot,
                upgrade_authority_address,
            } => {
                assert_eq!(parsed_slot, slot);
                assert_eq!(upgrade_authority_address, Some(authority));
            }
            other => panic!("expected ProgramData, got {other:?}"),
        }

        assert_eq!(&account.data[metadata_len..], elf_bytes);
        assert_eq!(account.owner, crate::BPF_LOADER_UPGRADEABLE);

        Ok(())
    }

    #[tokio::test]
    async fn test_program_data_account_without_authority_round_trip() -> Result<()> {
        let slot = 12345;
        let elf_bytes = b"\x7fELF_IMMUTABLE_PROGRAM_BYTES";

        let rpc =
            DeployRpcMockBuilder::new().with_program_data_account(slot, None, elf_bytes).build();

        let account = rpc.get_account(&Pubkey::new_unique()).await?;
        let metadata_len = UpgradeableLoaderState::size_of_programdata_metadata();
        assert_eq!(metadata_len, 45);

        let state: UpgradeableLoaderState = bincode::deserialize(&account.data[..metadata_len])?;
        match state {
            UpgradeableLoaderState::ProgramData {
                slot: parsed_slot,
                upgrade_authority_address,
            } => {
                assert_eq!(parsed_slot, slot);
                assert_eq!(upgrade_authority_address, None);
            }
            other => panic!("expected ProgramData, got {other:?}"),
        }

        assert_eq!(&account.data[metadata_len..], elf_bytes);
        Ok(())
    }

    #[tokio::test]
    async fn test_program_data_account_with_empty_elf_bytes() -> Result<()> {
        let slot = 55;
        let authority = Pubkey::new_unique();

        let rpc = DeployRpcMockBuilder::new()
            .with_program_data_account(slot, Some(&authority), b"")
            .build();

        let account = rpc.get_account(&Pubkey::new_unique()).await?;
        let metadata_len = UpgradeableLoaderState::size_of_programdata_metadata();
        assert_eq!(account.data.len(), metadata_len);
        assert_eq!(&account.data[metadata_len..], b"");
        Ok(())
    }

    #[tokio::test]
    async fn test_registry_account_passes_assert_registered_owner_for_matching_owner() -> Result<()>
    {
        let registry_program = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let owner = Pubkey::new_unique();

        let rpc =
            DeployRpcMockBuilder::new().with_registry_account(&registry_program, &owner).build();

        let res = crate::assert_registered_owner(&rpc, &registry_program, &program, &owner).await;
        assert!(res.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_account_fails_assert_registered_owner_for_different_owner() -> Result<()>
    {
        let registry_program = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let different_wallet = Pubkey::new_unique();

        let rpc =
            DeployRpcMockBuilder::new().with_registry_account(&registry_program, &owner).build();

        let err =
            crate::assert_registered_owner(&rpc, &registry_program, &program, &different_wallet)
                .await
                .unwrap_err();
        assert!(err.to_string().contains("sign with the registered wallet to upgrade"));

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_account_missing_fails_assert_registered_owner() -> Result<()> {
        let registry_program = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let wallet = Pubkey::new_unique();

        let rpc = DeployRpcMockBuilder::new().with_account_not_found().build();

        let err = crate::assert_registered_owner(&rpc, &registry_program, &program, &wallet)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("has no deploy-registry entry"));

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_account_malformed_length_fails_assert_registered_owner() -> Result<()> {
        let registry_program = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let wallet = Pubkey::new_unique();

        let malformed_account = Account {
            lamports: 1_000_000,
            data: vec![0u8; 32],
            owner: registry_program,
            executable: false,
            rent_epoch: 0,
        };

        let rpc = DeployRpcMockBuilder::new().with_account_info(&malformed_account).build();

        let err = crate::assert_registered_owner(&rpc, &registry_program, &program, &wallet)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("is malformed"));

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_account_wrong_program_owner_fails_assert_registered_owner() -> Result<()>
    {
        let registry_program = Pubkey::new_unique();
        let wrong_program_owner = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let wallet = Pubkey::new_unique();

        let account = Account {
            lamports: 1_000_000,
            data: vec![0u8; crate::REGISTRY_ENTRY_LEN],
            owner: wrong_program_owner,
            executable: false,
            rent_epoch: 0,
        };

        let rpc = DeployRpcMockBuilder::new().with_account_info(&account).build();

        let err = crate::assert_registered_owner(&rpc, &registry_program, &program, &wallet)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("is malformed"));

        Ok(())
    }

    #[tokio::test]
    async fn test_sequential_accounts_resolution() -> Result<()> {
        let registry_program = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let slot = 42;
        let authority = Pubkey::new_unique();
        let elf_bytes = b"sample_elf";

        let rpc = DeployRpcMockBuilder::new()
            .with_registry_account(&registry_program, &owner)
            .with_program_data_account(slot, Some(&authority), elf_bytes)
            .with_account_not_found()
            .build();

        let acc1 = rpc.get_account(&Pubkey::new_unique()).await?;
        assert_eq!(acc1.owner, registry_program);

        let acc2 = rpc.get_account(&Pubkey::new_unique()).await?;
        assert_eq!(acc2.owner, crate::BPF_LOADER_UPGRADEABLE);

        let acc3_res = rpc.get_account(&Pubkey::new_unique()).await;
        assert!(acc3_res.is_err());

        Ok(())
    }
}
