use std::collections::HashMap;

use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use super::IxUtils;
use crate::{
    constant::instruction_indexes,
    error::KoraError,
    sanitize_error,
    transaction::{
        token2022_security::{Token2022InterfaceFamily, Token2022SecurityParser},
        VersionedTransactionResolved,
    },
};

/// Discriminator of the p-token `Batch` instruction (`spl_token_interface` variant `Batch = 255`).
pub(super) const BATCH_DISCRIMINATOR: u8 = 255;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSPLInstructionType {
    SplTokenTransfer,
    SplTokenBurn,
    SplTokenCloseAccount,
    SplTokenApprove,
    SplTokenRevoke,
    SplTokenSetAuthority,
    SplTokenMintTo,
    SplTokenInitializeMint,
    SplTokenInitializeAccount,
    SplTokenInitializeMultisig,
    SplTokenFreezeAccount,
    SplTokenThawAccount,
    SplTokenReallocate,
    SplTokenInitializePausable,
    SplTokenPause,
    SplTokenResume,
    SplTokenInitializeTransferHook,
    SplTokenTransferHookUpdate,
    SplTokenWithdrawExcessLamports,
    SplTokenUnwrapLamports,
    /// A Token-2022 extension instruction with no dedicated fee-payer parser. Its fee-payer
    /// checks run through `Token2022SecurityParser`, not this variant.
    SplTokenUnknownExtension,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSPLInstructionData {
    SplTokenTransfer {
        amount: u64,
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        mint: Option<Pubkey>,
        source_address: Pubkey,
        destination_address: Pubkey,
        is_2022: bool,
    },
    SplTokenBurn {
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenCloseAccount {
        owner: Pubkey,
        account: Pubkey,
        destination: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenApprove {
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenRevoke {
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenSetAuthority {
        authority: Pubkey,
        new_authority: Option<Pubkey>,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    // MintTo and MintToChecked
    SplTokenMintTo {
        mint_authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    // InitializeMint and InitializeMint2
    SplTokenInitializeMint {
        mint_authority: Pubkey,
        freeze_authority: Option<Pubkey>,
        is_2022: bool,
    },
    // InitializeAccount, InitializeAccount2, InitializeAccount3
    SplTokenInitializeAccount {
        owner: Pubkey,
        is_2022: bool,
    },
    // InitializeMultisig and InitializeMultisig2
    SplTokenInitializeMultisig {
        signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenFreezeAccount {
        freeze_authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenThawAccount {
        freeze_authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    // Token2022 Reallocate
    SplTokenReallocate {
        account: Pubkey,
        payer: Pubkey,
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenInitializePausable {
        authority: Pubkey,
    },
    SplTokenPause {
        authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
    },
    SplTokenResume {
        authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
    },
    SplTokenInitializeTransferHook {
        authority: Option<Pubkey>,
        program_id: Option<Pubkey>,
    },
    SplTokenTransferHookUpdate {
        authority: Pubkey,
        multisig_signers: Vec<Pubkey>,
        program_id: Option<Pubkey>,
    },
    // WithdrawExcessLamports (both spl and spl 2022)
    SplTokenWithdrawExcessLamports {
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenUnwrapLamports {
        owner: Pubkey,
        multisig_signers: Vec<Pubkey>,
        is_2022: bool,
    },
    SplTokenUnknownExtension {
        accounts: Vec<Pubkey>,
    },
}

instruction_type!(ParsedSPLInstructionData => ParsedSPLInstructionType {
    SplTokenTransfer,
    SplTokenBurn,
    SplTokenCloseAccount,
    SplTokenApprove,
    SplTokenRevoke,
    SplTokenSetAuthority,
    SplTokenMintTo,
    SplTokenInitializeMint,
    SplTokenInitializeAccount,
    SplTokenInitializeMultisig,
    SplTokenFreezeAccount,
    SplTokenThawAccount,
    SplTokenReallocate,
    SplTokenInitializePausable,
    SplTokenPause,
    SplTokenResume,
    SplTokenInitializeTransferHook,
    SplTokenTransferHookUpdate,
    SplTokenWithdrawExcessLamports,
    SplTokenUnwrapLamports,
    SplTokenUnknownExtension,
});

/// Emits the instruction arms spl-token and Token-2022 share. `TokenInstruction` resolves at
/// the call site, so each program matches against its own interface crate.
macro_rules! match_shared_token_instruction {
    (
        $token_ix:expr, $instruction:ident, $parsed:ident, $is_2022:expr,
        { $($program_arms:tt)* }
    ) => {
        match $token_ix {
            #[allow(deprecated)]
            TokenInstruction::Transfer { amount } => {
                use instruction_indexes::spl_token_transfer as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_token_transfer(
                    &mut $parsed,
                    $instruction,
                    (
                        ix::OWNER_INDEX,
                        ix::SOURCE_ADDRESS_INDEX,
                        ix::DESTINATION_ADDRESS_INDEX,
                        ix::REQUIRED_NUMBER_OF_ACCOUNTS,
                    ),
                    amount,
                    None,
                    $is_2022,
                );
            }
            TokenInstruction::TransferChecked { amount, .. } => {
                use instruction_indexes::spl_token_transfer_checked as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_token_transfer(
                    &mut $parsed,
                    $instruction,
                    (
                        ix::OWNER_INDEX,
                        ix::SOURCE_ADDRESS_INDEX,
                        ix::DESTINATION_ADDRESS_INDEX,
                        ix::REQUIRED_NUMBER_OF_ACCOUNTS,
                    ),
                    amount,
                    Some($instruction.accounts[ix::MINT_INDEX].pubkey),
                    $is_2022,
                );
            }
            TokenInstruction::Burn { .. } => {
                let owner = Self::parse_burn_owner_with_mint_fallback($instruction)?;
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenBurn {
                        owner,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::BurnChecked { .. } => {
                use instruction_indexes::spl_token_burn as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenBurn {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::CloseAccount => {
                use instruction_indexes::spl_token_close_account as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenCloseAccount {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        account: $instruction.accounts[ix::ACCOUNT_INDEX].pubkey,
                        destination: $instruction.accounts[ix::DESTINATION_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::Approve { .. } => {
                use instruction_indexes::spl_token_approve as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenApprove {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::ApproveChecked { .. } => {
                use instruction_indexes::spl_token_approve_checked as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenApprove {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 4),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::Revoke => {
                use instruction_indexes::spl_token_revoke as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenRevoke {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 2),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::SetAuthority { new_authority, .. } => {
                use instruction_indexes::spl_token_set_authority as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenSetAuthority {
                        authority: $instruction.accounts[ix::CURRENT_AUTHORITY_INDEX].pubkey,
                        new_authority: new_authority.into(),
                        multisig_signers: Self::extract_multisig_signers($instruction, 2),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::MintTo { .. } => {
                use instruction_indexes::spl_token_mint_to as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenMintTo {
                        mint_authority: $instruction.accounts[ix::MINT_AUTHORITY_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::MintToChecked { .. } => {
                use instruction_indexes::spl_token_mint_to_checked as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenMintTo {
                        mint_authority: $instruction.accounts[ix::MINT_AUTHORITY_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeMint { mint_authority, freeze_authority, .. } => {
                use instruction_indexes::spl_token_initialize_mint as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeMint {
                        mint_authority,
                        freeze_authority: freeze_authority.into(),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeMint2 { mint_authority, freeze_authority, .. } => {
                use instruction_indexes::spl_token_initialize_mint2 as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeMint {
                        mint_authority,
                        freeze_authority: freeze_authority.into(),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeAccount => {
                use instruction_indexes::spl_token_initialize_account as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeAccount {
                        owner: $instruction.accounts[ix::OWNER_INDEX].pubkey,
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeAccount2 { owner } => {
                use instruction_indexes::spl_token_initialize_account2 as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeAccount {
                        owner,
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeAccount3 { owner } => {
                use instruction_indexes::spl_token_initialize_account3 as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeAccount {
                        owner,
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeMultisig { .. } => {
                use instruction_indexes::spl_token_initialize_multisig as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                // Signers follow the multisig account and the rent sysvar.
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeMultisig {
                        signers: Self::extract_multisig_signers($instruction, 2),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::InitializeMultisig2 { .. } => {
                use instruction_indexes::spl_token_initialize_multisig2 as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenInitializeMultisig {
                        signers: Self::extract_multisig_signers($instruction, 1),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::FreezeAccount => {
                use instruction_indexes::spl_token_freeze_account as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenFreezeAccount {
                        freeze_authority: $instruction.accounts[ix::FREEZE_AUTHORITY_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::ThawAccount => {
                use instruction_indexes::spl_token_thaw_account as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenThawAccount {
                        freeze_authority: $instruction.accounts[ix::FREEZE_AUTHORITY_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers($instruction, 3),
                        is_2022: $is_2022,
                    },
                );
            }
            TokenInstruction::WithdrawExcessLamports => {
                use instruction_indexes::spl_token_withdraw_excess_lamports as ix;
                validate_number_accounts!($instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                Self::push_parsed_spl_instruction(
                    &mut $parsed,
                    ParsedSPLInstructionData::SplTokenWithdrawExcessLamports {
                        owner: $instruction.accounts[ix::AUTHORITY_INDEX].pubkey,
                        multisig_signers: Self::extract_multisig_signers(
                            $instruction,
                            ix::MULTISIG_SIGNERS_START_INDEX,
                        ),
                        is_2022: $is_2022,
                    },
                );
            }
            $($program_arms)*
        }
    };
}

impl IxUtils {
    fn parse_burn_owner_with_mint_fallback(instruction: &Instruction) -> Result<Pubkey, KoraError> {
        // Standard Burn has 3 accounts: [source, mint, authority]
        // Reconstructed from parsed RPC data may have only 2: [source, authority]
        // (Solana RPC doesn't include mint in parsed "burn" non-checked JSON)
        if instruction.accounts.len()
            >= instruction_indexes::spl_token_burn::REQUIRED_NUMBER_OF_ACCOUNTS
        {
            return Ok(
                instruction.accounts[instruction_indexes::spl_token_burn::OWNER_INDEX].pubkey
            );
        }

        if instruction.accounts.len() == 2 {
            log::debug!(
                "Burn instruction has 2 accounts (reconstructed without mint), using index 1 as authority"
            );
            return Ok(instruction.accounts[1].pubkey);
        }

        log::error!("Burn instruction has less than 2 accounts: {:?}", instruction);
        Err(KoraError::InvalidTransaction(
            "Burn instruction doesn't have the required number of accounts".to_string(),
        ))
    }

    fn push_parsed_spl_instruction(
        parsed_instructions: &mut HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>,
        instruction_data: ParsedSPLInstructionData,
    ) {
        parsed_instructions
            .entry(instruction_data.instruction_type())
            .or_default()
            .push(instruction_data);
    }

    fn decode_token2022_extension_type<T: TryFrom<u8>>(
        instruction: &Instruction,
        extension: &str,
    ) -> Result<T, KoraError> {
        if instruction.data.len() < 2 {
            return Err(KoraError::InvalidTransaction(format!(
                "Failed to parse Token-2022 {extension} instruction"
            )));
        }
        spl_token_2022_interface::instruction::decode_instruction_type::<T>(&instruction.data[1..])
            .map_err(|e| {
                KoraError::InvalidTransaction(format!(
                    "Failed to parse Token-2022 {extension} instruction: {}",
                    sanitize_error!(e)
                ))
            })
    }

    fn push_parsed_token_transfer(
        parsed_instructions: &mut HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>,
        instruction: &Instruction,
        transfer_indexes: (usize, usize, usize, usize),
        amount: u64,
        mint: Option<Pubkey>,
        is_2022: bool,
    ) {
        let (owner_index, source_index, destination_index, multisig_start_index) = transfer_indexes;
        Self::push_parsed_spl_instruction(
            parsed_instructions,
            ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner: instruction.accounts[owner_index].pubkey,
                multisig_signers: Self::extract_multisig_signers(instruction, multisig_start_index),
                mint,
                source_address: instruction.accounts[source_index].pubkey,
                destination_address: instruction.accounts[destination_index].pubkey,
                is_2022,
            },
        );
    }

    fn push_unhandled_token2022_extension(
        parsed_instructions: &mut HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>,
        instruction: &Instruction,
    ) {
        Self::push_parsed_spl_instruction(
            parsed_instructions,
            ParsedSPLInstructionData::SplTokenUnknownExtension {
                accounts: instruction.accounts.iter().map(|a| a.pubkey).collect(),
            },
        );
    }

    fn is_spl_token_batch(instruction: &Instruction) -> bool {
        instruction.program_id == spl_token_interface::ID
            && instruction.data.first() == Some(&BATCH_DISCRIMINATOR)
    }

    fn expand_spl_token_batches(
        instructions: &[Instruction],
    ) -> Result<Vec<Instruction>, KoraError> {
        if instructions.iter().any(|ix| {
            ix.program_id == spl_token_2022_interface::ID
                && ix.data.first() == Some(&BATCH_DISCRIMINATOR)
        }) {
            return Err(KoraError::InvalidTransaction(
                "Token-2022 batch instructions are not supported".to_string(),
            ));
        }

        if !instructions.iter().any(Self::is_spl_token_batch) {
            return Ok(instructions.to_vec());
        }

        let mut expanded = Vec::with_capacity(instructions.len());
        for instruction in instructions {
            if Self::is_spl_token_batch(instruction) {
                Self::decode_spl_token_batch(instruction, &mut expanded)?;
            } else {
                expanded.push(instruction.clone());
            }
        }
        Ok(expanded)
    }

    fn decode_spl_token_batch(
        batch: &Instruction,
        out: &mut Vec<Instruction>,
    ) -> Result<(), KoraError> {
        let data = &batch.data[1..];
        let mut data_cursor = 0usize;
        let mut account_cursor = 0usize;

        while data_cursor < data.len() {
            if data_cursor + 2 > data.len() {
                return Err(KoraError::InvalidTransaction(
                    "Malformed p-token batch: truncated sub-instruction header".to_string(),
                ));
            }
            let account_count = data[data_cursor] as usize;
            let data_len = data[data_cursor + 1] as usize;
            data_cursor += 2;

            if data_cursor + data_len > data.len() {
                return Err(KoraError::InvalidTransaction(
                    "Malformed p-token batch: sub-instruction data out of bounds".to_string(),
                ));
            }
            let sub_data = data[data_cursor..data_cursor + data_len].to_vec();
            data_cursor += data_len;

            if account_cursor + account_count > batch.accounts.len() {
                return Err(KoraError::InvalidTransaction(
                    "Malformed p-token batch: sub-instruction accounts out of bounds".to_string(),
                ));
            }
            let sub_accounts =
                batch.accounts[account_cursor..account_cursor + account_count].to_vec();
            account_cursor += account_count;

            if sub_data.first() == Some(&BATCH_DISCRIMINATOR) {
                return Err(KoraError::InvalidTransaction(
                    "Nested p-token batch instructions are not allowed".to_string(),
                ));
            }

            if spl_token_interface::instruction::TokenInstruction::unpack(&sub_data).is_err() {
                return Err(KoraError::InvalidTransaction(
                    "Malformed p-token batch: unrecognized sub-instruction".to_string(),
                ));
            }

            out.push(Instruction {
                program_id: batch.program_id,
                accounts: sub_accounts,
                data: sub_data,
            });
        }

        if account_cursor != batch.accounts.len() {
            return Err(KoraError::InvalidTransaction(
                "Malformed p-token batch: unused accounts remain after decoding sub-instructions"
                    .to_string(),
            ));
        }

        Ok(())
    }

    pub fn parse_token_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>, KoraError> {
        let mut parsed_instructions: HashMap<
            ParsedSPLInstructionType,
            Vec<ParsedSPLInstructionData>,
        > = HashMap::new();

        let expanded_instructions = Self::expand_spl_token_batches(&transaction.all_instructions)?;

        for instruction in &expanded_instructions {
            let program_id = instruction.program_id;

            if program_id == spl_token_interface::ID {
                use spl_token_interface::instruction::TokenInstruction;

                let Ok(spl_ix) = TokenInstruction::unpack(&instruction.data) else {
                    continue;
                };
                match_shared_token_instruction!(spl_ix, instruction, parsed_instructions, false, {
                    TokenInstruction::UnwrapLamports { .. } => {
                        use instruction_indexes::spl_token_unwrap_lamports as ix;
                        validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                        Self::push_parsed_spl_instruction(
                            &mut parsed_instructions,
                            ParsedSPLInstructionData::SplTokenUnwrapLamports {
                                owner: instruction.accounts[ix::AUTHORITY_INDEX].pubkey,
                                multisig_signers: Self::extract_multisig_signers(
                                    instruction,
                                    ix::MULTISIG_SIGNERS_START_INDEX,
                                ),
                                is_2022: false,
                            },
                        );
                    }
                    _ => {}
                });
            } else if program_id == spl_token_2022_interface::ID {
                use spl_token_2022_interface::{
                    extension::{
                        pausable::instruction::{
                            InitializeInstructionData as PausableInitialize, PausableInstruction,
                        },
                        transfer_fee::instruction::TransferFeeInstruction,
                        transfer_hook::instruction::{
                            InitializeInstructionData as TransferHookInitialize,
                            TransferHookInstruction, UpdateInstructionData as TransferHookUpdate,
                        },
                    },
                    instruction::{decode_instruction_data, TokenInstruction},
                };

                let spl_ix = match TokenInstruction::unpack(&instruction.data) {
                    Ok(spl_ix) => spl_ix,
                    Err(e) => {
                        // Token-2022 also processes opt-in token-metadata/token-group instructions, which
                        // aren't TokenInstruction variants; route allowed families through the
                        // unknown-extension channel, validated by Token2022SecurityParser.
                        if let Some(family) =
                            Token2022SecurityParser::token_2022_interface_instruction_family(
                                &instruction.data,
                            )
                        {
                            let allowed = crate::state::get_config()
                                .map(|c| match family {
                                    Token2022InterfaceFamily::TokenMetadata => {
                                        c.validation.token_2022.allow_token_metadata_instructions
                                    }
                                    Token2022InterfaceFamily::TokenGroup => {
                                        c.validation.token_2022.allow_token_group_instructions
                                    }
                                })
                                .unwrap_or(false);

                            if allowed {
                                Self::push_unhandled_token2022_extension(
                                    &mut parsed_instructions,
                                    instruction,
                                );
                                continue;
                            }

                            return Err(KoraError::InvalidTransaction(format!(
                                "Token-2022 {} interface instructions are not supported",
                                family.name()
                            )));
                        }

                        return Err(KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 instruction: {}",
                            sanitize_error!(e)
                        )));
                    }
                };

                match_shared_token_instruction!(spl_ix, instruction, parsed_instructions, true, {
                    TokenInstruction::TransferFeeExtension => {
                        if instruction.data.len() < 2 {
                            return Err(KoraError::InvalidTransaction(
                                "Failed to parse Token-2022 TransferFee instruction".to_string(),
                            ));
                        }
                        let transfer_fee_ix = TransferFeeInstruction::unpack(&instruction.data[1..])
                            .map_err(|e| {
                                KoraError::InvalidTransaction(format!(
                                    "Failed to parse Token-2022 TransferFee instruction: {}",
                                    sanitize_error!(e)
                                ))
                            })?;

                        match transfer_fee_ix {
                            TransferFeeInstruction::TransferCheckedWithFee { amount, .. } => {
                                use instruction_indexes::spl_token_transfer_checked as ix;
                                validate_number_accounts!(
                                    instruction,
                                    ix::REQUIRED_NUMBER_OF_ACCOUNTS
                                );
                                Self::push_parsed_token_transfer(
                                    &mut parsed_instructions,
                                    instruction,
                                    (
                                        ix::OWNER_INDEX,
                                        ix::SOURCE_ADDRESS_INDEX,
                                        ix::DESTINATION_ADDRESS_INDEX,
                                        ix::REQUIRED_NUMBER_OF_ACCOUNTS,
                                    ),
                                    amount,
                                    Some(instruction.accounts[ix::MINT_INDEX].pubkey),
                                    true,
                                );
                            }
                            _ => {
                                Self::push_unhandled_token2022_extension(
                                    &mut parsed_instructions,
                                    instruction,
                                );
                            }
                        }
                    }
                    TokenInstruction::Reallocate { .. } => {
                        use instruction_indexes::spl_token_reallocate as ix;
                        validate_number_accounts!(instruction, ix::REQUIRED_NUMBER_OF_ACCOUNTS);
                        Self::push_parsed_spl_instruction(
                            &mut parsed_instructions,
                            ParsedSPLInstructionData::SplTokenReallocate {
                                account: instruction.accounts[ix::ACCOUNT_INDEX].pubkey,
                                payer: instruction.accounts[ix::PAYER_INDEX].pubkey,
                                owner: instruction.accounts[ix::OWNER_INDEX].pubkey,
                                multisig_signers: Self::extract_multisig_signers(instruction, 4),
                                is_2022: true,
                            },
                        );
                    }
                    TokenInstruction::PausableExtension => {
                        match Self::decode_token2022_extension_type::<PausableInstruction>(
                            instruction,
                            "Pausable",
                        )? {
                            PausableInstruction::Initialize => {
                                validate_number_accounts!(instruction, 1);
                                let initialize = *decode_instruction_data::<PausableInitialize>(
                                    &instruction.data[1..],
                                )
                                .map_err(|e| {
                                    KoraError::InvalidTransaction(format!(
                                        "Failed to parse Token-2022 Pausable initialize instruction: {}",
                                        sanitize_error!(e)
                                    ))
                                })?;
                                Self::push_parsed_spl_instruction(
                                    &mut parsed_instructions,
                                    ParsedSPLInstructionData::SplTokenInitializePausable {
                                        authority: initialize.authority,
                                    },
                                );
                            }
                            PausableInstruction::Pause => {
                                validate_number_accounts!(instruction, 2);
                                Self::push_parsed_spl_instruction(
                                    &mut parsed_instructions,
                                    ParsedSPLInstructionData::SplTokenPause {
                                        authority: instruction.accounts[1].pubkey,
                                        multisig_signers: Self::extract_multisig_signers(
                                            instruction,
                                            2,
                                        ),
                                    },
                                );
                            }
                            PausableInstruction::Resume => {
                                validate_number_accounts!(instruction, 2);
                                Self::push_parsed_spl_instruction(
                                    &mut parsed_instructions,
                                    ParsedSPLInstructionData::SplTokenResume {
                                        authority: instruction.accounts[1].pubkey,
                                        multisig_signers: Self::extract_multisig_signers(
                                            instruction,
                                            2,
                                        ),
                                    },
                                );
                            }
                        }
                    }
                    TokenInstruction::TransferHookExtension => {
                        match Self::decode_token2022_extension_type::<TransferHookInstruction>(
                            instruction,
                            "TransferHook",
                        )? {
                            TransferHookInstruction::Initialize => {
                                validate_number_accounts!(instruction, 1);
                                let initialize = *decode_instruction_data::<TransferHookInitialize>(
                                    &instruction.data[1..],
                                )
                                .map_err(|e| {
                                    KoraError::InvalidTransaction(format!(
                                        "Failed to parse Token-2022 TransferHook initialize instruction: {}",
                                        sanitize_error!(e)
                                    ))
                                })?;
                                Self::push_parsed_spl_instruction(
                                    &mut parsed_instructions,
                                    ParsedSPLInstructionData::SplTokenInitializeTransferHook {
                                        authority: initialize.authority.into(),
                                        program_id: initialize.program_id.into(),
                                    },
                                );
                            }
                            TransferHookInstruction::Update => {
                                validate_number_accounts!(instruction, 2);
                                let update = *decode_instruction_data::<TransferHookUpdate>(
                                    &instruction.data[1..],
                                )
                                .map_err(|e| {
                                    KoraError::InvalidTransaction(format!(
                                        "Failed to parse Token-2022 TransferHook update instruction: {}",
                                        sanitize_error!(e)
                                    ))
                                })?;
                                Self::push_parsed_spl_instruction(
                                    &mut parsed_instructions,
                                    ParsedSPLInstructionData::SplTokenTransferHookUpdate {
                                        authority: instruction.accounts[1].pubkey,
                                        multisig_signers: Self::extract_multisig_signers(
                                            instruction,
                                            2,
                                        ),
                                        program_id: update.program_id.into(),
                                    },
                                );
                            }
                        }
                    }
                    TokenInstruction::ConfidentialTransferExtension
                    | TokenInstruction::ConfidentialTransferFeeExtension
                    | TokenInstruction::ConfidentialMintBurnExtension => {
                        let allowed = crate::state::get_config()
                            .map(|c| c.validation.token_2022.allow_confidential_transfers)
                            .unwrap_or(false);
                        if !allowed {
                            return Err(KoraError::InvalidTransaction(
                                "Confidential Token-2022 instructions are not supported"
                                    .to_string(),
                            ));
                        }
                        Self::push_unhandled_token2022_extension(
                            &mut parsed_instructions,
                            instruction,
                        );
                    }
                    _ => {
                        Self::push_unhandled_token2022_extension(
                            &mut parsed_instructions,
                            instruction,
                        );
                    }
                });
            }
        }
        Ok(parsed_instructions)
    }
}
