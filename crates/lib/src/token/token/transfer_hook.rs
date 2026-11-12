use crate::{
    config::{Config, TransferHookPolicy},
    error::KoraError,
    token::{
        spl_token_2022::{Token2022Extensions, Token2022Mint, Token2022Program},
        spl_token_2022_util::{MintExtension, ParsedExtension},
        TokenInterface,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
    validator::parse_pubkey_set,
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

use super::{TokenUtil, TransferHookValidationFlow};

impl TokenUtil {
    pub(crate) fn should_reject_mutable_transfer_hook(
        config: &Config,
        validation_flow: TransferHookValidationFlow,
    ) -> bool {
        match config.validation.token_2022.transfer_hook_policy {
            TransferHookPolicy::DenyAll => true,
            TransferHookPolicy::DenyMutableForDelayedSigning => {
                matches!(validation_flow, TransferHookValidationFlow::DelayedSigning)
            }
            TransferHookPolicy::AllowAll => false,
        }
    }

    fn validate_immutable_transfer_hook_for_mint(
        mint: &Token2022Mint,
        mint_pubkey: &Pubkey,
    ) -> Result<(), KoraError> {
        if let Some(ParsedExtension::Mint(MintExtension::TransferHook(transfer_hook))) =
            mint.get_extension(spl_token_2022_interface::extension::ExtensionType::TransferHook)
        {
            if transfer_hook.authority != solana_nullable::MaybeNull::default() {
                return Err(KoraError::ValidationError(format!(
                    "Mutable transfer-hook authority found on mint account {mint_pubkey}",
                )));
            }
        }

        Ok(())
    }

    async fn resolve_token2022_transfer_mint(
        config: &Config,
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Pubkey, KoraError> {
        if let Some(mint) = mint {
            return Ok(mint);
        }
        let source_account =
            CacheUtil::get_account(config, rpc_client, source_address, true).await?;
        let source_state = Token2022Program::new().unpack_token_account(&source_account.data)?;
        Ok(source_state.mint())
    }

    /// Reject Token2022 transfers, burns and approvals authorized by the fee payer whose mint
    /// carries a blocked mint extension.
    pub async fn validate_fee_payer_token2022_transfer_mints(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
    ) -> Result<(), KoraError> {
        let token2022_config = &config.validation.token_2022;
        if token2022_config.blocked_mint_extensions.is_empty() {
            return Ok(());
        }

        let token_program = Token2022Program::new();
        let mut validated_mints = HashSet::new();

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;
        let token_movements: Vec<ParsedSPLInstructionData> = [
            ParsedSPLInstructionType::SplTokenTransfer,
            ParsedSPLInstructionType::SplTokenBurn,
            ParsedSPLInstructionType::SplTokenApprove,
        ]
        .iter()
        .filter_map(|instruction_type| spl_instructions.get(instruction_type))
        .flatten()
        .cloned()
        .collect();

        for movement in token_movements {
            let (ParsedSPLInstructionData::SplTokenTransfer {
                owner,
                multisig_signers,
                source_address,
                mint,
                is_2022,
                ..
            }
            | ParsedSPLInstructionData::SplTokenBurn {
                owner,
                multisig_signers,
                source_address,
                mint,
                is_2022,
                ..
            }
            | ParsedSPLInstructionData::SplTokenApprove {
                owner,
                multisig_signers,
                source_address,
                mint,
                is_2022,
                ..
            }) = movement
            else {
                continue;
            };

            if !is_2022 || (owner != *fee_payer && !multisig_signers.contains(fee_payer)) {
                continue;
            }

            let transfer_mint =
                Self::resolve_token2022_transfer_mint(config, rpc_client, &source_address, mint)
                    .await?;
            if !validated_mints.insert(transfer_mint) {
                continue;
            }

            let mint_account =
                CacheUtil::get_account(config, rpc_client, &transfer_mint, true).await?;
            let mint_state = token_program.unpack_mint(&transfer_mint, &mint_account.data)?;
            let mint_with_extensions =
                mint_state.as_any().downcast_ref::<Token2022Mint>().ok_or_else(|| {
                    KoraError::SerializationError("Failed to downcast mint state.".to_string())
                })?;

            if mint_with_extensions
                .get_extension_types()
                .iter()
                .any(|extension_type| token2022_config.is_mint_extension_blocked(*extension_type))
            {
                return Err(KoraError::ValidationError(format!(
                    "Blocked mint extension found on mint account {transfer_mint}",
                )));
            }
        }

        Ok(())
    }

    /// Validate that every Token2022 transfer in the transaction uses a mint with immutable
    /// transfer-hook authority (or no transfer-hook extension). Mints in `allowed_tokens` are
    /// exempt when `transfer_hook_policy` permits mutable authorities on this flow; every other
    /// mint must have an immutable authority under any policy.
    pub async fn validate_token2022_transfer_hooks_in_transaction(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        validation_flow: TransferHookValidationFlow,
    ) -> Result<(), KoraError> {
        let exempt_mints = if Self::should_reject_mutable_transfer_hook(config, validation_flow) {
            HashSet::new()
        } else {
            parse_pubkey_set(&config.validation.allowed_tokens)?
        };

        let token_program = Token2022Program::new();
        let mut validated_mints = HashSet::new();

        let token_transfers = transaction_resolved
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .cloned()
            .unwrap_or_default();

        for transfer in token_transfers {
            let ParsedSPLInstructionData::SplTokenTransfer {
                source_address, mint, is_2022, ..
            } = transfer
            else {
                continue;
            };

            if !is_2022 {
                continue;
            }

            let transfer_mint =
                Self::resolve_token2022_transfer_mint(config, rpc_client, &source_address, mint)
                    .await?;

            if exempt_mints.contains(&transfer_mint) || !validated_mints.insert(transfer_mint) {
                continue;
            }

            let mint_account =
                CacheUtil::get_account(config, rpc_client, &transfer_mint, true).await?;
            let mint_state = token_program.unpack_mint(&transfer_mint, &mint_account.data)?;
            let mint_with_extensions =
                mint_state.as_any().downcast_ref::<Token2022Mint>().ok_or_else(|| {
                    KoraError::SerializationError("Failed to downcast mint state.".to_string())
                })?;

            Self::validate_immutable_transfer_hook_for_mint(mint_with_extensions, &transfer_mint)?;
        }

        Ok(())
    }
}
