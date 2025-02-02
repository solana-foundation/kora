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
