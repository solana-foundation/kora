use crate::{
    config::Config,
    error::KoraError,
    token::{
        interface::TokenMint,
        spl_token::TokenProgram,
        spl_token_2022::{Token2022Account, Token2022Extensions, Token2022Mint, Token2022Program},
        TokenInterface,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::collections::{HashMap, HashSet};

use super::{PaymentLamportTotals, TokenUtil};

impl TokenUtil {
    async fn calculate_token2022_net_amount(
        amount: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
        config: &Config,
        cached_epoch: &mut Option<u64>,
        token2022_mints: &mut HashMap<Pubkey, Box<dyn TokenMint>>,
    ) -> Result<u64, KoraError> {
        let current_epoch = match *cached_epoch {
            Some(epoch) => epoch,
            None => {
                let epoch = rpc_client
                    .get_epoch_info()
                    .await
                    .map_err(|e| KoraError::RpcError(e.to_string()))?
                    .epoch;
                *cached_epoch = Some(epoch);
                epoch
            }
        };

        if !token2022_mints.contains_key(mint) {
            let mint_account = CacheUtil::get_account(config, rpc_client, mint, true).await?;
            let token_program = Token2022Program::new();
            let mint_state = token_program.unpack_mint(mint, &mint_account.data)?;
            token2022_mints.insert(*mint, mint_state);
        }

        let mint_2022 = token2022_mints
            .get(mint)
            .ok_or_else(|| {
                KoraError::InternalServerError(format!(
                    "Missing cached Token2022 mint state for mint {mint}",
                ))
            })?
            .as_any()
            .downcast_ref::<Token2022Mint>()
            .ok_or_else(|| {
                KoraError::SerializationError(
                    "Failed to downcast mint state for transfer fee check".to_string(),
                )
            })?;

        if let Some(fee) = mint_2022.calculate_transfer_fee(amount, current_epoch)? {
            Ok(amount.saturating_sub(fee))
        } else {
            Ok(amount)
        }
    }

    /// Validate Token2022 extensions for payment instructions
    /// This checks if any blocked extensions are present on the payment accounts
    pub async fn validate_token2022_extensions_for_payment(
        config: &Config,
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        destination_address: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        Self::validate_token2022_partial_for_ata_creation(config, rpc_client, source_address, mint)
            .await?;
        Self::validate_token2022_account_extensions(
            config,
            rpc_client,
            destination_address,
            "destination",
        )
        .await
    }

    /// Validate Token2022 extensions for payment when destination ATA is being created.
    /// Only validates mint and source account extensions (destination doesn't exist yet).
    pub async fn validate_token2022_partial_for_ata_creation(
        config: &Config,
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        let token2022_config = &config.validation.token_2022;
        let token_program = Token2022Program::new();

        // Get mint account data and validate mint extensions
        let mint_account = CacheUtil::get_account(config, rpc_client, mint, true).await?;
        let mint_state = token_program.unpack_mint(mint, &mint_account.data)?;

        let mint_with_extensions =
            mint_state.as_any().downcast_ref::<Token2022Mint>().ok_or_else(|| {
                KoraError::SerializationError("Failed to downcast mint state.".to_string())
            })?;

        for extension_type in mint_with_extensions.get_extension_types() {
            if token2022_config.is_mint_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked mint extension found on mint account {mint}",
                )));
            }
        }

        Self::validate_token2022_account_extensions(config, rpc_client, source_address, "source")
            .await
    }

    async fn validate_token2022_account_extensions(
        config: &Config,
        rpc_client: &RpcClient,
        address: &Pubkey,
        label: &str,
    ) -> Result<(), KoraError> {
        let account = CacheUtil::get_account(config, rpc_client, address, true).await?;
        let state = Token2022Program::new().unpack_token_account(&account.data)?;

        let with_extensions =
            state.as_any().downcast_ref::<Token2022Account>().ok_or_else(|| {
                KoraError::SerializationError(format!("Failed to downcast {label} state."))
            })?;

        for extension_type in with_extensions.get_extension_types() {
            if config.validation.token_2022.is_account_extension_blocked(*extension_type) {
                return Err(KoraError::ValidationError(format!(
                    "Blocked account extension found on {label} account {address}",
                )));
            }
        }

        Ok(())
    }

    pub(crate) async fn resolve_token_account_owner_and_mint(
        config: &Config,
        rpc_client: &RpcClient,
        token_program: &dyn TokenInterface,
        account_address: &Pubkey,
        all_instructions: &[Instruction],
    ) -> Result<Option<(Pubkey, Pubkey, bool)>, KoraError> {
        match CacheUtil::get_account(config, rpc_client, account_address, true).await {
            Ok(account) => {
                let token_state =
                    token_program.unpack_token_account(&account.data).map_err(|e| {
                        KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                    })?;

                Ok(Some((token_state.owner(), token_state.mint(), true)))
            }
            Err(e) => {
                if matches!(e, KoraError::AccountNotFound(_)) {
                    Ok(Self::find_ata_creation_for_destination(all_instructions, account_address)
                        .map(|(wallet_owner, ata_mint)| (wallet_owner, ata_mint, false)))
                } else {
                    Err(KoraError::RpcError(e.to_string()))
                }
            }
        }
    }

    /// Calculate payment inflow/outflow totals for transfers involving the expected destination.
    ///
    /// For bundles, pass `bundle_instructions` to enable cross-tx ATA lookup
    /// (e.g., ATA created in Tx1, payment in Tx2).
    pub(crate) async fn calculate_payment_lamport_totals(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        expected_destination_owner: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<PaymentLamportTotals, KoraError> {
        let mut totals = PaymentLamportTotals::default();
        let mut cached_epoch: Option<u64> = None;
        let mut token2022_mints: HashMap<Pubkey, Box<dyn TokenMint>> = HashMap::new();
        let mut payment_mints: HashSet<Pubkey> = HashSet::new();
        struct ValidTransfer {
            is_inflow: bool,
            is_outflow: bool,
            token_mint: Pubkey,
            inflow_amount: u64,
            amount: u64,
        }
        let mut valid_transfers = Vec::new();

        let all_instructions = bundle_instructions
            .map(|instructions| instructions.to_vec())
            .unwrap_or_else(|| transaction_resolved.all_instructions.clone());

        for instruction in transaction_resolved
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                source_address,
                destination_address,
                mint,
                amount,
                is_2022,
                ..
            } = instruction
            {
                let token_program: Box<dyn TokenInterface> = if *is_2022 {
                    Box::new(Token2022Program::new())
                } else {
                    Box::new(TokenProgram::new())
                };

                let source_account_info = Self::resolve_token_account_owner_and_mint(
                    config,
                    rpc_client,
                    token_program.as_ref(),
                    source_address,
                    &all_instructions,
                )
                .await?;
                let destination_account_info = Self::resolve_token_account_owner_and_mint(
                    config,
                    rpc_client,
                    token_program.as_ref(),
                    destination_address,
                    &all_instructions,
                )
                .await?;

                let is_inflow = destination_account_info
                    .as_ref()
                    .map(|(owner, _, _)| owner == expected_destination_owner)
                    .unwrap_or(false);
                let is_outflow = source_account_info
                    .as_ref()
                    .map(|(owner, _, _)| owner == expected_destination_owner)
                    .unwrap_or(false);

                if !is_inflow && !is_outflow {
                    continue;
                }

                let token_mint = mint
                    .or(source_account_info.as_ref().map(|(_, token_mint, _)| *token_mint))
                    .or(destination_account_info.as_ref().map(|(_, token_mint, _)| *token_mint))
                    .ok_or_else(|| {
                        KoraError::InvalidTransaction(
                            "Unable to resolve token mint for payment transfer".to_string(),
                        )
                    })?;

                if !config.validation.supports_token(&token_mint.to_string()) {
                    log::warn!("Ignoring payment with unsupported token mint: {}", token_mint,);
                    continue;
                }

                if *is_2022 && is_inflow {
                    if let Some((_, _, destination_exists)) = destination_account_info.as_ref() {
                        if *destination_exists {
                            TokenUtil::validate_token2022_extensions_for_payment(
                                config,
                                rpc_client,
                                source_address,
                                destination_address,
                                &mint.unwrap_or(token_mint),
                            )
                            .await?;
                        } else {
                            TokenUtil::validate_token2022_partial_for_ata_creation(
                                config,
                                rpc_client,
                                source_address,
                                &token_mint,
                            )
                            .await?;
                        }
                    } else {
                        continue;
                    }
                }

                payment_mints.insert(token_mint);

                let inflow_amount = if *is_2022 && is_inflow {
                    Self::calculate_token2022_net_amount(
                        *amount,
                        &token_mint,
                        rpc_client,
                        config,
                        &mut cached_epoch,
                        &mut token2022_mints,
                    )
                    .await?
                } else {
                    *amount
                };

                valid_transfers.push(ValidTransfer {
                    is_inflow,
                    is_outflow,
                    token_mint,
                    inflow_amount,
                    amount: *amount,
                });
            }
        }

        if payment_mints.is_empty() {
            return Ok(totals);
        }

        let mints: Vec<Pubkey> = payment_mints.into_iter().collect();
        let (prices, mint_decimals) =
            Self::fetch_prices_and_decimals(config, rpc_client, &mints).await?;

        for transfer in valid_transfers {
            let ValidTransfer { is_inflow, is_outflow, token_mint, inflow_amount, amount } =
                transfer;
            let decimals = *mint_decimals.get(&token_mint).ok_or_else(|| {
                KoraError::RpcError(format!("No decimals data for mint {token_mint}"))
            })?;
            let price = prices.get(&token_mint.to_string()).ok_or_else(|| {
                KoraError::RpcError(format!("No price data for mint {token_mint}"))
            })?;

            let inflow_lamports = if is_inflow {
                Self::calculate_token_value_in_lamports_from_price(
                    inflow_amount,
                    price.price,
                    decimals,
                )?
            } else {
                0
            };

            let outflow_lamports = if is_outflow {
                Self::calculate_token_value_in_lamports_from_price(amount, price.price, decimals)?
            } else {
                0
            };

            totals.checked_add_assign(PaymentLamportTotals {
                inflow: inflow_lamports,
                outflow: outflow_lamports,
            })?;
        }

        Ok(totals)
    }

    /// Find the net payment amount in a transaction to the expected destination.
    /// Returns the total payment in lamports, saturating at 0 when outflow exceeds inflow.
    ///
    /// For bundles, pass `bundle_instructions` to enable cross-tx ATA lookup
    /// (e.g., ATA created in Tx1, payment in Tx2).
    pub async fn find_payment_in_transaction(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        expected_destination_owner: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<u64, KoraError> {
        Self::calculate_payment_lamport_totals(
            config,
            transaction_resolved,
            rpc_client,
            expected_destination_owner,
            bundle_instructions,
        )
        .await
        .map(PaymentLamportTotals::net_payment)
    }

    /// Verify that a transaction contains sufficient payment to the expected destination.
    ///
    /// For bundles, pass `bundle_instructions` to enable cross-tx ATA lookup
    /// (e.g., ATA created in Tx1, payment in Tx2).
    pub async fn verify_token_payment(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        required_lamports: u64,
        expected_destination_owner: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<bool, KoraError> {
        let payment = Self::find_payment_in_transaction(
            config,
            transaction_resolved,
            rpc_client,
            expected_destination_owner,
            bundle_instructions,
        )
        .await?;

        Ok(payment >= required_lamports)
    }
}
