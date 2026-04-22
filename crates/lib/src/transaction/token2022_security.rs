use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{
        group_member_pointer::instruction::GroupMemberPointerInstruction,
        group_pointer::instruction::GroupPointerInstruction,
        interest_bearing_mint::instruction::InterestBearingMintInstruction,
        metadata_pointer::instruction::MetadataPointerInstruction,
        pausable::instruction::PausableInstruction,
        scaled_ui_amount::instruction::ScaledUiAmountMintInstruction,
        transfer_fee::instruction::TransferFeeInstruction,
        transfer_hook::instruction::TransferHookInstruction, ExtensionType,
    },
    instruction::{decode_instruction_data, decode_instruction_type, TokenInstruction},
};

use crate::{error::KoraError, sanitize_error};

#[derive(Debug, Clone, PartialEq)]
pub struct Token2022SecurityField {
    pub context: &'static str,
    pub pubkey: Pubkey,
    pub plants_extension_authority: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token2022SecurityInstruction {
    pub instruction_name: &'static str,
    pub extension_type: Option<ExtensionType>,
    pub accounts: Vec<Pubkey>,
    pub reject_if_fee_payer_in_accounts: bool,
    pub update_authority: Option<Pubkey>,
    pub multisig_signers: Vec<Pubkey>,
    pub data_pubkeys: Vec<Token2022SecurityField>,
}

impl Token2022SecurityInstruction {
    pub fn uses_fee_payer_as_update_authority(&self, fee_payer: &Pubkey) -> bool {
        self.update_authority == Some(*fee_payer) || self.multisig_signers.contains(fee_payer)
    }

    pub fn planted_fee_payer_authority(
        &self,
        fee_payer: &Pubkey,
    ) -> Option<&Token2022SecurityField> {
        self.data_pubkeys
            .iter()
            .find(|field| field.plants_extension_authority && field.pubkey == *fee_payer)
    }
}

pub(crate) struct Token2022SecurityParser;

impl Token2022SecurityParser {
    pub fn parse(
        instructions: &[Instruction],
    ) -> Result<Vec<Token2022SecurityInstruction>, KoraError> {
        let mut parsed = Vec::new();

        for instruction in instructions {
            if instruction.program_id != spl_token_2022_interface::id() {
                continue;
            }

            let token_instruction = TokenInstruction::unpack(&instruction.data).map_err(|e| {
                KoraError::InvalidTransaction(format!(
                    "Failed to parse Token-2022 instruction for security validation: {}",
                    sanitize_error!(e)
                ))
            })?;

            match token_instruction {
                TokenInstruction::InitializeMintCloseAuthority { close_authority } => {
                    parsed.push(Token2022SecurityInstruction {
                        instruction_name: "Token2022 InitializeMintCloseAuthority",
                        extension_type: Some(ExtensionType::MintCloseAuthority),
                        accounts: Self::instruction_accounts(instruction),
                        reject_if_fee_payer_in_accounts: false,
                        update_authority: None,
                        multisig_signers: vec![],
                        data_pubkeys: Self::optional_field(
                            close_authority.into(),
                            "Token2022 InitializeMintCloseAuthority newAuthority",
                            true,
                        )
                        .into_iter()
                        .collect(),
                    });
                }
                TokenInstruction::InitializePermanentDelegate { delegate } => {
                    parsed.push(Token2022SecurityInstruction {
                        instruction_name: "Token2022 InitializePermanentDelegate",
                        extension_type: Some(ExtensionType::PermanentDelegate),
                        accounts: Self::instruction_accounts(instruction),
                        reject_if_fee_payer_in_accounts: false,
                        update_authority: None,
                        multisig_signers: vec![],
                        data_pubkeys: vec![Token2022SecurityField {
                            context: "Token2022 InitializePermanentDelegate delegate",
                            pubkey: delegate,
                            plants_extension_authority: true,
                        }],
                    });
                }
                TokenInstruction::TransferFeeExtension => {
                    if let Some(parsed_instruction) =
                        Self::parse_transfer_fee_extension(instruction)?
                    {
                        parsed.push(parsed_instruction);
                    }
                }
                TokenInstruction::InterestBearingMintExtension => {
                    if let Some(parsed_instruction) =
                        Self::parse_interest_bearing_extension(instruction)?
                    {
                        parsed.push(parsed_instruction);
                    }
                }
                TokenInstruction::TransferHookExtension => {
                    if let Some(parsed_instruction) =
                        Self::parse_transfer_hook_extension(instruction)?
                    {
                        parsed.push(parsed_instruction);
                    }
                }
                TokenInstruction::MetadataPointerExtension => {
                    parsed.push(Self::parse_metadata_pointer_extension(instruction)?);
                }
                TokenInstruction::GroupPointerExtension => {
                    parsed.push(Self::parse_group_pointer_extension(instruction)?);
                }
                TokenInstruction::GroupMemberPointerExtension => {
                    parsed.push(Self::parse_group_member_pointer_extension(instruction)?);
                }
                TokenInstruction::ScaledUiAmountExtension => {
                    parsed.push(Self::parse_scaled_ui_amount_extension(instruction)?);
                }
                TokenInstruction::PausableExtension => {
                    if let Some(parsed_instruction) = Self::parse_pausable_extension(instruction)? {
                        parsed.push(parsed_instruction);
                    }
                }
                TokenInstruction::DefaultAccountStateExtension
                | TokenInstruction::MemoTransferExtension
                | TokenInstruction::CpiGuardExtension
                | TokenInstruction::InitializeNonTransferableMint
                | TokenInstruction::WithdrawExcessLamports => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "unsupported Token-2022 extension instruction",
                    ));
                }
                _ => {}
            }
        }

        Ok(parsed)
    }

    fn parse_transfer_fee_extension(
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
                reject_if_fee_payer_in_accounts: false,
                update_authority: None,
                multisig_signers: vec![],
                data_pubkeys: [
                    Self::optional_field(
                        transfer_fee_config_authority.into(),
                        "Token2022 InitializeTransferFeeConfig transferFeeConfigAuthority",
                        true,
                    ),
                    Self::optional_field(
                        withdraw_withheld_authority.into(),
                        "Token2022 InitializeTransferFeeConfig withdrawWithheldAuthority",
                        true,
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
                    reject_if_fee_payer_in_accounts: false,
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
                    reject_if_fee_payer_in_accounts: false,
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
                    reject_if_fee_payer_in_accounts: false,
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
                )))
            }
        }
    }

    fn parse_interest_bearing_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let interest_bearing_instruction = Self::decode_extension_type::<
            InterestBearingMintInstruction,
        >(instruction, "InterestBearing")?;

        match interest_bearing_instruction {
            InterestBearingMintInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::interest_bearing_mint::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 InterestBearing initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeInterestBearingConfig",
                    extension_type: Some(ExtensionType::InterestBearingConfig),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: Self::optional_field(
                        initialize.rate_authority.into(),
                        "Token2022 InitializeInterestBearingConfig rateAuthority",
                        true,
                    )
                    .into_iter()
                    .collect(),
                }))
            }
            InterestBearingMintInstruction::UpdateRate => Ok(Some(Token2022SecurityInstruction {
                instruction_name: "Token2022 UpdateInterestBearingConfigRate",
                extension_type: Some(ExtensionType::InterestBearingConfig),
                accounts: Self::instruction_accounts(instruction),
                reject_if_fee_payer_in_accounts: false,
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

    fn parse_transfer_hook_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let transfer_hook_instruction =
            Self::decode_extension_type::<TransferHookInstruction>(instruction, "TransferHook")?;

        match transfer_hook_instruction {
            TransferHookInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::transfer_hook::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 TransferHook initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeTransferHook",
                    extension_type: Some(ExtensionType::TransferHook),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeTransferHook authority",
                            true,
                        ),
                        Self::optional_field(
                            initialize.program_id.into(),
                            "Token2022 InitializeTransferHook program_id",
                            false,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                }))
            }
            TransferHookInstruction::Update => {
                let update =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::transfer_hook::instruction::UpdateInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 TransferHook update instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 TransferHookUpdate",
                    extension_type: Some(ExtensionType::TransferHook),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 TransferHookUpdate authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.program_id.into(),
                        "Token2022 TransferHookUpdate program_id",
                        false,
                    )
                    .into_iter()
                    .collect(),
                }))
            }
        }
    }

    fn parse_metadata_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let metadata_pointer_instruction = Self::decode_extension_type::<MetadataPointerInstruction>(
            instruction,
            "MetadataPointer",
        )?;

        match metadata_pointer_instruction {
            MetadataPointerInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::metadata_pointer::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 MetadataPointer initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeMetadataPointer",
                    extension_type: Some(ExtensionType::MetadataPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeMetadataPointer authority",
                            true,
                        ),
                        Self::optional_field(
                            initialize.metadata_address.into(),
                            "Token2022 InitializeMetadataPointer metadataAddress",
                            false,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            MetadataPointerInstruction::Update => {
                let update =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::metadata_pointer::instruction::UpdateInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 MetadataPointer update instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateMetadataPointer",
                    extension_type: Some(ExtensionType::MetadataPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateMetadataPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.metadata_address.into(),
                        "Token2022 UpdateMetadataPointer metadataAddress",
                        false,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    fn parse_group_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let group_pointer_instruction =
            Self::decode_extension_type::<GroupPointerInstruction>(instruction, "GroupPointer")?;

        match group_pointer_instruction {
            GroupPointerInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::group_pointer::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 GroupPointer initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeGroupPointer",
                    extension_type: Some(ExtensionType::GroupPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeGroupPointer authority",
                            true,
                        ),
                        Self::optional_field(
                            initialize.group_address.into(),
                            "Token2022 InitializeGroupPointer groupAddress",
                            false,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            GroupPointerInstruction::Update => {
                let update =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::group_pointer::instruction::UpdateInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 GroupPointer update instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateGroupPointer",
                    extension_type: Some(ExtensionType::GroupPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateGroupPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.group_address.into(),
                        "Token2022 UpdateGroupPointer groupAddress",
                        false,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    fn parse_group_member_pointer_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let group_member_pointer_instruction = Self::decode_extension_type::<
            GroupMemberPointerInstruction,
        >(instruction, "GroupMemberPointer")?;

        match group_member_pointer_instruction {
            GroupMemberPointerInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::group_member_pointer::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 GroupMemberPointer initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeGroupMemberPointer",
                    extension_type: Some(ExtensionType::GroupMemberPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: [
                        Self::optional_field(
                            initialize.authority.into(),
                            "Token2022 InitializeGroupMemberPointer authority",
                            true,
                        ),
                        Self::optional_field(
                            initialize.member_address.into(),
                            "Token2022 InitializeGroupMemberPointer memberAddress",
                            false,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                })
            }
            GroupMemberPointerInstruction::Update => {
                let update =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::group_member_pointer::instruction::UpdateInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 GroupMemberPointer update instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 UpdateGroupMemberPointer",
                    extension_type: Some(ExtensionType::GroupMemberPointer),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 UpdateGroupMemberPointer authority",
                    )?),
                    multisig_signers: Self::extract_multisig_signers(instruction, 2, None),
                    data_pubkeys: Self::optional_field(
                        update.member_address.into(),
                        "Token2022 UpdateGroupMemberPointer memberAddress",
                        false,
                    )
                    .into_iter()
                    .collect(),
                })
            }
        }
    }

    fn parse_scaled_ui_amount_extension(
        instruction: &Instruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let scaled_ui_amount_instruction = Self::decode_extension_type::<
            ScaledUiAmountMintInstruction,
        >(instruction, "ScaledUiAmount")?;

        match scaled_ui_amount_instruction {
            ScaledUiAmountMintInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::scaled_ui_amount::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 ScaledUiAmount initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializeScaledUiAmountConfig",
                    extension_type: Some(ExtensionType::ScaledUiAmount),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: Self::optional_field(
                        initialize.authority.into(),
                        "Token2022 InitializeScaledUiAmountConfig authority",
                        true,
                    )
                    .into_iter()
                    .collect(),
                })
            }
            ScaledUiAmountMintInstruction::UpdateMultiplier => Ok(Token2022SecurityInstruction {
                instruction_name: "Token2022 UpdateScaledUiAmountConfigMultiplier",
                extension_type: Some(ExtensionType::ScaledUiAmount),
                accounts: Self::instruction_accounts(instruction),
                reject_if_fee_payer_in_accounts: false,
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

    fn parse_pausable_extension(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        let pausable_instruction =
            Self::decode_extension_type::<PausableInstruction>(instruction, "Pausable")?;

        match pausable_instruction {
            PausableInstruction::Initialize => {
                let initialize =
                    *decode_instruction_data::<
                        spl_token_2022_interface::extension::pausable::instruction::InitializeInstructionData,
                    >(&instruction.data[1..])
                    .map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Failed to parse Token-2022 Pausable initialize instruction: {}",
                            sanitize_error!(e)
                        ))
                    })?;

                Ok(Some(Token2022SecurityInstruction {
                    instruction_name: "Token2022 InitializePausable",
                    extension_type: Some(ExtensionType::Pausable),
                    accounts: Self::instruction_accounts(instruction),
                    reject_if_fee_payer_in_accounts: false,
                    update_authority: None,
                    multisig_signers: vec![],
                    data_pubkeys: vec![Token2022SecurityField {
                        context: "Token2022 InitializePausable authority",
                        pubkey: initialize.authority,
                        plants_extension_authority: true,
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

    fn optional_field(
        pubkey: Option<Pubkey>,
        context: &'static str,
        plants_extension_authority: bool,
    ) -> Option<Token2022SecurityField> {
        pubkey.map(|pubkey| Token2022SecurityField { context, pubkey, plants_extension_authority })
    }

    fn required_account(
        instruction: &Instruction,
        index: usize,
        context: &'static str,
    ) -> Result<Pubkey, KoraError> {
        instruction.accounts.get(index).map(|account| account.pubkey).ok_or_else(|| {
            KoraError::InvalidTransaction(format!("{context} is missing the required account meta"))
        })
    }

    fn extract_multisig_signers(
        instruction: &Instruction,
        start: usize,
        end: Option<usize>,
    ) -> Vec<Pubkey> {
        let end = end.unwrap_or(instruction.accounts.len());
        instruction
            .accounts
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .map(|account| account.pubkey)
            .collect()
    }

    fn instruction_accounts(instruction: &Instruction) -> Vec<Pubkey> {
        instruction.accounts.iter().map(|account| account.pubkey).collect()
    }

    fn unsupported_fee_payer_account_check(
        instruction: &Instruction,
        instruction_name: &'static str,
    ) -> Token2022SecurityInstruction {
        Token2022SecurityInstruction {
            instruction_name,
            extension_type: None,
            accounts: Self::instruction_accounts(instruction),
            reject_if_fee_payer_in_accounts: true,
            update_authority: None,
            multisig_signers: vec![],
            data_pubkeys: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::instruction::AccountMeta;

    #[test]
    fn test_parse_metadata_pointer_initialize_security_fields() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();

        let instruction =
            spl_token_2022_interface::extension::metadata_pointer::instruction::initialize(
                &spl_token_2022_interface::id(),
                &mint,
                Some(authority),
                Some(metadata_address),
            )
            .unwrap();

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].instruction_name, "Token2022 InitializeMetadataPointer");
        assert_eq!(parsed[0].extension_type, Some(ExtensionType::MetadataPointer));
        assert_eq!(
            parsed[0].planted_fee_payer_authority(&authority).map(|field| field.context),
            Some("Token2022 InitializeMetadataPointer authority")
        );
        assert!(parsed[0].data_pubkeys.iter().any(|field| field.pubkey == metadata_address
            && field.context == "Token2022 InitializeMetadataPointer metadataAddress"));
    }

    #[test]
    fn test_parse_transfer_fee_withdraw_from_accounts_uses_only_signer_slice() {
        let authority = Pubkey::new_unique();
        let signer = Pubkey::new_unique();
        let source_1 = Pubkey::new_unique();
        let source_2 = Pubkey::new_unique();
        let mut data = TokenInstruction::TransferFeeExtension.pack();
        spl_token_2022_interface::extension::transfer_fee::instruction::TransferFeeInstruction::WithdrawWithheldTokensFromAccounts {
            num_token_accounts: 2,
        }
        .pack(&mut data);

        let instruction = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(authority, false),
                AccountMeta::new_readonly(signer, true),
                AccountMeta::new(source_1, false),
                AccountMeta::new(source_2, false),
            ],
            data,
        };

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].update_authority,
            Some(authority),
            "authority should come from the dedicated authority account"
        );
        assert_eq!(
            parsed[0].multisig_signers,
            vec![signer],
            "source accounts should not be treated as multisig signers"
        );
    }
}
