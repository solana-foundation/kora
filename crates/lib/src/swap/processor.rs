use std::{str::FromStr, sync::Arc};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_keychain::SolanaSigner;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use solana_system_interface::instruction::transfer as system_transfer;

use crate::{
    config::Config,
    error::KoraError,
    swap::get_swap_quote_provider,
    token::token::TokenUtil,
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
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

struct ValidatedSwapForGasInput {
    source_wallet: Pubkey,
    destination_wallet: Pubkey,
    fee_token: Pubkey,
    lamports_out: u64,
}

pub struct SwapForGasProcessor;

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

    fn validate_build_input(
        input: &SwapForGasBuildInput,
        signer_pubkey: Pubkey,
        config: &Config,
    ) -> Result<ValidatedSwapForGasInput, KoraError> {
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
        if destination_wallet == signer_pubkey {
            return Err(KoraError::ValidationError(
                "destination_wallet must not be the Kora fee payer".to_string(),
            ));
        }

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

        Ok(ValidatedSwapForGasInput {
            source_wallet,
            destination_wallet,
            fee_token,
            lamports_out: input.lamports_out,
        })
    }

    async fn quote_token_amount_with_spread(
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        fee_token: &Pubkey,
        lamports_out: u64,
    ) -> Result<(u64, u16), KoraError> {
        let quote_provider = get_swap_quote_provider(config);
        let quoted_token_amount = quote_provider
            .quote_token_amount_in_for_lamports_out(rpc_client, fee_token, lamports_out, config)
            .await?;

        let spread_bps = config.kora.swap_for_gas.spread_bps;
        let token_amount_in = Self::apply_spread_bps(quoted_token_amount, spread_bps)?;
        Ok((token_amount_in, spread_bps))
    }

    async fn build_swap_message(
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        signer_pubkey: Pubkey,
        validated: &ValidatedSwapForGasInput,
        payment_destination: &Pubkey,
        token_amount_in: u64,
    ) -> Result<VersionedMessage, KoraError> {
        let token_mint = TokenUtil::get_mint(config, rpc_client, &validated.fee_token).await?;
        let token_program = token_mint.get_token_program();
        let token_decimals = token_mint.decimals();

        let source_token_account = token_program
            .get_associated_token_address(&validated.source_wallet, &validated.fee_token);
        let destination_token_account =
            token_program.get_associated_token_address(payment_destination, &validated.fee_token);

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
                    payment_destination,
                    &validated.fee_token,
                ));
            }
            Err(e) => return Err(e),
        }

        instructions.push(
            token_program
                .create_transfer_checked_instruction(
                    &source_token_account,
                    &validated.fee_token,
                    &destination_token_account,
                    &validated.source_wallet,
                    token_amount_in,
                    token_decimals,
                )
                .map_err(|e| {
                    KoraError::InvalidTransaction(format!("Failed to build token transfer: {e}"))
                })?,
        );

        instructions.push(system_transfer(
            &signer_pubkey,
            &validated.destination_wallet,
            validated.lamports_out,
        ));

        let blockhash = CacheUtil::get_or_fetch_latest_blockhash(config, rpc_client).await?;
        Ok(VersionedMessage::Legacy(Message::new_with_blockhash(
            &instructions,
            Some(&signer_pubkey),
            &blockhash,
        )))
    }

    async fn sign_with_fee_payer(
        message: VersionedMessage,
        signer: &Arc<solana_keychain::Signer>,
        signer_pubkey: Pubkey,
    ) -> Result<VersionedTransaction, KoraError> {
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&transaction)?;

        let message_bytes = resolved.transaction.message.serialize();
        let signature = signer
            .sign_message(&message_bytes)
            .await
            .map_err(|e| KoraError::SigningError(e.to_string()))?;

        let signer_position = resolved.find_signer_position(&signer_pubkey)?;
        resolved.transaction.signatures[signer_position] = signature;

        Ok(resolved.transaction)
    }

    pub async fn build_and_sign_transaction(
        input: &SwapForGasBuildInput,
        signer: &Arc<solana_keychain::Signer>,
        signer_pubkey: Pubkey,
        config: &Config,
        rpc_client: &Arc<RpcClient>,
    ) -> Result<SwapForGasBuildOutput, KoraError> {
        let validated = Self::validate_build_input(input, signer_pubkey, config)?;

        let (token_amount_in, spread_bps) = Self::quote_token_amount_with_spread(
            config,
            rpc_client,
            &validated.fee_token,
            validated.lamports_out,
        )
        .await?;

        let payment_destination = config.kora.get_payment_address(&signer_pubkey)?;
        let message = Self::build_swap_message(
            config,
            rpc_client,
            signer_pubkey,
            &validated,
            &payment_destination,
            token_amount_in,
        )
        .await?;

        let transaction = Self::sign_with_fee_payer(message, signer, signer_pubkey).await?;

        Ok(SwapForGasBuildOutput {
            transaction,
            payment_address: payment_destination,
            fee_token: validated.fee_token,
            token_amount_in,
            lamports_out: validated.lamports_out,
            spread_bps,
            destination_wallet: validated.destination_wallet,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
