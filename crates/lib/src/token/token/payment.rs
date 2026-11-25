use crate::{
    config::Config,
    error::KoraError,
    sanitize_error,
    token::{
        interface::TokenMint,
        spl_token::TokenProgram,
        spl_token_2022::{Token2022Account, Token2022Extensions, Token2022Mint, Token2022Program},
        TokenInterface,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, Token2022SecurityParser,
        VersionedTransactionResolved,
    },
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use std::collections::{HashMap, HashSet};

use super::{PaymentLamportTotals, TokenUtil};

pub(crate) struct KoraTokenTransfer {
    pub source_address: Pubkey,
    pub destination_address: Pubkey,
    pub existing_source_owner: Option<Pubkey>,
    pub destination_exists: bool,
    pub authority: Pubkey,
    pub multisig_signers: Vec<Pubkey>,
    pub mint: Pubkey,
    pub amount: u64,
    pub is_2022: bool,
    pub is_inflow: bool,
    pub is_outflow: bool,
}

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
                    .map_err(|e| KoraError::RpcError(sanitize_error!(e)))?
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
    ) -> Result<Option<(Pubkey, Pubkey, bool, Option<Pubkey>)>, KoraError> {
        match CacheUtil::get_account(config, rpc_client, account_address, true).await {
            Ok(account) => {
                let token_state =
                    token_program.unpack_token_account(&account.data).map_err(|e| {
                        KoraError::InvalidTransaction(format!("Invalid token account: {e}"))
                    })?;

                Ok(Some((
                    token_state.owner(),
                    token_state.mint(),
                    true,
                    token_state.close_authority(),
                )))
            }
            Err(e) => {
                if matches!(e, KoraError::AccountNotFound(_)) {
                    Ok(Self::find_ata_creation_for_destination(all_instructions, account_address)
                        .map(|(wallet_owner, ata_mint)| (wallet_owner, ata_mint, false, None)))
                } else {
                    Err(KoraError::RpcError(e.to_string()))
                }
            }
        }
    }

    /// The destination's mint is read before the transaction, so whoever can close the account
    /// could recreate it at the same address with another mint and owner, inside the transaction
    /// or between signing and landing. A signature from the fee payer does not help: a caller can
    /// collect one for a close or SetAuthority before the account becomes a payment destination
    /// and land it afterwards. So when the payment address is the fee payer and the policy lets
    /// the fee payer close or re-authorize token accounts, only its associated token account
    /// counts, since that address can only ever hold the same mint and owner.
    #[allow(clippy::too_many_arguments)]
    fn is_stable_payment_destination(
        config: &Config,
        destination: &Pubkey,
        destination_mint: &Pubkey,
        close_authority: Option<Pubkey>,
        kora_owner: &Pubkey,
        fee_payer: &Pubkey,
        token_program_id: Pubkey,
        is_2022: bool,
    ) -> bool {
        if kora_owner != fee_payer {
            return close_authority.is_none_or(|authority| authority == *kora_owner);
        }
        if close_authority.is_some() {
            return false;
        }
        let policy = &config.validation.fee_payer_policy;
        let (can_close, can_set_authority) = if is_2022 {
            (policy.token_2022.allow_close_account, policy.token_2022.allow_set_authority)
        } else {
            (policy.spl_token.allow_close_account, policy.spl_token.allow_set_authority)
        };
        !(can_close || can_set_authority)
            || *destination
                == get_associated_token_address_with_program_id(
                    kora_owner,
                    destination_mint,
                    &token_program_id,
                )
    }

    /// Returns the SPL transfers of a token Kora accepts that move funds into or out of token
    /// accounts owned by `kora_owner`. Token accounts created earlier in the transaction (or
    /// bundle, via `bundle_instructions`) are resolved from their ATA creation instruction.
    pub(crate) async fn find_kora_token_transfers(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        kora_owner: &Pubkey,
        fee_payer: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<Vec<KoraTokenTransfer>, KoraError> {
        let all_instructions = bundle_instructions
            .map(|instructions| instructions.to_vec())
            .unwrap_or_else(|| transaction_resolved.all_instructions().to_vec());
        let mut transfers = Vec::new();

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
                owner,
                multisig_signers,
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

                let is_inflow = match &destination_account_info {
                    Some((account_owner, destination_mint, _, close_authority))
                        if account_owner == kora_owner =>
                    {
                        let stable = Self::is_stable_payment_destination(
                            config,
                            destination_address,
                            destination_mint,
                            *close_authority,
                            kora_owner,
                            fee_payer,
                            token_program.program_id(),
                            *is_2022,
                        );
                        if !stable {
                            log::warn!(
                                "Ignoring payment into {destination_address}: an authority other \
                                 than the payment address can close it or rewrite it"
                            );
                        }
                        stable
                    }
                    _ => false,
                };
                let is_outflow = source_account_info
                    .as_ref()
                    .map(|(account_owner, _, _, _)| account_owner == kora_owner)
                    .unwrap_or(false);

                if !is_inflow && !is_outflow {
                    continue;
                }

                if let (None, Some((_, source_mint, _, _)), Some((_, destination_mint, _, _))) =
                    (mint, &source_account_info, &destination_account_info)
                {
                    if source_mint != destination_mint {
                        return Err(KoraError::InvalidTransaction(format!(
                            "Transfer source mint {source_mint} does not match destination mint \
                             {destination_mint}"
                        )));
                    }
                }

                let token_mint = mint
                    .or(source_account_info.as_ref().map(|(_, token_mint, _, _)| *token_mint))
                    .or(destination_account_info.as_ref().map(|(_, token_mint, _, _)| *token_mint))
                    .ok_or_else(|| {
                        KoraError::InvalidTransaction(
                            "Unable to resolve token mint for payment transfer".to_string(),
                        )
                    })?;

                if !config.validation.supports_token(&token_mint.to_string()) {
                    log::warn!("Ignoring payment with unsupported token mint: {}", token_mint,);
                    continue;
                }

                transfers.push(KoraTokenTransfer {
                    source_address: *source_address,
                    destination_address: *destination_address,
                    existing_source_owner: source_account_info
                        .filter(|(_, _, exists, _)| *exists)
                        .map(|(source_owner, _, _, _)| source_owner),
                    destination_exists: destination_account_info
                        .map(|(_, _, exists, _)| exists)
                        .unwrap_or(false),
                    authority: *owner,
                    multisig_signers: multisig_signers.clone(),
                    mint: token_mint,
                    amount: *amount,
                    is_2022: *is_2022,
                    is_inflow,
                    is_outflow,
                });
            }
        }

        Ok(transfers)
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
        fee_payer: &Pubkey,
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

        for transfer in Self::find_kora_token_transfers(
            config,
            transaction_resolved,
            rpc_client,
            expected_destination_owner,
            fee_payer,
            bundle_instructions,
        )
        .await?
        {
            let KoraTokenTransfer {
                source_address,
                destination_address,
                destination_exists,
                mint: token_mint,
                amount,
                is_2022,
                is_inflow,
                is_outflow,
                ..
            } = transfer;
            if is_2022 && is_inflow {
                if destination_exists {
                    TokenUtil::validate_token2022_extensions_for_payment(
                        config,
                        rpc_client,
                        &source_address,
                        &destination_address,
                        &token_mint,
                    )
                    .await?;
                } else {
                    TokenUtil::validate_token2022_partial_for_ata_creation(
                        config,
                        rpc_client,
                        &source_address,
                        &token_mint,
                    )
                    .await?;
                }
            }

            payment_mints.insert(token_mint);

            let inflow_amount = if is_2022 && is_inflow {
                Self::calculate_token2022_net_amount(
                    amount,
                    &token_mint,
                    rpc_client,
                    config,
                    &mut cached_epoch,
                    &mut token2022_mints,
                )
                .await?
            } else {
                amount
            };

            valid_transfers.push(ValidTransfer {
                is_inflow,
                is_outflow,
                token_mint,
                inflow_amount,
                amount,
            });
        }

        let all_instructions = bundle_instructions
            .map(|instructions| instructions.to_vec())
            .unwrap_or_else(|| transaction_resolved.all_instructions().to_vec());
        for instruction in transaction_resolved
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenBurn)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenBurn {
                amount,
                mint,
                source_address,
                is_2022,
                ..
            } = instruction
            {
                let token_program: Box<dyn TokenInterface> = if *is_2022 {
                    Box::new(Token2022Program::new())
                } else {
                    Box::new(TokenProgram::new())
                };
                let Some((owner, source_mint, _, _)) = Self::resolve_token_account_owner_and_mint(
                    config,
                    rpc_client,
                    token_program.as_ref(),
                    source_address,
                    &all_instructions,
                )
                .await?
                else {
                    continue;
                };
                if owner != *expected_destination_owner {
                    continue;
                }
                let token_mint = mint.unwrap_or(source_mint);
                if !config.validation.supports_token(&token_mint.to_string()) {
                    continue;
                }

                payment_mints.insert(token_mint);
                valid_transfers.push(ValidTransfer {
                    is_inflow: false,
                    is_outflow: true,
                    token_mint,
                    inflow_amount: 0,
                    amount: *amount,
                });
            }
        }

        for source in all_instructions
            .iter()
            .filter(|instruction| instruction.program_id == spl_token_2022_interface::id())
            .filter_map(Token2022SecurityParser::permissioned_burn_source)
        {
            let source_owner = Self::resolve_token_account_owner_and_mint(
                config,
                rpc_client,
                &Token2022Program::new(),
                &source,
                &all_instructions,
            )
            .await?
            .map(|(owner, _, _, _)| owner);
            if source_owner == Some(*expected_destination_owner) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Permissioned burn from Kora token account {source} is not supported"
                )));
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
            let (decimals, ui_scale) = *mint_decimals.get(&token_mint).ok_or_else(|| {
                KoraError::RpcError(format!("No decimals data for mint {token_mint}"))
            })?;
            let price = prices.get(&token_mint.to_string()).ok_or_else(|| {
                KoraError::RpcError(format!("No price data for mint {token_mint}"))
            })?;

            let inflow_lamports = if is_inflow {
                Self::calculate_token_value_in_lamports_from_price(
                    inflow_amount,
                    Self::scaled_price(price.price, ui_scale.lower)?,
                    decimals,
                )?
            } else {
                0
            };

            let outflow_lamports = if is_outflow {
                Self::calculate_token_value_in_lamports_from_price(
                    amount,
                    Self::scaled_price(price.price, ui_scale.upper)?,
                    decimals,
                )?
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
        fee_payer: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<u64, KoraError> {
        Self::calculate_payment_lamport_totals(
            config,
            transaction_resolved,
            rpc_client,
            expected_destination_owner,
            fee_payer,
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
        fee_payer: &Pubkey,
        bundle_instructions: Option<&[Instruction]>,
    ) -> Result<bool, KoraError> {
        let payment = Self::find_payment_in_transaction(
            config,
            transaction_resolved,
            rpc_client,
            expected_destination_owner,
            fee_payer,
            bundle_instructions,
        )
        .await?;

        Ok(payment >= required_lamports)
    }
}
