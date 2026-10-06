use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022_interface::{extension::ExtensionType, instruction::TokenInstruction};

use crate::{error::KoraError, sanitize_error, transaction::TOKEN_2022_BATCH_UNSUPPORTED};

mod extensions;
mod interface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token2022FieldRole {
    Reference,
    ProgramId,
    PlantedAuthority,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token2022SecurityField {
    pub context: &'static str,
    pub pubkey: Pubkey,
    pub role: Token2022FieldRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token2022AccountUsagePolicy {
    Ignore,
    RejectIfFeePayerPresent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token2022SecurityInstruction {
    pub instruction_name: &'static str,
    pub extension_type: Option<ExtensionType>,
    pub accounts: Vec<Pubkey>,
    pub account_usage_policy: Token2022AccountUsagePolicy,
    pub update_authority: Option<Pubkey>,
    pub multisig_signers: Vec<Pubkey>,
    pub data_pubkeys: Vec<Token2022SecurityField>,
}

impl Token2022SecurityInstruction {
    pub fn uses_fee_payer_as_current_extension_authority(&self, fee_payer: &Pubkey) -> bool {
        self.update_authority == Some(*fee_payer) || self.multisig_signers.contains(fee_payer)
    }

    pub fn find_planted_fee_payer_authority(
        &self,
        fee_payer: &Pubkey,
    ) -> Option<&Token2022SecurityField> {
        self.data_pubkeys.iter().find(|field| {
            matches!(field.role, Token2022FieldRole::PlantedAuthority) && field.pubkey == *fee_payer
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Token2022InterfaceFamily {
    TokenMetadata,
    TokenGroup,
}

impl Token2022InterfaceFamily {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::TokenMetadata => "token-metadata",
            Self::TokenGroup => "token-group",
        }
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

            let token_instruction = match TokenInstruction::unpack(&instruction.data) {
                Ok(token_instruction) => token_instruction,
                Err(e) => {
                    // Token-2022 also processes the token-metadata and token-group
                    // interface instructions, which are not TokenInstruction variants.
                    // Mirror the on-chain program's dispatch fallback so their accounts
                    // and planted authorities are still policy-validated.
                    if let Some(parsed_instruction) =
                        Self::parse_token_2022_interface_instruction(instruction)?
                    {
                        parsed.push(parsed_instruction);
                        continue;
                    }

                    return Err(KoraError::InvalidTransaction(format!(
                        "Failed to parse Token-2022 instruction for security validation: {}",
                        sanitize_error!(e)
                    )));
                }
            };

            match token_instruction {
                TokenInstruction::InitializeMintCloseAuthority { close_authority } => {
                    parsed.push(Token2022SecurityInstruction {
                        instruction_name: "Token2022 InitializeMintCloseAuthority",
                        extension_type: Some(ExtensionType::MintCloseAuthority),
                        accounts: Self::instruction_accounts(instruction),
                        account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                        update_authority: None,
                        multisig_signers: vec![],
                        data_pubkeys: Self::optional_field(
                            close_authority.into(),
                            "Token2022 InitializeMintCloseAuthority newAuthority",
                            Token2022FieldRole::PlantedAuthority,
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
                        account_usage_policy: Token2022AccountUsagePolicy::Ignore,
                        update_authority: None,
                        multisig_signers: vec![],
                        data_pubkeys: vec![Token2022SecurityField {
                            context: "Token2022 InitializePermanentDelegate delegate",
                            pubkey: delegate,
                            role: Token2022FieldRole::PlantedAuthority,
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
                TokenInstruction::DefaultAccountStateExtension => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "DefaultAccountState extension instruction",
                        Some(ExtensionType::DefaultAccountState),
                    ));
                }
                TokenInstruction::MemoTransferExtension => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "MemoTransfer extension instruction",
                        Some(ExtensionType::MemoTransfer),
                    ));
                }
                TokenInstruction::CpiGuardExtension => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "CpiGuard extension instruction",
                        Some(ExtensionType::CpiGuard),
                    ));
                }
                TokenInstruction::InitializeImmutableOwner => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "InitializeImmutableOwner instruction",
                        Some(ExtensionType::ImmutableOwner),
                    ));
                }
                TokenInstruction::InitializeNonTransferableMint => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "InitializeNonTransferableMint instruction",
                        Some(ExtensionType::NonTransferable),
                    ));
                }
                TokenInstruction::PermissionedBurnExtension => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "PermissionedBurn extension instruction",
                        Some(ExtensionType::PermissionedBurn),
                    ));
                }
                TokenInstruction::ConfidentialTransferExtension
                | TokenInstruction::ConfidentialTransferFeeExtension
                | TokenInstruction::ConfidentialMintBurnExtension => {
                    parsed.push(Self::unsupported_fee_payer_account_check(
                        instruction,
                        "unsupported Token-2022 extension instruction",
                        None,
                    ));
                }
                TokenInstruction::Batch { .. } => {
                    return Err(KoraError::InvalidTransaction(
                        TOKEN_2022_BATCH_UNSUPPORTED.to_string(),
                    ));
                }
                _ => {}
            }
        }

        Ok(parsed)
    }

    fn optional_field(
        pubkey: Option<Pubkey>,
        context: &'static str,
        role: Token2022FieldRole,
    ) -> Option<Token2022SecurityField> {
        pubkey.map(|pubkey| Token2022SecurityField { context, pubkey, role })
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

    fn required_account_field(
        instruction: &Instruction,
        index: usize,
        context: &'static str,
        role: Token2022FieldRole,
    ) -> Result<Token2022SecurityField, KoraError> {
        Ok(Token2022SecurityField {
            context,
            pubkey: Self::required_account(instruction, index, context)?,
            role,
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
        extension_type: Option<ExtensionType>,
    ) -> Token2022SecurityInstruction {
        Token2022SecurityInstruction {
            instruction_name,
            extension_type,
            accounts: Self::instruction_accounts(instruction),
            account_usage_policy: Token2022AccountUsagePolicy::RejectIfFeePayerPresent,
            update_authority: None,
            multisig_signers: vec![],
            data_pubkeys: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_nullable::MaybeNull;
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
            parsed[0].find_planted_fee_payer_authority(&authority).map(|field| field.context),
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

    #[test]
    fn test_parse_metadata_initialize_records_current_and_planted_authorities() {
        let mint = Pubkey::new_unique();
        let update_authority = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();

        let instruction = spl_token_metadata_interface::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            &update_authority,
            &mint,
            &mint_authority,
            "USDP".to_string(),
            "USDP".to_string(),
            "https://example.com/usdp.json".to_string(),
        );

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].instruction_name, "Token2022 TokenMetadataInitialize");
        assert_eq!(parsed[0].extension_type, Some(ExtensionType::TokenMetadata));
        assert_eq!(parsed[0].account_usage_policy, Token2022AccountUsagePolicy::Ignore);
        assert_eq!(parsed[0].update_authority, Some(mint_authority));
        assert_eq!(
            parsed[0]
                .find_planted_fee_payer_authority(&update_authority)
                .map(|field| field.context),
            Some("Token2022 TokenMetadataInitialize updateAuthority")
        );
    }

    #[test]
    fn test_parse_metadata_update_authority_plants_new_authority_from_data() {
        let metadata = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let instruction = spl_token_metadata_interface::instruction::update_authority(
            &spl_token_2022_interface::id(),
            &metadata,
            &current_authority,
            MaybeNull::try_from(Some(new_authority)).unwrap(),
        );

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].instruction_name, "Token2022 TokenMetadataUpdateAuthority");
        assert_eq!(parsed[0].account_usage_policy, Token2022AccountUsagePolicy::Ignore);
        assert_eq!(parsed[0].update_authority, Some(current_authority));
        // The new authority rides in instruction data, so it must be surfaced as a
        // planted authority so the validator can catch a planted fee payer.
        assert_eq!(
            parsed[0].find_planted_fee_payer_authority(&new_authority).map(|field| field.context),
            Some("Token2022 TokenMetadataUpdateAuthority newAuthority")
        );
    }

    #[test]
    fn test_parse_group_initialize_plants_update_authority_from_data() {
        let group = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        let update_authority = Pubkey::new_unique();

        let instruction = spl_token_group_interface::instruction::initialize_group(
            &spl_token_2022_interface::id(),
            &group,
            &mint,
            &mint_authority,
            Some(update_authority),
            100,
        );

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].instruction_name, "Token2022 TokenGroupInitializeGroup");
        assert_eq!(parsed[0].extension_type, Some(ExtensionType::TokenGroup));
        assert_eq!(parsed[0].account_usage_policy, Token2022AccountUsagePolicy::Ignore);
        assert_eq!(parsed[0].update_authority, Some(mint_authority));
        assert!(parsed[0].find_planted_fee_payer_authority(&update_authority).is_some());
    }

    #[test]
    fn test_parse_group_update_authority_plants_new_authority_from_data() {
        let group = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let instruction = spl_token_group_interface::instruction::update_group_authority(
            &spl_token_2022_interface::id(),
            &group,
            &current_authority,
            Some(new_authority),
        );

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].update_authority, Some(current_authority));
        assert!(parsed[0].find_planted_fee_payer_authority(&new_authority).is_some());
    }

    #[test]
    fn test_parse_group_initialize_member_has_no_planted_authority() {
        let member = Pubkey::new_unique();
        let member_mint = Pubkey::new_unique();
        let member_mint_authority = Pubkey::new_unique();
        let group = Pubkey::new_unique();
        let group_update_authority = Pubkey::new_unique();

        let instruction = spl_token_group_interface::instruction::initialize_member(
            &spl_token_2022_interface::id(),
            &member,
            &member_mint,
            &member_mint_authority,
            &group,
            &group_update_authority,
        );

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].instruction_name, "Token2022 TokenGroupInitializeMember");
        assert_eq!(parsed[0].extension_type, Some(ExtensionType::TokenGroupMember));
        assert_eq!(parsed[0].update_authority, Some(member_mint_authority));
        assert_eq!(parsed[0].multisig_signers, vec![group_update_authority]);
        assert!(parsed[0].data_pubkeys.is_empty());
    }

    #[test]
    fn test_unrecognized_token2022_instruction_fails_closed() {
        // Bytes that are neither a TokenInstruction nor a metadata/group interface
        // instruction must be rejected, not silently accepted.
        let instruction = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![],
            data: vec![0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        };

        assert!(Token2022SecurityParser::parse(&[instruction]).is_err());
    }

    #[test]
    fn test_token2022_batch_instruction_fails_closed() {
        let instruction = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![],
            data: vec![0xff, 0x00, 0x01, 0x09],
        };

        assert!(Token2022SecurityParser::parse(&[instruction]).is_err());
    }

    #[test]
    fn test_parse_permissioned_burn_marks_accounts_reject_if_fee_payer_present() {
        let instruction = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), false)],
            data: vec![46, 0],
        };

        let parsed = Token2022SecurityParser::parse(&[instruction]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].extension_type, Some(ExtensionType::PermissionedBurn));
        assert_eq!(
            parsed[0].account_usage_policy,
            Token2022AccountUsagePolicy::RejectIfFeePayerPresent
        );
    }

    #[test]
    fn test_withdraw_excess_lamports_is_left_to_fee_payer_policy() {
        let instruction = spl_token_2022_interface::instruction::withdraw_excess_lamports(
            &spl_token_2022_interface::id(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &[],
        )
        .unwrap();

        assert!(Token2022SecurityParser::parse(&[instruction]).unwrap().is_empty());
    }
}
