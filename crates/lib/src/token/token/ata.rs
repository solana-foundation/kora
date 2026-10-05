use crate::constant;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_associated_token_account_interface::program::id as ata_program_id;

use super::{AtaCreationInstructionInfo, TokenUtil};

impl TokenUtil {
    /// Check if the transaction contains an ATA creation instruction for the given destination address.
    /// Supports both CreateAssociatedTokenAccount and CreateAssociatedTokenAccountIdempotent instructions.
    /// Returns Some((wallet_owner, mint)) if found, None otherwise.
    pub fn find_ata_creation_for_destination(
        instructions: &[Instruction],
        destination_address: &Pubkey,
    ) -> Option<(Pubkey, Pubkey)> {
        for ix in instructions {
            if let Some(info) = Self::parse_ata_creation_instruction(ix) {
                if info.ata_address == *destination_address {
                    return Some((info.wallet_owner, info.mint));
                }
            }
        }
        None
    }

    /// Parse ATA Create/CreateIdempotent instructions.
    /// Returns None for non-ATA instructions or unsupported ATA variants.
    pub fn parse_ata_creation_instruction(
        instruction: &Instruction,
    ) -> Option<AtaCreationInstructionInfo> {
        if instruction.program_id != ata_program_id()
            || instruction.accounts.len()
                < constant::instruction_indexes::ata_instruction_indexes::MIN_ACCOUNTS
        {
            return None;
        }

        // The ATA program treats empty instruction data as the legacy Create variant.
        let discriminator = match instruction.data.first() {
            Some(discriminator) => *discriminator,
            None => 0,
        };
        let is_idempotent = match discriminator {
            0 => false,
            1 => true,
            _ => return None,
        };

        Some(AtaCreationInstructionInfo {
            payer: instruction.accounts
                [constant::instruction_indexes::ata_instruction_indexes::PAYER_INDEX]
                .pubkey,
            ata_address: instruction.accounts
                [constant::instruction_indexes::ata_instruction_indexes::ATA_ADDRESS_INDEX]
                .pubkey,
            wallet_owner: instruction.accounts
                [constant::instruction_indexes::ata_instruction_indexes::WALLET_OWNER_INDEX]
                .pubkey,
            mint: instruction.accounts
                [constant::instruction_indexes::ata_instruction_indexes::MINT_INDEX]
                .pubkey,
            token_program: instruction.accounts
                [constant::instruction_indexes::ata_instruction_indexes::TOKEN_PROGRAM_INDEX]
                .pubkey,
            is_idempotent,
        })
    }

    /// Extract ATA Create/CreateIdempotent instructions where the fee payer funds account creation.
    pub fn find_fee_payer_ata_creations(
        instructions: &[Instruction],
        fee_payer: &Pubkey,
    ) -> Vec<AtaCreationInstructionInfo> {
        instructions
            .iter()
            .filter_map(Self::parse_ata_creation_instruction)
            .filter(|info| info.payer == *fee_payer)
            .collect()
    }
}
