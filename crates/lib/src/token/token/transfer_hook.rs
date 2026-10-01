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
            if transfer_hook.authority != spl_pod::optional_keys::OptionalNonZeroPubkey::default() {
                return Err(KoraError::ValidationError(format!(
                    "Mutable transfer-hook authority found on mint account {mint_pubkey}",
                )));
            }
        }

        Ok(())
    }

    /// Validate that every Token2022 transfer in the transaction uses a mint with immutable
    /// transfer-hook authority (or no transfer-hook extension).
    pub async fn validate_token2022_transfer_hooks_in_transaction(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        validation_flow: TransferHookValidationFlow,
    ) -> Result<(), KoraError> {
        if !Self::should_reject_mutable_transfer_hook(config, validation_flow) {
            return Ok(());
        }

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

            let transfer_mint = if let Some(mint) = mint {
                mint
            } else {
                // Legacy Transfer (without explicit mint) still needs mint-level transfer-hook checks.
                let source_account =
                    CacheUtil::get_account(config, rpc_client, &source_address, true).await?;
                let source_state = token_program.unpack_token_account(&source_account.data)?;
                source_state.mint()
            };

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

            Self::validate_immutable_transfer_hook_for_mint(mint_with_extensions, &transfer_mint)?;
        }

        Ok(())
    }
}
