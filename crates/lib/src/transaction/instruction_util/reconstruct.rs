use std::collections::HashMap;

use solana_message::compiled_instruction::CompiledInstruction;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};
use solana_transaction_status::parse_token::UiExtensionType;
use solana_transaction_status_client_types::{
    UiInstruction, UiParsedInstruction, UiPartiallyDecodedInstruction,
};

use super::{spl_token::BATCH_DISCRIMINATOR, IxUtils};
use crate::{error::KoraError, sanitize_error};

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
pub const PARSED_DATA_FIELD_GET_ACCOUNT_DATA_SIZE: &str = "getAccountDataSize";
pub const PARSED_DATA_FIELD_INITIALIZE_IMMUTABLE_OWNER: &str = "initializeImmutableOwner";
pub const PARSED_DATA_FIELD_SYNC_NATIVE: &str = "syncNative";
pub const PARSED_DATA_FIELD_EXTENSION_TYPES: &str = "extensionTypes";
pub const PARSED_DATA_FIELD_BATCH: &str = "batch";
pub const PARSED_DATA_FIELD_INSTRUCTIONS: &str = "instructions";

pub const PARSED_DATA_FIELD_MINT_AUTHORITY: &str = "mintAuthority";
pub const PARSED_DATA_FIELD_FREEZE_AUTHORITY: &str = "freezeAuthority";
pub const PARSED_DATA_FIELD_AUTHORITY_TYPE: &str = "authorityType";
pub const PARSED_DATA_FIELD_MULTISIG_ACCOUNT: &str = "multisig";
pub const PARSED_DATA_FIELD_SIGNERS: &str = "signers";
pub const PARSED_DATA_FIELD_MULTISIG_AUTHORITY: &str = "multisigAuthority";
pub const PARSED_DATA_FIELD_MULTISIG_OWNER: &str = "multisigOwner";
pub const PARSED_DATA_FIELD_MULTISIG_MINT_AUTHORITY: &str = "multisigMintAuthority";
pub const PARSED_DATA_FIELD_MULTISIG_FREEZE_AUTHORITY: &str = "multisigFreezeAuthority";
pub const PARSED_DATA_FIELD_M: &str = "m";
pub const PARSED_DATA_FIELD_RENT_SYSVAR: &str = "rentSysvar";

pub const PARSED_DATA_FIELD_INITIALIZE_METADATA_POINTER: &str = "initializeMetadataPointer";
pub const PARSED_DATA_FIELD_UPDATE_METADATA_POINTER: &str = "updateMetadataPointer";
pub const PARSED_DATA_FIELD_INITIALIZE_TOKEN_METADATA: &str = "initializeTokenMetadata";
pub const PARSED_DATA_FIELD_UPDATE_TOKEN_METADATA_FIELD: &str = "updateTokenMetadataField";
pub const PARSED_DATA_FIELD_REMOVE_TOKEN_METADATA_KEY: &str = "removeTokenMetadataKey";
pub const PARSED_DATA_FIELD_UPDATE_TOKEN_METADATA_AUTHORITY: &str = "updateTokenMetadataAuthority";
pub const PARSED_DATA_FIELD_EMIT_TOKEN_METADATA: &str = "emitTokenMetadata";
pub const PARSED_DATA_FIELD_INITIALIZE_MINT_CLOSE_AUTHORITY: &str = "initializeMintCloseAuthority";
pub const PARSED_DATA_FIELD_INITIALIZE_PERMANENT_DELEGATE: &str = "initializePermanentDelegate";
pub const PARSED_DATA_FIELD_INITIALIZE_NON_TRANSFERABLE_MINT: &str =
    "initializeNonTransferableMint";
pub const PARSED_DATA_FIELD_INITIALIZE_TRANSFER_HOOK: &str = "initializeTransferHook";
pub const PARSED_DATA_FIELD_INITIALIZE_GROUP_POINTER: &str = "initializeGroupPointer";
pub const PARSED_DATA_FIELD_INITIALIZE_GROUP_MEMBER_POINTER: &str = "initializeGroupMemberPointer";
pub const PARSED_DATA_FIELD_INITIALIZE_TRANSFER_FEE_CONFIG: &str = "initializeTransferFeeConfig";

pub const PARSED_DATA_FIELD_METADATA: &str = "metadata";
pub const PARSED_DATA_FIELD_METADATA_ADDRESS: &str = "metadataAddress";
pub const PARSED_DATA_FIELD_UPDATE_AUTHORITY: &str = "updateAuthority";
pub const PARSED_DATA_FIELD_NAME: &str = "name";
pub const PARSED_DATA_FIELD_SYMBOL: &str = "symbol";
pub const PARSED_DATA_FIELD_URI: &str = "uri";
pub const PARSED_DATA_FIELD_FIELD: &str = "field";
pub const PARSED_DATA_FIELD_VALUE: &str = "value";
pub const PARSED_DATA_FIELD_KEY: &str = "key";
pub const PARSED_DATA_FIELD_IDEMPOTENT: &str = "idempotent";
pub const PARSED_DATA_FIELD_START: &str = "start";
pub const PARSED_DATA_FIELD_END: &str = "end";
pub const PARSED_DATA_FIELD_PROGRAM_ID: &str = "programId";
pub const PARSED_DATA_FIELD_GROUP_ADDRESS: &str = "groupAddress";
pub const PARSED_DATA_FIELD_MEMBER_ADDRESS: &str = "memberAddress";
pub const PARSED_DATA_FIELD_TRANSFER_FEE_CONFIG_AUTHORITY: &str = "transferFeeConfigAuthority";
pub const PARSED_DATA_FIELD_WITHDRAW_WITHHELD_AUTHORITY: &str = "withdrawWithheldAuthority";
pub const PARSED_DATA_FIELD_TRANSFER_FEE_BASIS_POINTS: &str = "transferFeeBasisPoints";
pub const PARSED_DATA_FIELD_MAXIMUM_FEE: &str = "maximumFee";

impl IxUtils {
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

    fn get_field_as_optional_pubkey(
        info: &serde_json::Value,
        field_name: &str,
    ) -> Result<Option<Pubkey>, KoraError> {
        match info.get(field_name) {
            Some(v) if !v.is_null() => Self::get_field_as_pubkey(info, field_name).map(Some),
            _ => Ok(None),
        }
    }

    fn get_authority_and_signers(
        info: &serde_json::Value,
        field_name: &str,
        multisig_field_name: &str,
    ) -> Result<(Pubkey, Vec<Pubkey>), KoraError> {
        if info.get(multisig_field_name).is_none() {
            return Ok((Self::get_field_as_pubkey(info, field_name)?, vec![]));
        }
        Ok((Self::get_field_as_pubkey(info, multisig_field_name)?, Self::get_signers(info)?))
    }

    fn get_signers(info: &serde_json::Value) -> Result<Vec<Pubkey>, KoraError> {
        let signers =
            info.get(PARSED_DATA_FIELD_SIGNERS).and_then(|v| v.as_array()).ok_or_else(|| {
                KoraError::SerializationError("Missing or invalid 'signers' field".to_string())
            })?;
        signers
            .iter()
            .map(|signer| {
                let signer_str = signer.as_str().ok_or_else(|| {
                    KoraError::SerializationError("'signers' entry is not a string".to_string())
                })?;
                signer_str.parse::<Pubkey>().map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Invalid multisig signer '{}': {}",
                        signer_str,
                        sanitize_error!(e)
                    ))
                })
            })
            .collect()
    }

    pub(super) fn get_field_as_u64(
        info: &serde_json::Value,
        field_name: &str,
    ) -> Result<u64, KoraError> {
        let value = info.get(field_name).ok_or_else(|| {
            KoraError::SerializationError(format!("Missing field '{}'", field_name))
        })?;

        if let Some(num) = value.as_u64() {
            return Ok(num);
        }

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

    fn get_token2022_extension_types(
        info: &serde_json::Value,
    ) -> Result<Vec<spl_token_2022_interface::extension::ExtensionType>, KoraError> {
        let Some(extension_types) = info.get(PARSED_DATA_FIELD_EXTENSION_TYPES) else {
            return Ok(vec![]);
        };

        let extension_types: Vec<UiExtensionType> = serde_json::from_value(extension_types.clone())
            .map_err(|e| {
                KoraError::SerializationError(format!(
                    "Field '{}' is not a valid Token-2022 extension list: {}",
                    PARSED_DATA_FIELD_EXTENSION_TYPES, e
                ))
            })?;

        use spl_token_2022_interface::extension::ExtensionType;
        use UiExtensionType as Ui;

        extension_types
            .into_iter()
            .map(|extension_type| {
                Ok(match extension_type {
                    Ui::Uninitialized => ExtensionType::Uninitialized,
                    Ui::TransferFeeConfig => ExtensionType::TransferFeeConfig,
                    Ui::TransferFeeAmount => ExtensionType::TransferFeeAmount,
                    Ui::MintCloseAuthority => ExtensionType::MintCloseAuthority,
                    Ui::ConfidentialTransferMint => ExtensionType::ConfidentialTransferMint,
                    Ui::ConfidentialTransferAccount => ExtensionType::ConfidentialTransferAccount,
                    Ui::DefaultAccountState => ExtensionType::DefaultAccountState,
                    Ui::ImmutableOwner => ExtensionType::ImmutableOwner,
                    Ui::MemoTransfer => ExtensionType::MemoTransfer,
                    Ui::NonTransferable => ExtensionType::NonTransferable,
                    Ui::InterestBearingConfig => ExtensionType::InterestBearingConfig,
                    Ui::CpiGuard => ExtensionType::CpiGuard,
                    Ui::PermanentDelegate => ExtensionType::PermanentDelegate,
                    Ui::NonTransferableAccount => ExtensionType::NonTransferableAccount,
                    Ui::TransferHook => ExtensionType::TransferHook,
                    Ui::TransferHookAccount => ExtensionType::TransferHookAccount,
                    Ui::ConfidentialTransferFeeConfig => {
                        ExtensionType::ConfidentialTransferFeeConfig
                    }
                    Ui::ConfidentialTransferFeeAmount => {
                        ExtensionType::ConfidentialTransferFeeAmount
                    }
                    Ui::MetadataPointer => ExtensionType::MetadataPointer,
                    Ui::TokenMetadata => ExtensionType::TokenMetadata,
                    Ui::GroupPointer => ExtensionType::GroupPointer,
                    Ui::GroupMemberPointer => ExtensionType::GroupMemberPointer,
                    Ui::TokenGroup => ExtensionType::TokenGroup,
                    Ui::TokenGroupMember => ExtensionType::TokenGroupMember,
                    Ui::ConfidentialMintBurn => ExtensionType::ConfidentialMintBurn,
                    Ui::ScaledUiAmount => ExtensionType::ScaledUiAmount,
                    Ui::Pausable => ExtensionType::Pausable,
                    Ui::PausableAccount => ExtensionType::PausableAccount,
                    Ui::PermissionedBurn => {
                        return Err(KoraError::InvalidTransaction(
                            "Unsupported Token-2022 extension type 'PermissionedBurn'".to_string(),
                        ))
                    }
                })
            })
            .collect()
    }

    fn decode_base58_instruction_data(
        encoded_data: &str,
        error_context: &str,
    ) -> Result<Vec<u8>, KoraError> {
        bs58::decode(encoded_data).into_vec().map_err(|e| {
            KoraError::SerializationError(format!(
                "Failed to decode {error_context} from base58: {}",
                sanitize_error!(e)
            ))
        })
    }

    fn get_or_insert_account_index(
        account_keys_hashmap: &mut HashMap<Pubkey, u8>,
        all_account_keys: &mut Vec<Pubkey>,
        pubkey: Pubkey,
        overflow_context: &str,
        inserted_keys: &mut Vec<Pubkey>,
    ) -> Result<u8, KoraError> {
        if let Some(&idx) = account_keys_hashmap.get(&pubkey) {
            return Ok(idx);
        }

        let idx = u8::try_from(all_account_keys.len()).map_err(|_| {
            KoraError::SerializationError(format!(
                "{overflow_context} exceeds u8 index limit (len={})",
                all_account_keys.len()
            ))
        })?;

        all_account_keys.push(pubkey);
        account_keys_hashmap.insert(pubkey, idx);
        inserted_keys.push(pubkey);

        Ok(idx)
    }

    fn rollback_account_key_extensions(
        account_keys_hashmap: &mut HashMap<Pubkey, u8>,
        all_account_keys: &mut Vec<Pubkey>,
        snapshot_len: usize,
        inserted_keys: &[Pubkey],
    ) {
        all_account_keys.truncate(snapshot_len);
        for key in inserted_keys {
            account_keys_hashmap.remove(key);
        }
    }

    /// Simulating with the inner-instructions flag makes the RPC pre-parse SPL/System
    /// instructions into "Parsed" form, but Kora's parsing logic expects "Compiled"
    /// instructions; there's no RPC option to disable this, so we reconstruct (unparse) them.
    /// Reference: https://github.com/anza-xyz/agave/blob/68032b576dc4c14b31c15974c6734ae1513980a3/transaction-status/src/parse_system.rs#L11
    pub fn reconstruct_instruction_from_ui(
        ui_instruction: &UiInstruction,
        all_account_keys: &mut Vec<Pubkey>,
    ) -> Result<CompiledInstruction, KoraError> {
        let mut account_keys_hashmap = Self::build_account_keys_hashmap(all_account_keys);
        Self::reconstruct_instruction_from_ui_with_account_key_cache(
            ui_instruction,
            all_account_keys,
            &mut account_keys_hashmap,
        )
    }

    pub fn reconstruct_instruction_from_ui_with_account_key_cache(
        ui_instruction: &UiInstruction,
        all_account_keys: &mut Vec<Pubkey>,
        account_keys_hashmap: &mut HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        match ui_instruction {
            UiInstruction::Compiled(compiled) => {
                let data = Self::decode_base58_instruction_data(
                    &compiled.data,
                    "compiled instruction data",
                )?;

                Ok(CompiledInstruction {
                    program_id_index: compiled.program_id_index,
                    accounts: compiled.accounts.clone(),
                    data,
                })
            }
            UiInstruction::Parsed(ui_parsed) => Self::reconstruct_parsed_instruction(
                ui_parsed,
                all_account_keys,
                account_keys_hashmap,
            ),
        }
    }

    fn reconstruct_parsed_instruction(
        ui_parsed: &UiParsedInstruction,
        all_account_keys: &mut Vec<Pubkey>,
        account_keys_hashmap: &mut HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        match ui_parsed {
            UiParsedInstruction::Parsed(parsed) => {
                if parsed.program_id == SYSTEM_PROGRAM_ID.to_string() {
                    Self::reconstruct_system_instruction(parsed, account_keys_hashmap)
                } else if parsed.program_id == spl_token_interface::ID.to_string()
                    || parsed.program_id == spl_token_2022_interface::ID.to_string()
                {
                    Self::reconstruct_spl_token_instruction(parsed, account_keys_hashmap)
                } else {
                    // Unsupported program: stub instruction keeps only the program ID, needed for security validation
                    let program_id = parsed.program_id.parse::<Pubkey>().map_err(|e| {
                        KoraError::SerializationError(format!(
                            "Invalid parsed instruction program_id '{}': {}",
                            parsed.program_id,
                            sanitize_error!(e)
                        ))
                    })?;
                    let program_id_index = *account_keys_hashmap.get(&program_id).ok_or_else(|| {
                        KoraError::SerializationError(format!(
                            "Unsupported parsed instruction program_id {} not found in account keys",
                            program_id
                        ))
                    })?;

                    Ok(Self::build_default_compiled_instruction(program_id_index))
                }
            }
            UiParsedInstruction::PartiallyDecoded(partial) => {
                Self::reconstruct_partially_decoded_instruction(
                    partial,
                    all_account_keys,
                    account_keys_hashmap,
                )
            }
        }
    }

    fn reconstruct_partially_decoded_instruction(
        partial: &UiPartiallyDecodedInstruction,
        all_account_keys: &mut Vec<Pubkey>,
        account_keys_hashmap: &mut HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        let snapshot_len = all_account_keys.len();
        let mut inserted_keys = Vec::new();
        let result = (|| -> Result<CompiledInstruction, KoraError> {
            let program_id = partial.program_id.parse::<Pubkey>().map_err(|e| {
                KoraError::SerializationError(format!(
                    "Invalid partially decoded instruction program_id '{}': {}",
                    partial.program_id,
                    sanitize_error!(e)
                ))
            })?;

            // Program ID itself may be a CPI-only account not in the
            // outer transaction's keys (e.g. an invoked program via CPI).
            let program_idx = Self::get_or_insert_account_index(
                account_keys_hashmap,
                all_account_keys,
                program_id,
                "CPI program key",
                &mut inserted_keys,
            )?;

            // Convert account addresses to indices, extending the keys vec
            // for any CPI PDA accounts not in the original keys.
            let mut account_indices: Vec<u8> = Vec::with_capacity(partial.accounts.len());
            for addr_str in &partial.accounts {
                let pubkey = addr_str.parse::<Pubkey>().map_err(|e| {
                    KoraError::SerializationError(format!(
                        "Invalid partially decoded instruction account '{}': {}",
                        addr_str,
                        sanitize_error!(e)
                    ))
                })?;

                // CPI PDA signers (e.g. Kamino lending authority) are not in
                // the outer transaction's account keys.
                let idx = Self::get_or_insert_account_index(
                    account_keys_hashmap,
                    all_account_keys,
                    pubkey,
                    "CPI account key",
                    &mut inserted_keys,
                )?;
                account_indices.push(idx);
            }

            let data = Self::decode_base58_instruction_data(
                &partial.data,
                "partially decoded instruction data",
            )?;

            Ok(CompiledInstruction {
                program_id_index: program_idx,
                accounts: account_indices,
                data,
            })
        })();

        if result.is_err() {
            Self::rollback_account_key_extensions(
                account_keys_hashmap,
                all_account_keys,
                snapshot_len,
                &inserted_keys,
            );
        }

        result
    }

    pub(super) fn reconstruct_system_instruction(
        parsed: &solana_transaction_status_client_types::ParsedInstruction,
        account_keys_hashmap: &HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        let program_id_index = Self::get_account_index(account_keys_hashmap, &SYSTEM_PROGRAM_ID)?;

        let parsed_data = &parsed.parsed;
        let instruction_type = Self::get_field_as_str(parsed_data, PARSED_DATA_FIELD_TYPE)?;
        let info = parsed_data
            .get(PARSED_DATA_FIELD_INFO)
            .ok_or_else(|| KoraError::SerializationError("Missing 'info' field".to_string()))?;

        let pubkey = |field: &str| Self::get_field_as_pubkey(info, field);
        let index = |field: &str| -> Result<u8, KoraError> {
            Self::get_account_index(account_keys_hashmap, &pubkey(field)?)
        };
        let number = |field: &str| Self::get_field_as_u64(info, field);
        let seed = |field: &str| Self::get_field_as_str(info, field).map(str::to_string);

        let (system_instruction, accounts) = match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => (
                SystemInstruction::Transfer { lamports: number(PARSED_DATA_FIELD_LAMPORTS)? },
                vec![index(PARSED_DATA_FIELD_SOURCE)?, index(PARSED_DATA_FIELD_DESTINATION)?],
            ),
            PARSED_DATA_FIELD_CREATE_ACCOUNT => (
                SystemInstruction::CreateAccount {
                    lamports: number(PARSED_DATA_FIELD_LAMPORTS)?,
                    space: number(PARSED_DATA_FIELD_SPACE)?,
                    owner: pubkey(PARSED_DATA_FIELD_OWNER)?,
                },
                vec![index(PARSED_DATA_FIELD_SOURCE)?, index(PARSED_DATA_FIELD_NEW_ACCOUNT)?],
            ),
            PARSED_DATA_FIELD_ASSIGN => (
                SystemInstruction::Assign { owner: pubkey(PARSED_DATA_FIELD_OWNER)? },
                vec![index(PARSED_DATA_FIELD_ACCOUNT)?],
            ),
            PARSED_DATA_FIELD_TRANSFER_WITH_SEED => (
                SystemInstruction::TransferWithSeed {
                    lamports: number(PARSED_DATA_FIELD_LAMPORTS)?,
                    from_seed: seed(PARSED_DATA_FIELD_SOURCE_SEED)?,
                    from_owner: pubkey(PARSED_DATA_FIELD_SOURCE_OWNER)?,
                },
                vec![
                    index(PARSED_DATA_FIELD_SOURCE)?,
                    index(PARSED_DATA_FIELD_SOURCE_BASE)?,
                    index(PARSED_DATA_FIELD_DESTINATION)?,
                ],
            ),
            PARSED_DATA_FIELD_CREATE_ACCOUNT_WITH_SEED => (
                SystemInstruction::CreateAccountWithSeed {
                    base: pubkey(PARSED_DATA_FIELD_BASE)?,
                    seed: seed(PARSED_DATA_FIELD_SEED)?,
                    lamports: number(PARSED_DATA_FIELD_LAMPORTS)?,
                    space: number(PARSED_DATA_FIELD_SPACE)?,
                    owner: pubkey(PARSED_DATA_FIELD_OWNER)?,
                },
                vec![
                    index(PARSED_DATA_FIELD_SOURCE)?,
                    index(PARSED_DATA_FIELD_NEW_ACCOUNT)?,
                    index(PARSED_DATA_FIELD_BASE)?,
                ],
            ),
            PARSED_DATA_FIELD_ASSIGN_WITH_SEED => (
                SystemInstruction::AssignWithSeed {
                    base: pubkey(PARSED_DATA_FIELD_BASE)?,
                    seed: seed(PARSED_DATA_FIELD_SEED)?,
                    owner: pubkey(PARSED_DATA_FIELD_OWNER)?,
                },
                vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_BASE)?],
            ),
            PARSED_DATA_FIELD_WITHDRAW_NONCE_ACCOUNT => (
                SystemInstruction::WithdrawNonceAccount(number(PARSED_DATA_FIELD_LAMPORTS)?),
                vec![
                    index(PARSED_DATA_FIELD_NONCE_ACCOUNT)?,
                    index(PARSED_DATA_FIELD_DESTINATION)?,
                    index(PARSED_DATA_FIELD_NONCE_AUTHORITY)?,
                ],
            ),
            PARSED_DATA_FIELD_ALLOCATE => (
                SystemInstruction::Allocate { space: number(PARSED_DATA_FIELD_SPACE)? },
                vec![index(PARSED_DATA_FIELD_ACCOUNT)?],
            ),
            PARSED_DATA_FIELD_ALLOCATE_WITH_SEED => (
                SystemInstruction::AllocateWithSeed {
                    base: pubkey(PARSED_DATA_FIELD_BASE)?,
                    seed: seed(PARSED_DATA_FIELD_SEED)?,
                    space: number(PARSED_DATA_FIELD_SPACE)?,
                    owner: pubkey(PARSED_DATA_FIELD_OWNER)?,
                },
                vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_BASE)?],
            ),
            // The parsed form omits the recent-blockhashes and rent sysvars, so only the
            // accounts present in the key map are emitted for the nonce instructions.
            PARSED_DATA_FIELD_INITIALIZE_NONCE_ACCOUNT => (
                SystemInstruction::InitializeNonceAccount(pubkey(
                    PARSED_DATA_FIELD_NONCE_AUTHORITY,
                )?),
                vec![index(PARSED_DATA_FIELD_NONCE_ACCOUNT)?],
            ),
            PARSED_DATA_FIELD_ADVANCE_NONCE_ACCOUNT => (
                SystemInstruction::AdvanceNonceAccount,
                vec![
                    index(PARSED_DATA_FIELD_NONCE_ACCOUNT)?,
                    index(PARSED_DATA_FIELD_NONCE_AUTHORITY)?,
                ],
            ),
            PARSED_DATA_FIELD_AUTHORIZE_NONCE_ACCOUNT => (
                SystemInstruction::AuthorizeNonceAccount(pubkey(PARSED_DATA_FIELD_NEW_AUTHORITY)?),
                vec![
                    index(PARSED_DATA_FIELD_NONCE_ACCOUNT)?,
                    index(PARSED_DATA_FIELD_NONCE_AUTHORITY)?,
                ],
            ),
            _ => {
                return Err(KoraError::InvalidTransaction(format!(
                    "Unrecognized system instruction type '{}' in CPI — cannot validate fee payer policy",
                    instruction_type
                )))
            }
        };

        let data = bincode::serialize(&system_instruction).map_err(|e| {
            KoraError::SerializationError(format!(
                "Failed to serialize system instruction '{instruction_type}': {e}"
            ))
        })?;

        Ok(CompiledInstruction { program_id_index, accounts, data })
    }

    pub(super) fn reconstruct_spl_token_instruction(
        parsed: &solana_transaction_status_client_types::ParsedInstruction,
        account_keys_hashmap: &HashMap<Pubkey, u8>,
    ) -> Result<CompiledInstruction, KoraError> {
        let program_id = parsed.program_id.parse::<Pubkey>().map_err(|e| {
            KoraError::SerializationError(format!("Invalid program ID: {}", sanitize_error!(e)))
        })?;
        let program_id_index = Self::get_account_index(account_keys_hashmap, &program_id)?;
        let is_spl_token_program = program_id == spl_token_interface::ID;

        let parsed_data = &parsed.parsed;
        let instruction_type = Self::get_field_as_str(parsed_data, PARSED_DATA_FIELD_TYPE)?;
        let info = parsed_data
            .get(PARSED_DATA_FIELD_INFO)
            .ok_or_else(|| KoraError::SerializationError("Missing 'info' field".to_string()))?;

        let pubkey = |field: &str| Self::get_field_as_pubkey(info, field);
        let optional_pubkey = |field: &str| Self::get_field_as_optional_pubkey(info, field);
        let index = |field: &str| -> Result<u8, KoraError> {
            Self::get_account_index(account_keys_hashmap, &pubkey(field)?)
        };
        let token_amount = || -> Result<(u64, u8), KoraError> {
            let token_amount = info.get(PARSED_DATA_FIELD_TOKEN_AMOUNT).ok_or_else(|| {
                KoraError::SerializationError("Missing 'tokenAmount' field".to_string())
            })?;
            let amount = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_AMOUNT)?;
            let decimals = Self::get_field_as_u64(token_amount, PARSED_DATA_FIELD_DECIMALS)? as u8;
            Ok((amount, decimals))
        };
        let signer_indices = || -> Result<Vec<u8>, KoraError> {
            Self::get_signers(info)?
                .iter()
                .map(|signer| Self::get_account_index(account_keys_hashmap, signer))
                .collect()
        };
        let authority_accounts =
            |field: &str, multisig_field: &str| -> Result<Vec<u8>, KoraError> {
                let (authority, signers) =
                    Self::get_authority_and_signers(info, field, multisig_field)?;
                std::iter::once(authority)
                    .chain(signers)
                    .map(|key| Self::get_account_index(account_keys_hashmap, &key))
                    .collect()
            };

        macro_rules! pack {
            ($($variant:tt)+) => {
                if is_spl_token_program {
                    spl_token_interface::instruction::TokenInstruction::$($variant)+.pack()
                } else {
                    #[allow(deprecated)]
                    spl_token_2022_interface::instruction::TokenInstruction::$($variant)+.pack()
                }
            };
        }

        let (data, accounts) = match instruction_type {
            PARSED_DATA_FIELD_TRANSFER => {
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;
                (
                    pack!(Transfer { amount }),
                    [
                        vec![
                            index(PARSED_DATA_FIELD_SOURCE)?,
                            index(PARSED_DATA_FIELD_DESTINATION)?,
                        ],
                        authority_accounts(
                            PARSED_DATA_FIELD_AUTHORITY,
                            PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_TRANSFER_CHECKED => {
                let (amount, decimals) = token_amount()?;
                (
                    pack!(TransferChecked { amount, decimals }),
                    [
                        vec![
                            index(PARSED_DATA_FIELD_SOURCE)?,
                            index(PARSED_DATA_FIELD_MINT)?,
                            index(PARSED_DATA_FIELD_DESTINATION)?,
                        ],
                        authority_accounts(
                            PARSED_DATA_FIELD_AUTHORITY,
                            PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_BURN_CHECKED => {
                let (amount, decimals) = token_amount()?;
                (
                    pack!(BurnChecked { amount, decimals }),
                    [
                        vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_MINT)?],
                        authority_accounts(
                            PARSED_DATA_FIELD_AUTHORITY,
                            PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_BURN => {
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT).unwrap_or(0);
                let account_idx = index(PARSED_DATA_FIELD_ACCOUNT)?;
                let authority = authority_accounts(
                    PARSED_DATA_FIELD_AUTHORITY,
                    PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                )?;
                // Parsed non-checked burns may omit the mint, or name one missing from the key
                // map; fall back to [source, authority].
                let accounts = match index(PARSED_DATA_FIELD_MINT) {
                    Ok(mint_idx) => [vec![account_idx, mint_idx], authority].concat(),
                    Err(_) if authority.len() == 1 => [vec![account_idx], authority].concat(),
                    Err(e) => return Err(e),
                };
                (pack!(Burn { amount }), accounts)
            }
            PARSED_DATA_FIELD_CLOSE_ACCOUNT => (
                pack!(CloseAccount),
                [
                    vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_DESTINATION)?],
                    authority_accounts(PARSED_DATA_FIELD_OWNER, PARSED_DATA_FIELD_MULTISIG_OWNER)?,
                ]
                .concat(),
            ),
            PARSED_DATA_FIELD_APPROVE => {
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;
                (
                    pack!(Approve { amount }),
                    [
                        vec![index(PARSED_DATA_FIELD_SOURCE)?, index(PARSED_DATA_FIELD_DELEGATE)?],
                        authority_accounts(
                            PARSED_DATA_FIELD_OWNER,
                            PARSED_DATA_FIELD_MULTISIG_OWNER,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_APPROVE_CHECKED => {
                let (amount, decimals) = token_amount()?;
                (
                    pack!(ApproveChecked { amount, decimals }),
                    [
                        vec![
                            index(PARSED_DATA_FIELD_SOURCE)?,
                            index(PARSED_DATA_FIELD_MINT)?,
                            index(PARSED_DATA_FIELD_DELEGATE)?,
                        ],
                        authority_accounts(
                            PARSED_DATA_FIELD_OWNER,
                            PARSED_DATA_FIELD_MULTISIG_OWNER,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_REVOKE => (
                pack!(Revoke),
                [
                    vec![index(PARSED_DATA_FIELD_SOURCE)?],
                    authority_accounts(PARSED_DATA_FIELD_OWNER, PARSED_DATA_FIELD_MULTISIG_OWNER)?,
                ]
                .concat(),
            ),
            PARSED_DATA_FIELD_SET_AUTHORITY => {
                // The parser names the target field by authority level: `account` for
                // AccountOwner/CloseAccount, `mint` for all mint-level authority types
                // (MintTokens, FreezeAccount, and the Token-2022 extension authorities).
                // See agave's parse_token.rs `TokenInstruction::SetAuthority` arm.
                let target_field = if info.get(PARSED_DATA_FIELD_ACCOUNT).is_some() {
                    PARSED_DATA_FIELD_ACCOUNT
                } else {
                    PARSED_DATA_FIELD_MINT
                };
                let account_idx = index(target_field)?;
                let current_authority = authority_accounts(
                    PARSED_DATA_FIELD_AUTHORITY,
                    PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                )?;
                let new_authority = optional_pubkey(PARSED_DATA_FIELD_NEW_AUTHORITY)?;

                // authority_type is dropped during parsing and never read downstream; any
                // valid variant lets TokenInstruction::unpack succeed so the policy gate fires.
                let data = if is_spl_token_program {
                    spl_token_interface::instruction::TokenInstruction::SetAuthority {
                        authority_type:
                            spl_token_interface::instruction::AuthorityType::AccountOwner,
                        new_authority: new_authority.into(),
                    }
                    .pack()
                } else {
                    spl_token_2022_interface::instruction::TokenInstruction::SetAuthority {
                        authority_type:
                            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
                        new_authority: new_authority.into(),
                    }
                    .pack()
                };
                (data, [vec![account_idx], current_authority].concat())
            }
            PARSED_DATA_FIELD_MINT_TO => {
                let amount = Self::get_field_as_u64(info, PARSED_DATA_FIELD_AMOUNT)?;
                (
                    pack!(MintTo { amount }),
                    [
                        vec![index(PARSED_DATA_FIELD_MINT)?, index(PARSED_DATA_FIELD_ACCOUNT)?],
                        authority_accounts(
                            PARSED_DATA_FIELD_MINT_AUTHORITY,
                            PARSED_DATA_FIELD_MULTISIG_MINT_AUTHORITY,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_MINT_TO_CHECKED => {
                let (amount, decimals) = token_amount()?;
                (
                    pack!(MintToChecked { amount, decimals }),
                    [
                        vec![index(PARSED_DATA_FIELD_MINT)?, index(PARSED_DATA_FIELD_ACCOUNT)?],
                        authority_accounts(
                            PARSED_DATA_FIELD_MINT_AUTHORITY,
                            PARSED_DATA_FIELD_MULTISIG_MINT_AUTHORITY,
                        )?,
                    ]
                    .concat(),
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_MINT | PARSED_DATA_FIELD_INITIALIZE_MINT2 => {
                let mint_idx = index(PARSED_DATA_FIELD_MINT)?;
                let mint_authority = pubkey(PARSED_DATA_FIELD_MINT_AUTHORITY)?;
                let decimals =
                    u8::try_from(Self::get_field_as_u64(info, PARSED_DATA_FIELD_DECIMALS)?)
                        .map_err(|_| {
                            KoraError::SerializationError(
                                "Mint 'decimals' exceeds u8 range".to_string(),
                            )
                        })?;
                let freeze_authority = optional_pubkey(PARSED_DATA_FIELD_FREEZE_AUTHORITY)?.into();

                if instruction_type == PARSED_DATA_FIELD_INITIALIZE_MINT {
                    let rent_idx = index(PARSED_DATA_FIELD_RENT_SYSVAR)?;
                    (
                        pack!(InitializeMint { decimals, mint_authority, freeze_authority }),
                        vec![mint_idx, rent_idx],
                    )
                } else {
                    (
                        pack!(InitializeMint2 { decimals, mint_authority, freeze_authority }),
                        vec![mint_idx],
                    )
                }
            }
            PARSED_DATA_FIELD_INITIALIZE_ACCOUNT => (
                pack!(InitializeAccount),
                vec![
                    index(PARSED_DATA_FIELD_ACCOUNT)?,
                    index(PARSED_DATA_FIELD_MINT)?,
                    index(PARSED_DATA_FIELD_OWNER)?,
                ],
            ),
            PARSED_DATA_FIELD_INITIALIZE_ACCOUNT2 => {
                let owner = pubkey(PARSED_DATA_FIELD_OWNER)?;
                (
                    pack!(InitializeAccount2 { owner }),
                    vec![
                        index(PARSED_DATA_FIELD_ACCOUNT)?,
                        index(PARSED_DATA_FIELD_MINT)?,
                        index(PARSED_DATA_FIELD_RENT_SYSVAR)?,
                    ],
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_ACCOUNT3 => {
                let owner = pubkey(PARSED_DATA_FIELD_OWNER)?;
                (
                    pack!(InitializeAccount3 { owner }),
                    vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_MINT)?],
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_MULTISIG | PARSED_DATA_FIELD_INITIALIZE_MULTISIG2 => {
                let multisig_idx = index(PARSED_DATA_FIELD_MULTISIG_ACCOUNT)?;
                let m = u8::try_from(Self::get_field_as_u64(info, PARSED_DATA_FIELD_M)?)
                    .map_err(|_| {
                        KoraError::SerializationError(
                            "Multisig threshold 'm' exceeds u8 range".to_string(),
                        )
                    })?;
                let signer_indices = signer_indices()?;

                let (data, mut accounts) =
                    if instruction_type == PARSED_DATA_FIELD_INITIALIZE_MULTISIG {
                        (
                            pack!(InitializeMultisig { m }),
                            vec![multisig_idx, index(PARSED_DATA_FIELD_RENT_SYSVAR)?],
                        )
                    } else {
                        (pack!(InitializeMultisig2 { m }), vec![multisig_idx])
                    };
                accounts.extend(signer_indices);
                (data, accounts)
            }
            PARSED_DATA_FIELD_FREEZE_ACCOUNT => (
                pack!(FreezeAccount),
                [
                    vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_MINT)?],
                    authority_accounts(
                        PARSED_DATA_FIELD_FREEZE_AUTHORITY,
                        PARSED_DATA_FIELD_MULTISIG_FREEZE_AUTHORITY,
                    )?,
                ]
                .concat(),
            ),
            PARSED_DATA_FIELD_THAW_ACCOUNT => (
                pack!(ThawAccount),
                [
                    vec![index(PARSED_DATA_FIELD_ACCOUNT)?, index(PARSED_DATA_FIELD_MINT)?],
                    authority_accounts(
                        PARSED_DATA_FIELD_FREEZE_AUTHORITY,
                        PARSED_DATA_FIELD_MULTISIG_FREEZE_AUTHORITY,
                    )?,
                ]
                .concat(),
            ),
            PARSED_DATA_FIELD_GET_ACCOUNT_DATA_SIZE => {
                let mint_idx = index(PARSED_DATA_FIELD_MINT)?;
                let data = if is_spl_token_program {
                    spl_token_interface::instruction::TokenInstruction::GetAccountDataSize.pack()
                } else {
                    let extension_types = Self::get_token2022_extension_types(info)?;
                    spl_token_2022_interface::instruction::TokenInstruction::GetAccountDataSize {
                        extension_types,
                    }
                    .pack()
                };
                (data, vec![mint_idx])
            }
            PARSED_DATA_FIELD_INITIALIZE_IMMUTABLE_OWNER => {
                (pack!(InitializeImmutableOwner), vec![index(PARSED_DATA_FIELD_ACCOUNT)?])
            }
            PARSED_DATA_FIELD_SYNC_NATIVE => {
                (pack!(SyncNative), vec![index(PARSED_DATA_FIELD_ACCOUNT)?])
            }
            PARSED_DATA_FIELD_BATCH => {
                let inner_instructions = info
                    .get(PARSED_DATA_FIELD_INSTRUCTIONS)
                    .and_then(|value| value.as_array())
                    .ok_or_else(|| {
                        KoraError::SerializationError(
                            "Missing 'instructions' field in batch".to_string(),
                        )
                    })?;

                let mut data = vec![BATCH_DISCRIMINATOR];
                let mut accounts = Vec::new();
                for inner_parsed in inner_instructions {
                    let inner = Self::reconstruct_spl_token_instruction(
                        &solana_transaction_status_client_types::ParsedInstruction {
                            program: parsed.program.clone(),
                            program_id: parsed.program_id.clone(),
                            parsed: inner_parsed.clone(),
                            stack_height: parsed.stack_height,
                        },
                        account_keys_hashmap,
                    )?;
                    let account_count: u8 = inner.accounts.len().try_into().map_err(|_| {
                        KoraError::InvalidTransaction(
                            "Batch sub-instruction account count exceeds u8".to_string(),
                        )
                    })?;
                    let data_len: u8 = inner.data.len().try_into().map_err(|_| {
                        KoraError::InvalidTransaction(
                            "Batch sub-instruction data length exceeds u8".to_string(),
                        )
                    })?;
                    data.push(account_count);
                    data.push(data_len);
                    data.extend_from_slice(&inner.data);
                    accounts.extend_from_slice(&inner.accounts);
                }
                (data, accounts)
            }
            _ => match Self::reconstruct_token2022_extension_instruction(
                &program_id,
                instruction_type,
                info,
            )? {
                Some(instruction) => {
                    let accounts = instruction
                        .accounts
                        .iter()
                        .map(|meta| Self::get_account_index(account_keys_hashmap, &meta.pubkey))
                        .collect::<Result<Vec<_>, _>>()?;
                    (instruction.data, accounts)
                }
                None => {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Unrecognized SPL Token instruction type '{}' in CPI: cannot validate fee payer policy",
                        instruction_type
                    )))
                }
            },
        };

        Ok(CompiledInstruction { program_id_index, accounts, data })
    }

    fn reconstruct_token2022_extension_instruction(
        program_id: &Pubkey,
        instruction_type: &str,
        info: &serde_json::Value,
    ) -> Result<Option<Instruction>, KoraError> {
        use spl_token_2022_interface::{
            extension::{
                group_member_pointer, group_pointer, metadata_pointer, transfer_fee, transfer_hook,
            },
            instruction as token_2022,
        };
        use spl_token_metadata_interface::{instruction as token_metadata, state::Field};

        if *program_id != spl_token_2022_interface::ID {
            return Ok(None);
        }

        let pubkey = |field: &str| Self::get_field_as_pubkey(info, field);
        let optional_pubkey = |field: &str| Self::get_field_as_optional_pubkey(info, field);
        let optional_u64 = |field: &str| -> Result<Option<u64>, KoraError> {
            match info.get(field) {
                Some(v) if !v.is_null() => Self::get_field_as_u64(info, field).map(Some),
                _ => Ok(None),
            }
        };
        let string = |field: &str| Self::get_field_as_str(info, field).map(str::to_string);
        let mint = || pubkey(PARSED_DATA_FIELD_MINT);
        let metadata = || pubkey(PARSED_DATA_FIELD_METADATA);
        let update_authority = || pubkey(PARSED_DATA_FIELD_UPDATE_AUTHORITY);

        let instruction = match instruction_type {
            PARSED_DATA_FIELD_INITIALIZE_METADATA_POINTER => {
                metadata_pointer::instruction::initialize(
                    program_id,
                    &mint()?,
                    optional_pubkey(PARSED_DATA_FIELD_AUTHORITY)?,
                    optional_pubkey(PARSED_DATA_FIELD_METADATA_ADDRESS)?,
                )
            }
            PARSED_DATA_FIELD_UPDATE_METADATA_POINTER => {
                let (authority, signers) = Self::get_authority_and_signers(
                    info,
                    PARSED_DATA_FIELD_AUTHORITY,
                    PARSED_DATA_FIELD_MULTISIG_AUTHORITY,
                )?;
                metadata_pointer::instruction::update(
                    program_id,
                    &mint()?,
                    &authority,
                    &signers.iter().collect::<Vec<_>>(),
                    optional_pubkey(PARSED_DATA_FIELD_METADATA_ADDRESS)?,
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_TOKEN_METADATA => Ok(token_metadata::initialize(
                program_id,
                &metadata()?,
                &update_authority()?,
                &mint()?,
                &pubkey(PARSED_DATA_FIELD_MINT_AUTHORITY)?,
                string(PARSED_DATA_FIELD_NAME)?,
                string(PARSED_DATA_FIELD_SYMBOL)?,
                string(PARSED_DATA_FIELD_URI)?,
            )),
            PARSED_DATA_FIELD_UPDATE_TOKEN_METADATA_FIELD => {
                let field = match Self::get_field_as_str(info, PARSED_DATA_FIELD_FIELD)? {
                    PARSED_DATA_FIELD_NAME => Field::Name,
                    PARSED_DATA_FIELD_SYMBOL => Field::Symbol,
                    PARSED_DATA_FIELD_URI => Field::Uri,
                    key => Field::Key(key.to_string()),
                };
                Ok(token_metadata::update_field(
                    program_id,
                    &metadata()?,
                    &update_authority()?,
                    field,
                    string(PARSED_DATA_FIELD_VALUE)?,
                ))
            }
            PARSED_DATA_FIELD_REMOVE_TOKEN_METADATA_KEY => {
                let idempotent = info
                    .get(PARSED_DATA_FIELD_IDEMPOTENT)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                    KoraError::SerializationError(
                        "Missing or invalid 'idempotent' field".to_string(),
                    )
                })?;
                Ok(token_metadata::remove_key(
                    program_id,
                    &metadata()?,
                    &update_authority()?,
                    string(PARSED_DATA_FIELD_KEY)?,
                    idempotent,
                ))
            }
            PARSED_DATA_FIELD_UPDATE_TOKEN_METADATA_AUTHORITY => {
                let new_authority =
                    optional_pubkey(PARSED_DATA_FIELD_NEW_AUTHORITY)?.try_into().map_err(|_| {
                        KoraError::SerializationError(
                            "Field 'newAuthority' is not a valid metadata authority".to_string(),
                        )
                    })?;
                Ok(token_metadata::update_authority(
                    program_id,
                    &metadata()?,
                    &update_authority()?,
                    new_authority,
                ))
            }
            PARSED_DATA_FIELD_EMIT_TOKEN_METADATA => Ok(token_metadata::emit(
                program_id,
                &metadata()?,
                optional_u64(PARSED_DATA_FIELD_START)?,
                optional_u64(PARSED_DATA_FIELD_END)?,
            )),
            PARSED_DATA_FIELD_INITIALIZE_MINT_CLOSE_AUTHORITY => {
                token_2022::initialize_mint_close_authority(
                    program_id,
                    &mint()?,
                    optional_pubkey(PARSED_DATA_FIELD_NEW_AUTHORITY)?.as_ref(),
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_PERMANENT_DELEGATE => {
                token_2022::initialize_permanent_delegate(
                    program_id,
                    &mint()?,
                    &pubkey(PARSED_DATA_FIELD_DELEGATE)?,
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_NON_TRANSFERABLE_MINT => {
                token_2022::initialize_non_transferable_mint(program_id, &mint()?)
            }
            PARSED_DATA_FIELD_INITIALIZE_TRANSFER_HOOK => transfer_hook::instruction::initialize(
                program_id,
                &mint()?,
                optional_pubkey(PARSED_DATA_FIELD_AUTHORITY)?,
                optional_pubkey(PARSED_DATA_FIELD_PROGRAM_ID)?,
            ),
            PARSED_DATA_FIELD_INITIALIZE_GROUP_POINTER => group_pointer::instruction::initialize(
                program_id,
                &mint()?,
                optional_pubkey(PARSED_DATA_FIELD_AUTHORITY)?,
                optional_pubkey(PARSED_DATA_FIELD_GROUP_ADDRESS)?,
            ),
            PARSED_DATA_FIELD_INITIALIZE_GROUP_MEMBER_POINTER => {
                group_member_pointer::instruction::initialize(
                    program_id,
                    &mint()?,
                    optional_pubkey(PARSED_DATA_FIELD_AUTHORITY)?,
                    optional_pubkey(PARSED_DATA_FIELD_MEMBER_ADDRESS)?,
                )
            }
            PARSED_DATA_FIELD_INITIALIZE_TRANSFER_FEE_CONFIG => {
                let basis_points = u16::try_from(Self::get_field_as_u64(
                    info,
                    PARSED_DATA_FIELD_TRANSFER_FEE_BASIS_POINTS,
                )?)
                .map_err(|_| {
                    KoraError::SerializationError(
                        "'transferFeeBasisPoints' exceeds u16 range".to_string(),
                    )
                })?;
                transfer_fee::instruction::initialize_transfer_fee_config(
                    program_id,
                    &mint()?,
                    optional_pubkey(PARSED_DATA_FIELD_TRANSFER_FEE_CONFIG_AUTHORITY)?.as_ref(),
                    optional_pubkey(PARSED_DATA_FIELD_WITHDRAW_WITHHELD_AUTHORITY)?.as_ref(),
                    basis_points,
                    Self::get_field_as_u64(info, PARSED_DATA_FIELD_MAXIMUM_FEE)?,
                )
            }
            _ => return Ok(None),
        };

        instruction.map(Some).map_err(|e| {
            KoraError::InvalidTransaction(format!(
                "Invalid Token-2022 '{instruction_type}' instruction in CPI: {}",
                sanitize_error!(e)
            ))
        })
    }
}
