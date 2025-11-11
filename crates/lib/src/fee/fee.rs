use std::str::FromStr;

use crate::{
    config::Config,
    constant::{ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION, LAMPORTS_PER_SIGNATURE},
    error::KoraError,
    fee::price::PriceModel,
    oracle::PriceSource,
    token::{
        spl_token_2022::Token2022Mint,
        token::{TokenType, TokenUtil},
        TokenState,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, ParsedSystemInstructionData,
        ParsedSystemInstructionType, VersionedTransactionResolved,
    },
};
use solana_message::Message;

#[cfg(not(test))]
use crate::cache::CacheUtil;

#[cfg(test)]
use crate::tests::cache_mock::MockCacheUtil as CacheUtil;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct TotalFeeCalculation {
    pub total_fee_lamports: u64,
    pub base_fee: u64,
    pub kora_signature_fee: u64,
    pub fee_payer_outflow: u64,
    pub payment_instruction_fee: u64,
    pub transfer_fee_amount: u64,
}

impl TotalFeeCalculation {
    pub fn new(
        total_fee_lamports: u64,
        base_fee: u64,
        kora_signature_fee: u64,
        fee_payer_outflow: u64,
        payment_instruction_fee: u64,
        transfer_fee_amount: u64,
    ) -> Self {
        Self {
            total_fee_lamports,
            base_fee,
            kora_signature_fee,
            fee_payer_outflow,
            payment_instruction_fee,
            transfer_fee_amount,
        }
    }

    pub fn new_fixed(total_fee_lamports: u64) -> Self {
        Self {
            total_fee_lamports,
            base_fee: 0,
            kora_signature_fee: 0,
            fee_payer_outflow: 0,
            payment_instruction_fee: 0,
            transfer_fee_amount: 0,
        }
    }

    pub fn get_total_fee_lamports(&self) -> Result<u64, KoraError> {
        self.base_fee
            .checked_add(self.kora_signature_fee)
            .and_then(|sum| sum.checked_add(self.fee_payer_outflow))
            .and_then(|sum| sum.checked_add(self.payment_instruction_fee))
            .and_then(|sum| sum.checked_add(self.transfer_fee_amount))
            .ok_or_else(|| {
                log::error!("Fee calculation overflow: base_fee={}, kora_signature_fee={}, fee_payer_outflow={}, payment_instruction_fee={}, transfer_fee_amount={}",
                    self.base_fee, self.kora_signature_fee, self.fee_payer_outflow, self.payment_instruction_fee, self.transfer_fee_amount);
                KoraError::ValidationError("Fee calculation overflow".to_string())
            })
    }
}

pub struct FeeConfigUtil {}

impl FeeConfigUtil {
    fn is_fee_payer_in_signers(
        transaction: &VersionedTransactionResolved,
        fee_payer: &Pubkey,
    ) -> Result<bool, KoraError> {
        let all_account_keys = &transaction.all_account_keys;
        let transaction_inner = &transaction.transaction;

        // In messages, the first num_required_signatures accounts are signers
        Ok(match &transaction_inner.message {
            VersionedMessage::Legacy(legacy_message) => {
                let num_signers = legacy_message.header.num_required_signatures as usize;
                all_account_keys.iter().take(num_signers).any(|key| *key == *fee_payer)
            }
            VersionedMessage::V0(v0_message) => {
                let num_signers = v0_message.header.num_required_signatures as usize;
                all_account_keys.iter().take(num_signers).any(|key| *key == *fee_payer)
            }
        })
    }

    /// Helper function to check if a token transfer instruction is a payment to Kora
    /// Returns Some(token_account_data) if it's a payment, None otherwise
    async fn get_payment_instruction_info(
        config: &Config,
        rpc_client: &RpcClient,
        destination_address: &Pubkey,
        payment_destination: &Pubkey,
        skip_missing_accounts: bool,
    ) -> Result<Option<Box<dyn TokenState + Send + Sync>>, KoraError> {
        // Get destination account - handle missing accounts based on skip_missing_accounts
        let destination_account =
            match CacheUtil::get_account(config, rpc_client, destination_address, false).await {
                Ok(account) => account,
                Err(_) if skip_missing_accounts => {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(e);
                }
            };

        let token_program = TokenType::get_token_program_from_owner(&destination_account.owner)?;
        let token_account = token_program.unpack_token_account(&destination_account.data)?;

        // Check if this is a payment to Kora
        if token_account.owner() == *payment_destination {
            Ok(Some(token_account))
        } else {
            Ok(None)
        }
    }

    /// Analyze payment instructions in transaction
    /// Returns (has_payment, total_transfer_fees)
    async fn analyze_payment_instructions(
        config: &Config,
        resolved_transaction: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
    ) -> Result<(bool, u64), KoraError> {
        let payment_destination = config.kora.get_payment_address(fee_payer)?;
        let mut has_payment = false;
        let mut total_transfer_fees = 0u64;

        let parsed_spl_instructions = resolved_transaction.get_or_parse_spl_instructions()?;

        for instruction in parsed_spl_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                mint,
                amount,
                is_2022,
                destination_address,
                ..
            } = instruction
            {
                // Check if this is a payment to Kora
                let payment_info = Self::get_payment_instruction_info(
                    config,
                    rpc_client,
                    destination_address,
                    &payment_destination,
                    true, // Skip missing accounts
                )
                .await?;

                if payment_info.is_some() {
                    has_payment = true;

                    // Calculate Token2022 transfer fees if applicable
                    if *is_2022 {
                        if let Some(mint_pubkey) = mint {
                            let mint_account =
                                CacheUtil::get_account(config, rpc_client, mint_pubkey, true).await?;

                            let token_program =
                                TokenType::get_token_program_from_owner(&mint_account.owner)?;
                            let mint_state =
                                token_program.unpack_mint(mint_pubkey, &mint_account.data)?;

                            if let Some(token2022_mint) =
                                mint_state.as_any().downcast_ref::<Token2022Mint>()
                            {
                                let current_epoch = rpc_client.get_epoch_info().await?.epoch;

                                if let Some(fee_amount) =
                                    token2022_mint.calculate_transfer_fee(*amount, current_epoch)?
                                {
                                    total_transfer_fees = total_transfer_fees
                                        .checked_add(fee_amount)
                                        .ok_or_else(|| {
                                            log::error!(
                                                "Transfer fee accumulation overflow: total={}, new_fee={}",
                                                total_transfer_fees,
                                                fee_amount
                                            );
                                            KoraError::ValidationError(
                                                "Transfer fee accumulation overflow".to_string(),
                                            )
                                        })?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((has_payment, total_transfer_fees))
    }

    async fn estimate_transaction_fee(
        config: &Config,
        rpc_client: &RpcClient,
        transaction: &mut VersionedTransactionResolved,
        fee_payer: &Pubkey,
        is_payment_required: bool,
    ) -> Result<TotalFeeCalculation, KoraError> {
        // Get base transaction fee using resolved transaction to handle lookup tables
        let base_fee =
            TransactionFeeUtil::get_estimate_fee_resolved(rpc_client, transaction).await?;

        // Priority fees are now included in the calculate done by the RPC getFeeForMessage
        // ATA and Token account creation fees are captured in the calculate fee payer outflow (System Transfer)

        // If the Kora signer is not inclded in the signers, we add another base fee, since each transaction will be 5000 lamports
        let mut kora_signature_fee = 0u64;
        if !FeeConfigUtil::is_fee_payer_in_signers(transaction, fee_payer)? {
            kora_signature_fee = LAMPORTS_PER_SIGNATURE;
        }

        // Calculate fee payer outflow if fee payer is provided, to better estimate the potential fee
        let fee_payer_outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            config,
            fee_payer,
            transaction,
            rpc_client,
            &config.validation.price_source,
        )
        .await?;

        // Analyze payment instructions (checks if payment exists + calculates Token2022 fees)
        let (has_payment, transfer_fee_config_amount) =
            FeeConfigUtil::analyze_payment_instructions(config, transaction, rpc_client, fee_payer).await?;

        // If payment is required but not found, add estimated payment instruction fee
        let fee_for_payment_instruction = if is_payment_required && !has_payment {
            ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION
        } else {
            0
        };

        let total_fee_lamports = base_fee
            .checked_add(kora_signature_fee)
            .and_then(|sum| sum.checked_add(fee_payer_outflow))
            .and_then(|sum| sum.checked_add(fee_for_payment_instruction))
            .and_then(|sum| sum.checked_add(transfer_fee_config_amount))
            .ok_or_else(|| {
                log::error!("Fee calculation overflow: base_fee={}, kora_signature_fee={}, fee_payer_outflow={}, payment_instruction_fee={}, transfer_fee_amount={}",
                    base_fee, kora_signature_fee, fee_payer_outflow, fee_for_payment_instruction, transfer_fee_config_amount);
                KoraError::ValidationError("Fee calculation overflow".to_string())
            })?;

        Ok(TotalFeeCalculation {
            total_fee_lamports,
            base_fee,
            kora_signature_fee,
            fee_payer_outflow,
            payment_instruction_fee: fee_for_payment_instruction,
            transfer_fee_amount: transfer_fee_config_amount,
        })
    }

    /// Main entry point for fee calculation with Kora's price model applied
    pub async fn estimate_kora_fee(
        config: &Config,
        rpc_client: &RpcClient,
        transaction: &mut VersionedTransactionResolved,
        fee_payer: &Pubkey,
        is_payment_required: bool,
        price_source: PriceSource,
    ) -> Result<TotalFeeCalculation, KoraError> {
        match &config.validation.price.model {
            PriceModel::Free => Ok(TotalFeeCalculation::new_fixed(0)),
            PriceModel::Fixed { strict, .. } => {
                let fixed_fee_lamports = config
                    .validation
                    .price
                    .get_required_lamports_with_fixed(rpc_client, price_source)
                    .await?;

                if *strict {
                    let fee_calculation = Self::estimate_transaction_fee(
                        config,
                        rpc_client,
                        transaction,
                        fee_payer,
                        is_payment_required,
                    )
                    .await?;

                    Ok(TotalFeeCalculation::new(
                        fixed_fee_lamports,
                        fee_calculation.base_fee,
                        fee_calculation.kora_signature_fee,
                        fee_calculation.fee_payer_outflow,
                        fee_calculation.payment_instruction_fee,
                        fee_calculation.transfer_fee_amount,
                    ))
                } else {
                    Ok(TotalFeeCalculation::new_fixed(fixed_fee_lamports))
                }
            }
            PriceModel::Margin { .. } => {
                // Get the raw transaction
                let fee_calculation = Self::estimate_transaction_fee(
                    config,
                    rpc_client,
                    transaction,
                    fee_payer,
                    is_payment_required,
                )
                .await?;

                let total_fee_lamports = config
                    .validation
                    .price
                    .get_required_lamports_with_margin(fee_calculation.total_fee_lamports)
                    .await?;

                Ok(TotalFeeCalculation::new(
                    total_fee_lamports,
                    fee_calculation.base_fee,
                    fee_calculation.kora_signature_fee,
                    fee_calculation.fee_payer_outflow,
                    fee_calculation.payment_instruction_fee,
                    fee_calculation.transfer_fee_amount,
                ))
            }
        }
    }

    /// Calculate the fee in a specific token if provided
    pub async fn calculate_fee_in_token(
        config: &Config,
        rpc_client: &RpcClient,
        fee_in_lamports: u64,
        fee_token: Option<&str>,
    ) -> Result<Option<u64>, KoraError> {
        if let Some(fee_token) = fee_token {
            let token_mint = Pubkey::from_str(fee_token).map_err(|_| {
                KoraError::InvalidTransaction("Invalid fee token mint address".to_string())
            })?;

            let validation_config = &config.validation;

            if !validation_config.supports_token(fee_token) {
                return Err(KoraError::InvalidRequest(format!(
                    "Token {fee_token} is not supported"
                )));
            }

            let fee_value_in_token = TokenUtil::calculate_lamports_value_in_token(
                fee_in_lamports,
                &token_mint,
                &validation_config.price_source,
                rpc_client,
            )
            .await?;

            Ok(Some(fee_value_in_token))
        } else {
            Ok(None)
        }
    }

    /// Calculate the total outflow (SOL + SPL token value) that could occur for a fee payer account in a transaction.
    /// This includes SOL transfers, account creation, SPL token transfers, and other operations that could drain the fee payer's balance.
    pub async fn calculate_fee_payer_outflow(
        config: &Config,
        fee_payer_pubkey: &Pubkey,
        transaction: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        price_source: &PriceSource,
    ) -> Result<u64, KoraError> {
        let mut total = 0u64;

        // Calculate SOL outflow from System Program instructions
        let parsed_system_instructions = transaction.get_or_parse_system_instructions()?;

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemTransfer { lamports, sender, receiver } =
                instruction
            {
                if *sender == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
                if *receiver == *fee_payer_pubkey {
                    total = total.saturating_sub(*lamports);
                }
            }
        }

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemCreateAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemCreateAccount { lamports, payer } =
                instruction
            {
                if *payer == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
            }
        }

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemWithdrawNonceAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemWithdrawNonceAccount {
                lamports,
                nonce_authority,
                recipient,
            } = instruction
            {
                if *nonce_authority == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
                if *recipient == *fee_payer_pubkey {
                    total = total.saturating_sub(*lamports);
                }
            }
        }

        // Calculate SPL token transfer outflow (converted to lamports value)
        let spl_instructions = transaction.get_or_parse_spl_instructions()?;
        let empty_vec = vec![];
        let spl_transfers =
            spl_instructions.get(&ParsedSPLInstructionType::SplTokenTransfer).unwrap_or(&empty_vec);

        if !spl_transfers.is_empty() {
            let spl_outflow = TokenUtil::calculate_spl_transfers_value_in_lamports(
                config,
                spl_transfers,
                fee_payer_pubkey,
                price_source,
                rpc_client,
            )
            .await?;

            total = total.checked_add(spl_outflow).ok_or_else(|| {
                log::error!("Fee payer outflow overflow: sol={}, spl={}", total, spl_outflow);
                KoraError::ValidationError("Fee payer outflow calculation overflow".to_string())
            })?;
        }

        Ok(total)
    }
}

pub struct TransactionFeeUtil {}

impl TransactionFeeUtil {
    pub async fn get_estimate_fee(
        rpc_client: &RpcClient,
        message: &VersionedMessage,
    ) -> Result<u64, KoraError> {
        match message {
            VersionedMessage::Legacy(message) => rpc_client.get_fee_for_message(message).await,
            VersionedMessage::V0(message) => rpc_client.get_fee_for_message(message).await,
        }
        .map_err(|e| KoraError::RpcError(e.to_string()))
    }

    /// Get fee estimate for a resolved transaction, handling V0 transactions with lookup tables
    pub async fn get_estimate_fee_resolved(
        rpc_client: &RpcClient,
        resolved_transaction: &VersionedTransactionResolved,
    ) -> Result<u64, KoraError> {
        let message = &resolved_transaction.transaction.message;

        match message {
            VersionedMessage::Legacy(message) => {
                // Legacy transactions don't have lookup tables, use as-is
                rpc_client.get_fee_for_message(message).await
            }
            VersionedMessage::V0(v0_message) => {
                // Create a legacy message with resolved account keys to avoid lookup table index issues
                // This is a workaround for https://github.com/anza-xyz/agave/pull/7719

                let resolved_message = Message::new_with_compiled_instructions(
                    v0_message.header.num_required_signatures,
                    v0_message.header.num_readonly_signed_accounts,
                    v0_message.header.num_readonly_unsigned_accounts,
                    resolved_transaction.all_account_keys.clone(),
                    v0_message.recent_blockhash,
                    v0_message.instructions.clone(),
                );

                rpc_client.get_fee_for_message(&resolved_message).await
            }
        }
        .map_err(|e| KoraError::RpcError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constant::{ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION, LAMPORTS_PER_SIGNATURE},
        fee::fee::{FeeConfigUtil, TransactionFeeUtil},
        tests::{
            common::{
                create_mock_rpc_client_with_account, create_mock_token_account,
                setup_or_get_test_config, setup_or_get_test_signer,
            },
            config_mock::{mock_state::get_config, ConfigMockBuilder},
            rpc_mock::RpcMockBuilder,
        },
        token::{interface::TokenInterface, spl_token::TokenProgram},
        transaction::TransactionUtil,
    };
    use solana_message::{v0, Message, VersionedMessage};
    use solana_sdk::{
        account::Account,
        hash::Hash,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
    };
    use solana_system_interface::{
        instruction::{
            create_account, create_account_with_seed, transfer, transfer_with_seed,
            withdraw_nonce_account,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };
    use spl_associated_token_account_interface::address::get_associated_token_address;

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));

        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction, &fee_payer).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_not_signer() {
        let fee_payer_pubkey = setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&sender.pubkey(), &recipient.pubkey(), 1000);

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));

        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(!FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction, &fee_payer_pubkey)
            .unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &fee_payer,
            &[transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction, &fee_payer).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_not_signer() {
        let fee_payer_pubkey = setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &sender.pubkey(),
            &[transfer(&sender.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(!FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction, &fee_payer_pubkey)
            .unwrap());
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as sender - should add to outflow
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient - should subtract from outflow
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow (saturating)");

        // Test 3: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer_with_seed() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as sender (index 1 for TransferWithSeed)
        let transfer_instruction = transfer_with_seed(
            &fee_payer,
            &fee_payer,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &recipient,
            150_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        // Test 2: Fee payer as recipient (index 2 for TransferWithSeed)
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer_with_seed(
            &other_sender,
            &other_sender,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &fee_payer,
            75_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 0,
            "TransferWithSeed to fee payer should subtract from outflow (saturating)"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_create_account() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Test 1: Fee payer funding CreateAccount
        let create_instruction =
            create_account(&fee_payer, &new_account, 200_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        // Test 2: Other account funding CreateAccount
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_create_account_with_seed() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Test: Fee payer funding CreateAccountWithSeed
        let create_instruction = create_account_with_seed(
            &fee_payer,
            &new_account,
            &fee_payer,
            "test_seed",
            300_000,
            100,
            &SYSTEM_PROGRAM_ID,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_nonce_withdraw() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let nonce_account = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as nonce account (outflow)
        let withdraw_instruction =
            withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 50_000,
            "WithdrawNonceAccount from fee payer nonce should add to outflow"
        );

        // Test 2: Fee payer as recipient (inflow)
        let nonce_account = Pubkey::new_unique();
        let withdraw_instruction =
            withdraw_nonce_account(&nonce_account, &fee_payer, &fee_payer, 25_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 0,
            "WithdrawNonceAccount to fee payer should subtract from outflow (saturating)"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_multiple_instructions() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Multiple instructions involving fee payer
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100,000
            transfer(&sender, &fee_payer, 30_000),     // -30,000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50,000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_non_system_program() {
        setup_or_get_test_config();
        let mocked_rpc_client = RpcMockBuilder::new().build();
        let fee_payer = Pubkey::new_unique();
        let fake_program = Pubkey::new_unique();

        // Test with non-system program - should not affect outflow
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let config = get_config().unwrap();
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &config,
            &fee_payer,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &crate::oracle::PriceSource::Mock,
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Non-system program should not affect outflow");
    }

    #[tokio::test]
    async fn test_analyze_payment_instructions_with_payment() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let cache_ctx = CacheUtil::get_account_context();
        cache_ctx.checkpoint();
        let signer = setup_or_get_test_signer();
        let mint = Pubkey::new_unique();

        let mocked_account = create_mock_token_account(&signer, &mint);
        let mocked_rpc_client = create_mock_rpc_client_with_account(&mocked_account);

        // Set up cache expectation for token account lookup
        cache_ctx.expect().times(1).returning(move |_, _, _, _| Ok(mocked_account.clone()));

        let sender = Keypair::new();

        let sender_token_account = get_associated_token_address(&sender.pubkey(), &mint);
        let payment_token_account = get_associated_token_address(&signer, &mint);

        let transfer_instruction = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &payment_token_account,
                &sender.pubkey(),
                1000,
            )
            .unwrap();

        // Create message with the payment instruction
        let message = VersionedMessage::Legacy(Message::new(&[transfer_instruction], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let (has_payment, transfer_fees) = FeeConfigUtil::analyze_payment_instructions(
            &config,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &signer,
        )
        .await
        .unwrap();

        assert!(has_payment, "Should detect payment instruction");
        assert_eq!(transfer_fees, 0, "Should have no transfer fees for SPL token");
    }

    #[tokio::test]
    async fn test_analyze_payment_instructions_without_payment() {
        let signer = setup_or_get_test_signer();
        setup_or_get_test_config();
        let mocked_rpc_client = create_mock_rpc_client_with_account(&Account::default());

        let sender = Keypair::new();
        let recipient = Pubkey::new_unique();

        // Create SOL transfer instruction (no SPL transfer to payment destination)
        let sol_transfer = transfer(&sender.pubkey(), &recipient, 100_000);

        // Create message without payment instruction
        let message = VersionedMessage::Legacy(Message::new(&[sol_transfer], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let (has_payment, transfer_fees) = FeeConfigUtil::analyze_payment_instructions(
            &config,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &signer,
        )
        .await
        .unwrap();

        assert!(!has_payment, "Should not detect payment instruction");
        assert_eq!(transfer_fees, 0, "Should have no transfer fees");
    }

    #[tokio::test]
    async fn test_analyze_payment_instructions_with_wrong_destination() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let cache_ctx = CacheUtil::get_account_context();
        cache_ctx.checkpoint();
        let signer = setup_or_get_test_signer();
        let sender = Keypair::new();
        let mint = Pubkey::new_unique();

        let mocked_account = create_mock_token_account(&sender.pubkey(), &mint);
        let mocked_rpc_client = create_mock_rpc_client_with_account(&mocked_account);

        // Set up cache expectation for token account lookup
        cache_ctx.expect().times(1).returning(move |_, _, _, _| Ok(mocked_account.clone()));

        // Create token accounts
        let sender_token_account = get_associated_token_address(&sender.pubkey(), &mint);
        let recipient_token_account = get_associated_token_address(&sender.pubkey(), &mint);

        // Create SPL transfer instruction to DIFFERENT destination (not payment)
        let transfer_instruction = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &recipient_token_account,
                &sender.pubkey(),
                1000,
            )
            .unwrap();

        // Create message with non-payment transfer
        let message = VersionedMessage::Legacy(Message::new(&[transfer_instruction], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let (has_payment, transfer_fees) = FeeConfigUtil::analyze_payment_instructions(
            &config,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &signer,
        )
        .await
        .unwrap();

        assert!(!has_payment, "Should not detect payment to wrong destination");
        assert_eq!(transfer_fees, 0, "Should have no transfer fees");
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_basic() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let fee_payer = Keypair::new();
        let recipient = Pubkey::new_unique();

        // Mock RPC client that returns base fee
        let mocked_rpc_client = RpcMockBuilder::new().with_fee_estimate(5000).build();

        // Create simple SOL transfer
        let transfer_instruction = transfer(&fee_payer.pubkey(), &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer_instruction],
            Some(&fee_payer.pubkey()),
        ));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let result = FeeConfigUtil::estimate_transaction_fee(
            &config,
            &mocked_rpc_client,
            &mut resolved_transaction,
            &fee_payer.pubkey(),
            false,
        )
        .await
        .unwrap();

        // Should include base fee (5000) + fee payer outflow (100_000)
        assert_eq!(result.total_fee_lamports, 105_000, "Should return base fee + outflow");
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_kora_signer_not_in_signers() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let sender = Keypair::new();
        let kora_fee_payer = Keypair::new();
        let recipient = Pubkey::new_unique();

        let mocked_rpc_client = RpcMockBuilder::new().with_fee_estimate(5000).build();

        // Create transaction where sender pays, but kora_fee_payer is different
        let transfer_instruction = transfer(&sender.pubkey(), &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&sender.pubkey())));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let result = FeeConfigUtil::estimate_transaction_fee(
            &config,
            &mocked_rpc_client,
            &mut resolved_transaction,
            &kora_fee_payer.pubkey(),
            false,
        )
        .await
        .unwrap();

        // Should include base fee + kora signature fee since kora signer not in transaction signers
        assert_eq!(
            result.total_fee_lamports,
            5000 + LAMPORTS_PER_SIGNATURE,
            "Should add Kora signature fee"
        );
    }

    #[tokio::test]
    async fn test_estimate_transaction_fee_with_payment_required() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let cache_ctx = CacheUtil::get_account_context();
        cache_ctx.checkpoint();

        let fee_payer = Keypair::new();
        let recipient = Pubkey::new_unique();

        let mocked_rpc_client = RpcMockBuilder::new().with_fee_estimate(5000).build();

        // Create transaction with no payment instruction
        let transfer_instruction = transfer(&fee_payer.pubkey(), &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer_instruction],
            Some(&fee_payer.pubkey()),
        ));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let result = FeeConfigUtil::estimate_transaction_fee(
            &config,
            &mocked_rpc_client,
            &mut resolved_transaction,
            &fee_payer.pubkey(),
            true, // payment required
        )
        .await
        .unwrap();

        // Should include base fee + fee payer outflow + payment instruction fee
        let expected = 5000 + 100_000 + ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION;
        assert_eq!(
            result.total_fee_lamports, expected,
            "Should include payment instruction fee when required"
        );
    }

    #[tokio::test]
    async fn test_analyze_payment_instructions_with_multiple_payments() {
        let _m = ConfigMockBuilder::new().build_and_setup();
        let cache_ctx = CacheUtil::get_account_context();
        cache_ctx.checkpoint();
        let signer = setup_or_get_test_signer();
        let mint = Pubkey::new_unique();

        let mocked_account = create_mock_token_account(&signer, &mint);
        let mocked_rpc_client = create_mock_rpc_client_with_account(&mocked_account);

        cache_ctx.expect().times(2).returning(move |_, _, _, _| Ok(mocked_account.clone()));

        let sender = Keypair::new();
        let sender_token_account = get_associated_token_address(&sender.pubkey(), &mint);
        let payment_token_account = get_associated_token_address(&signer, &mint);

        let transfer_1 = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &payment_token_account,
                &sender.pubkey(),
                500,
            )
            .unwrap();

        let transfer_2 = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &payment_token_account,
                &sender.pubkey(),
                500,
            )
            .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_1, transfer_2], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let config = get_config().unwrap();
        let (has_payment, transfer_fees) = FeeConfigUtil::analyze_payment_instructions(
            &config,
            &mut resolved_transaction,
            &mocked_rpc_client,
            &signer,
        )
        .await
        .unwrap();

        assert!(has_payment, "Should detect payment instructions");
        assert_eq!(transfer_fees, 0, "Should have no transfer fees for SPL tokens");
    }

    #[tokio::test]
    async fn test_transaction_fee_util_get_estimate_fee_legacy() {
        let mocked_rpc_client = RpcMockBuilder::new().with_fee_estimate(7500).build();

        let fee_payer = Keypair::new();
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer.pubkey(), &recipient, 50_000);

        let legacy_message = Message::new(&[transfer_instruction], Some(&fee_payer.pubkey()));
        let versioned_message = VersionedMessage::Legacy(legacy_message);

        let result = TransactionFeeUtil::get_estimate_fee(&mocked_rpc_client, &versioned_message)
            .await
            .unwrap();

        assert_eq!(result, 7500, "Should return mocked base fee for legacy message");
    }

    #[tokio::test]
    async fn test_transaction_fee_util_get_estimate_fee_v0() {
        let mocked_rpc_client = RpcMockBuilder::new().with_fee_estimate(12500).build();

        let fee_payer = Keypair::new();
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer.pubkey(), &recipient, 50_000);

        let v0_message = v0::Message::try_compile(
            &fee_payer.pubkey(),
            &[transfer_instruction],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let versioned_message = VersionedMessage::V0(v0_message);

        let result = TransactionFeeUtil::get_estimate_fee(&mocked_rpc_client, &versioned_message)
            .await
            .unwrap();

        assert_eq!(result, 12500, "Should return mocked base fee for V0 message");
    }
}
