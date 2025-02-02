use solana_program::{
    instruction::{AccountMeta, Instruction as ProgramInstruction},
    pubkey::Pubkey as ProgramPubkey,
};
use solana_sdk::{instruction::Instruction as SdkInstruction, pubkey::Pubkey as SdkPubkey};

#[derive(Debug)]
pub struct SolanaTypeConverter;

impl SolanaTypeConverter {
    pub fn sdk_instruction(ix: &ProgramInstruction) -> SdkInstruction {
        SdkInstruction {
            program_id: Self::sdk_pubkey(&ix.program_id),
            accounts: ix
                .accounts
                .iter()
                .map(|meta| AccountMeta {
                    pubkey: Self::sdk_pubkey(&meta.pubkey),
                    is_signer: meta.is_signer,
                    is_writable: meta.is_writable,
                })
                .collect(),
            data: ix.data.clone(),
        }
    }

    pub fn program_instruction(ix: &SdkInstruction) -> ProgramInstruction {
        ProgramInstruction {
            program_id: Self::program_pubkey(&ix.program_id),
            accounts: ix
                .accounts
                .iter()
                .map(|meta| AccountMeta {
                    pubkey: Self::program_pubkey(&meta.pubkey),
                    is_signer: meta.is_signer,
                    is_writable: meta.is_writable,
                })
                .collect(),
            data: ix.data.clone(),
        }
    }

    pub fn sdk_pubkey(pubkey: &ProgramPubkey) -> SdkPubkey {
        SdkPubkey::new_from_array(pubkey.to_bytes())
    }

    pub fn program_pubkey(pubkey: &SdkPubkey) -> ProgramPubkey {
        ProgramPubkey::new_from_array(pubkey.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_conversions() {
        let original_program_pubkey = ProgramPubkey::new_unique();

        let sdk_pubkey = SolanaTypeConverter::sdk_pubkey(&original_program_pubkey);
        let converted_program_pubkey = SolanaTypeConverter::program_pubkey(&sdk_pubkey);

        assert_eq!(original_program_pubkey, converted_program_pubkey);

        let original_sdk_pubkey = SdkPubkey::new_unique();
        let program_pubkey = SolanaTypeConverter::program_pubkey(&original_sdk_pubkey);
        let converted_sdk_pubkey = SolanaTypeConverter::sdk_pubkey(&program_pubkey);

        assert_eq!(original_sdk_pubkey, converted_sdk_pubkey);
    }

    #[test]
    fn test_instruction_conversions() {
        let program_id = ProgramPubkey::new_unique();
        let account1 = ProgramPubkey::new_unique();
        let account2 = ProgramPubkey::new_unique();
        let data = vec![1, 2, 3, 4];

        let original_program_ix = ProgramInstruction {
            program_id,
            accounts: vec![
                AccountMeta { pubkey: account1, is_signer: true, is_writable: true },
                AccountMeta { pubkey: account2, is_signer: false, is_writable: true },
            ],
            data: data.clone(),
        };

        let sdk_ix = SolanaTypeConverter::sdk_instruction(&original_program_ix);
        let converted_program_ix = SolanaTypeConverter::program_instruction(&sdk_ix);

        assert_eq!(original_program_ix.program_id, converted_program_ix.program_id);
        assert_eq!(original_program_ix.data, converted_program_ix.data);
        assert_eq!(original_program_ix.accounts.len(), converted_program_ix.accounts.len());

        for (original, converted) in
            original_program_ix.accounts.iter().zip(converted_program_ix.accounts.iter())
        {
            assert_eq!(original.pubkey, converted.pubkey);
            assert_eq!(original.is_signer, converted.is_signer);
            assert_eq!(original.is_writable, converted.is_writable);
        }
    }
}
