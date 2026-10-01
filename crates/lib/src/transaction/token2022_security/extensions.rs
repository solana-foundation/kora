use solana_sdk::instruction::Instruction;
use spl_token_2022_interface::{
    extension::{
        group_member_pointer::{self, instruction::GroupMemberPointerInstruction},
        group_pointer::{self, instruction::GroupPointerInstruction},
        interest_bearing_mint::{self, instruction::InterestBearingMintInstruction},
        metadata_pointer::{self, instruction::MetadataPointerInstruction},
        pausable::{self, instruction::PausableInstruction},
        scaled_ui_amount::{self, instruction::ScaledUiAmountMintInstruction},
        transfer_fee::instruction::TransferFeeInstruction,
        transfer_hook::{self, instruction::TransferHookInstruction},
        ExtensionType,
    },
    instruction::{decode_instruction_data, decode_instruction_type},
};

use crate::{error::KoraError, sanitize_error};

use super::{
    Token2022AccountUsagePolicy, Token2022FieldRole, Token2022SecurityField,
    Token2022SecurityInstruction, Token2022SecurityParser,
};

impl Token2022SecurityParser {
    pub(super) fn parse_transfer_fee_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        if instruction.data.len() < 2 {
            return Err(KoraError::InvalidTransaction(
                "Failed to parse Token-2022 TransferFee instruction".to_string(),
            ));
        }

        let transfer_fee_instruction = TransferFeeInstruction::unpack(&instruction.data[1..])
            .map_err(|e| {
                KoraError::InvalidTransaction(format!(
                    "Failed to parse Token-2022 TransferFee instruction: {}",
                    sanitize_error!(e)
                ))
            })?;

        match transfer_fee_instruction {
            TransferFeeInstruction::InitializeTransferFeeConfig {
                transfer_fee_config_authority,
                withdraw_withheld_authority,
                ..
            } => Ok(Some(Token2022SecurityInstruction {
                instruction_name: "Token2022 InitializeTransferFeeConfig",
                extension_type: Some(ExtensionType::TransferFeeConfig),
                accounts: Self::instruction_accounts(instruction),
                account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                update_authority: None,
                multisig_signers: vec![],
                data_pubkeys: [
                    Self::optional_field(
                        transfer_fee_config_authority.into(),
                        "Token2022 InitializeTransferFeeConfig transferFeeConfigAuthority",
                        Token2022FieldRole::PlantedAuthority,
                    ),
                    Self::optional_field(
                        withdraw_withheld_authority.into(),
                        "Token2022 InitializeTransferFeeConfig withdrawWithheldAuthority",
                        Token2022FieldRole::PlantedAuthority,
                    ),
                ]
                .into_iter()
                .flatten()
                .collect(),
            })),
            TransferFeeInstruction::SetTransferFee { .. } => {
                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 SetTransferFee",
                    extension_type: Some(ExtensionType::TransferFeeConfig),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 SetTransferFee authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: vec![],
                }))
            }
            TransferFeeInstruction::WithdrawWithheldTokensFromMint => {
                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 WithdrawWithheldTokensFromMint",
                    extension_type: Some(ExtensionType::TransferFeeConfig),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        2,
                        "Token2022 WithdrawWithheldTokensFromMint withdrawWithheldAuthority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 3, None),
                    data_pubkeys: vec![],
                }))
            }
            TransferFeeInstruction::WithdrawWithheldTokensFromAccounts { num_token_accounts } => {
                let signer_end =
                    instruction.accounts.len().saturating_sub(num_token_accounts as usize);
                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 WithdrawWithheldTokensFromAccounts",
                    extension_type: Some(ExtensionType::TransferFeeConfig),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        2,
                        "Token2022 WithdrawWithheldTokensFromAccounts withdrawWithheldAuthority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(
                        instruction,
                        3,
                        Some(signer_end),
                    ),
                    data_pubkeys: vec![],
                }))
            }
            TransferFeeInstruction::TransferCheckedWithFee { .. } => Ok(None),
            TransferFeeInstruction::HarvestWithheldTokensToMint => {
                Ok(Some(Self::unsupported_fee_payer_account_check(
                    instruction,
                    "Token2022 HarvestWithheldTokensToMint",
                    None,
                )))
            }
        }
    }

    pub(super) fn parse_interest_bearing_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let interest_bearing_instruction = Self::decode_extension_type::<
            InterestBearingMintInstruction,
        >(instruction, "InterestBearing")?;

        match interest_bearing_instruction {
            InterestBearingMintInstruction::Initialize => {
                let initialize: interest_bearing_mint::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "InterestBearing initialize",
                    )?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeInterestBearingConfig",
                    extension_type: Some(ExtensionType::InterestBearingConfig),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: Self::optional_field(
                        initialize.rate_authority.into(),
                        "Token2022 InitializeInterestBearingConfig rateAuthority",
                        Token2022FieldRole::PlantedAuthority,
                    )
                    .into_iter()
                    .collect(),
                }))
            }
            InterestBearingMintInstruction::UpdateRate => Ok(Some(Token2022SecurityInstruction {
                instruction_name: "Token2022 UpdateInterestBearingConfigRate",
                extension_type: Some(ExtensionType::InterestBearingConfig),
                accounts: Self::instruction_accounts(instruction),
                account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                update_authority: Some(Self::required_account(
                    instruction,
                    1,
                    "Token2022 UpdateInterestBearingConfigRate rateAuthority",
                )?),
                multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                data_pubkeys: vec![],
            })),
        }
    }

    pub(super) fn parse_transfer_hook_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let transfer_hook_instruction =
            Self::decode_extension_type::<TransferHookInstruction>(instruction, "TransferHook")?;

        match transfer_hook_instruction {
            TransferHookInstruction::Initialize => {
                let initialize: transfer_hook::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "TransferHook initialize",
                    )?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeTransferHook",
                    extension_type: Some(ExtensionType::TransferHook),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeTransferHook authority",
                            Token2022FieldRole::PlantedAuthority,
                        ),
                        Self::optional_field(
                            initialize.program_id.into(),
                            "Token2022 InitializeTransferHook program_id",
                            Token2022FieldRole::ProgramId,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                }))
            }
            TransferHookInstruction::Update => {
                let update: transfer_hook::instruction::UpdateInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "TransferHook update",
                    )?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 TransferHookUpdate",
                    extension_type: Some(ExtensionType::TransferHook),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 TransferHookUpdate authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.program_id.into(),
                        "Token2022 TransferHookUpdate program_id",
                        Token2022FieldRole::ProgramId,
                    )
                    .into_iter()
                    .collect(),
                }))
            }
        }
    }

    pub(super) fn parse_metadata_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let metadata_pointer_instruction = Self::decode_extension_type::<MetadataPointerInstruction>(
            instruction,
            "MetadataPointer",
        )?;

        match metadata_pointer_instruction {
            MetadataPointerInstruction::Initialize => {
                let initialize: metadata_pointer::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "MetadataPointer initialize",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeMetadataPointer",
                    extension_type: Some(ExtensionType::MetadataPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeMetadataPointer authority",
                            Token2022FieldRole::PlantedAuthority,
                        ),
                        Self::optional_field(
                            initialize.metadata_address.into(),
                            "Token2022 InitializeMetadataPointer metadataAddress",
                            Token2022FieldRole::Reference,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            MetadataPointerInstruction::Update => {
                let update: metadata_pointer::instruction::UpdateInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "MetadataPointer update",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateMetadataPointer",
                    extension_type: Some(ExtensionType::MetadataPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateMetadataPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.metadata_address.into(),
                        "Token2022 UpdateMetadataPointer metadataAddress",
                        Token2022FieldRole::Reference,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    pub(super) fn parse_group_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let group_pointer_instruction =
            Self::decode_extension_type::<GroupPointerInstruction>(instruction, "GroupPointer")?;

        match group_pointer_instruction {
            GroupPointerInstruction::Initialize => {
                let initialize: group_pointer::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "GroupPointer initialize",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeGroupPointer",
                    extension_type: Some(ExtensionType::GroupPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeGroupPointer authority",
                            Token2022FieldRole::PlantedAuthority,
                        ),
                        Self::optional_field(
                            initialize.group_address.into(),
                            "Token2022 InitializeGroupPointer groupAddress",
                            Token2022FieldRole::Reference,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            GroupPointerInstruction::Update => {
                let update: group_pointer::instruction::UpdateInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "GroupPointer update",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateGroupPointer",
                    extension_type: Some(ExtensionType::GroupPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateGroupPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.group_address.into(),
                        "Token2022 UpdateGroupPointer groupAddress",
                        Token2022FieldRole::Reference,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    pub(super) fn parse_group_member_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let group_member_pointer_instruction = Self::decode_extension_type::<
            GroupMemberPointerInstruction,
        >(instruction, "GroupMemberPointer")?;

        match group_member_pointer_instruction {
            GroupMemberPointerInstruction::Initialize => {
                let initialize: group_member_pointer::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "GroupMemberPointer initialize",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeGroupMemberPointer",
                    extension_type: Some(ExtensionType::GroupMemberPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeGroupMemberPointer authority",
                            Token2022FieldRole::PlantedAuthority,
                        ),
                        Self::optional_field(
                            initialize.member_address.into(),
                            "Token2022 InitializeGroupMemberPointer memberAddress",
                            Token2022FieldRole::Reference,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            GroupMemberPointerInstruction::Update => {
                let update: group_member_pointer::instruction::UpdateInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "GroupMemberPointer update",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateGroupMemberPointer",
                    extension_type: Some(ExtensionType::GroupMemberPointer),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateGroupMemberPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.member_address.into(),
                        "Token2022 UpdateGroupMemberPointer memberAddress",
                        Token2022FieldRole::Reference,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    pub(super) fn parse_scaled_ui_amount_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let scaled_ui_amount_instruction = Self::decode_extension_type::<
            ScaledUiAmountMintInstruction,
        >(instruction, "ScaledUiAmount")?;

        match scaled_ui_amount_instruction {
            ScaledUiAmountMintInstruction::Initialize => {
                let initialize: scaled_ui_amount::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "ScaledUiAmount initialize",
                    )?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeScaledUiAmountConfig",
                    extension_type: Some(ExtensionType::ScaledUiAmount),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: Self::optional_field(
                        initialize.authority.into(),
                        "Token2022 InitializeScaledUiAmountConfig authority",
                        Token2022FieldRole::PlantedAuthority,
                    )
                    .into_iter()
                    .collect(),
                })
            }
            ScaledUiAmountMintInstruction::UpdateMultiplier => Ok(Token2022SecurityInstruction {
                instruction_name: "Token2022 UpdateScaledUiAmountConfigMultiplier",
                extension_type: Some(ExtensionType::ScaledUiAmount),
                accounts: Self::instruction_accounts(instruction),
                account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                update_authority: Some(Self::required_account(
                    instruction,
                    1,
                    "Token2022 UpdateScaledUiAmountConfigMultiplier authority",
                )?),
                multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                data_pubkeys: vec![],
            }),
        }
    }

    pub(super) fn parse_pausable_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let pausable_instruction =
            Self::decode_extension_type::<PausableInstruction>(instruction, "Pausable")?;

        match pausable_instruction {
            PausableInstruction::Initialize => {
                let initialize: pausable::instruction::InitializeInstructionData =
                    Self::extension_data(
                        decode_instruction_data(&instruction.data[1..]),
                        "Pausable initialize",
                    )?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializePausable",
                    extension_type: Some(ExtensionType::Pausable),
                    accounts: Self::instruction_accounts(instruction),
                    account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: vec![Token2022SecurityField {
                        context: "Token2022 InitializePausable authority",
                        pubkey: initialize.authority,
                        role: Token2022FieldRole::PlantedAuthority,
                    }],
                }))
            }
            PausableInstruction::Pause | PausableInstruction::Resume => Ok(None),
        }
    }

    fn decode_extension_type<T>(instruction: &Instruction, name: &str) -> Result<T, KoraError>
    where
        T: TryFrom<u8>,
    {
        if instruction.data.len() < 2 {
            return Err(KoraError::InvalidTransaction(format!(
                "Failed to parse Token-2022 {name} instruction"
            )));
        }

        decode_instruction_type::<T>(&instruction.data[1..]).map_err(|e| {
            KoraError::InvalidTransaction(format!(
                "Failed to parse Token-2022 {name} instruction: {}",
                sanitize_error!(e)
            ))
        })
    }

    fn extension_data<T: Copy, E: std::fmt::Display>(
        decoded: Result<&T, E>,
        name: &str,
    ) -> Result<T, KoraError> {
        decoded.copied().map_err(|e| {
            KoraError::InvalidTransaction(format!(
                "Failed to parse Token-2022 {name} instruction: {}",
                sanitize_error!(e)
            ))
        })
    }
}
