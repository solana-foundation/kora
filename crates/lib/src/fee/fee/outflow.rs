use std::collections::{HashMap, HashSet};

use crate::{
    config::Config,
    error::KoraError,
    token::{
        spl_token_2022::Token2022Mint,
        token::{AtaCreationInstructionInfo, TokenUtil},
    },
    transaction::{
        ParsedALTInstructionData, ParsedALTInstructionType,
        ParsedBpfLoaderUpgradeableInstructionData, ParsedBpfLoaderUpgradeableInstructionType,
        ParsedSPLInstructionData, ParsedSPLInstructionType, ParsedSystemInstructionData,
        ParsedSystemInstructionType, VersionedTransactionResolved,
    },
};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token_2022_interface::{extension::ExtensionType, state::Account as Token2022Account};

use super::FeeConfigUtil;

impl FeeConfigUtil {
    async fn estimate_ata_account_len(
        ata_creation: &AtaCreationInstructionInfo,
        rpc_client: &RpcClient,
        config: &Config,
        token2022_account_len_cache: &mut HashMap<Pubkey, usize>,
    ) -> Result<usize, KoraError> {
        if ata_creation.token_program == spl_token_interface::id() {
            return Ok(spl_token_interface::state::Account::LEN);
        }

        if ata_creation.token_program == spl_token_2022_interface::id() {
            if let Some(account_len) = token2022_account_len_cache.get(&ata_creation.mint) {
                return Ok(*account_len);
            }

            let mint_state = TokenUtil::get_mint(config, rpc_client, &ata_creation.mint).await?;
            let account_len =
                if let Some(token2022_mint) = mint_state.as_any().downcast_ref::<Token2022Mint>() {
                    #[allow(deprecated)]
                    let mut required_account_extensions =
                        ExtensionType::get_required_init_account_extensions(
                            &token2022_mint.extensions_types,
                        );
                    if !required_account_extensions.contains(&ExtensionType::ImmutableOwner) {
                        required_account_extensions.push(ExtensionType::ImmutableOwner);
                    }

                    ExtensionType::try_calculate_account_len::<Token2022Account>(
                        &required_account_extensions,
                    )
                    .map_err(|e| {
                        KoraError::ValidationError(format!(
                            "Failed to estimate Token2022 ATA account size for mint {}: {}",
                            ata_creation.mint, e
                        ))
                    })?
                } else {
                    spl_token_interface::state::Account::LEN
                };

            token2022_account_len_cache.insert(ata_creation.mint, account_len);
            return Ok(account_len);
        }

        Err(KoraError::ValidationError(format!(
            "Unsupported token program {} for ATA {}; cannot safely estimate rent",
            ata_creation.token_program, ata_creation.ata_address
        )))
    }

    async fn calculate_ata_creation_outflow(
        fee_payer_pubkey: &Pubkey,
        transaction: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<u64, KoraError> {
        let ata_creations = TokenUtil::find_fee_payer_ata_creations(
            &transaction.all_instructions,
            fee_payer_pubkey,
        );
        if ata_creations.is_empty() {
            return Ok(0);
        }

        let system_created_accounts: HashSet<Pubkey> =
            transaction
                .get_or_parse_system_instructions()?
                .get(&ParsedSystemInstructionType::SystemCreateAccount)
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|instruction| match instruction {
                    ParsedSystemInstructionData::SystemCreateAccount {
                        payer, new_account, ..
                    } if *payer == *fee_payer_pubkey => Some(*new_account),
                    _ => None,
                })
                .collect();
        let mut rent_cache_by_account_len: HashMap<usize, u64> = HashMap::new();
        let mut token2022_account_len_cache: HashMap<Pubkey, usize> = HashMap::new();
        let mut seen_ata_addresses = HashSet::new();
        let mut total = 0u64;

        for ata_creation in ata_creations {
            // Simulation may surface inner SystemCreateAccount CPI; skip to avoid double-counting
            if system_created_accounts.contains(&ata_creation.ata_address) {
                continue;
            }
            if !seen_ata_addresses.insert(ata_creation.ata_address) {
                continue;
            }

            let account_len = Self::estimate_ata_account_len(
                &ata_creation,
                rpc_client,
                config,
                &mut token2022_account_len_cache,
            )
            .await?;

            let rent_lamports = if let Some(rent) = rent_cache_by_account_len.get(&account_len) {
                *rent
            } else {
                let rent = rpc_client
                    .get_minimum_balance_for_rent_exemption(account_len)
                    .await
                    .map_err(|e| {
                        KoraError::RpcError(format!(
                            "Failed to fetch rent exemption for account length {}: {}",
                            account_len, e
                        ))
                    })?;
                rent_cache_by_account_len.insert(account_len, rent);
                rent
            };

            total = total.checked_add(rent_lamports).ok_or_else(|| {
                log::error!(
                    "Outflow calculation overflow in ATA creation accounting: total={}, rent={}",
                    total,
                    rent_lamports
                );
                KoraError::ValidationError("Outflow calculation overflow".to_string())
            })?;
        }

        Ok(total)
    }

    fn add_outflow(total: &mut i128, lamports: u64, source: &str) -> Result<(), KoraError> {
        *total = total.checked_add(lamports as i128).ok_or_else(|| {
            log::error!("Outflow calculation overflow in {source}");
            KoraError::ValidationError("Outflow calculation overflow".to_string())
        })?;
        Ok(())
    }

    fn add_inflow(total: &mut i128, lamports: u64, source: &str) -> Result<(), KoraError> {
        *total = total.checked_sub(lamports as i128).ok_or_else(|| {
            log::error!("Inflow calculation overflow in {source}");
            KoraError::ValidationError("Inflow calculation overflow".to_string())
        })?;
        Ok(())
    }

    async fn add_closed_account_flow(
        total: &mut i128,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        closed_account: &Pubkey,
        authority: &Pubkey,
        recipient: &Pubkey,
        source: &str,
    ) -> Result<(), KoraError> {
        let is_fee_payer_authority = authority == fee_payer;
        let is_fee_payer_recipient = recipient == fee_payer;
        if is_fee_payer_authority == is_fee_payer_recipient {
            return Ok(());
        }

        let lamports = rpc_client.get_account(closed_account).await?.lamports;
        if is_fee_payer_recipient {
            Self::add_inflow(total, lamports, source)
        } else {
            Self::add_outflow(total, lamports, source)
        }
    }

    /// Calculate the total outflow (SOL + SPL token value) that could occur for a fee payer account in a transaction.
    /// This includes SOL transfers, account creation, SPL token transfers, and other operations that could drain the fee payer's balance.
    pub async fn calculate_fee_payer_outflow(
        fee_payer_pubkey: &Pubkey,
        transaction: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
        config: &Config,
    ) -> Result<i128, KoraError> {
        // Use i128 to correctly handle net outflow when inflows are processed
        // before outflows. With u64, saturating_sub on 0 would silently discard inflows.
        let mut total: i128 = 0;

        let parsed_system_instructions = transaction.get_or_parse_system_instructions()?;

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemTransfer { lamports, sender, receiver } =
                instruction
            {
                if *sender == *fee_payer_pubkey {
                    Self::add_outflow(&mut total, *lamports, "SystemTransfer")?;
                }
                if *receiver == *fee_payer_pubkey {
                    Self::add_inflow(&mut total, *lamports, "SystemTransfer")?;
                }
            }
        }

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemCreateAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemCreateAccount { lamports, payer, .. } =
                instruction
            {
                if *payer == *fee_payer_pubkey {
                    Self::add_outflow(&mut total, *lamports, "SystemCreateAccount")?;
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
                if *recipient == *fee_payer_pubkey && *nonce_authority == *fee_payer_pubkey {
                    // Self-withdrawals only move lamports between fee-payer-controlled accounts.
                    // They should not reduce unrelated outflow elsewhere in the transaction.
                    continue;
                } else if *recipient == *fee_payer_pubkey {
                    // Lamports arriving from a nonce account not controlled by the fee payer are
                    // a real inflow that reduces net outflow.
                    Self::add_inflow(&mut total, *lamports, "SystemWithdrawNonceAccount")?;
                } else if *nonce_authority == *fee_payer_pubkey {
                    // Fee payer authorized a withdrawal to a third party. The lamports leave a
                    // nonce account the fee payer controls, so count them as outflow so that
                    // max_allowed_lamports enforcement is not bypassed.
                    Self::add_outflow(&mut total, *lamports, "SystemWithdrawNonceAccount")?;
                }
            }
        }

        let parsed_alt_instructions = transaction.get_or_parse_alt_instructions()?;
        for instruction in parsed_alt_instructions
            .get(&ParsedALTInstructionType::AltCloseLookupTable)
            .unwrap_or(&vec![])
        {
            if let ParsedALTInstructionData::AltCloseLookupTable {
                lookup_table_account,
                lookup_table_authority,
                recipient,
            } = instruction
            {
                Self::add_closed_account_flow(
                    &mut total,
                    rpc_client,
                    fee_payer_pubkey,
                    lookup_table_account,
                    lookup_table_authority,
                    recipient,
                    "AltCloseLookupTable",
                )
                .await?;
            }
        }

        // Loader-v3 ExtendProgram/ExtendProgramChecked grow a ProgramData account and top up its
        // rent from the payer. When the fee payer funds the extension, count that rent so a large
        // extension cannot bypass max_allowed_lamports.
        let bpf_v3_instructions = transaction.get_or_parse_bpf_loader_upgradeable_instructions()?;
        for instruction in [
            ParsedBpfLoaderUpgradeableInstructionType::ExtendProgram,
            ParsedBpfLoaderUpgradeableInstructionType::ExtendProgramChecked,
        ]
        .iter()
        .flat_map(|ty| bpf_v3_instructions.get(ty).map(Vec::as_slice).unwrap_or(&[]))
        {
            let (payer, additional_bytes) = match instruction {
                ParsedBpfLoaderUpgradeableInstructionData::ExtendProgram {
                    payer,
                    additional_bytes,
                    ..
                }
                | ParsedBpfLoaderUpgradeableInstructionData::ExtendProgramChecked {
                    payer,
                    additional_bytes,
                    ..
                } => (payer, *additional_bytes),
                _ => continue,
            };

            if *payer == Some(*fee_payer_pubkey) {
                // Conservatively charge the rent-exempt minimum for the added bytes per extension
                // (matching the ATA-creation accounting below).
                let extension_rent = rpc_client
                    .get_minimum_balance_for_rent_exemption(additional_bytes as usize)
                    .await?;
                Self::add_outflow(&mut total, extension_rent, "ExtendProgram rent")?;
            }
        }

        // ATA Create/CreateIdempotent can be no-ops during simulation depending on prestate.
        // Charge conservative rent for fee-payer-funded ATA creations whenever inner SystemCreateAccount
        // did not surface, preventing stale-state rent drain windows.
        let ata_outflow =
            Self::calculate_ata_creation_outflow(fee_payer_pubkey, transaction, rpc_client, config)
                .await?;
        total = total.checked_add(ata_outflow as i128).ok_or_else(|| {
            log::error!(
                "Outflow calculation overflow in ATA accounting: sol_total={}, ata_outflow={}",
                total,
                ata_outflow
            );
            KoraError::ValidationError("Outflow calculation overflow".to_string())
        })?;

        let spl_instructions = transaction.get_or_parse_spl_instructions()?;
        let empty_vec = vec![];
        let spl_transfers =
            spl_instructions.get(&ParsedSPLInstructionType::SplTokenTransfer).unwrap_or(&empty_vec);

        if !spl_transfers.is_empty() {
            let spl_outflow = TokenUtil::calculate_spl_transfers_value_in_lamports(
                spl_transfers,
                fee_payer_pubkey,
                rpc_client,
                config,
            )
            .await?;

            total = total.checked_add(spl_outflow as i128).ok_or_else(|| {
                log::error!("Fee payer outflow overflow: sol={}, spl={}", total, spl_outflow);
                KoraError::ValidationError("Fee payer outflow calculation overflow".to_string())
            })?;
        }

        // A fee-payer-authorized close to a third party moves the closed account's rent out.
        let close_accounts = spl_instructions
            .get(&ParsedSPLInstructionType::SplTokenCloseAccount)
            .unwrap_or(&empty_vec);
        for instruction in close_accounts {
            if let ParsedSPLInstructionData::SplTokenCloseAccount {
                owner,
                account,
                destination,
                ..
            } = instruction
            {
                Self::add_closed_account_flow(
                    &mut total,
                    rpc_client,
                    fee_payer_pubkey,
                    account,
                    owner,
                    destination,
                    "SplTokenCloseAccount",
                )
                .await?;
            }
        }

        Ok(total)
    }
}
