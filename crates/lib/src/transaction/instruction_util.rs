use std::collections::HashMap;

use solana_sdk::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
    system_instruction::SystemInstruction,
    system_program::ID as SYSTEM_PROGRAM_ID,
};
use solana_transaction_status_client_types::{UiInstruction, UiParsedInstruction};

use crate::{
    constant::instruction_indexes, error::KoraError, transaction::VersionedTransactionResolved,
};

// Instruction type that we support to parse from the transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionType {
    SystemTransfer,
    SystemCreateAccount,
    SystemWithdrawNonceAccount,
    SystemAssign,
}

// Instruction type that we support to parse from the transaction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSystemInstructionData {
    // Includes transfer and transfer with seed
    SystemTransfer { lamports: u64, sender: Pubkey, receiver: Pubkey },
    // Includes create account and create account with seed
    SystemCreateAccount { lamports: u64, payer: Pubkey },
    // Includes withdraw nonce account
    SystemWithdrawNonceAccount { lamports: u64, nonce_authority: Pubkey, recipient: Pubkey },
    // Includes assign and assign with seed
    SystemAssign { authority: Pubkey },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSPLInstructionType {
    SplTokenTransfer,
    SplTokenBurn,
    SplTokenCloseAccount,
    SplTokenApprove,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParsedSPLInstructionData {
    // Includes transfer and transfer with seed (both spl and spl 2022)
    SplTokenTransfer {
        amount: u64,
        owner: Pubkey,
        mint: Option<Pubkey>,
        source_address: Pubkey,
        destination_address: Pubkey,
        is_2022: bool,
    },
    // Includes burn and burn with seed
    SplTokenBurn {
        owner: Pubkey,
        is_2022: bool,
    },
    // Includes close account
    SplTokenCloseAccount {
        owner: Pubkey,
        is_2022: bool,
    },
    // Includes approve and approve with seed
    SplTokenApprove {
        owner: Pubkey,
        is_2022: bool,
    },
}

/// Macro to validate that an instruction has the required number of accounts
/// Usage: validate_accounts!(instruction, min_count)
macro_rules! validate_number_accounts {
    ($instruction:expr, $min_count:expr) => {
        if $instruction.accounts.len() < $min_count {
            log::error!("Instruction {:?} has less than {} accounts", $instruction, $min_count);
            return Err(KoraError::InvalidTransaction(format!(
                "Instruction doesn't have the required number of accounts",
            )));
        }
    };
}

pub struct IxUtils;

pub const PARSED_DATA_FIELD_TYPE: &str = "type";
pub const PARSED_DATA_FIELD_INFO: &str = "info";

pub const PARSED_DATA_FIELD_SOURCE: &str = "source";
pub const PARSED_DATA_FIELD_DESTINATION: &str = "destination";
pub const PARSED_DATA_FIELD_OWNER: &str = "owner";

pub const PARSED_DATA_FIELD_TRANSFER: &str = "transfer";
pub const PARSED_DATA_FIELD_CREATE_ACCOUNT: &str = "createAccount";
pub const PARSED_DATA_FIELD_ASSIGN: &str = "assign";
pub const PARSED_DATA_FIELD_TRANSFER_WITH_SEED: &str = "transferWithSeed";
pub const PARSED_DATA_FIELD_CREATE_ACCOUNT_WITH_SEED: &str = "createAccountWithSeed";
pub const PARSED_DATA_FIELD_ASSIGN_WITH_SEED: &str = "assignWithSeed";
pub const PARSED_DATA_FIELD_WITHDRAW_NONCE_ACCOUNT: &str = "withdrawFromNonce";
pub const PARSED_DATA_FIELD_BURN: &str = "burn";
pub const PARSED_DATA_FIELD_BURN_CHECKED: &str = "burnChecked";
pub const PARSED_DATA_FIELD_CLOSE_ACCOUNT: &str = "closeAccount";
pub const PARSED_DATA_FIELD_TRANSFER_CHECKED: &str = "transferChecked";
pub const PARSED_DATA_FIELD_APPROVE: &str = "approve";
pub const PARSED_DATA_FIELD_APPROVE_CHECKED: &str = "approveChecked";

pub const PARSED_DATA_FIELD_AMOUNT: &str = "amount";
pub const PARSED_DATA_FIELD_LAMPORTS: &str = "lamports";
pub const PARSED_DATA_FIELD_DECIMALS: &str = "decimals";
pub const PARSED_DATA_FIELD_UI_AMOUNT: &str = "uiAmount";
pub const PARSED_DATA_FIELD_UI_AMOUNT_STRING: &str = "uiAmountString";
pub const PARSED_DATA_FIELD_TOKEN_AMOUNT: &str = "tokenAmount";
pub const PARSED_DATA_FIELD_ACCOUNT: &str = "account";
pub const PARSED_DATA_FIELD_NEW_ACCOUNT: &str = "newAccount";
pub const PARSED_DATA_FIELD_AUTHORITY: &str = "authority";
pub const PARSED_DATA_FIELD_MINT: &str = "mint";
pub const PARSED_DATA_FIELD_SPACE: &str = "space";
pub const PARSED_DATA_FIELD_DELEGATE: &str = "delegate";
pub const PARSED_DATA_FIELD_BASE: &str = "base";
pub const PARSED_DATA_FIELD_SEED: &str = "seed";
pub const PARSED_DATA_FIELD_SOURCE_BASE: &str = "sourceBase";
pub const PARSED_DATA_FIELD_SOURCE_SEED: &str = "sourceSeed";
pub const PARSED_DATA_FIELD_SOURCE_OWNER: &str = "sourceOwner";
pub const PARSED_DATA_FIELD_NONCE_ACCOUNT: &str = "nonceAccount";
pub const PARSED_DATA_FIELD_RECIPIENT: &str = "recipient";
pub const PARSED_DATA_FIELD_NONCE_AUTHORITY: &str = "nonceAuthority";

impl IxUtils {
    pub fn get_account_key_if_present(ix: &Instruction, index: usize) -> Option<Pubkey> {
        if ix.accounts.is_empty() {
            return None;
        }

        if index >= ix.accounts.len() {
            return None;
        }

        Some(ix.accounts[index].pubkey)
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

    /// Reconstruct a CompiledInstruction from various UiInstruction formats
    pub fn reconstruct_instruction_from_ui(
        ui_instruction: &UiInstruction,
        all_account_keys: &[Pubkey],
    ) -> Option<CompiledInstruction> {
        match ui_instruction {
            UiInstruction::Compiled(compiled) => {
                // Already compiled, decode data and return
                Some(CompiledInstruction {
                    program_id_index: compiled.program_id_index,
                    accounts: compiled.accounts.clone(),
                    data: bs58::decode(&compiled.data).into_vec().unwrap_or_default(),
                })
            }
            UiInstruction::Parsed(ui_parsed) => match ui_parsed {
                UiParsedInstruction::Parsed(parsed) => {
                    // Reconstruct based on program type
                    if parsed.program_id == SYSTEM_PROGRAM_ID.to_string() {
                        Self::reconstruct_system_instruction(parsed, all_account_keys)
                    } else if parsed.program == spl_token::ID.to_string()
                        || parsed.program == spl_token_2022::ID.to_string()
                    {
                        Self::reconstruct_spl_token_instruction(parsed, all_account_keys)
                    } else {
                        log::error!("Unsupported parsed program: {}", parsed.program);
                        None
                    }
                }
                UiParsedInstruction::PartiallyDecoded(partial) => {
                    if let Ok(program_id) = partial.program_id.parse::<Pubkey>() {
                        if let Some(program_idx) =
                            all_account_keys.iter().position(|k| k == &program_id)
                        {
                            // Convert account addresses to indices
                            let account_indices: Vec<u8> = partial
                                .accounts
                                .iter()
                                .filter_map(|addr_str| {
                                    addr_str
                                        .parse::<Pubkey>()
                                        .ok()
                                        .and_then(|pubkey| {
                                            all_account_keys.iter().position(|k| k == &pubkey)
                                        })
                                        .map(|idx| idx as u8)
                                })
                                .collect();

                            return Some(CompiledInstruction {
                                program_id_index: program_idx as u8,
                                accounts: account_indices,
                                data: bs58::decode(&partial.data).into_vec().unwrap_or_default(),
                            });
                        }
                    }

                    log::error!("Failed to reconstruct partially decoded instruction");
                    None
                }
            },
        }
    }

    /// Reconstruct system program instructions from parsed format
    fn reconstruct_system_instruction(
        parsed: &solana_transaction_status_client_types::ParsedInstruction,
        all_account_keys: &[Pubkey],
    ) -> Option<CompiledInstruction> {
        let program_id_index = all_account_keys.iter().position(|k| k == &SYSTEM_PROGRAM_ID)? as u8;

        let parsed_data = &parsed.parsed;
        let instruction_type = parsed_data.get(PARSED_DATA_FIELD_TYPE)?.as_str()?;
        let info = parsed_data.get(PARSED_DATA_FIELD_INFO)?;

        match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let destination =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let lamports = info.get(PARSED_DATA_FIELD_LAMPORTS)?.as_u64()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let destination_idx =
                    all_account_keys.iter().position(|k| k == &destination)? as u8;

                let transfer_ix = SystemInstruction::Transfer { lamports };
                let data = bincode::serialize(&transfer_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, destination_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_CREATE_ACCOUNT => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let new_account =
                    info.get(PARSED_DATA_FIELD_NEW_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;
                let lamports = info.get(PARSED_DATA_FIELD_LAMPORTS)?.as_u64()?;
                let space = info.get(PARSED_DATA_FIELD_SPACE)?.as_u64()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let new_account_idx =
                    all_account_keys.iter().position(|k| k == &new_account)? as u8;

                let create_ix = SystemInstruction::CreateAccount { lamports, space, owner };
                let data = bincode::serialize(&create_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, new_account_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ASSIGN => {
                let authority =
                    info.get(PARSED_DATA_FIELD_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;

                let authority_idx = all_account_keys.iter().position(|k| k == &authority)? as u8;

                let assign_ix = SystemInstruction::Assign { owner };
                let data = bincode::serialize(&assign_ix).ok()?;

                Some(CompiledInstruction { program_id_index, accounts: vec![authority_idx], data })
            }
            PARSED_DATA_FIELD_TRANSFER_WITH_SEED => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let destination =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let lamports = info.get(PARSED_DATA_FIELD_LAMPORTS)?.as_u64()?;
                let source_base =
                    info.get(PARSED_DATA_FIELD_SOURCE_BASE)?.as_str()?.parse::<Pubkey>().ok()?;
                let source_seed = info.get(PARSED_DATA_FIELD_SOURCE_SEED)?.as_str()?.to_string();
                let source_owner =
                    info.get(PARSED_DATA_FIELD_SOURCE_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let destination_idx =
                    all_account_keys.iter().position(|k| k == &destination)? as u8;
                let source_base_idx =
                    all_account_keys.iter().position(|k| k == &source_base)? as u8;

                let transfer_ix = SystemInstruction::TransferWithSeed {
                    lamports,
                    from_seed: source_seed,
                    from_owner: source_owner,
                };
                let data = bincode::serialize(&transfer_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, source_base_idx, destination_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_CREATE_ACCOUNT_WITH_SEED => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let new_account =
                    info.get(PARSED_DATA_FIELD_NEW_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let base = info.get(PARSED_DATA_FIELD_BASE)?.as_str()?.parse::<Pubkey>().ok()?;
                let seed = info.get(PARSED_DATA_FIELD_SEED)?.as_str()?.to_string();
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;
                let lamports = info.get(PARSED_DATA_FIELD_LAMPORTS)?.as_u64()?;
                let space = info.get(PARSED_DATA_FIELD_SPACE)?.as_u64()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let new_account_idx =
                    all_account_keys.iter().position(|k| k == &new_account)? as u8;
                let base_idx = all_account_keys.iter().position(|k| k == &base)? as u8;

                let create_ix =
                    SystemInstruction::CreateAccountWithSeed { base, seed, lamports, space, owner };
                let data = bincode::serialize(&create_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, new_account_idx, base_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ASSIGN_WITH_SEED => {
                let account =
                    info.get(PARSED_DATA_FIELD_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let base = info.get(PARSED_DATA_FIELD_BASE)?.as_str()?.parse::<Pubkey>().ok()?;
                let seed = info.get(PARSED_DATA_FIELD_SEED)?.as_str()?.to_string();
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;

                let account_idx = all_account_keys.iter().position(|k| k == &account)? as u8;
                let base_idx = all_account_keys.iter().position(|k| k == &base)? as u8;

                let assign_ix = SystemInstruction::AssignWithSeed { base, seed, owner };
                let data = bincode::serialize(&assign_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, base_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_WITHDRAW_NONCE_ACCOUNT => {
                let nonce_account =
                    info.get(PARSED_DATA_FIELD_NONCE_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let recipient =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let nonce_authority = info
                    .get(PARSED_DATA_FIELD_NONCE_AUTHORITY)?
                    .as_str()?
                    .parse::<Pubkey>()
                    .ok()?;
                let lamports = info.get(PARSED_DATA_FIELD_LAMPORTS)?.as_u64()?;

                let nonce_account_idx =
                    all_account_keys.iter().position(|k| k == &nonce_account)? as u8;
                let recipient_idx = all_account_keys.iter().position(|k| k == &recipient)? as u8;
                let nonce_authority_idx =
                    all_account_keys.iter().position(|k| k == &nonce_authority)? as u8;

                let withdraw_ix = SystemInstruction::WithdrawNonceAccount(lamports);
                let data = bincode::serialize(&withdraw_ix).ok()?;

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![nonce_account_idx, recipient_idx, nonce_authority_idx],
                    data,
                })
            }
            _ => {
                log::error!("Unsupported system instruction type: {}", instruction_type);
                None
            }
        }
    }

    /// Reconstruct SPL token program instructions from parsed format
    fn reconstruct_spl_token_instruction(
        parsed: &solana_transaction_status_client_types::ParsedInstruction,
        all_account_keys: &[Pubkey],
    ) -> Option<CompiledInstruction> {
        let program_id_index =
            all_account_keys.iter().position(|k| k.to_string() == parsed.program_id)? as u8;

        let parsed_data = &parsed.parsed;
        let instruction_type = parsed_data.get(PARSED_DATA_FIELD_TYPE)?.as_str()?;
        let info = parsed_data.get(PARSED_DATA_FIELD_INFO)?;

        match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let destination =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let authority =
                    info.get(PARSED_DATA_FIELD_AUTHORITY)?.as_str()?.parse::<Pubkey>().ok()?;
                let amount = info.get(PARSED_DATA_FIELD_AMOUNT)?.as_str()?.parse::<u64>().ok()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let destination_idx =
                    all_account_keys.iter().position(|k| k == &destination)? as u8;
                let authority_idx = all_account_keys.iter().position(|k| k == &authority)? as u8;

                let data = if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::Transfer { amount }.pack()
                } else {
                    #[allow(deprecated)]
                    spl_token_2022::instruction::TokenInstruction::Transfer { amount }.pack()
                };

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_TRANSFER_CHECKED => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let destination =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let authority =
                    info.get(PARSED_DATA_FIELD_AUTHORITY)?.as_str()?.parse::<Pubkey>().ok()?;
                let mint = info.get(PARSED_DATA_FIELD_MINT)?.as_str()?.parse::<Pubkey>().ok()?;

                let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT)?;
                let amount =
                    token_amount.get(PARSED_DATA_FIELD_AMOUNT)?.as_str()?.parse::<u64>().ok()?;
                let decimals = token_amount.get(PARSED_DATA_FIELD_DECIMALS)?.as_u64()? as u8;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let mint_idx = all_account_keys.iter().position(|k| k == &mint)? as u8;
                let destination_idx =
                    all_account_keys.iter().position(|k| k == &destination)? as u8;
                let authority_idx = all_account_keys.iter().position(|k| k == &authority)? as u8;

                let data = if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::TransferChecked { amount, decimals }
                        .pack()
                } else {
                    spl_token_2022::instruction::TokenInstruction::TransferChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                };

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, mint_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_BURN | PARSED_DATA_FIELD_BURN_CHECKED => {
                let account =
                    info.get(PARSED_DATA_FIELD_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let authority =
                    info.get(PARSED_DATA_FIELD_AUTHORITY)?.as_str()?.parse::<Pubkey>().ok()?;

                let (amount, decimals) = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT)?;
                    let amount = token_amount
                        .get(PARSED_DATA_FIELD_AMOUNT)?
                        .as_str()?
                        .parse::<u64>()
                        .ok()?;
                    let decimals = token_amount.get(PARSED_DATA_FIELD_DECIMALS)?.as_u64()? as u8;
                    (amount, Some(decimals))
                } else {
                    let amount =
                        info.get(PARSED_DATA_FIELD_AMOUNT)?.as_str()?.parse::<u64>().unwrap_or(0);
                    (amount, None)
                };

                let account_idx = all_account_keys.iter().position(|k| k == &account)? as u8;
                let authority_idx = all_account_keys.iter().position(|k| k == &authority)? as u8;

                let accounts = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let mint =
                        info.get(PARSED_DATA_FIELD_MINT)?.as_str()?.parse::<Pubkey>().ok()?;
                    let mint_idx = all_account_keys.iter().position(|k| k == &mint)? as u8;
                    vec![account_idx, mint_idx, authority_idx]
                } else {
                    vec![account_idx, authority_idx]
                };

                let data = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let decimals = decimals.unwrap(); // Safe because we set it above for burnChecked
                    if parsed.program_id == spl_token::ID.to_string() {
                        spl_token::instruction::TokenInstruction::BurnChecked { amount, decimals }
                            .pack()
                    } else {
                        spl_token_2022::instruction::TokenInstruction::BurnChecked {
                            amount,
                            decimals,
                        }
                        .pack()
                    }
                } else if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::Burn { amount }.pack()
                } else {
                    spl_token_2022::instruction::TokenInstruction::Burn { amount }.pack()
                };

                Some(CompiledInstruction { program_id_index, accounts, data })
            }
            PARSED_DATA_FIELD_CLOSE_ACCOUNT => {
                let account =
                    info.get(PARSED_DATA_FIELD_ACCOUNT)?.as_str()?.parse::<Pubkey>().ok()?;
                let destination =
                    info.get(PARSED_DATA_FIELD_DESTINATION)?.as_str()?.parse::<Pubkey>().ok()?;
                let authority =
                    info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;

                let account_idx = all_account_keys.iter().position(|k| k == &account)? as u8;
                let destination_idx =
                    all_account_keys.iter().position(|k| k == &destination)? as u8;
                let authority_idx = all_account_keys.iter().position(|k| k == &authority)? as u8;

                let data = if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::CloseAccount.pack()
                } else {
                    spl_token_2022::instruction::TokenInstruction::CloseAccount.pack()
                };

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_APPROVE => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let delegate =
                    info.get(PARSED_DATA_FIELD_DELEGATE)?.as_str()?.parse::<Pubkey>().ok()?;
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;
                let amount = info.get(PARSED_DATA_FIELD_AMOUNT)?.as_str()?.parse::<u64>().ok()?;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let delegate_idx = all_account_keys.iter().position(|k| k == &delegate)? as u8;
                let owner_idx = all_account_keys.iter().position(|k| k == &owner)? as u8;

                let data = if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::Approve { amount }.pack()
                } else {
                    spl_token_2022::instruction::TokenInstruction::Approve { amount }.pack()
                };

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, delegate_idx, owner_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_APPROVE_CHECKED => {
                let source =
                    info.get(PARSED_DATA_FIELD_SOURCE)?.as_str()?.parse::<Pubkey>().ok()?;
                let delegate =
                    info.get(PARSED_DATA_FIELD_DELEGATE)?.as_str()?.parse::<Pubkey>().ok()?;
                let owner = info.get(PARSED_DATA_FIELD_OWNER)?.as_str()?.parse::<Pubkey>().ok()?;
                let mint = info.get(PARSED_DATA_FIELD_MINT)?.as_str()?.parse::<Pubkey>().ok()?;

                let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT)?;
                let amount =
                    token_amount.get(PARSED_DATA_FIELD_AMOUNT)?.as_str()?.parse::<u64>().ok()?;
                let decimals = token_amount.get(PARSED_DATA_FIELD_DECIMALS)?.as_u64()? as u8;

                let source_idx = all_account_keys.iter().position(|k| k == &source)? as u8;
                let mint_idx = all_account_keys.iter().position(|k| k == &mint)? as u8;
                let delegate_idx = all_account_keys.iter().position(|k| k == &delegate)? as u8;
                let owner_idx = all_account_keys.iter().position(|k| k == &owner)? as u8;

                let data = if parsed.program_id == spl_token::ID.to_string() {
                    spl_token::instruction::TokenInstruction::ApproveChecked { amount, decimals }
                        .pack()
                } else {
                    spl_token_2022::instruction::TokenInstruction::ApproveChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                };

                Some(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, mint_idx, delegate_idx, owner_idx],
                    data,
                })
            }
            _ => {
                log::error!("Unsupported token instruction type: {}", instruction_type);
                None
            }
        }
    }

    pub fn parse_system_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedSystemInstructionType, Vec<ParsedSystemInstructionData>>, KoraError>
    {
        let mut parsed_instructions: HashMap<
            ParsedSystemInstructionType,
            Vec<ParsedSystemInstructionData>,
        > = HashMap::new();

        for instruction in transaction.all_instructions.iter() {
            let program_id = instruction.program_id;

            // Handle System Program transfers and account creation
            if program_id == SYSTEM_PROGRAM_ID {
                match bincode::deserialize::<SystemInstruction>(&instruction.data) {
                    // Account creation instructions - funding account pays lamports
                    Ok(SystemInstruction::CreateAccount { lamports, .. })
                    | Ok(SystemInstruction::CreateAccountWithSeed { lamports, .. }) => {
                        validate_number_accounts!(
                            instruction,
                            instruction_indexes::system_create_account::REQUIRED_NUMBER_OF_ACCOUNTS
                        );

                        let payer = instruction.accounts
                            [instruction_indexes::system_create_account::PAYER_INDEX]
                            .pubkey;

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemCreateAccount)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemCreateAccount {
                                lamports,
                                payer,
                            });
                    }
                    // Transfer instructions
                    Ok(SystemInstruction::Transfer { lamports }) => {
                        validate_number_accounts!(
                            instruction,
                            instruction_indexes::system_transfer::REQUIRED_NUMBER_OF_ACCOUNTS
                        );

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemTransfer)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemTransfer {
                                lamports,
                                sender: instruction.accounts
                                    [instruction_indexes::system_transfer::SENDER_INDEX]
                                    .pubkey,
                                receiver: instruction.accounts
                                    [instruction_indexes::system_transfer::RECEIVER_INDEX]
                                    .pubkey,
                            });
                    }
                    Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                        validate_number_accounts!(instruction, instruction_indexes::system_transfer_with_seed::REQUIRED_NUMBER_OF_ACCOUNTS);

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemTransfer)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemTransfer {
                                lamports,
                                sender: instruction.accounts[instruction_indexes::system_transfer_with_seed::SENDER_INDEX].pubkey,
                                receiver: instruction.accounts[instruction_indexes::system_transfer_with_seed::RECEIVER_INDEX].pubkey,
                            });
                    }
                    // Nonce account withdrawal
                    Ok(SystemInstruction::WithdrawNonceAccount(lamports)) => {
                        validate_number_accounts!(instruction, instruction_indexes::system_withdraw_nonce_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemWithdrawNonceAccount)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemWithdrawNonceAccount {
                                lamports,
                                nonce_authority: instruction.accounts[instruction_indexes::system_withdraw_nonce_account::NONCE_AUTHORITY_INDEX].pubkey,
                                recipient: instruction.accounts[instruction_indexes::system_withdraw_nonce_account::RECIPIENT_INDEX].pubkey,
                            });
                    }
                    Ok(SystemInstruction::Assign { .. }) => {
                        validate_number_accounts!(
                            instruction,
                            instruction_indexes::system_assign::REQUIRED_NUMBER_OF_ACCOUNTS
                        );

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemAssign)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemAssign {
                                authority: instruction.accounts
                                    [instruction_indexes::system_assign::AUTHORITY_INDEX]
                                    .pubkey,
                            });
                    }
                    Ok(SystemInstruction::AssignWithSeed { .. }) => {
                        validate_number_accounts!(instruction, instruction_indexes::system_assign_with_seed::REQUIRED_NUMBER_OF_ACCOUNTS);

                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemAssign)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemAssign {
                                authority: instruction.accounts
                                    [instruction_indexes::system_assign_with_seed::AUTHORITY_INDEX]
                                    .pubkey,
                            });
                    }
                    _ => {}
                }
            }
        }
        Ok(parsed_instructions)
    }

    pub fn parse_token_instructions(
        transaction: &VersionedTransactionResolved,
    ) -> Result<HashMap<ParsedSPLInstructionType, Vec<ParsedSPLInstructionData>>, KoraError> {
        let mut parsed_instructions: HashMap<
            ParsedSPLInstructionType,
            Vec<ParsedSPLInstructionData>,
        > = HashMap::new();

        for instruction in &transaction.all_instructions {
            let program_id = instruction.program_id;

            if program_id == spl_token::ID {
                if let Ok(spl_ix) =
                    spl_token::instruction::TokenInstruction::unpack(&instruction.data)
                {
                    match spl_ix {
                        spl_token::instruction::TokenInstruction::Transfer { amount } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_transfer::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenTransfer)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenTransfer {
                                    owner: instruction.accounts[instruction_indexes::spl_token_transfer::OWNER_INDEX].pubkey,
                                    amount,
                                    mint: None,
                                    source_address: instruction.accounts[instruction_indexes::spl_token_transfer::SOURCE_ADDRESS_INDEX].pubkey,
                                    destination_address: instruction.accounts[instruction_indexes::spl_token_transfer::DESTINATION_ADDRESS_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token::instruction::TokenInstruction::TransferChecked {
                            amount,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_transfer_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenTransfer)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenTransfer {
                                    owner: instruction.accounts[instruction_indexes::spl_token_transfer_checked::OWNER_INDEX].pubkey,
                                    amount,
                                    mint: Some(instruction.accounts[instruction_indexes::spl_token_transfer_checked::MINT_INDEX].pubkey),
                                    source_address: instruction.accounts[instruction_indexes::spl_token_transfer_checked::SOURCE_ADDRESS_INDEX].pubkey,
                                    destination_address: instruction.accounts[instruction_indexes::spl_token_transfer_checked::DESTINATION_ADDRESS_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token::instruction::TokenInstruction::Burn { .. }
                        | spl_token::instruction::TokenInstruction::BurnChecked { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_burn::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenBurn)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenBurn {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_burn::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token::instruction::TokenInstruction::CloseAccount { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_close_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenCloseAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenCloseAccount {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_close_account::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token::instruction::TokenInstruction::Approve { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_approve::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenApprove)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenApprove {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_approve::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token::instruction::TokenInstruction::ApproveChecked { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_approve_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenApprove)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenApprove {
                                    owner: instruction.accounts[instruction_indexes::spl_token_approve_checked::OWNER_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        _ => {}
                    };
                }
            } else if program_id == spl_token_2022::ID {
                if let Ok(spl_ix) =
                    spl_token_2022::instruction::TokenInstruction::unpack(&instruction.data)
                {
                    match spl_ix {
                        #[allow(deprecated)]
                        spl_token_2022::instruction::TokenInstruction::Transfer { amount } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_transfer::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenTransfer)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenTransfer {
                                    owner: instruction.accounts[instruction_indexes::spl_token_transfer::OWNER_INDEX].pubkey,
                                    amount,
                                    mint: None,
                                    source_address: instruction.accounts[instruction_indexes::spl_token_transfer::SOURCE_ADDRESS_INDEX].pubkey,
                                    destination_address: instruction.accounts[instruction_indexes::spl_token_transfer::DESTINATION_ADDRESS_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022::instruction::TokenInstruction::TransferChecked {
                            amount,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_transfer_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenTransfer)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenTransfer {
                                    owner: instruction.accounts[instruction_indexes::spl_token_transfer_checked::OWNER_INDEX].pubkey,
                                    amount,
                                    mint: Some(instruction.accounts[instruction_indexes::spl_token_transfer_checked::MINT_INDEX].pubkey),
                                    source_address: instruction.accounts[instruction_indexes::spl_token_transfer_checked::SOURCE_ADDRESS_INDEX].pubkey,
                                    destination_address: instruction.accounts[instruction_indexes::spl_token_transfer_checked::DESTINATION_ADDRESS_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022::instruction::TokenInstruction::Burn { .. }
                        | spl_token_2022::instruction::TokenInstruction::BurnChecked { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_burn::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenBurn)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenBurn {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_burn::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022::instruction::TokenInstruction::CloseAccount { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_close_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenCloseAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenCloseAccount {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_close_account::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022::instruction::TokenInstruction::Approve { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_approve::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenApprove)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenApprove {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_approve::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022::instruction::TokenInstruction::ApproveChecked {
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_approve_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenApprove)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenApprove {
                                    owner: instruction.accounts[instruction_indexes::spl_token_approve_checked::OWNER_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        _ => {}
                    };
                }
            }
        }
        Ok(parsed_instructions)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use solana_sdk::message::{AccountKeys, Message};
    use solana_transaction_status::parse_instruction;

    fn create_parsed_system_transfer(
        source: &Pubkey,
        destination: &Pubkey,
        lamports: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction =
            solana_sdk::system_instruction::transfer(source, destination, lamports);

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_transfer_with_seed(
        source: &Pubkey,
        destination: &Pubkey,
        lamports: u64,
        source_base: &Pubkey,
        seed: &str,
        source_owner: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = solana_sdk::system_instruction::transfer_with_seed(
            source,
            source_base,
            seed.to_string(),
            source_owner,
            destination,
            lamports,
        );

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_create_account(
        source: &Pubkey,
        new_account: &Pubkey,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = solana_sdk::system_instruction::create_account(
            source,
            new_account,
            lamports,
            space,
            owner,
        );

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_create_account_with_seed(
        source: &Pubkey,
        new_account: &Pubkey,
        base: &Pubkey,
        seed: &str,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = solana_sdk::system_instruction::create_account_with_seed(
            source,
            new_account,
            base,
            seed,
            lamports,
            space,
            owner,
        );

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_assign(
        account: &Pubkey,
        owner: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = solana_sdk::system_instruction::assign(account, owner);

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_assign_with_seed(
        account: &Pubkey,
        base: &Pubkey,
        seed: &str,
        owner: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction =
            solana_sdk::system_instruction::assign_with_seed(account, base, seed, owner);

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_system_withdraw_nonce_account(
        nonce_account: &Pubkey,
        nonce_authority: &Pubkey,
        recipient: &Pubkey,
        lamports: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = solana_sdk::system_instruction::withdraw_nonce_account(
            nonce_account,
            nonce_authority,
            recipient,
            lamports,
        );

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &SYSTEM_PROGRAM_ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_transfer(
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::transfer(
            &spl_token::ID,
            source,
            destination,
            authority,
            &[],
            amount,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_transfer_checked(
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::transfer_checked(
            &spl_token::ID,
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_burn(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction =
            spl_token::instruction::burn(&spl_token::ID, account, mint, authority, &[], amount)?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_burn_checked(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::burn_checked(
            &spl_token::ID,
            account,
            mint,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_close_account(
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::close_account(
            &spl_token::ID,
            account,
            destination,
            authority,
            &[],
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_approve(
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::approve(
            &spl_token::ID,
            source,
            delegate,
            authority,
            &[],
            amount,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_spl_token_approve_checked(
        source: &Pubkey,
        mint: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token::instruction::approve_checked(
            &spl_token::ID,
            source,
            mint,
            delegate,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_transfer(
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::transfer(
            &spl_token_2022::ID,
            source,
            destination,
            authority,
            &[],
            amount,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_transfer_checked(
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::ID,
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_burn(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::burn(
            &spl_token_2022::ID,
            account,
            mint,
            authority,
            &[],
            amount,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_burn_checked(
        account: &Pubkey,
        mint: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::burn_checked(
            &spl_token_2022::ID,
            account,
            mint,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_close_account(
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::close_account(
            &spl_token_2022::ID,
            account,
            destination,
            authority,
            &[],
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_approve(
        source: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::approve(
            &spl_token_2022::ID,
            source,
            delegate,
            authority,
            &[],
            amount,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    fn create_parsed_token2022_approve_checked(
        source: &Pubkey,
        mint: &Pubkey,
        delegate: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>>
    {
        let solana_instruction = spl_token_2022::instruction::approve_checked(
            &spl_token_2022::ID,
            source,
            mint,
            delegate,
            authority,
            &[],
            amount,
            decimals,
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
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

        let instructions = IxUtils::uncompile_instructions(&[compiled_ix], &account_keys);

        assert_eq!(instructions.len(), 1);
        let uncompiled = &instructions[0];
        assert_eq!(uncompiled.program_id, program_id);
        assert_eq!(uncompiled.accounts.len(), 2);
        assert_eq!(uncompiled.accounts[0].pubkey, account1);
        assert_eq!(uncompiled.accounts[1].pubkey, account2);
        assert_eq!(uncompiled.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_reconstruct_instruction_from_ui_compiled() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account_keys = vec![program_id, account1];

        let ui_compiled = solana_transaction_status_client_types::UiCompiledInstruction {
            program_id_index: 0,
            accounts: vec![1],
            data: bs58::encode(&[1, 2, 3]).into_string(),
            stack_height: None,
        };

        let result = IxUtils::reconstruct_instruction_from_ui(
            &UiInstruction::Compiled(ui_compiled),
            &account_keys,
        );

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1]);
        assert_eq!(compiled.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_reconstruct_partially_decoded_instruction() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();
        let account_keys = vec![program_id, account1, account2];

        let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
            program_id: program_id.to_string(),
            accounts: vec![account1.to_string(), account2.to_string()],
            data: bs58::encode(&[5, 6, 7]).into_string(),
            stack_height: None,
        };

        let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

        let result = IxUtils::reconstruct_instruction_from_ui(
            &UiInstruction::Parsed(ui_parsed),
            &account_keys,
        );

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2]); // account1, account2 indices
        assert_eq!(compiled.data, vec![5, 6, 7]);
    }

    #[test]
    fn test_reconstruct_system_transfer_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, source, destination];
        let lamports = 1000000u64;

        let transfer_instruction =
            solana_sdk::system_instruction::transfer(&source, &destination, lamports);

        let solana_parsed_transfer = create_parsed_system_transfer(&source, &destination, lamports)
            .expect("Failed to create authentic parsed instruction");

        let result =
            IxUtils::reconstruct_system_instruction(&solana_parsed_transfer, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2]); // source, destination indices
        assert_eq!(compiled.data, transfer_instruction.data);
    }

    #[test]
    fn test_reconstruct_system_transfer_with_seed_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let source_base = Pubkey::new_unique();
        let source_owner = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, source, source_base, destination];
        let lamports = 5000000u64;

        let instruction = solana_sdk::system_instruction::transfer_with_seed(
            &source,
            &source_base,
            "test_seed".to_string(),
            &source_owner,
            &destination,
            lamports,
        );

        let solana_parsed = create_parsed_system_transfer_with_seed(
            &source,
            &destination,
            lamports,
            &source_base,
            "test_seed",
            &source_owner,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, source_base, destination indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_system_create_account_instruction() {
        let source = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, source, new_account];
        let lamports = 2000000u64;
        let space = 165u64;

        let instruction = solana_sdk::system_instruction::create_account(
            &source,
            &new_account,
            lamports,
            space,
            &owner,
        );

        let solana_parsed =
            create_parsed_system_create_account(&source, &new_account, lamports, space, &owner)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2]); // source, new_account indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_system_create_account_with_seed_instruction() {
        let source = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let base = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, source, new_account, base];
        let lamports = 3000000u64;
        let space = 200u64;

        let instruction = solana_sdk::system_instruction::create_account_with_seed(
            &source,
            &new_account,
            &base,
            "test_seed_create",
            lamports,
            space,
            &owner,
        );

        let solana_parsed = create_parsed_system_create_account_with_seed(
            &source,
            &new_account,
            &base,
            "test_seed_create",
            lamports,
            space,
            &owner,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, new_account, base indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_system_assign_instruction() {
        let account = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, account];

        let instruction = solana_sdk::system_instruction::assign(&account, &owner);

        let solana_parsed = create_parsed_system_assign(&account, &owner)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1]); // account index
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_system_assign_with_seed_instruction() {
        let account = Pubkey::new_unique();
        let base = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, account, base];

        let instruction = solana_sdk::system_instruction::assign_with_seed(
            &account,
            &base,
            "test_assign_seed",
            &owner,
        );

        let solana_parsed =
            create_parsed_system_assign_with_seed(&account, &base, "test_assign_seed", &owner)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2]); // account, base indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_system_withdraw_nonce_account_instruction() {
        let nonce_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique();
        let system_program_id = SYSTEM_PROGRAM_ID;
        let account_keys = vec![system_program_id, nonce_account, recipient, nonce_authority];
        let lamports = 1500000u64;

        let instruction = solana_sdk::system_instruction::withdraw_nonce_account(
            &nonce_account,
            &nonce_authority,
            &recipient,
            lamports,
        );

        let solana_parsed = create_parsed_system_withdraw_nonce_account(
            &nonce_account,
            &nonce_authority,
            &recipient,
            lamports,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // nonce_account, recipient, nonce_authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_transfer_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, source, destination, authority];
        let amount = 1000000u64;

        let transfer_instruction = spl_token::instruction::transfer(
            &spl_token::ID,
            &source,
            &destination,
            &authority,
            &[],
            amount,
        )
        .expect("Failed to create transfer instruction");

        let solana_parsed_transfer =
            create_parsed_spl_token_transfer(&source, &destination, &authority, amount)
                .expect("Failed to create parsed instruction");

        let result =
            IxUtils::reconstruct_spl_token_instruction(&solana_parsed_transfer, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, destination, authority indices
        assert_eq!(compiled.data, transfer_instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_transfer_checked_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, source, mint, destination, authority];
        let amount = 2000000u64;
        let decimals = 6u8;

        let instruction = spl_token::instruction::transfer_checked(
            &spl_token::ID,
            &source,
            &mint,
            &destination,
            &authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create transfer_checked instruction");

        let solana_parsed = create_parsed_spl_token_transfer_checked(
            &source,
            &mint,
            &destination,
            &authority,
            amount,
            decimals,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, destination, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_burn_instruction() {
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 500000u64;

        let instruction =
            spl_token::instruction::burn(&spl_token::ID, &account, &mint, &authority, &[], amount)
                .expect("Failed to create burn instruction");

        let solana_parsed = create_parsed_spl_token_burn(&account, &mint, &authority, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 3]); // account, authority indices (mint at index 2 is skipped)
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_burn_checked_instruction() {
        let account = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 750000u64;
        let decimals = 6u8;

        let instruction = spl_token::instruction::burn_checked(
            &spl_token::ID,
            &account,
            &mint,
            &authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create burn_checked instruction");

        let solana_parsed =
            create_parsed_spl_token_burn_checked(&account, &mint, &authority, amount, decimals)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_close_account_instruction() {
        let account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, account, destination, authority];

        let instruction = spl_token::instruction::close_account(
            &spl_token::ID,
            &account,
            &destination,
            &authority,
            &[],
        )
        .expect("Failed to create close_account instruction");

        let solana_parsed =
            create_parsed_spl_token_close_account(&account, &destination, &authority)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, destination, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_approve_instruction() {
        let source = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, source, delegate, owner];
        let amount = 1000000u64;

        let instruction = spl_token::instruction::approve(
            &spl_token::ID,
            &source,
            &delegate,
            &owner,
            &[],
            amount,
        )
        .expect("Failed to create approve instruction");

        let solana_parsed = create_parsed_spl_token_approve(&source, &delegate, &owner, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, delegate, owner indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_spl_token_approve_checked_instruction() {
        let source = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token::ID;
        let account_keys = vec![token_program_id, source, mint, delegate, owner];
        let amount = 2500000u64;
        let decimals = 6u8;

        let instruction = spl_token::instruction::approve_checked(
            &spl_token::ID,
            &source,
            &mint,
            &delegate,
            &owner,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create approve_checked instruction");

        let solana_parsed = create_parsed_spl_token_approve_checked(
            &source, &mint, &delegate, &owner, amount, decimals,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, delegate, owner indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_transfer_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, source, destination, authority];
        let amount = 1500000u64;

        let instruction = spl_token_2022::instruction::transfer(
            &spl_token_2022::ID,
            &source,
            &destination,
            &authority,
            &[],
            amount,
        )
        .expect("Failed to create transfer instruction");

        let solana_parsed =
            create_parsed_token2022_transfer(&source, &destination, &authority, amount)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, destination, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_transfer_checked_instruction() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, source, mint, destination, authority];
        let amount = 3000000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022::instruction::transfer_checked(
            &spl_token_2022::ID,
            &source,
            &mint,
            &destination,
            &authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create transfer_checked instruction");

        let solana_parsed = create_parsed_token2022_transfer_checked(
            &source,
            &mint,
            &destination,
            &authority,
            amount,
            decimals,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, destination, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_burn_instruction() {
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 800000u64;

        let instruction = spl_token_2022::instruction::burn(
            &spl_token_2022::ID,
            &account,
            &mint,
            &authority,
            &[],
            amount,
        )
        .expect("Failed to create burn instruction");

        let solana_parsed = create_parsed_token2022_burn(&account, &mint, &authority, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 3]); // account, authority indices (mint at index 2 is skipped)
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_burn_checked_instruction() {
        let account = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 900000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022::instruction::burn_checked(
            &spl_token_2022::ID,
            &account,
            &mint,
            &authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create burn_checked instruction");

        let solana_parsed =
            create_parsed_token2022_burn_checked(&account, &mint, &authority, amount, decimals)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_close_account_instruction() {
        let account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, account, destination, authority];

        let instruction = spl_token_2022::instruction::close_account(
            &spl_token_2022::ID,
            &account,
            &destination,
            &authority,
            &[],
        )
        .expect("Failed to create close_account instruction");

        let solana_parsed =
            create_parsed_token2022_close_account(&account, &destination, &authority)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, destination, authority indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_approve_instruction() {
        let source = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, source, delegate, owner];
        let amount = 1200000u64;

        let instruction = spl_token_2022::instruction::approve(
            &spl_token_2022::ID,
            &source,
            &delegate,
            &owner,
            &[],
            amount,
        )
        .expect("Failed to create approve instruction");

        let solana_parsed = create_parsed_token2022_approve(&source, &delegate, &owner, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, delegate, owner indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_token2022_approve_checked_instruction() {
        let source = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program_id = spl_token_2022::ID;
        let account_keys = vec![token_program_id, source, mint, delegate, owner];
        let amount = 3500000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022::instruction::approve_checked(
            &spl_token_2022::ID,
            &source,
            &mint,
            &delegate,
            &owner,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create approve_checked instruction");

        let solana_parsed = create_parsed_token2022_approve_checked(
            &source, &mint, &delegate, &owner, amount, decimals,
        )
        .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(&solana_parsed, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, delegate, owner indices
        assert_eq!(compiled.data, instruction.data);
    }
}
