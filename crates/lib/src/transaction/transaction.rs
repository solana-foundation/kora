use solana_message::VersionedMessage;
use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
    signature::Signature,
    transaction::{Transaction, VersionedTransaction},
};

use crate::error::KoraError;
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub struct TransactionUtil {}

impl TransactionUtil {
    pub fn decode_b64_transaction(encoded: &str) -> Result<VersionedTransaction, KoraError> {
        let decoded = STANDARD.decode(encoded).map_err(|e| {
            KoraError::InvalidTransaction(format!("Failed to decode base64 transaction: {e}"))
        })?;

        // First try to deserialize as VersionedTransaction
        if let Ok(versioned_tx) = bincode::deserialize::<VersionedTransaction>(&decoded) {
            return Ok(versioned_tx);
        }

        // Fall back to legacy Transaction and convert to VersionedTransaction
        let legacy_tx: Transaction = bincode::deserialize(&decoded).map_err(|e| {
            KoraError::InvalidTransaction(format!("Failed to deserialize transaction: {e}"))
        })?;

        // Convert legacy Transaction to VersionedTransaction
        Ok(VersionedTransaction {
            signatures: legacy_tx.signatures,
            message: VersionedMessage::Legacy(legacy_tx.message),
        })
    }

    pub fn new_unsigned_versioned_transaction(message: VersionedMessage) -> VersionedTransaction {
        let num_required_signatures = message.header().num_required_signatures as usize;
        VersionedTransaction {
            signatures: vec![Signature::default(); num_required_signatures],
            message,
        }
    }

    pub fn uncompile_instructions(
        instructions: &[CompiledInstruction],
        account_keys: &[Pubkey],
    ) -> Vec<Instruction> {
        instructions
            .iter()
            .map(|ix| {
                let program_id = account_keys[ix.program_id_index as usize];
                let accounts = ix
                    .accounts
                    .iter()
                    .map(|idx| AccountMeta {
                        pubkey: account_keys[*idx as usize],
                        is_signer: false,
                        is_writable: true,
                    })
                    .collect();

                Instruction { program_id, accounts, data: ix.data.clone() }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::*;
    use crate::error::KoraError;
    use serde_json::json;
    use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
    use solana_message::{v0, Message};
    use solana_sdk::{account::Account, hash::Hash, signature::Keypair, signer::Signer as _};

    fn get_mock_rpc_client(account: &Account) -> Arc<RpcClient> {
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
        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    #[test]
    fn test_decode_b64_transaction_invalid_input() {
        let result = TransactionUtil::decode_b64_transaction("not-base64!");
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));

        let result = TransactionUtil::decode_b64_transaction("AQID"); // base64 of [1,2,3]
        assert!(matches!(result, Err(KoraError::InvalidTransaction(_))));
    }

    #[test]
    fn test_uncompile_instructions() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let account_keys = vec![program_id, account1, account2];
        let compiled_ix = CompiledInstruction {
            program_id_index: 0,
            accounts: vec![1, 2], // indices into account_keys
            data: vec![1, 2, 3],
        };

        let instructions = TransactionUtil::uncompile_instructions(&[compiled_ix], &account_keys);

        assert_eq!(instructions.len(), 1);
        let uncompiled = &instructions[0];
        assert_eq!(uncompiled.program_id, program_id);
        assert_eq!(uncompiled.accounts.len(), 2);
        assert_eq!(uncompiled.accounts[0].pubkey, account1);
        assert_eq!(uncompiled.accounts[1].pubkey, account2);
        assert_eq!(uncompiled.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_new_unsigned_versioned_transaction() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));

        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message.clone());

        // Should have correct number of signatures (all default/empty)
        assert_eq!(transaction.signatures.len(), message.header().num_required_signatures as usize);
        // All signatures should be default (empty)
        for sig in &transaction.signatures {
            assert_eq!(*sig, Signature::default());
        }
        assert_eq!(transaction.message, message);
    }

    #[test]
    fn test_new_unsigned_versioned_transaction_v0() {
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        // Create V0 message
        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![keypair.pubkey(), instruction.program_id],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0],
                data: instruction.data,
            }],
            address_table_lookups: vec![],
        };
        let message = VersionedMessage::V0(v0_message);

        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message.clone());

        assert_eq!(transaction.signatures.len(), 1);
        assert_eq!(transaction.signatures[0], Signature::default());
        assert_eq!(transaction.message, message);
    }

    #[test]
    fn test_decode_b64_transaction_legacy_fallback() {
        // Test that we can decode legacy transactions and convert them to versioned
        let keypair = Keypair::new();
        let instruction = Instruction::new_with_bytes(
            Pubkey::new_unique(),
            &[1, 2, 3],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let legacy_message = Message::new(&[instruction], Some(&keypair.pubkey()));
        let legacy_tx = Transaction::new(&[&keypair], legacy_message, Hash::default());

        let serialized = bincode::serialize(&legacy_tx).unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(serialized);

        let decoded = TransactionUtil::decode_b64_transaction(&encoded).unwrap();

        match decoded.message {
            VersionedMessage::Legacy(msg) => {
                assert_eq!(msg.instructions.len(), 1);
                assert_eq!(msg.account_keys.len(), 2); // keypair + program_id
            }
            VersionedMessage::V0(_) => panic!("Expected legacy message after conversion"),
        }
    }
}
