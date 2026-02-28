use std::collections::HashMap;

use solana_message::compiled_instruction::CompiledInstruction;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};
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
    SystemAllocate,
    SystemInitializeNonceAccount,
    SystemAdvanceNonceAccount,
    SystemAuthorizeNonceAccount,
    // Note: SystemUpgradeNonceAccount not included - no authority parameter
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
    // Includes allocate and allocate with seed
    SystemAllocate { account: Pubkey },
    // Initialize nonce account
    SystemInitializeNonceAccount { nonce_account: Pubkey, nonce_authority: Pubkey },
    // Advance nonce account
    SystemAdvanceNonceAccount { nonce_account: Pubkey, nonce_authority: Pubkey },
    // Authorize nonce account
    SystemAuthorizeNonceAccount { nonce_account: Pubkey, nonce_authority: Pubkey },
    // Note: SystemUpgradeNonceAccount not included - no authority parameter
}

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
    // Revoke
    SplTokenRevoke {
        owner: Pubkey,
        is_2022: bool,
    },
    // SetAuthority
    SplTokenSetAuthority {
        authority: Pubkey,
        is_2022: bool,
    },
    // MintTo and MintToChecked
    SplTokenMintTo {
        mint_authority: Pubkey,
        is_2022: bool,
    },
    // InitializeMint and InitializeMint2
    SplTokenInitializeMint {
        mint_authority: Pubkey,
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
    // FreezeAccount
    SplTokenFreezeAccount {
        freeze_authority: Pubkey,
        is_2022: bool,
    },
    // ThawAccount
    SplTokenThawAccount {
        freeze_authority: Pubkey,
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

/// Macro to parse system instructions with validation and account extraction
/// Usage: parse_system_instruction!(parsed_instructions, instruction, validate_module, EnumVariant, DataVariant { fields })
macro_rules! parse_system_instruction {
    // Simple version: separate constant module path and enum variant names
    ($parsed:ident, $ix:ident, $const_mod:ident, $enum_variant:ident, $data_variant:ident { $($field:ident: $account_path:expr),* $(,)? }) => {
        validate_number_accounts!($ix, instruction_indexes::$const_mod::REQUIRED_NUMBER_OF_ACCOUNTS);
        $parsed
            .entry(ParsedSystemInstructionType::$enum_variant)
            .or_default()
            .push(ParsedSystemInstructionData::$data_variant {
                $($field: $ix.accounts[$account_path].pubkey,)*
            });
    };
    // Version with extra fields (like lamports) that come from instruction data
    ($parsed:ident, $ix:ident, $const_mod:ident, $enum_variant:ident, $data_variant:ident { $($data_field:ident: $data_val:expr),* ; $($field:ident: $account_path:expr),* $(,)? }) => {
        validate_number_accounts!($ix, instruction_indexes::$const_mod::REQUIRED_NUMBER_OF_ACCOUNTS);
        $parsed
            .entry(ParsedSystemInstructionType::$enum_variant)
            .or_default()
            .push(ParsedSystemInstructionData::$data_variant {
                $($data_field: $data_val,)*
                $($field: $ix.accounts[$account_path].pubkey,)*
            });
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
pub const PARSED_DATA_FIELD_ALLOCATE: &str = "allocate";
pub const PARSED_DATA_FIELD_ALLOCATE_WITH_SEED: &str = "allocateWithSeed";
pub const PARSED_DATA_FIELD_INITIALIZE_NONCE_ACCOUNT: &str = "initializeNonce";
pub const PARSED_DATA_FIELD_ADVANCE_NONCE_ACCOUNT: &str = "advanceNonce";
pub const PARSED_DATA_FIELD_AUTHORIZE_NONCE_ACCOUNT: &str = "authorizeNonce";
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
pub const PARSED_DATA_FIELD_NEW_AUTHORITY: &str = "newAuthority";

// SPL Token instruction type constants
pub const PARSED_DATA_FIELD_REVOKE: &str = "revoke";
pub const PARSED_DATA_FIELD_SET_AUTHORITY: &str = "setAuthority";
pub const PARSED_DATA_FIELD_MINT_TO: &str = "mintTo";
pub const PARSED_DATA_FIELD_MINT_TO_CHECKED: &str = "mintToChecked";
pub const PARSED_DATA_FIELD_INITIALIZE_MINT: &str = "initializeMint";
pub const PARSED_DATA_FIELD_INITIALIZE_MINT2: &str = "initializeMint2";
pub const PARSED_DATA_FIELD_INITIALIZE_ACCOUNT: &str = "initializeAccount";
pub const PARSED_DATA_FIELD_INITIALIZE_ACCOUNT2: &str = "initializeAccount2";
pub const PARSED_DATA_FIELD_INITIALIZE_ACCOUNT3: &str = "initializeAccount3";
pub const PARSED_DATA_FIELD_INITIALIZE_MULTISIG: &str = "initializeMultisig";
pub const PARSED_DATA_FIELD_INITIALIZE_MULTISIG2: &str = "initializeMultisig2";
pub const PARSED_DATA_FIELD_FREEZE_ACCOUNT: &str = "freezeAccount";
pub const PARSED_DATA_FIELD_THAW_ACCOUNT: &str = "thawAccount";

// Additional field names for new instructions
pub const PARSED_DATA_FIELD_MINT_AUTHORITY: &str = "mintAuthority";
pub const PARSED_DATA_FIELD_FREEZE_AUTHORITY: &str = "freezeAuthority";
pub const PARSED_DATA_FIELD_AUTHORITY_TYPE: &str = "authorityType";
pub const PARSED_DATA_FIELD_MULTISIG_ACCOUNT: &str = "multisig";
pub const PARSED_DATA_FIELD_SIGNERS: &str = "signers";

impl IxUtils {
    /// Helper method to extract a field as a string from JSON with proper error handling
    fn get_field_as_str<'a>(
        info: &'a serde_json::Value,
        field_name: &str,
    ) -> Result<&'a str, KoraError> {
        info.get(field_name)
            .ok_or_else(|| {
                KoraError::SerializationError(format!("Missing field '{}'", field_name))
            })?
            .as_str()
            .ok_or_else(|| {
                KoraError::SerializationError(format!("Field '{}' is not a string", field_name))
            })
    }

    /// Helper method to extract a field as a Pubkey from JSON with proper error handling
    fn get_field_as_pubkey(
        info: &serde_json::Value,
        field_name: &str,
    ) -> Result<Pubkey, KoraError> {
        let pubkey_str = Self::get_field_as_str(info, field_name)?;
        pubkey_str.parse::<Pubkey>().map_err(|e| {
            KoraError::SerializationError(format!(
                "Field '{}' is not a valid pubkey: {}",
                field_name, e
            ))
        })
    }

    /// Helper method to extract a field as u64 from JSON string with proper error handling
    fn get_field_as_u64(info: &serde_json::Value, field_name: &str) -> Result<u64, KoraError> {
        let value = info.get(field_name).ok_or_else(|| {
            KoraError::SerializationError(format!("Missing field '{}'", field_name))
        })?;

        // Try as native JSON number first
        if let Some(num) = value.as_u64() {
            return Ok(num);
        }

        // Fall back to string parsing
        if let Some(str_val) = value.as_str() {
            return str_val.parse::<u64>().map_err(|e| {
                KoraError::SerializationError(format!(
                    "Field '{}' is not a valid u64: {}",
                    field_name, e
                ))
            });
        }

        Err(KoraError::SerializationError(format!(
            "Field '{}' is neither a number nor a string",
            field_name
        )))
    }

    /// Helper method to get account index from hashmap with proper error handling
    fn get_account_index(
        account_keys_hashmap: &HashMap<Pubkey, u8>,
        pubkey: &Pubkey,
    ) -> Result<u8, KoraError> {
        account_keys_hashmap.get(pubkey).copied().ok_or_else(|| {
            KoraError::SerializationError(format!("{} not found in account keys", pubkey))
        })
    }

    pub fn build_account_keys_hashmap(account_keys: &[Pubkey]) -> HashMap<Pubkey, u8> {
        account_keys.iter().enumerate().map(|(idx, key)| (*key, idx as u8)).collect()
    }

    pub fn get_account_key_if_present(ix: &Instruction, index: usize) -> Option<Pubkey> {
        if ix.accounts.is_empty() {
            return None;
        }

        if index >= ix.accounts.len() {
            return None;
        }

        Some(ix.accounts[index].pubkey)
    }

    pub fn get_account_key_required(
        account_keys: &[Pubkey],
        index: usize,
    ) -> Result<Pubkey, KoraError> {
        account_keys.get(index).copied().ok_or_else(|| {
            KoraError::SerializationError(format!("Account key at index {} not found", index))
        })
    }

    pub fn build_default_compiled_instruction(program_id_index: u8) -> CompiledInstruction {
        CompiledInstruction { program_id_index, accounts: vec![], data: vec![] }
    }

    pub fn uncompile_instructions(
        instructions: &[CompiledInstruction],
        account_keys: &[Pubkey],
    ) -> Result<Vec<Instruction>, KoraError> {
        instructions
            .iter()
            .map(|ix| {
                let program_id =
                    Self::get_account_key_required(account_keys, ix.program_id_index as usize)?;
                let accounts: Result<Vec<AccountMeta>, KoraError> = ix
                    .accounts
                    .iter()
                    .map(|idx| {
                        Ok(AccountMeta {
                            pubkey: Self::get_account_key_required(account_keys, *idx as usize)?,
                            is_signer: false,
                            is_writable: true,
                        })
                    })
                    .collect();

                Ok(Instruction { program_id, accounts: accounts?, data: ix.data.clone() })
            })
            .collect()
    }

    /// Reconstruct a CompiledInstruction from various UiInstruction formats
    ///
    /// This is required because when you simulate a transaction with inner instructions flag,
    /// the RPC pre-parses some of the instructions (like for SPL program and System Program),
    /// however this is an issue for Kora, as we expected "Compiled" instructions rather than "Parsed" instructions,
    /// because we have our own parsing logic on our Kora's side.
    ///
    /// So we need to reconstruct the "Compiled" instructions from the "Parsed" instructions, by "unparsing" the "Parsed" instructions.
    ///
    /// There's no known way to force the RPC to not parsed the instructions, so we need this "hack" to reverse the process.
    ///
    /// Example: https://github.com/anza-xyz/agave/blob/68032b576dc4c14b31c15974c6734ae1513980a3/transaction-status/src/parse_system.rs#L11
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
                    let account_keys_hashmap = Self::build_account_keys_hashmap(all_account_keys);
                    // Reconstruct based on program type
                    if parsed.program_id == SYSTEM_PROGRAM_ID.to_string() {
                        Self::reconstruct_system_instruction(parsed, &account_keys_hashmap).ok()
                    } else if parsed.program_id == spl_token_interface::ID.to_string()
                        || parsed.program_id == spl_token_2022_interface::ID.to_string()
                    {
                        Self::reconstruct_spl_token_instruction(parsed, &account_keys_hashmap).ok()
                    } else {
                        // For unsupported programs, create a stub instruction with just the program ID
                        // This ensures the program ID is preserved for security validation
                        let program_id = parsed.program_id.parse::<Pubkey>().ok()?;
                        let program_id_index = *account_keys_hashmap.get(&program_id)?;

                        Some(Self::build_default_compiled_instruction(program_id_index))
                    }
                }
                UiParsedInstruction::PartiallyDecoded(partial) => {
                    let account_keys_hashmap = Self::build_account_keys_hashmap(all_account_keys);
                    if let Ok(program_id) = partial.program_id.parse::<Pubkey>() {
                        if let Some(program_idx) = account_keys_hashmap.get(&program_id) {
                            // Convert account addresses to indices
                            let account_indices: Vec<u8> = partial
                                .accounts
                                .iter()
                                .filter_map(|addr_str| {
                                    addr_str
                                        .parse::<Pubkey>()
                                        .ok()
                                        .and_then(|pubkey| account_keys_hashmap.get(&pubkey))
                                        .copied()
                                })
                                .collect();

                            return Some(CompiledInstruction {
                                program_id_index: *program_idx,
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
        account_keys_hashmap: &HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        let program_id_index = Self::get_account_index(account_keys_hashmap, &SYSTEM_PROGRAM_ID)?;

        let parsed_data = &parsed.parsed;
        let instruction_type = Self::get_field_as_str(parsed_data, PARSED_DATA_FIELD_TYPE)?;
        let info = parsed_data
            .get(PARSED_DATA_FIELD_INFO)
            .ok_or_else(|| KoraError::SerializationError("Missing 'info' field".to_string()))?;

        match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let destination = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let lamports = Self::get_field_as_u64(info, PARSED_DATA_FIELD_LAMPORTS)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let destination_idx = Self::get_account_index(account_keys_hashmap, &destination)?;

                let transfer_ix = SystemInstruction::Transfer { lamports };
                let data = bincode::serialize(&transfer_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize Transfer instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, destination_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_CREATE_ACCOUNT => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let new_account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NEW_ACCOUNT)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;
                let lamports = Self::get_field_as_u64(info, PARSED_DATA_FIELD_LAMPORTS)?;
                let space = Self::get_field_as_u64(info, PARSED_DATA_FIELD_SPACE)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let new_account_idx = Self::get_account_index(account_keys_hashmap, &new_account)?;

                let create_ix = SystemInstruction::CreateAccount { lamports, space, owner };
                let data = bincode::serialize(&create_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize CreateAccount instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, new_account_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ASSIGN => {
                let authority = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let authority_idx = Self::get_account_index(account_keys_hashmap, &authority)?;

                let assign_ix = SystemInstruction::Assign { owner };
                let data = bincode::serialize(&assign_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize Assign instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction { program_id_index, accounts: vec![authority_idx], data })
            }
            PARSED_DATA_FIELD_TRANSFER_WITH_SEED => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let destination = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let lamports = Self::get_field_as_u64(info, PARSED_DATA_FIELD_LAMPORTS)?;
                let source_base = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE_BASE)?;
                let source_seed =
                    Self::get_field_as_str(info, PARSED_DATA_FIELD_SOURCE_SEED)?.to_string();
                let source_owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE_OWNER)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let destination_idx = Self::get_account_index(account_keys_hashmap, &destination)?;
                let source_base_idx = Self::get_account_index(account_keys_hashmap, &source_base)?;

                let transfer_ix = SystemInstruction::TransferWithSeed {
                    lamports,
                    from_seed: source_seed,
                    from_owner: source_owner,
                };
                let data = bincode::serialize(&transfer_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize TransferWithSeed instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, source_base_idx, destination_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_CREATE_ACCOUNT_WITH_SEED => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let new_account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NEW_ACCOUNT)?;
                let base = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_BASE)?;
                let seed = Self::get_field_as_str(info, PARSED_DATA_FIELD_SEED)?.to_string();
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;
                let lamports = Self::get_field_as_u64(info, PARSED_DATA_FIELD_LAMPORTS)?;
                let space = Self::get_field_as_u64(info, PARSED_DATA_FIELD_SPACE)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let new_account_idx = Self::get_account_index(account_keys_hashmap, &new_account)?;
                let base_idx = Self::get_account_index(account_keys_hashmap, &base)?;

                let create_ix =
                    SystemInstruction::CreateAccountWithSeed { base, seed, lamports, space, owner };
                let data = bincode::serialize(&create_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize CreateAccountWithSeed instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, new_account_idx, base_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ASSIGN_WITH_SEED => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let base = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_BASE)?;
                let seed = Self::get_field_as_str(info, PARSED_DATA_FIELD_SEED)?.to_string();
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let base_idx = Self::get_account_index(account_keys_hashmap, &base)?;

                let assign_ix = SystemInstruction::AssignWithSeed { base, seed, owner };
                let data = bincode::serialize(&assign_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize AssignWithSeed instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, base_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_WITHDRAW_NONCE_ACCOUNT => {
                let nonce_account =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_ACCOUNT)?;
                let recipient = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let nonce_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_AUTHORITY)?;
                let lamports = Self::get_field_as_u64(info, PARSED_DATA_FIELD_LAMPORTS)?;

                let nonce_account_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_account)?;
                let recipient_idx = Self::get_account_index(account_keys_hashmap, &recipient)?;
                let nonce_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_authority)?;

                let withdraw_ix = SystemInstruction::WithdrawNonceAccount(lamports);
                let data = bincode::serialize(&withdraw_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize WithdrawNonceAccount instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![nonce_account_idx, recipient_idx, nonce_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ALLOCATE => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let space = Self::get_field_as_u64(info, PARSED_DATA_FIELD_SPACE)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;

                let allocate_ix = SystemInstruction::Allocate { space };
                let data = bincode::serialize(&allocate_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize Allocate instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction { program_id_index, accounts: vec![account_idx], data })
            }
            PARSED_DATA_FIELD_ALLOCATE_WITH_SEED => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let base = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_BASE)?;
                let seed = Self::get_field_as_str(info, PARSED_DATA_FIELD_SEED)?.to_string();
                let space = Self::get_field_as_u64(info, PARSED_DATA_FIELD_SPACE)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let base_idx = Self::get_account_index(account_keys_hashmap, &base)?;

                let allocate_ix = SystemInstruction::AllocateWithSeed { base, seed, space, owner };
                let data = bincode::serialize(&allocate_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize AllocateWithSeed instruction: {}",
                        e
                    ))
                })?;

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, base_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_INITIALIZE_NONCE_ACCOUNT => {
                let nonce_account =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_ACCOUNT)?;
                let nonce_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_AUTHORITY)?;

                let nonce_account_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_account)?;

                let initialize_ix = SystemInstruction::InitializeNonceAccount(nonce_authority);
                let data = bincode::serialize(&initialize_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize InitializeNonceAccount instruction: {}",
                        e
                    ))
                })?;

                // Accounts: [nonce_account, recent_blockhashes_sysvar, rent_sysvar]
                // We only have nonce_account in the hashmap for inner instructions
                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![nonce_account_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_ADVANCE_NONCE_ACCOUNT => {
                let nonce_account =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_ACCOUNT)?;
                let nonce_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_AUTHORITY)?;

                let nonce_account_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_account)?;
                let nonce_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_authority)?;

                let advance_ix = SystemInstruction::AdvanceNonceAccount;
                let data = bincode::serialize(&advance_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize AdvanceNonceAccount instruction: {}",
                        e
                    ))
                })?;

                // Accounts: [nonce_account, recent_blockhashes_sysvar, nonce_authority]
                // We only include accounts that are in the hashmap (from inner instructions)
                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![nonce_account_idx, nonce_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_AUTHORIZE_NONCE_ACCOUNT => {
                let nonce_account =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_ACCOUNT)?;
                let nonce_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NONCE_AUTHORITY)?;
                let new_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_NEW_AUTHORITY)?;

                let nonce_account_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_account)?;
                let nonce_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &nonce_authority)?;

                let authorize_ix = SystemInstruction::AuthorizeNonceAccount(new_authority);
                let data = bincode::serialize(&authorize_ix).map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Failed to serialize AuthorizeNonceAccount instruction: {}",
                        e
                    ))
                })?;

                // Accounts: [nonce_account, nonce_authority]
                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![nonce_account_idx, nonce_authority_idx],
                    data,
                })
            }
            _ => {
                log::error!("Unsupported system instruction type: {}", instruction_type);
                Ok(Self::build_default_compiled_instruction(program_id_index))
            }
        }
    }

    /// Reconstruct SPL token program instructions from parsed format
    fn reconstruct_spl_token_instruction(
        parsed: &solana_transaction_status_client_types::ParsedInstruction,
        account_keys_hashmap: &HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        let program_id = parsed
            .program_id
            .parse::<Pubkey>()
            .map_err(|e| KoraError::SerializationError(format!("Invalid program ID: {}", e)))?;
        let program_id_index = Self::get_account_index(account_keys_hashmap, &program_id)?;

        let parsed_data = &parsed.parsed;
        let instruction_type = Self::get_field_as_str(parsed_data, PARSED_DATA_FIELD_TYPE)?;
        let info = parsed_data
            .get(PARSED_DATA_FIELD_INFO)
            .ok_or_else(|| KoraError::SerializationError("Missing 'info' field".to_string()))?;

        match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let destination = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let authority = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_AUTHORITY)?;
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let destination_idx = Self::get_account_index(account_keys_hashmap, &destination)?;
                let authority_idx = Self::get_account_index(account_keys_hashmap, &authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::Transfer { amount }.pack()
                } else {
                    #[allow(deprecated)]
                    spl_token_2022_interface::instruction::TokenInstruction::Transfer { amount }
                        .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_TRANSFER_CHECKED => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let destination = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let authority = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_AUTHORITY)?;
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;

                let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT).ok_or_else(|| {
                    KoraError::SerializationError("Missing 'tokenAmount' field".to_string())
                })?;
                let amount = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_AMOUNT)?;
                let decimals =
                    Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_DECIMALS)? as u8;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let destination_idx = Self::get_account_index(account_keys_hashmap, &destination)?;
                let authority_idx = Self::get_account_index(account_keys_hashmap, &authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::TransferChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::TransferChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, mint_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_BURN | PARSED_DATA_FIELD_BURN_CHECKED => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let authority = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_AUTHORITY)?;

                let (amount, decimals) = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let token_amount =
                        info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT).ok_or_else(|| {
                            KoraError::SerializationError(
                                "Missing 'tokenAmount' field for burnChecked".to_string(),
                            )
                        })?;
                    let amount = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_AMOUNT)?;
                    let decimals =
                        Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_DECIMALS)? as u8;
                    (amount, Some(decimals))
                } else {
                    let amount =
                        Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT).unwrap_or(0);
                    (amount, None)
                };

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let authority_idx = Self::get_account_index(account_keys_hashmap, &authority)?;

                let accounts = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                    let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                    vec![account_idx, mint_idx, authority_idx]
                } else {
                    vec![account_idx, authority_idx]
                };

                let data = if instruction_type == PARSED_DATA_FIELD_BURN_CHECKED {
                    let decimals = decimals.unwrap(); // Safe because we set it above for burnChecked
                    if parsed.program_id == spl_token_interface::ID.to_string() {
                        spl_token_interface::instruction::TokenInstruction::BurnChecked {
                            amount,
                            decimals,
                        }
                        .pack()
                    } else {
                        spl_token_2022_interface::instruction::TokenInstruction::BurnChecked {
                            amount,
                            decimals,
                        }
                        .pack()
                    }
                } else if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::Burn { amount }.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::Burn { amount }.pack()
                };

                Ok(CompiledInstruction { program_id_index, accounts, data })
            }
            PARSED_DATA_FIELD_CLOSE_ACCOUNT => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let destination = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DESTINATION)?;
                let authority = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let destination_idx = Self::get_account_index(account_keys_hashmap, &destination)?;
                let authority_idx = Self::get_account_index(account_keys_hashmap, &authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::CloseAccount.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::CloseAccount.pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, destination_idx, authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_APPROVE => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let delegate = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DELEGATE)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let delegate_idx = Self::get_account_index(account_keys_hashmap, &delegate)?;
                let owner_idx = Self::get_account_index(account_keys_hashmap, &owner)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::Approve { amount }.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::Approve { amount }
                        .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, delegate_idx, owner_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_APPROVE_CHECKED => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let delegate = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_DELEGATE)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;

                let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT).ok_or_else(|| {
                    KoraError::SerializationError("Missing 'tokenAmount' field".to_string())
                })?;
                let amount = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_AMOUNT)?;
                let decimals =
                    Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_DECIMALS)? as u8;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let delegate_idx = Self::get_account_index(account_keys_hashmap, &delegate)?;
                let owner_idx = Self::get_account_index(account_keys_hashmap, &owner)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::ApproveChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::ApproveChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, mint_idx, delegate_idx, owner_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_REVOKE => {
                let source = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_SOURCE)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let source_idx = Self::get_account_index(account_keys_hashmap, &source)?;
                let owner_idx = Self::get_account_index(account_keys_hashmap, &owner)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::Revoke.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::Revoke.pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![source_idx, owner_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_SET_AUTHORITY => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let current_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_AUTHORITY)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let current_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &current_authority)?;

                // SetAuthority has variable data - we reconstruct minimal version
                // Real validation happens when checking if fee payer is the authority
                let data = vec![6];

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, current_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_MINT_TO => {
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let mint_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT_AUTHORITY)?;
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;

                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let mint_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &mint_authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::MintTo { amount }.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::MintTo { amount }
                        .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![mint_idx, account_idx, mint_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_MINT_TO_CHECKED => {
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let mint_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT_AUTHORITY)?;

                let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT).ok_or_else(|| {
                    KoraError::SerializationError("Missing 'tokenAmount' field".to_string())
                })?;
                let amount = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_AMOUNT)?;
                let decimals =
                    Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_DECIMALS)? as u8;

                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let mint_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &mint_authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::MintToChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::MintToChecked {
                        amount,
                        decimals,
                    }
                    .pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![mint_idx, account_idx, mint_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_INITIALIZE_MINT | PARSED_DATA_FIELD_INITIALIZE_MINT2 => {
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                // mint_authority is in instruction data, not used for reconstruction
                let _mint_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT_AUTHORITY)?;

                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;

                // InitializeMint has discriminator only, authority is in data
                let data = if instruction_type == PARSED_DATA_FIELD_INITIALIZE_MINT {
                    vec![0] // InitializeMint discriminator
                } else {
                    vec![20] // InitializeMint2 discriminator
                };

                Ok(CompiledInstruction { program_id_index, accounts: vec![mint_idx], data })
            }
            PARSED_DATA_FIELD_INITIALIZE_ACCOUNT
            | PARSED_DATA_FIELD_INITIALIZE_ACCOUNT2
            | PARSED_DATA_FIELD_INITIALIZE_ACCOUNT3 => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                let owner = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_OWNER)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;

                // Different variants have different account structures and discriminators
                let (data, accounts) = match instruction_type {
                    PARSED_DATA_FIELD_INITIALIZE_ACCOUNT => {
                        // InitializeAccount: [account, mint, owner, rent]
                        // Owner is in accounts, not data
                        let owner_idx = Self::get_account_index(account_keys_hashmap, &owner)?;
                        (vec![1], vec![account_idx, mint_idx, owner_idx])
                    }
                    PARSED_DATA_FIELD_INITIALIZE_ACCOUNT2 => {
                        // InitializeAccount2: [account, mint, rent], owner in data
                        (vec![16], vec![account_idx, mint_idx])
                    }
                    PARSED_DATA_FIELD_INITIALIZE_ACCOUNT3 => {
                        // InitializeAccount3: [account, mint], owner in data
                        (vec![18], vec![account_idx, mint_idx])
                    }
                    _ => unreachable!(),
                };

                Ok(CompiledInstruction { program_id_index, accounts, data })
            }
            PARSED_DATA_FIELD_INITIALIZE_MULTISIG | PARSED_DATA_FIELD_INITIALIZE_MULTISIG2 => {
                let multisig = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MULTISIG_ACCOUNT)?;
                let multisig_idx = Self::get_account_index(account_keys_hashmap, &multisig)?;

                // Extract signer pubkeys from signers array (not currently used for reconstruction)
                let _signers_value = info.get(PARSED_DATA_FIELD_SIGNERS).ok_or_else(|| {
                    KoraError::SerializationError("Missing 'signers' field".to_string())
                })?;

                // Discriminator based on instruction variant
                let data = if instruction_type == PARSED_DATA_FIELD_INITIALIZE_MULTISIG {
                    vec![2] // InitializeMultisig discriminator
                } else {
                    vec![19] // InitializeMultisig2 discriminator
                };

                Ok(CompiledInstruction { program_id_index, accounts: vec![multisig_idx], data })
            }
            PARSED_DATA_FIELD_FREEZE_ACCOUNT => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                let freeze_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_FREEZE_AUTHORITY)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let freeze_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &freeze_authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::FreezeAccount.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::FreezeAccount.pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, mint_idx, freeze_authority_idx],
                    data,
                })
            }
            PARSED_DATA_FIELD_THAW_ACCOUNT => {
                let account = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_ACCOUNT)?;
                let mint = Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_MINT)?;
                let freeze_authority =
                    Self::get_field_as_pubkey(info, PARSED_DATA_FIELD_FREEZE_AUTHORITY)?;

                let account_idx = Self::get_account_index(account_keys_hashmap, &account)?;
                let mint_idx = Self::get_account_index(account_keys_hashmap, &mint)?;
                let freeze_authority_idx =
                    Self::get_account_index(account_keys_hashmap, &freeze_authority)?;

                let data = if parsed.program_id == spl_token_interface::ID.to_string() {
                    spl_token_interface::instruction::TokenInstruction::ThawAccount.pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::ThawAccount.pack()
                };

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: vec![account_idx, mint_idx, freeze_authority_idx],
                    data,
                })
            }
            _ => {
                log::error!("Unsupported token instruction type: {}", instruction_type);
                Ok(Self::build_default_compiled_instruction(program_id_index))
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
                    Ok(SystemInstruction::CreateAccount { lamports, .. })
                    | Ok(SystemInstruction::CreateAccountWithSeed { lamports, .. }) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_create_account, SystemCreateAccount, SystemCreateAccount {
                            lamports: lamports;
                            payer: instruction_indexes::system_create_account::PAYER_INDEX
                        });
                    }
                    Ok(SystemInstruction::Transfer { lamports }) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_transfer, SystemTransfer, SystemTransfer {
                            lamports: lamports;
                            sender: instruction_indexes::system_transfer::SENDER_INDEX,
                            receiver: instruction_indexes::system_transfer::RECEIVER_INDEX
                        });
                    }
                    Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                        // Note: uses system_transfer_with_seed for validation but maps to SystemTransfer type
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
                    Ok(SystemInstruction::WithdrawNonceAccount(lamports)) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_withdraw_nonce_account, SystemWithdrawNonceAccount, SystemWithdrawNonceAccount {
                            lamports: lamports;
                            nonce_authority: instruction_indexes::system_withdraw_nonce_account::NONCE_AUTHORITY_INDEX,
                            recipient: instruction_indexes::system_withdraw_nonce_account::RECIPIENT_INDEX
                        });
                    }
                    Ok(SystemInstruction::Assign { .. }) => {
                        parse_system_instruction!(
                            parsed_instructions,
                            instruction,
                            system_assign,
                            SystemAssign,
                            SystemAssign {
                                authority: instruction_indexes::system_assign::AUTHORITY_INDEX
                            }
                        );
                    }
                    Ok(SystemInstruction::AssignWithSeed { .. }) => {
                        // Note: uses system_assign_with_seed for validation but maps to SystemAssign type
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
                    Ok(SystemInstruction::Allocate { .. }) => {
                        parse_system_instruction!(
                            parsed_instructions,
                            instruction,
                            system_allocate,
                            SystemAllocate,
                            SystemAllocate {
                                account: instruction_indexes::system_allocate::ACCOUNT_INDEX
                            }
                        );
                    }
                    Ok(SystemInstruction::AllocateWithSeed { .. }) => {
                        // Note: uses system_allocate_with_seed for validation but maps to SystemAllocate type
                        validate_number_accounts!(instruction, instruction_indexes::system_allocate_with_seed::REQUIRED_NUMBER_OF_ACCOUNTS);
                        parsed_instructions
                            .entry(ParsedSystemInstructionType::SystemAllocate)
                            .or_default()
                            .push(ParsedSystemInstructionData::SystemAllocate {
                                account: instruction.accounts
                                    [instruction_indexes::system_allocate_with_seed::ACCOUNT_INDEX]
                                    .pubkey,
                            });
                    }
                    Ok(SystemInstruction::InitializeNonceAccount(authority)) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_initialize_nonce_account, SystemInitializeNonceAccount, SystemInitializeNonceAccount {
                            nonce_authority: authority;
                            nonce_account: instruction_indexes::system_initialize_nonce_account::NONCE_ACCOUNT_INDEX
                        });
                    }
                    Ok(SystemInstruction::AdvanceNonceAccount) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_advance_nonce_account, SystemAdvanceNonceAccount, SystemAdvanceNonceAccount {
                            nonce_account: instruction_indexes::system_advance_nonce_account::NONCE_ACCOUNT_INDEX,
                            nonce_authority: instruction_indexes::system_advance_nonce_account::NONCE_AUTHORITY_INDEX
                        });
                    }
                    Ok(SystemInstruction::AuthorizeNonceAccount(_new_authority)) => {
                        parse_system_instruction!(parsed_instructions, instruction, system_authorize_nonce_account, SystemAuthorizeNonceAccount, SystemAuthorizeNonceAccount {
                            nonce_account: instruction_indexes::system_authorize_nonce_account::NONCE_ACCOUNT_INDEX,
                            nonce_authority: instruction_indexes::system_authorize_nonce_account::NONCE_AUTHORITY_INDEX
                        });
                    }
                    // UpgradeNonceAccount: Not parsed - no authority parameter, cannot validate fee payer involvement
                    // Anyone can upgrade any nonce account without signing
                    Ok(SystemInstruction::UpgradeNonceAccount) => {
                        // Skip parsing
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

            if program_id == spl_token_interface::ID {
                if let Ok(spl_ix) =
                    spl_token_interface::instruction::TokenInstruction::unpack(&instruction.data)
                {
                    match spl_ix {
                        spl_token_interface::instruction::TokenInstruction::Transfer { amount } => {
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
                        spl_token_interface::instruction::TokenInstruction::TransferChecked {
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
                        spl_token_interface::instruction::TokenInstruction::Burn { .. }
                        | spl_token_interface::instruction::TokenInstruction::BurnChecked { .. } => {
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
                        spl_token_interface::instruction::TokenInstruction::CloseAccount { .. } => {
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
                        spl_token_interface::instruction::TokenInstruction::Approve { .. } => {
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
                        spl_token_interface::instruction::TokenInstruction::ApproveChecked { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_approve_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenApprove)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenApprove {
                                    owner: instruction.accounts[instruction_indexes::spl_token_approve_checked::OWNER_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::Revoke => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_revoke::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenRevoke)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenRevoke {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_revoke::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::SetAuthority { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_set_authority::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenSetAuthority)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenSetAuthority {
                                    authority: instruction.accounts[instruction_indexes::spl_token_set_authority::CURRENT_AUTHORITY_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::MintTo { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_mint_to::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenMintTo)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenMintTo {
                                    mint_authority: instruction.accounts[instruction_indexes::spl_token_mint_to::MINT_AUTHORITY_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::MintToChecked { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_mint_to_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenMintTo)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenMintTo {
                                    mint_authority: instruction.accounts[instruction_indexes::spl_token_mint_to_checked::MINT_AUTHORITY_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeMint {
                            mint_authority,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_mint::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMint)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMint {
                                    mint_authority,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeMint2 {
                            mint_authority,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_mint2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMint)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMint {
                                    mint_authority,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner: instruction.accounts[instruction_indexes::spl_token_initialize_account::OWNER_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeAccount2 { owner } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeAccount3 { owner } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account3::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeMultisig { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_multisig::REQUIRED_NUMBER_OF_ACCOUNTS);

                            // Extract signers from accounts (skip first 2: multisig + rent sysvar)
                            let signers: Vec<Pubkey> =
                                instruction.accounts.iter().skip(2).map(|a| a.pubkey).collect();

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMultisig)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMultisig {
                                    signers,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::InitializeMultisig2 {
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_multisig2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            // Extract signers from accounts (skip first: multisig only)
                            let signers: Vec<Pubkey> =
                                instruction.accounts.iter().skip(1).map(|a| a.pubkey).collect();

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMultisig)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMultisig {
                                    signers,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::FreezeAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_freeze_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenFreezeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenFreezeAccount {
                                    freeze_authority: instruction.accounts[instruction_indexes::spl_token_freeze_account::FREEZE_AUTHORITY_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        spl_token_interface::instruction::TokenInstruction::ThawAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_thaw_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenThawAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenThawAccount {
                                    freeze_authority: instruction.accounts[instruction_indexes::spl_token_thaw_account::FREEZE_AUTHORITY_INDEX].pubkey,
                                    is_2022: false,
                                });
                        }
                        _ => {}
                    };
                }
            } else if program_id == spl_token_2022_interface::ID {
                if let Ok(spl_ix) = spl_token_2022_interface::instruction::TokenInstruction::unpack(
                    &instruction.data,
                ) {
                    match spl_ix {
                        #[allow(deprecated)]
                        spl_token_2022_interface::instruction::TokenInstruction::Transfer { amount } => {
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
                        spl_token_2022_interface::instruction::TokenInstruction::TransferChecked {
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
                        spl_token_2022_interface::instruction::TokenInstruction::Burn { .. }
                        | spl_token_2022_interface::instruction::TokenInstruction::BurnChecked { .. } => {
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
                        spl_token_2022_interface::instruction::TokenInstruction::CloseAccount { .. } => {
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
                        spl_token_2022_interface::instruction::TokenInstruction::Approve { .. } => {
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
                        spl_token_2022_interface::instruction::TokenInstruction::ApproveChecked {
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
                        spl_token_2022_interface::instruction::TokenInstruction::Revoke => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_revoke::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenRevoke)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenRevoke {
                                    owner: instruction.accounts
                                        [instruction_indexes::spl_token_revoke::OWNER_INDEX]
                                        .pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::SetAuthority { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_set_authority::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenSetAuthority)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenSetAuthority {
                                    authority: instruction.accounts[instruction_indexes::spl_token_set_authority::CURRENT_AUTHORITY_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::MintTo { .. } => {
                            validate_number_accounts!(
                                instruction,
                                instruction_indexes::spl_token_mint_to::REQUIRED_NUMBER_OF_ACCOUNTS
                            );

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenMintTo)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenMintTo {
                                    mint_authority: instruction.accounts[instruction_indexes::spl_token_mint_to::MINT_AUTHORITY_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::MintToChecked { .. } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_mint_to_checked::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenMintTo)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenMintTo {
                                    mint_authority: instruction.accounts[instruction_indexes::spl_token_mint_to_checked::MINT_AUTHORITY_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeMint {
                            mint_authority,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_mint::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMint)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMint {
                                    mint_authority,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeMint2 {
                            mint_authority,
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_mint2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMint)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMint {
                                    mint_authority,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner: instruction.accounts[instruction_indexes::spl_token_initialize_account::OWNER_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeAccount2 {
                            owner,
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeAccount3 {
                            owner,
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_account3::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeAccount {
                                    owner,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeMultisig {
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_multisig::REQUIRED_NUMBER_OF_ACCOUNTS);

                            // Extract signers from accounts (skip first 2: multisig + rent sysvar)
                            let signers: Vec<Pubkey> =
                                instruction.accounts.iter().skip(2).map(|a| a.pubkey).collect();

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMultisig)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMultisig {
                                    signers,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::InitializeMultisig2 {
                            ..
                        } => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_initialize_multisig2::REQUIRED_NUMBER_OF_ACCOUNTS);

                            // Extract signers from accounts (skip first: multisig only)
                            let signers: Vec<Pubkey> =
                                instruction.accounts.iter().skip(1).map(|a| a.pubkey).collect();

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenInitializeMultisig)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenInitializeMultisig {
                                    signers,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::FreezeAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_freeze_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenFreezeAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenFreezeAccount {
                                    freeze_authority: instruction.accounts[instruction_indexes::spl_token_freeze_account::FREEZE_AUTHORITY_INDEX].pubkey,
                                    is_2022: true,
                                });
                        }
                        spl_token_2022_interface::instruction::TokenInstruction::ThawAccount => {
                            validate_number_accounts!(instruction, instruction_indexes::spl_token_thaw_account::REQUIRED_NUMBER_OF_ACCOUNTS);

                            parsed_instructions
                                .entry(ParsedSPLInstructionType::SplTokenThawAccount)
                                .or_default()
                                .push(ParsedSPLInstructionData::SplTokenThawAccount {
                                    freeze_authority: instruction.accounts[instruction_indexes::spl_token_thaw_account::FREEZE_AUTHORITY_INDEX].pubkey,
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
            solana_system_interface::instruction::transfer(source, destination, lamports);

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
        let solana_instruction = solana_system_interface::instruction::transfer_with_seed(
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
        let solana_instruction = solana_system_interface::instruction::create_account(
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
        let solana_instruction = solana_system_interface::instruction::create_account_with_seed(
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
        let solana_instruction = solana_system_interface::instruction::assign(account, owner);

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
            solana_system_interface::instruction::assign_with_seed(account, base, seed, owner);

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
        let solana_instruction = solana_system_interface::instruction::withdraw_nonce_account(
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
        let solana_instruction = spl_token_interface::instruction::transfer(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::transfer_checked(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::burn(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::burn_checked(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::close_account(
            &spl_token_interface::ID,
            account,
            destination,
            authority,
            &[],
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::approve(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        let solana_instruction = spl_token_interface::instruction::approve_checked(
            &spl_token_interface::ID,
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
            &spl_token_interface::ID,
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
        #[allow(deprecated)]
        let solana_instruction = spl_token_2022_interface::instruction::transfer(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::burn(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::burn_checked(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::close_account(
            &spl_token_2022_interface::ID,
            account,
            destination,
            authority,
            &[],
        )?;

        let message = Message::new(&[solana_instruction], None);
        let compiled_instruction = &message.instructions[0];

        let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

        let parsed = parse_instruction::parse(
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
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
        let solana_instruction = spl_token_2022_interface::instruction::approve_checked(
            &spl_token_2022_interface::ID,
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
            &spl_token_2022_interface::ID,
            compiled_instruction,
            &account_keys_for_parsing,
            None,
        )?;

        Ok(parsed)
    }

    #[test]
    fn test_get_field_as_u64() {
        // Valid JSON number
        let valid_number = serde_json::json!({
            "amount": 1000
        });
        assert_eq!(IxUtils::get_field_as_u64(&valid_number, "amount").unwrap(), 1000);

        // Valid JSON string containing a number
        let valid_string_number = serde_json::json!({
            "amount": "2000"
        });
        assert_eq!(IxUtils::get_field_as_u64(&valid_string_number, "amount").unwrap(), 2000);

        // Missing field
        let missing_field = serde_json::json!({
            "other": 3000
        });
        let err = IxUtils::get_field_as_u64(&missing_field, "amount").unwrap_err();
        assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

        // Invalid string that cannot be parsed as a u64
        let invalid_string = serde_json::json!({
            "amount": "invalid"
        });
        let err = IxUtils::get_field_as_u64(&invalid_string, "amount").unwrap_err();
        assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

        // JSON null or other unexpected type
        let null_value = serde_json::json!({
            "amount": null
        });
        let err = IxUtils::get_field_as_u64(&null_value, "amount").unwrap_err();
        assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

        // Unexpected type (array)
        let array_value = serde_json::json!({
            "amount": [1, 2, 3]
        });
        let err = IxUtils::get_field_as_u64(&array_value, "amount").unwrap_err();
        assert!(matches!(err, crate::error::KoraError::SerializationError(_)));
    }

    #[test]
    fn test_parse_system_instructions_transfer() {
        use crate::transaction::versioned_transaction::VersionedTransactionResolved;
        use solana_message::{Message, VersionedMessage};
        use solana_sdk::{
            signature::{Keypair, Signer},
            transaction::VersionedTransaction,
        };
        use solana_system_interface::instruction::transfer;

        let sender = Keypair::new();
        let receiver = Pubkey::new_unique();
        let lamports = 1000u64;

        let instruction = transfer(&sender.pubkey(), &receiver, lamports);

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&sender]).unwrap();

        let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
            .expect("Failed to create resolved transaction");

        let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
            .expect("Failed to parse system instructions");

        let transfers = parsed_instructions
            .get(&ParsedSystemInstructionType::SystemTransfer)
            .expect("Expected SystemTransfer instructions");

        assert_eq!(transfers.len(), 1);

        match &transfers[0] {
            ParsedSystemInstructionData::SystemTransfer {
                lamports: parsed_lamports,
                sender: parsed_sender,
                receiver: parsed_receiver,
            } => {
                assert_eq!(*parsed_lamports, lamports);
                assert_eq!(*parsed_sender, sender.pubkey());
                assert_eq!(*parsed_receiver, receiver);
            }
            _ => panic!("Expected SystemTransfer variant"),
        }
    }

    #[test]
    fn test_parse_token_instructions_transfer() {
        use crate::transaction::versioned_transaction::VersionedTransactionResolved;
        use solana_message::{Message, VersionedMessage};
        use solana_sdk::{
            signature::{Keypair, Signer},
            transaction::VersionedTransaction,
        };
        use spl_token_interface::instruction::transfer;

        let owner = Keypair::new();
        let source_address = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let amount = 5000u64;

        let instruction = transfer(
            &spl_token_interface::ID,
            &source_address,
            &destination_address,
            &owner.pubkey(),
            &[],
            amount,
        )
        .expect("Failed to create transfer instruction");

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&owner.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&owner]).unwrap();

        let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
            .expect("Failed to create resolved transaction");

        let parsed_instructions = IxUtils::parse_token_instructions(&resolved_tx)
            .expect("Failed to parse token instructions");

        let transfers = parsed_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .expect("Expected SplTokenTransfer instructions");

        assert_eq!(transfers.len(), 1);

        match &transfers[0] {
            ParsedSPLInstructionData::SplTokenTransfer {
                amount: parsed_amount,
                owner: parsed_owner,
                mint,
                source_address: parsed_source,
                destination_address: parsed_destination,
                is_2022,
            } => {
                assert_eq!(*parsed_amount, amount);
                assert_eq!(*parsed_owner, owner.pubkey());
                assert_eq!(*parsed_source, source_address);
                assert_eq!(*parsed_destination, destination_address);
                assert_eq!(*is_2022, false);
                assert!(mint.is_none());
            }
            _ => panic!("Expected SplTokenTransfer variant"),
        }
    }

    #[test]
    fn test_parse_token_2022_instructions_transfer_checked() {
        use crate::transaction::versioned_transaction::VersionedTransactionResolved;
        use solana_message::{Message, VersionedMessage};
        use solana_sdk::{
            signature::{Keypair, Signer},
            transaction::VersionedTransaction,
        };
        use spl_token_2022_interface::instruction::transfer_checked;

        let owner = Keypair::new();
        let source_address = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_address = Pubkey::new_unique();
        let amount = 7500u64;
        let decimals = 6u8;

        let instruction = transfer_checked(
            &spl_token_2022_interface::ID,
            &source_address,
            &mint,
            &destination_address,
            &owner.pubkey(),
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create transfer_checked instruction");

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&owner.pubkey())));
        let tx = VersionedTransaction::try_new(message, &[&owner]).unwrap();

        let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
            .expect("Failed to create resolved transaction");

        let parsed_instructions = IxUtils::parse_token_instructions(&resolved_tx)
            .expect("Failed to parse token instructions");

        let transfers = parsed_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .expect("Expected SplTokenTransfer instructions");

        assert_eq!(transfers.len(), 1);

        match &transfers[0] {
            ParsedSPLInstructionData::SplTokenTransfer {
                amount: parsed_amount,
                owner: parsed_owner,
                mint: parsed_mint,
                source_address: parsed_source,
                destination_address: parsed_destination,
                is_2022,
            } => {
                assert_eq!(*parsed_amount, amount);
                assert_eq!(*parsed_owner, owner.pubkey());
                assert_eq!(*parsed_source, source_address);
                assert_eq!(*parsed_destination, destination_address);
                assert_eq!(*is_2022, true);
                assert_eq!(parsed_mint.unwrap(), mint);
            }
            _ => panic!("Expected SplTokenTransfer variant"),
        }
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

        let instructions = IxUtils::uncompile_instructions(&[compiled_ix], &account_keys).unwrap();

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
            solana_system_interface::instruction::transfer(&source, &destination, lamports);

        let solana_parsed_transfer = create_parsed_system_transfer(&source, &destination, lamports)
            .expect("Failed to create authentic parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed_transfer,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::transfer_with_seed(
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

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::create_account(
            &source,
            &new_account,
            lamports,
            space,
            &owner,
        );

        let solana_parsed =
            create_parsed_system_create_account(&source, &new_account, lamports, space, &owner)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::create_account_with_seed(
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

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::assign(&account, &owner);

        let solana_parsed = create_parsed_system_assign(&account, &owner)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::assign_with_seed(
            &account,
            &base,
            "test_assign_seed",
            &owner,
        );

        let solana_parsed =
            create_parsed_system_assign_with_seed(&account, &base, "test_assign_seed", &owner)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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

        let instruction = solana_system_interface::instruction::withdraw_nonce_account(
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

        let result = IxUtils::reconstruct_system_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, source, destination, authority];
        let amount = 1000000u64;

        let transfer_instruction = spl_token_interface::instruction::transfer(
            &spl_token_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed_transfer,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, source, mint, destination, authority];
        let amount = 2000000u64;
        let decimals = 6u8;

        let instruction = spl_token_interface::instruction::transfer_checked(
            &spl_token_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 500000u64;

        let instruction = spl_token_interface::instruction::burn(
            &spl_token_interface::ID,
            &account,
            &mint,
            &authority,
            &[],
            amount,
        )
        .expect("Failed to create burn instruction");

        let solana_parsed = create_parsed_spl_token_burn(&account, &mint, &authority, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 750000u64;
        let decimals = 6u8;

        let instruction = spl_token_interface::instruction::burn_checked(
            &spl_token_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, account, destination, authority];

        let instruction = spl_token_interface::instruction::close_account(
            &spl_token_interface::ID,
            &account,
            &destination,
            &authority,
            &[],
        )
        .expect("Failed to create close_account instruction");

        let solana_parsed =
            create_parsed_spl_token_close_account(&account, &destination, &authority)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, source, delegate, owner];
        let amount = 1000000u64;

        let instruction = spl_token_interface::instruction::approve(
            &spl_token_interface::ID,
            &source,
            &delegate,
            &owner,
            &[],
            amount,
        )
        .expect("Failed to create approve instruction");

        let solana_parsed = create_parsed_spl_token_approve(&source, &delegate, &owner, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_interface::ID;
        let account_keys = vec![token_program_id, source, mint, delegate, owner];
        let amount = 2500000u64;
        let decimals = 6u8;

        let instruction = spl_token_interface::instruction::approve_checked(
            &spl_token_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, source, destination, authority];
        let amount = 1500000u64;

        #[allow(deprecated)]
        let instruction = spl_token_2022_interface::instruction::transfer(
            &spl_token_2022_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, source, mint, destination, authority];
        let amount = 3000000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 800000u64;

        let instruction = spl_token_2022_interface::instruction::burn(
            &spl_token_2022_interface::ID,
            &account,
            &mint,
            &authority,
            &[],
            amount,
        )
        .expect("Failed to create burn instruction");

        let solana_parsed = create_parsed_token2022_burn(&account, &mint, &authority, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, account, mint, authority];
        let amount = 900000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022_interface::instruction::burn_checked(
            &spl_token_2022_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, account, destination, authority];

        let instruction = spl_token_2022_interface::instruction::close_account(
            &spl_token_2022_interface::ID,
            &account,
            &destination,
            &authority,
            &[],
        )
        .expect("Failed to create close_account instruction");

        let solana_parsed =
            create_parsed_token2022_close_account(&account, &destination, &authority)
                .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, source, delegate, owner];
        let amount = 1200000u64;

        let instruction = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::ID,
            &source,
            &delegate,
            &owner,
            &[],
            amount,
        )
        .expect("Failed to create approve instruction");

        let solana_parsed = create_parsed_token2022_approve(&source, &delegate, &owner, amount)
            .expect("Failed to create parsed instruction");

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
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
        let token_program_id = spl_token_2022_interface::ID;
        let account_keys = vec![token_program_id, source, mint, delegate, owner];
        let amount = 3500000u64;
        let decimals = 6u8;

        let instruction = spl_token_2022_interface::instruction::approve_checked(
            &spl_token_2022_interface::ID,
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

        let result = IxUtils::reconstruct_spl_token_instruction(
            &solana_parsed,
            &IxUtils::build_account_keys_hashmap(&account_keys),
        );

        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.program_id_index, 0);
        assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, delegate, owner indices
        assert_eq!(compiled.data, instruction.data);
    }

    #[test]
    fn test_reconstruct_unsupported_program_creates_stub() {
        let unsupported_program = Pubkey::new_unique();
        let account_keys = vec![unsupported_program];

        let parsed_instruction = solana_transaction_status_client_types::ParsedInstruction {
            program: "unsupported".to_string(),
            program_id: unsupported_program.to_string(),
            parsed: serde_json::json!({
                "type": "unknownInstruction",
                "info": {
                    "someField": "someValue"
                }
            }),
            stack_height: None,
        };

        let ui_instruction = UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed_instruction));

        let result = IxUtils::reconstruct_instruction_from_ui(&ui_instruction, &account_keys);

        assert!(result.is_some());
        let compiled = result.unwrap();

        assert_eq!(compiled.program_id_index, 0);
        assert!(compiled.accounts.is_empty());
        assert!(compiled.data.is_empty());
    }
}
