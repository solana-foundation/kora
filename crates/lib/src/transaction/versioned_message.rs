use solana_message::VersionedMessage;

use crate::error::KoraError;
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub trait VersionedMessageExt {
    fn encode_b64_message(&self) -> Result<String, KoraError>;
}

impl VersionedMessageExt for VersionedMessage {
    fn encode_b64_message(&self) -> Result<String, KoraError> {
        let serialized = self.serialize();
        Ok(STANDARD.encode(serialized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_message::{compiled_instruction::CompiledInstruction, v0, Message};
    use solana_sdk::{
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer as _,
    };

    #[test]
    fn test_encode_b64_message_legacy() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1, 2, 3, 4, 5],
            vec![AccountMeta::new(keypair.pubkey(), true)],
        );

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&keypair.pubkey())));

        let encoded = message.encode_b64_message().unwrap();

        // Verify we can decode the base64 back to bytes
        let decoded_bytes = STANDARD.decode(&encoded).unwrap();
        assert!(!decoded_bytes.is_empty());

        // Verify it matches the original serialized message
        let original_bytes = message.serialize();
        assert_eq!(decoded_bytes, original_bytes);
    }

    #[test]
    fn test_encode_b64_message_v0() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![keypair.pubkey(), recipient, program_id],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2,
                accounts: vec![0, 1],
                data: vec![1, 2, 3],
            }],
            address_table_lookups: vec![],
        };

        let message = VersionedMessage::V0(v0_message);

        let encoded = message.encode_b64_message().unwrap();

        // Verify we can decode the base64 back to bytes
        let decoded_bytes = STANDARD.decode(&encoded).unwrap();
        assert!(!decoded_bytes.is_empty());

        // Verify it matches the original serialized message
        let original_bytes = message.serialize();
        assert_eq!(decoded_bytes, original_bytes);
    }

    #[test]
    fn test_encode_b64_message_v0_with_lookup_tables() {
        let keypair = Keypair::new();
        let program_id = Pubkey::new_unique();
        let lookup_table_account = Pubkey::new_unique();

        let v0_message = v0::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![keypair.pubkey(), program_id],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![0, 2], // Account at index 2 will come from lookup table
                data: vec![42, 0, 1, 2],
            }],
            address_table_lookups: vec![solana_message::v0::MessageAddressTableLookup {
                account_key: lookup_table_account,
                writable_indexes: vec![0],
                readonly_indexes: vec![],
            }],
        };

        let message = VersionedMessage::V0(v0_message);

        let encoded = message.encode_b64_message().unwrap();

        // Verify we can decode the base64 back to bytes
        let decoded_bytes = STANDARD.decode(&encoded).unwrap();
        assert!(!decoded_bytes.is_empty());

        // Verify it matches the original serialized message
        let original_bytes = message.serialize();
        assert_eq!(decoded_bytes, original_bytes);

        // Verify lookup table data is preserved in original message
        match message {
            VersionedMessage::V0(v0_msg) => {
                assert_eq!(v0_msg.address_table_lookups.len(), 1);
                assert_eq!(v0_msg.address_table_lookups[0].account_key, lookup_table_account);
                assert_eq!(v0_msg.address_table_lookups[0].writable_indexes, vec![0]);
                assert_eq!(v0_msg.address_table_lookups[0].readonly_indexes.len(), 0);
            }
            _ => panic!("Expected V0 message"),
        }
    }
}
