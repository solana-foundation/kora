use std::{str::FromStr, sync::Arc};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use solana_system_interface::instruction::transfer as system_transfer;

use crate::{
    config::Config,
    error::KoraError,
    signer::bundle_signer::BundleSigner,
    swap::get_swap_quote_provider,
    token::{
        interface::TokenInterface, spl_token::TokenProgram, spl_token_2022::Token2022Program,
        token::TokenUtil,
    },
    transaction::{TransactionUtil, VersionedTransactionResolved},
    usage_limit::UsageTracker,
    validator::transaction_validator::TransactionValidator,
    CacheUtil,
};

pub struct SwapForGasBuildInput {
    pub source_wallet: String,
    pub destination_wallet: Option<String>,
    pub fee_token: String,
    pub lamports_out: u64,
}

pub struct SwapForGasBuildOutput {
    pub transaction: VersionedTransaction,
    pub payment_address: Pubkey,
    pub fee_token: Pubkey,
    pub token_amount_in: u64,
    pub lamports_out: u64,
    pub spread_bps: u16,
    pub destination_wallet: Pubkey,
}

pub struct SwapForGasProcessor;

struct PrebuiltSwapSummary {
    source_wallet: Pubkey,
    destination_wallet: Pubkey,
    fee_token: Pubkey,
    lamports_out: u64,
    token_amount_in: u64,
}

impl SwapForGasProcessor {
    pub fn apply_spread_bps(base_amount: u64, spread_bps: u16) -> Result<u64, KoraError> {
        let multiplier = 10_000u128.checked_add(spread_bps as u128).ok_or_else(|| {
            KoraError::ValidationError("Spread configuration overflow".to_string())
        })?;

        let adjusted = (base_amount as u128)
            .checked_mul(multiplier)
            .and_then(|v| v.checked_add(9_999))
            .and_then(|v| v.checked_div(10_000))
            .ok_or_else(|| KoraError::ValidationError("Spread calculation overflow".to_string()))?;

        u64::try_from(adjusted)
            .map_err(|_| KoraError::ValidationError("Spread-adjusted amount overflow".to_string()))
    }

    pub async fn build_and_sign_transaction(
        input: &SwapForGasBuildInput,
        signer: &Arc<solana_keychain::Signer>,
        signer_pubkey: Pubkey,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
    ) -> Result<SwapForGasBuildOutput, KoraError> {
        if input.lamports_out == 0 {
            return Err(KoraError::ValidationError(
                "lamports_out must be greater than zero".to_string(),
            ));
        }

        let validator = TransactionValidator::new(config, signer_pubkey)?;

        let source_wallet = Pubkey::from_str(&input.source_wallet)
            .map_err(|e| KoraError::ValidationError(format!("Invalid source_wallet: {e}")))?;
        if source_wallet == signer_pubkey {
            return Err(KoraError::ValidationError(
                "source_wallet must not be the Kora fee payer".to_string(),
            ));
        }

        let destination_wallet = input
            .destination_wallet
            .as_deref()
            .map(Pubkey::from_str)
            .transpose()
            .map_err(|e| KoraError::ValidationError(format!("Invalid destination_wallet: {e}")))?
            .unwrap_or(source_wallet);

        let fee_token = Pubkey::from_str(&input.fee_token)
            .map_err(|e| KoraError::ValidationError(format!("Invalid fee_token mint: {e}")))?;

        if validator.is_disallowed_account(&source_wallet) {
            return Err(KoraError::InvalidTransaction(format!(
                "Source wallet {source_wallet} is disallowed"
            )));
        }
        if validator.is_disallowed_account(&destination_wallet) {
            return Err(KoraError::InvalidTransaction(format!(
                "Destination wallet {destination_wallet} is disallowed"
            )));
        }

        validator.validate_lamport_fee(input.lamports_out)?;

        if !config.validation.supports_token(&fee_token.to_string()) {
            return Err(KoraError::UnsupportedFeeToken(fee_token.to_string()));
        }

        let quote_provider = get_swap_quote_provider(config);
        let quoted_token_amount = quote_provider
            .quote_token_amount_in_for_lamports_out(
                rpc_client,
                &fee_token,
                input.lamports_out,
                config,
            )
            .await?;

        let spread_bps = config.kora.swap_for_gas.spread_bps;
        let token_amount_in = Self::apply_spread_bps(quoted_token_amount, spread_bps)?;
        let payment_destination = config.kora.get_payment_address(&signer_pubkey)?;

        let token_mint = TokenUtil::get_mint(config, rpc_client, &fee_token).await?;
        let token_program = token_mint.get_token_program();
        let token_decimals = token_mint.decimals();

        let source_token_account =
            token_program.get_associated_token_address(&source_wallet, &fee_token);
        let destination_token_account =
            token_program.get_associated_token_address(&payment_destination, &fee_token);

        CacheUtil::get_account(config, rpc_client, &source_token_account, false).await.map_err(
            |e| match e {
                KoraError::AccountNotFound(_) => {
                    KoraError::AccountNotFound(source_token_account.to_string())
                }
                other => other,
            },
        )?;

        let mut instructions = Vec::new();
        match CacheUtil::get_account(config, rpc_client, &destination_token_account, false).await {
            Ok(_) => {}
            Err(KoraError::AccountNotFound(_)) => {
                instructions.push(token_program.create_associated_token_account_instruction(
                    &signer_pubkey,
                    &payment_destination,
                    &fee_token,
                ));
            }
            Err(e) => return Err(e),
        }

        instructions.push(
            token_program
                .create_transfer_checked_instruction(
                    &source_token_account,
                    &fee_token,
                    &destination_token_account,
                    &source_wallet,
                    token_amount_in,
                    token_decimals,
                )
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!("Failed to build token transfer: {e}"))
                })?,
        );

        instructions.push(system_transfer(&signer_pubkey, &destination_wallet, input.lamports_out));

        let blockhash = CacheUtil::get_or_fetch_latest_blockhash(config, rpc_client).await?;
        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &instructions,
            Some(&signer_pubkey),
            &blockhash,
        ));

        let mut transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction)?;
        BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            signer,
            &signer_pubkey,
            &Some(blockhash),
        )
        .await?;
        transaction = resolved.transaction;

        Ok(SwapForGasBuildOutput {
            transaction,
            payment_address: payment_destination,
            fee_token,
            token_amount_in,
            lamports_out: input.lamports_out,
            spread_bps,
            destination_wallet,
        })
    }

    fn signer_position(
        transaction: &VersionedTransaction,
        signer_pubkey: &Pubkey,
    ) -> Result<usize, KoraError> {
        let account_keys = transaction.message.static_account_keys();
        let required_signers = transaction.message.header().num_required_signatures as usize;

        let signer_position =
            account_keys.iter().position(|key| key == signer_pubkey).ok_or_else(|| {
                KoraError::InvalidTransaction(format!(
                    "Signer {signer_pubkey} not found in transaction account keys. \
                     Pass signer_key returned by signSwapForGas."
                ))
            })?;

        if signer_position >= required_signers {
            return Err(KoraError::InvalidTransaction(format!(
                "Signer {signer_pubkey} is not a required signer for this transaction"
            )));
        }

        Ok(signer_position)
    }

    fn validate_required_user_signatures(
        transaction: &VersionedTransaction,
        signer_pubkey: &Pubkey,
    ) -> Result<(), KoraError> {
        let account_keys = transaction.message.static_account_keys();
        let required_signers = transaction.message.header().num_required_signatures as usize;

        for (index, account_key) in account_keys.iter().take(required_signers).enumerate() {
            if account_key == signer_pubkey {
                continue;
            }

            if transaction.signatures[index] == Signature::default() {
                return Err(KoraError::ValidationError(format!(
                    "Missing required user signature for signer {}",
                    account_key
                )));
            }
        }

        Ok(())
    }

    async fn sign_with_kora_signer(
        transaction: &VersionedTransaction,
        signer: &Arc<solana_keychain::Signer>,
        signer_pubkey: &Pubkey,
    ) -> Result<VersionedTransaction, KoraError> {
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(transaction)?;
        BundleSigner::sign_transaction_for_bundle(&mut resolved, signer, signer_pubkey, &None)
            .await?;
        Ok(resolved.transaction)
    }

    async fn extract_prebuilt_swap_summary(
        resolved_transaction: &mut VersionedTransactionResolved,
        signer_pubkey: &Pubkey,
        payment_destination: &Pubkey,
        config: &Config,
    ) -> Result<PrebuiltSwapSummary, KoraError> {
        let system_instructions = resolved_transaction.get_or_parse_system_instructions()?;
        let mut lamports_out = 0u64;
        let mut destination_wallet: Option<Pubkey> = None;

        for system_ix in system_instructions
            .get(&crate::transaction::ParsedSystemInstructionType::SystemTransfer)
            .unwrap_or(&vec![])
        {
            if let crate::transaction::ParsedSystemInstructionData::SystemTransfer {
                lamports,
                sender,
                receiver,
            } = system_ix
            {
                if sender != signer_pubkey {
                    continue;
                }

                lamports_out = lamports_out.checked_add(*lamports).ok_or_else(|| {
                    KoraError::ValidationError(
                        "Swap SOL transfer amount overflow during validation".to_string(),
                    )
                })?;

                match destination_wallet {
                    None => destination_wallet = Some(*receiver),
                    Some(existing) if existing == *receiver => {}
                    Some(_) => {
                        return Err(KoraError::InvalidTransaction(
                            "Multiple destination wallets found for SOL transfer in swapForGas transaction"
                                .to_string(),
                        ));
                    }
                }
            }
        }

        let destination_wallet = destination_wallet.ok_or_else(|| {
            KoraError::InvalidTransaction(
                "No SOL transfer from Kora signer found in swapForGas transaction".to_string(),
            )
        })?;

        if lamports_out == 0 {
            return Err(KoraError::InvalidTransaction(
                "swapForGas transaction must transfer a non-zero SOL amount".to_string(),
            ));
        }

        let spl_instructions = resolved_transaction.get_or_parse_spl_instructions()?;
        let mut source_wallet: Option<Pubkey> = None;
        let mut fee_token: Option<Pubkey> = None;
        let mut token_amount_in = 0u64;

        for spl_ix in spl_instructions
            .get(&crate::transaction::ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            let crate::transaction::ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner,
                mint,
                destination_address,
                is_2022,
                ..
            } = spl_ix
            else {
                continue;
            };

            let transfer_mint = mint.ok_or_else(|| {
                KoraError::InvalidTransaction(
                    "swapForGas token transfer must include mint (use transferChecked)".to_string(),
                )
            })?;

            if !config.validation.supports_token(&transfer_mint.to_string()) {
                continue;
            }

            let expected_destination = if *is_2022 {
                Token2022Program::new()
                    .get_associated_token_address(payment_destination, &transfer_mint)
            } else {
                TokenProgram::new()
                    .get_associated_token_address(payment_destination, &transfer_mint)
            };

            if *destination_address != expected_destination {
                continue;
            }

            match source_wallet {
                None => source_wallet = Some(*owner),
                Some(existing) if existing == *owner => {}
                Some(_) => {
                    return Err(KoraError::InvalidTransaction(
                        "Multiple source wallets found in swapForGas token payment instructions"
                            .to_string(),
                    ));
                }
            }

            match fee_token {
                None => fee_token = Some(transfer_mint),
                Some(existing) if existing == transfer_mint => {}
                Some(_) => {
                    return Err(KoraError::InvalidTransaction(
                        "Multiple fee token mints found in swapForGas payment instructions"
                            .to_string(),
                    ));
                }
            }

            token_amount_in = token_amount_in.checked_add(*amount).ok_or_else(|| {
                KoraError::ValidationError(
                    "swapForGas token amount overflow during validation".to_string(),
                )
            })?;
        }

        let source_wallet = source_wallet.ok_or_else(|| {
            KoraError::InvalidTransaction(
                "No supported token payment to Kora payment address found in swapForGas transaction"
                    .to_string(),
            )
        })?;

        let fee_token = fee_token.ok_or_else(|| {
            KoraError::InvalidTransaction(
                "No fee token mint found in swapForGas payment instructions".to_string(),
            )
        })?;

        Ok(PrebuiltSwapSummary {
            source_wallet,
            destination_wallet,
            fee_token,
            lamports_out,
            token_amount_in,
        })
    }

    async fn validate_prebuilt_swap_business_rules(
        summary: &PrebuiltSwapSummary,
        signer_pubkey: &Pubkey,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
    ) -> Result<(), KoraError> {
        let validator = TransactionValidator::new(config, *signer_pubkey)?;

        if summary.source_wallet == *signer_pubkey {
            return Err(KoraError::ValidationError(
                "source_wallet must not be the Kora fee payer".to_string(),
            ));
        }

        if validator.is_disallowed_account(&summary.source_wallet) {
            return Err(KoraError::InvalidTransaction(format!(
                "Source wallet {} is disallowed",
                summary.source_wallet
            )));
        }

        if validator.is_disallowed_account(&summary.destination_wallet) {
            return Err(KoraError::InvalidTransaction(format!(
                "Destination wallet {} is disallowed",
                summary.destination_wallet
            )));
        }

        validator.validate_lamport_fee(summary.lamports_out)?;

        if !config.validation.supports_token(&summary.fee_token.to_string()) {
            return Err(KoraError::UnsupportedFeeToken(summary.fee_token.to_string()));
        }

        let quote_provider = get_swap_quote_provider(config);
        let quoted_token_amount = quote_provider
            .quote_token_amount_in_for_lamports_out(
                rpc_client,
                &summary.fee_token,
                summary.lamports_out,
                config,
            )
            .await?;

        let required_token_amount =
            Self::apply_spread_bps(quoted_token_amount, config.kora.swap_for_gas.spread_bps)?;

        if summary.token_amount_in < required_token_amount {
            return Err(KoraError::InvalidTransaction(format!(
                "Insufficient token payment for swapForGas. Required at least {required_token_amount}, got {}",
                summary.token_amount_in
            )));
        }

        Ok(())
    }

    pub async fn validate_transaction_for_send(
        transaction: &VersionedTransaction,
        signer_pubkey: &Pubkey,
        signer: &Arc<solana_keychain::Signer>,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        sig_verify: bool,
        user_id: Option<&str>,
    ) -> Result<VersionedTransaction, KoraError> {
        Self::signer_position(transaction, signer_pubkey)?;
        Self::validate_required_user_signatures(transaction, signer_pubkey)?;

        let mut resolved_transaction = VersionedTransactionResolved::from_transaction(
            transaction,
            config,
            rpc_client,
            sig_verify,
        )
        .await?;

        let validator = TransactionValidator::new(config, *signer_pubkey)?;
        validator.validate_transaction(config, &mut resolved_transaction, rpc_client).await?;

        let payment_destination = config.kora.get_payment_address(signer_pubkey)?;
        let summary = Self::extract_prebuilt_swap_summary(
            &mut resolved_transaction,
            signer_pubkey,
            &payment_destination,
            config,
        )
        .await?;

        Self::validate_prebuilt_swap_business_rules(&summary, signer_pubkey, config, rpc_client)
            .await?;

        UsageTracker::check_transaction_usage_limit(
            config,
            &mut resolved_transaction,
            user_id,
            signer_pubkey,
            rpc_client,
        )
        .await?;

        Self::sign_with_kora_signer(transaction, signer, signer_pubkey).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_system_interface::instruction::transfer;

    #[test]
    fn test_apply_spread_bps_zero() {
        let result = SwapForGasProcessor::apply_spread_bps(1_000_000, 0).unwrap();
        assert_eq!(result, 1_000_000);
    }

    #[test]
    fn test_apply_spread_bps_rounds_up() {
        let result = SwapForGasProcessor::apply_spread_bps(1, 25).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_validate_required_user_signatures_rejects_missing_signature() {
        let fee_payer = Pubkey::new_unique();
        let user_signer = Pubkey::new_unique();
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer(&user_signer, &Pubkey::new_unique(), 1)],
            Some(&fee_payer),
        ));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let err = SwapForGasProcessor::validate_required_user_signatures(&transaction, &fee_payer)
            .unwrap_err();
        assert!(matches!(err, KoraError::ValidationError(_)));
    }

    #[test]
    fn test_signer_position_rejects_signer_not_found() {
        let fee_payer = Pubkey::new_unique();
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer(&fee_payer, &Pubkey::new_unique(), 1)],
            Some(&fee_payer),
        ));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let err =
            SwapForGasProcessor::signer_position(&transaction, &Pubkey::new_unique()).unwrap_err();
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }
}
