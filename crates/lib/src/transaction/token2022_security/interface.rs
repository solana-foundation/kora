use solana_sdk::instruction::Instruction;
use spl_token_2022_interface::extension::ExtensionType;
use spl_token_group_interface::instruction::TokenGroupInstruction;
use spl_token_metadata_interface::instruction::TokenMetadataInstruction;

use crate::error::KoraError;

use super::{
    Token2022AccountUsagePolicy, Token2022FieldRole, Token2022InterfaceFamily,
    Token2022SecurityInstruction, Token2022SecurityParser,
};

impl Token2022SecurityParser {
    /// Cheap recognizer for the token-metadata / token-group interface
    /// instructions Token-2022 also processes. Lets the typed SPL parser
    /// classify an instruction without building a security instruction it
    /// would immediately discard.
    pub(crate) fn token_2022_interface_instruction_family(
        data: &[u8],
    ) -> Option<Token2022InterfaceFamily> {
        if TokenMetadataInstruction::unpack(data).is_ok() {
            Some(Token2022InterfaceFamily::TokenMetadata)
        } else if TokenGroupInstruction::unpack(data).is_ok() {
            Some(Token2022InterfaceFamily::TokenGroup)
        } else {
            None
        }
    }

    /// Recognizes token-metadata / token-group interface instructions that are
    /// routed to the Token-2022 program but are not `TokenInstruction` variants.
    /// Returns `None` for anything that is neither, so callers fail closed.
    pub(super) fn parse_token_2022_interface_instruction(
        instruction: &Instruction,
    ) -> Result<Option<Token2022SecurityInstruction>, KoraError> {
        if let Ok(metadata_instruction) = TokenMetadataInstruction::unpack(&instruction.data) {
            return Ok(Some(Self::parse_token_metadata_interface_instruction(
                instruction,
                metadata_instruction,
            )?));
        }

        if let Ok(group_instruction) = TokenGroupInstruction::unpack(&instruction.data) {
            return Ok(Some(Self::parse_token_group_interface_instruction(
                instruction,
                group_instruction,
            )?));
        }

        Ok(None)
    }

    fn parse_token_metadata_interface_instruction(
        instruction: &Instruction,
        metadata_instruction: TokenMetadataInstruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let (instruction_name, update_authority, data_pubkeys) = match metadata_instruction {
            TokenMetadataInstruction::Initialize(_) => (
                "Token2022 TokenMetadataInitialize",
                Some(Self::required_account(
                    instruction,
                    3,
                    "Token2022 TokenMetadataInitialize mintAuthority",
                )?),
                vec![Self::required_account_field(
                    instruction,
                    1,
                    "Token2022 TokenMetadataInitialize updateAuthority",
                    Token2022FieldRole::PlantedAuthority,
                )?],
            ),
            TokenMetadataInstruction::UpdateField(_) => (
                "Token2022 TokenMetadataUpdateField",
                Some(Self::required_account(
                    instruction,
                    1,
                    "Token2022 TokenMetadataUpdateField updateAuthority",
                )?),
                vec![],
            ),
            TokenMetadataInstruction::RemoveKey(_) => (
                "Token2022 TokenMetadataRemoveKey",
                Some(Self::required_account(
                    instruction,
                    1,
                    "Token2022 TokenMetadataRemoveKey updateAuthority",
                )?),
                vec![],
            ),
            TokenMetadataInstruction::UpdateAuthority(data) => (
                "Token2022 TokenMetadataUpdateAuthority",
                Some(Self::required_account(
                    instruction,
                    1,
                    "Token2022 TokenMetadataUpdateAuthority currentAuthority",
                )?),
                Self::optional_field(
                    data.new_authority.into(),
                    "Token2022 TokenMetadataUpdateAuthority newAuthority",
                    Token2022FieldRole::PlantedAuthority,
                )
                .into_iter()
                .collect(),
            ),
            TokenMetadataInstruction::Emit(_) => ("Token2022 TokenMetadataEmit", None, vec![]),
        };

        Ok(Token2022SecurityInstruction {
            instruction_name,
            extension_type: Some(ExtensionType::TokenMetadata),
            accounts: Self::instruction_accounts(instruction),
            account_usage_policy: Token2022AccountUsagePolicy::Ignore,
            update_authority,
            multisig_signers: vec![],
            data_pubkeys,
        })
    }

    fn parse_token_group_interface_instruction(
        instruction: &Instruction,
        group_instruction: TokenGroupInstruction,
    ) -> Result<Token2022SecurityInstruction, KoraError> {
        let (instruction_name, extension_type, update_authority, multisig_signers, data_pubkeys) =
            match group_instruction {
                TokenGroupInstruction::InitializeGroup(data) => (
                    "Token2022 TokenGroupInitializeGroup",
                    ExtensionType::TokenGroup,
                    Some(Self::required_account(
                        instruction,
                        2,
                        "Token2022 TokenGroupInitializeGroup mintAuthority",
                    )?),
                    vec![],
                    Self::optional_field(
                        data.update_authority.into(),
                        "Token2022 TokenGroupInitializeGroup updateAuthority",
                        Token2022FieldRole::PlantedAuthority,
                    )
                    .into_iter()
                    .collect(),
                ),
                TokenGroupInstruction::UpdateGroupMaxSize(_) => (
                    "Token2022 TokenGroupUpdateGroupMaxSize",
                    ExtensionType::TokenGroup,
                    Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 TokenGroupUpdateGroupMaxSize updateAuthority",
                    )?),
                    vec![],
                    vec![],
                ),
                TokenGroupInstruction::UpdateGroupAuthority(data) => (
                    "Token2022 TokenGroupUpdateGroupAuthority",
                    ExtensionType::TokenGroup,
                    Some(Self::required_account(
                        instruction,
                        1,
                        "Token2022 TokenGroupUpdateGroupAuthority currentAuthority",
                    )?),
                    vec![],
                    Self::optional_field(
                        data.new_authority.into(),
                        "Token2022 TokenGroupUpdateGroupAuthority newAuthority",
                        Token2022FieldRole::PlantedAuthority,
                    )
                    .into_iter()
                    .collect(),
                ),
                TokenGroupInstruction::InitializeMember(_) => (
                    "Token2022 TokenGroupInitializeMember",
                    ExtensionType::TokenGroupMember,
                    Some(Self::required_account(
                        instruction,
                        2,
                        "Token2022 TokenGroupInitializeMember memberMintAuthority",
                    )?),
                    vec![Self::required_account(
                        instruction,
                        4,
                        "Token2022 TokenGroupInitializeMember groupUpdateAuthority",
                    )?],
                    vec![],
                ),
            };

        Ok(Token2022SecurityInstruction {
            instruction_name,
            extension_type: Some(extension_type),
            accounts: Self::instruction_accounts(instruction),
            account_usage_policy: Token2022AccountUsagePolicy::Ignore,
            update_authority,
            multisig_signers,
            data_pubkeys,
        })
    }
}
