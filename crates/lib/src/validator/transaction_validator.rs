use crate::{
    config::{Config, FeePayerPolicy},
    error::KoraError,
    fee::fee::{FeeConfigUtil, TotalFeeCalculation},
    oracle::PriceSource,
    token::{
        interface::TokenMint,
        token::{TokenUtil, TransferHookValidationFlow},
    },
    transaction::{
        ParsedALTInstructionData, ParsedALTInstructionType, ParsedLoaderV4InstructionData,
        ParsedLoaderV4InstructionType, ParsedSPLInstructionData, ParsedSPLInstructionType,
        ParsedSystemInstructionData, ParsedSystemInstructionType, Token2022AccountUsagePolicy,
        VersionedTransactionResolved,
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::str::FromStr;

use crate::fee::price::PriceModel;

pub struct TransactionValidator {
    fee_payer_pubkey: Pubkey,
    max_allowed_lamports: u64,
    allowed_programs: Vec<Pubkey>,
    require_one_of_programs: Vec<Pubkey>,
    max_signatures: u64,
    allowed_tokens: Vec<Pubkey>,
    disallowed_accounts: Vec<Pubkey>,
    _price_source: PriceSource,
    fee_payer_policy: FeePayerPolicy,
    allow_durable_transactions: bool,
}

impl TransactionValidator {
    pub fn new(config: &Config, fee_payer_pubkey: Pubkey) -> Result<Self, KoraError> {
        let config = &config.validation;

        // Convert string program IDs to Pubkeys
        let allowed_programs = config
            .allowed_programs
            .iter()
            .map(|addr| {
                Pubkey::from_str(addr).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid program address in config: {e}"
                    ))
                })
            })
            .collect::<Result<Vec<Pubkey>, KoraError>>()?;

        let require_one_of_programs = config
            .require_one_of_programs
            .iter()
            .map(|addr| {
                Pubkey::from_str(addr).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid program address in require_one_of_programs config: {e}"
                    ))
                })
            })
            .collect::<Result<Vec<Pubkey>, KoraError>>()?;

        Ok(Self {
            fee_payer_pubkey,
            max_allowed_lamports: config.max_allowed_lamports,
            allowed_programs,
            require_one_of_programs,
            max_signatures: config.max_signatures,
            _price_source: config.price_source.clone(),
            allowed_tokens: config
                .allowed_tokens
                .iter()
                .map(|addr| Pubkey::from_str(addr))
                .collect::<Result<Vec<Pubkey>, _>>()
                .map_err(|e| {
                    KoraError::InternalServerError(format!("Invalid allowed token address: {e}"))
                })?,
            disallowed_accounts: config
                .disallowed_accounts
                .iter()
                .map(|addr| Pubkey::from_str(addr))
                .collect::<Result<Vec<Pubkey>, _>>()
                .map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid disallowed account address: {e}"
                    ))
                })?,
            fee_payer_policy: config.fee_payer_policy.clone(),
            allow_durable_transactions: config.allow_durable_transactions,
        })
    }

    pub async fn fetch_and_validate_token_mint(
        &self,
        mint: &Pubkey,
        config: &Config,
        rpc_client: &RpcClient,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        // First check if the mint is in allowed tokens
        if !self.allowed_tokens.contains(mint) {
            return Err(KoraError::InvalidTransaction(format!(
                "Mint {mint} is not a valid token mint"
            )));
        }

        let mint = TokenUtil::get_mint(config, rpc_client, mint).await?;

        Ok(mint)
    }

    /*
    This function is used to validate a transaction.
     */
    pub async fn validate_transaction(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        if transaction_resolved.all_instructions.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no instructions".to_string(),
            ));
        }

        self.validate_has_non_compute_instruction(transaction_resolved)?;

        if transaction_resolved.all_account_keys.is_empty() {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains no account keys".to_string(),
            ));
        }

        self.validate_signatures(&transaction_resolved.transaction)?;

        self.validate_programs(transaction_resolved)?;
        self.validate_require_one_of_programs(transaction_resolved)?;
        self.validate_transfer_amounts(config, transaction_resolved, rpc_client).await?;
        self.validate_disallowed_accounts(transaction_resolved)?;
        self.validate_fee_payer_usage(config, transaction_resolved)?;

        Ok(())
    }

    pub(crate) fn validate_token2022_transfer_hook_signing_policies(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        transfer_hook_validation_flow: TransferHookValidationFlow,
    ) -> Result<(), KoraError> {
        if !TokenUtil::should_reject_mutable_transfer_hook(config, transfer_hook_validation_flow) {
            return Ok(());
        }

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;

        validate_token2022!(self, spl_instructions, SplTokenInitializeTransferHook,
            ParsedSPLInstructionData::SplTokenInitializeTransferHook {
                authority: Some(authority),
                ..
            } => *authority == self.fee_payer_pubkey,
            "Fee payer cannot initialize mutable Token2022 TransferHook authority");

        validate_token2022!(self, spl_instructions, SplTokenTransferHookUpdate,
            ParsedSPLInstructionData::SplTokenTransferHookUpdate {
                authority,
                multisig_signers,
                ..
            } => *authority == self.fee_payer_pubkey
                || multisig_signers.contains(&self.fee_payer_pubkey) ,
            "Fee payer cannot authorize mutable Token2022 TransferHook updates");

        Ok(())
    }

    fn validate_has_non_compute_instruction(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        let compute_budget_program_id = solana_compute_budget_interface::id();
        let has_non_compute_instruction = transaction_resolved
            .all_instructions
            .iter()
            .any(|ix| ix.program_id != compute_budget_program_id);

        if !has_non_compute_instruction {
            return Err(KoraError::InvalidTransaction(
                "Transaction contains only ComputeBudget instructions".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate_lamport_fee(&self, fee: u64) -> Result<(), KoraError> {
        if fee > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Fee {} exceeds maximum allowed {}",
                fee, self.max_allowed_lamports
            )));
        }
        Ok(())
    }

    fn validate_signatures(&self, transaction: &VersionedTransaction) -> Result<(), KoraError> {
        if transaction.signatures.len() > self.max_signatures as usize {
            return Err(KoraError::InvalidTransaction(format!(
                "Too many signatures: {} > {}",
                transaction.signatures.len(),
                self.max_signatures
            )));
        }

        if transaction.signatures.is_empty() {
            return Err(KoraError::InvalidTransaction("No signatures found".to_string()));
        }

        Ok(())
    }

    fn validate_programs(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        for instruction in &transaction_resolved.all_instructions {
            if !self.allowed_programs.contains(&instruction.program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is not in the allowed list",
                    instruction.program_id
                )));
            }
        }
        Ok(())
    }

    fn validate_require_one_of_programs(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        if self.require_one_of_programs.is_empty() {
            return Ok(());
        }

        let called = transaction_resolved
            .all_instructions
            .iter()
            .any(|ix| self.require_one_of_programs.contains(&ix.program_id));
        if !called {
            return Err(KoraError::InvalidTransaction(format!(
                "Transaction must call at least one of the required programs: {:?}",
                self.require_one_of_programs
            )));
        }

        Ok(())
    }

    fn validate_fee_payer_usage(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        self.validate_ata_create_instructions(transaction_resolved)?;

        let system_instructions = transaction_resolved.get_or_parse_system_instructions()?;

        // Check for durable transactions (nonce-based) - reject if not allowed
        if !self.allow_durable_transactions
            && system_instructions
                .contains_key(&ParsedSystemInstructionType::SystemAdvanceNonceAccount)
        {
            return Err(KoraError::InvalidTransaction(
                "Durable transactions (nonce-based) are not allowed".to_string(),
            ));
        }

        // Validate system program instructions
        validate_system!(self, system_instructions, SystemTransfer,
            ParsedSystemInstructionData::SystemTransfer { sender, .. } => sender,
            self.fee_payer_policy.system.allow_transfer, "System Transfer");

        validate_system!(self, system_instructions, SystemAssign,
            ParsedSystemInstructionData::SystemAssign { authority } => authority,
            self.fee_payer_policy.system.allow_assign, "System Assign");

        validate_system!(self, system_instructions, SystemAllocate,
            ParsedSystemInstructionData::SystemAllocate { account } => account,
            self.fee_payer_policy.system.allow_allocate, "System Allocate");

        validate_system!(self, system_instructions, SystemCreateAccount,
        ParsedSystemInstructionData::SystemCreateAccount { payer, owner, .. } => payer,
        self.fee_payer_policy.system.allow_create_account, "System Create Account", {
            if !self.allowed_programs.contains(owner) {
                return Err(KoraError::InvalidTransaction(format!(
                    "CreateAccount owner program {} is not in the allowed programs list",
                    owner
                )));
            }
            if self.disallowed_accounts.contains(owner) {
                return Err(KoraError::InvalidTransaction(format!(
                    "CreateAccount owner program {} is in the disallowed accounts list",
                    owner
                )));
            }
        });

        validate_system!(self, system_instructions, SystemInitializeNonceAccount,
            ParsedSystemInstructionData::SystemInitializeNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_initialize, "System Initialize Nonce Account");

        validate_system!(self, system_instructions, SystemAdvanceNonceAccount,
            ParsedSystemInstructionData::SystemAdvanceNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_advance, "System Advance Nonce Account");

        validate_system!(self, system_instructions, SystemAuthorizeNonceAccount,
            ParsedSystemInstructionData::SystemAuthorizeNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_authorize, "System Authorize Nonce Account");

        // Note: SystemUpgradeNonceAccount not validated - no authority parameter

        validate_system!(self, system_instructions, SystemWithdrawNonceAccount,
            ParsedSystemInstructionData::SystemWithdrawNonceAccount { nonce_authority, .. } => nonce_authority,
            self.fee_payer_policy.system.nonce.allow_withdraw, "System Withdraw Nonce Account");

        // Validate SPL instructions
        {
            let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;

            validate_spl!(self, spl_instructions, SplTokenTransfer,
                ParsedSPLInstructionData::SplTokenTransfer { owner, is_2022, .. } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_transfer,
                self.fee_payer_policy.token_2022.allow_transfer,
                "SPL Token Transfer", "Token2022 Token Transfer");

            validate_spl!(self, spl_instructions, SplTokenApprove,
                ParsedSPLInstructionData::SplTokenApprove { owner, is_2022, .. } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_approve,
                self.fee_payer_policy.token_2022.allow_approve,
                "SPL Token Approve", "Token2022 Token Approve");

            validate_spl!(self, spl_instructions, SplTokenBurn,
                ParsedSPLInstructionData::SplTokenBurn { owner, is_2022 } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_burn,
                self.fee_payer_policy.token_2022.allow_burn,
                "SPL Token Burn", "Token2022 Token Burn");

            validate_spl!(self, spl_instructions, SplTokenCloseAccount,
                ParsedSPLInstructionData::SplTokenCloseAccount { owner, is_2022 } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_close_account,
                self.fee_payer_policy.token_2022.allow_close_account,
                "SPL Token Close Account", "Token2022 Token Close Account");

            validate_spl!(self, spl_instructions, SplTokenRevoke,
                ParsedSPLInstructionData::SplTokenRevoke { owner, is_2022 } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_revoke,
                self.fee_payer_policy.token_2022.allow_revoke,
                "SPL Token Revoke", "Token2022 Token Revoke");

            validate_spl!(self, spl_instructions, SplTokenSetAuthority,
                ParsedSPLInstructionData::SplTokenSetAuthority { authority, is_2022, .. } => { authority, is_2022 },
                self.fee_payer_policy.spl_token.allow_set_authority,
                self.fee_payer_policy.token_2022.allow_set_authority,
                "SPL Token SetAuthority", "Token2022 Token SetAuthority");

            validate_spl!(self, spl_instructions, SplTokenMintTo,
                ParsedSPLInstructionData::SplTokenMintTo { mint_authority, is_2022 } => { mint_authority, is_2022 },
                self.fee_payer_policy.spl_token.allow_mint_to,
                self.fee_payer_policy.token_2022.allow_mint_to,
                "SPL Token MintTo", "Token2022 Token MintTo");

            validate_spl!(self, spl_instructions, SplTokenInitializeMint,
                ParsedSPLInstructionData::SplTokenInitializeMint { mint_authority, is_2022, .. } => { mint_authority, is_2022 },
                self.fee_payer_policy.spl_token.allow_initialize_mint,
                self.fee_payer_policy.token_2022.allow_initialize_mint,
                "SPL Token InitializeMint", "Token2022 Token InitializeMint");

            validate_spl!(self, spl_instructions, SplTokenInitializeAccount,
                ParsedSPLInstructionData::SplTokenInitializeAccount { owner, is_2022 } => { owner, is_2022 },
                self.fee_payer_policy.spl_token.allow_initialize_account,
                self.fee_payer_policy.token_2022.allow_initialize_account,
                "SPL Token InitializeAccount", "Token2022 Token InitializeAccount");

            validate_spl_multisig!(self, spl_instructions, SplTokenInitializeMultisig,
                ParsedSPLInstructionData::SplTokenInitializeMultisig { signers, is_2022 } => { signers, is_2022 },
                self.fee_payer_policy.spl_token.allow_initialize_multisig,
                self.fee_payer_policy.token_2022.allow_initialize_multisig,
                "SPL Token InitializeMultisig", "Token2022 Token InitializeMultisig");

            validate_spl!(self, spl_instructions, SplTokenFreezeAccount,
                ParsedSPLInstructionData::SplTokenFreezeAccount { freeze_authority, is_2022 } => { freeze_authority, is_2022 },
                self.fee_payer_policy.spl_token.allow_freeze_account,
                self.fee_payer_policy.token_2022.allow_freeze_account,
                "SPL Token FreezeAccount", "Token2022 Token FreezeAccount");

            validate_spl!(self, spl_instructions, SplTokenThawAccount,
                ParsedSPLInstructionData::SplTokenThawAccount { freeze_authority, is_2022 } => { freeze_authority, is_2022 },
                self.fee_payer_policy.spl_token.allow_thaw_account,
                self.fee_payer_policy.token_2022.allow_thaw_account,
                "SPL Token ThawAccount", "Token2022 Token ThawAccount");
        }

        // Validate ALT instructions
        let alt_instructions = transaction_resolved.get_or_parse_alt_instructions()?;

        validate_alt!(self, alt_instructions, AltCreateLookupTable,
            ParsedALTInstructionData::AltCreateLookupTable {
                lookup_table_authority,
                payer_account,
                ..
            } => (*lookup_table_authority == self.fee_payer_pubkey
                || *payer_account == self.fee_payer_pubkey),
            self.fee_payer_policy.alt.allow_create,
            "ALT CreateLookupTable");

        validate_alt!(self, alt_instructions, AltExtendLookupTable,
            ParsedALTInstructionData::AltExtendLookupTable {
                lookup_table_authority,
                payer_account,
                ..
            } => (*lookup_table_authority == self.fee_payer_pubkey
                || payer_account.is_some_and(|payer| payer == self.fee_payer_pubkey)),
            self.fee_payer_policy.alt.allow_extend,
            "ALT ExtendLookupTable");

        validate_alt!(self, alt_instructions, AltFreezeLookupTable,
            ParsedALTInstructionData::AltFreezeLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            self.fee_payer_policy.alt.allow_freeze,
            "ALT FreezeLookupTable");

        validate_alt!(self, alt_instructions, AltDeactivateLookupTable,
            ParsedALTInstructionData::AltDeactivateLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            self.fee_payer_policy.alt.allow_deactivate,
            "ALT DeactivateLookupTable");

        validate_alt!(self, alt_instructions, AltCloseLookupTable,
            ParsedALTInstructionData::AltCloseLookupTable { lookup_table_authority, .. } =>
            *lookup_table_authority == self.fee_payer_pubkey,
            self.fee_payer_policy.alt.allow_close,
            "ALT CloseLookupTable");

        // Validate Loader-v4 (BPF loader successor) instructions.
        let loader_v4_instructions = transaction_resolved.get_or_parse_loader_v4_instructions()?;

        validate_loader_v4!(self, loader_v4_instructions, Write,
            ParsedLoaderV4InstructionData::Write { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_write,
            "Loader-v4 Write");

        validate_loader_v4!(self, loader_v4_instructions, Copy,
            ParsedLoaderV4InstructionData::Copy { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_copy,
            "Loader-v4 Copy");

        validate_loader_v4!(self, loader_v4_instructions, SetProgramLength,
            ParsedLoaderV4InstructionData::SetProgramLength { authority, recipient, .. } =>
            (*authority == self.fee_payer_pubkey
                || recipient.is_some_and(|r| r == self.fee_payer_pubkey)),
            self.fee_payer_policy.loader_v4.allow_set_program_length,
            "Loader-v4 SetProgramLength");

        // Drainage guard: when the fee payer is the SetProgramLength authority, the recipient
        // (if present) must be the fee payer. Otherwise shrink-to-zero or over-funded-growth
        // refunds would flow to an attacker-controlled account, draining Kora's rent.
        for instruction in loader_v4_instructions
            .get(&ParsedLoaderV4InstructionType::SetProgramLength)
            .unwrap_or(&vec![])
        {
            if let ParsedLoaderV4InstructionData::SetProgramLength {
                authority, recipient, ..
            } = instruction
            {
                if *authority == self.fee_payer_pubkey {
                    if let Some(r) = recipient {
                        if *r != self.fee_payer_pubkey {
                            return Err(KoraError::InvalidTransaction(
                                "Loader-v4 SetProgramLength: when fee payer is the authority, \
                                 recipient must also be the fee payer (drainage guard)"
                                    .to_string(),
                            ));
                        }
                    }
                }
            }
        }

        validate_loader_v4!(self, loader_v4_instructions, Deploy,
            ParsedLoaderV4InstructionData::Deploy { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_deploy,
            "Loader-v4 Deploy");

        validate_loader_v4!(self, loader_v4_instructions, Retract,
            ParsedLoaderV4InstructionData::Retract { authority, .. } =>
            *authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_retract,
            "Loader-v4 Retract");

        validate_loader_v4!(self, loader_v4_instructions, TransferAuthority,
            ParsedLoaderV4InstructionData::TransferAuthority { current_authority, .. } =>
            *current_authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_transfer_authority,
            "Loader-v4 TransferAuthority");

        validate_loader_v4!(self, loader_v4_instructions, Finalize,
            ParsedLoaderV4InstructionData::Finalize { current_authority, .. } =>
            *current_authority == self.fee_payer_pubkey,
            self.fee_payer_policy.loader_v4.allow_finalize,
            "Loader-v4 Finalize");

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;
        validate_token2022!(self, spl_instructions, SplTokenReallocate,
            ParsedSPLInstructionData::SplTokenReallocate {
                payer,
                owner,
                is_2022,
                ..
            } => *is_2022
                && (*payer == self.fee_payer_pubkey || *owner == self.fee_payer_pubkey) ,
            "Token2022 Reallocate is not allowed when involving fee payer");

        validate_token2022!(self, spl_instructions, SplTokenPause,
            ParsedSPLInstructionData::SplTokenPause { authority, multisig_signers } =>
            (*authority == self.fee_payer_pubkey
                || multisig_signers.contains(&self.fee_payer_pubkey)),
            self.fee_payer_policy.token_2022.allow_freeze_account,
            "Fee payer cannot be used for Token2022 Pause");

        validate_token2022!(self, spl_instructions, SplTokenResume,
            ParsedSPLInstructionData::SplTokenResume { authority, multisig_signers } =>
            (*authority == self.fee_payer_pubkey
                || multisig_signers.contains(&self.fee_payer_pubkey)),
            self.fee_payer_policy.token_2022.allow_thaw_account,
            "Fee payer cannot be used for Token2022 Resume");

        self.validate_token2022_extension_security(config, transaction_resolved)?;

        Ok(())
    }

    fn validate_ata_create_instructions(
        &self,
        transaction_resolved: &VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        if self.fee_payer_policy.system.allow_create_account {
            return Ok(());
        }

        let has_fee_payer_ata_create = !TokenUtil::find_fee_payer_ata_creations(
            &transaction_resolved.all_instructions,
            &self.fee_payer_pubkey,
        )
        .is_empty();

        if has_fee_payer_ata_create {
            return Err(KoraError::InvalidTransaction(
                "Fee payer cannot fund ATA creation (Create or CreateIdempotent)".to_string(),
            ));
        }

        Ok(())
    }

    async fn validate_transfer_amounts(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<(), KoraError> {
        let total_outflow =
            self.calculate_total_outflow(config, transaction_resolved, rpc_client).await?;

        if total_outflow > self.max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                total_outflow, self.max_allowed_lamports
            )));
        }

        Ok(())
    }

    fn validate_disallowed_accounts(
        &self,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        for instruction in &transaction_resolved.all_instructions {
            if self.disallowed_accounts.contains(&instruction.program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is disallowed",
                    instruction.program_id
                )));
            }

            for account_index in instruction.accounts.iter() {
                if self.disallowed_accounts.contains(&account_index.pubkey) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Account {} is disallowed",
                        account_index.pubkey
                    )));
                }
            }
        }
        // Validate instruction-data pubkeys that are not present in account metas.
        let system_instructions = transaction_resolved.get_or_parse_system_instructions()?;
        for instruction in system_instructions.values().flatten() {
            match instruction {
                ParsedSystemInstructionData::SystemAuthorizeNonceAccount {
                    new_authority, ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        new_authority,
                        "System AuthorizeNonceAccount new_authority",
                    )?;
                }
                ParsedSystemInstructionData::SystemInitializeNonceAccount {
                    nonce_authority,
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        nonce_authority,
                        "System InitializeNonceAccount nonce_authority",
                    )?;
                }
                _ => {}
            }
        }

        let spl_instructions = transaction_resolved.get_or_parse_spl_instructions()?;
        for instruction in spl_instructions.values().flatten() {
            match instruction {
                ParsedSPLInstructionData::SplTokenSetAuthority {
                    new_authority: Some(new_authority),
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        new_authority,
                        "SPL/Token2022 SetAuthority new_authority",
                    )?;
                }
                ParsedSPLInstructionData::SplTokenInitializeAccount { owner, .. } => {
                    self.validate_disallowed_instruction_data_account(
                        owner,
                        "SPL/Token2022 InitializeAccount owner",
                    )?;
                }
                ParsedSPLInstructionData::SplTokenInitializeMint {
                    mint_authority,
                    freeze_authority,
                    ..
                } => {
                    self.validate_disallowed_instruction_data_account(
                        mint_authority,
                        "SPL/Token2022 InitializeMint mint_authority",
                    )?;
                    if let Some(freeze_authority) = freeze_authority {
                        self.validate_disallowed_instruction_data_account(
                            freeze_authority,
                            "SPL/Token2022 InitializeMint freeze_authority",
                        )?;
                    }
                }
                _ => {}
            }
        }

        for instruction in transaction_resolved.get_or_parse_token2022_security_instructions()? {
            for field in &instruction.data_pubkeys {
                self.validate_disallowed_instruction_data_account(&field.pubkey, field.context)?;
            }
        }

        Ok(())
    }

    fn validate_disallowed_instruction_data_account(
        &self,
        account: &Pubkey,
        context: &str,
    ) -> Result<(), KoraError> {
        if self.is_disallowed_account(account) {
            return Err(KoraError::InvalidTransaction(format!(
                "Disallowed account {} found in instruction data for {}",
                account, context
            )));
        }
        Ok(())
    }

    pub fn is_disallowed_account(&self, account: &Pubkey) -> bool {
        self.disallowed_accounts.contains(account)
    }

    fn validate_token2022_extension_security(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
    ) -> Result<(), KoraError> {
        for instruction in transaction_resolved.get_or_parse_token2022_security_instructions()? {
            if matches!(
                instruction.account_usage_policy,
                Token2022AccountUsagePolicy::RejectIfFeePayerPresent
            ) && instruction.accounts.contains(&self.fee_payer_pubkey)
            {
                return Err(KoraError::InvalidTransaction(format!(
                    "Fee payer cannot be an account in {}",
                    instruction.instruction_name
                )));
            }

            if let Some(extension_type) = instruction.extension_type {
                if config.validation.token_2022.is_mint_extension_blocked(extension_type) {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Token2022 instruction '{}' is not allowed because extension '{extension_type:?}' is blocked",
                        instruction.instruction_name
                    )));
                }
            }

            if instruction.uses_fee_payer_as_current_extension_authority(&self.fee_payer_pubkey)
                && !self.fee_payer_policy.token_2022.allow_update_extension_authority
            {
                return Err(KoraError::InvalidTransaction(format!(
                    "Fee payer cannot be used as the current Token2022 extension authority for '{}'",
                    instruction.instruction_name
                )));
            }

            if let Some(field) =
                instruction.find_planted_fee_payer_authority(&self.fee_payer_pubkey)
            {
                if !self.fee_payer_policy.token_2022.allow_initialize_extension_authority {
                    return Err(KoraError::InvalidTransaction(format!(
                        "Fee payer cannot be planted as a Token2022 extension authority via {}",
                        field.context
                    )));
                }
            }
        }

        Ok(())
    }

    async fn calculate_total_outflow(
        &self,
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let net = FeeConfigUtil::calculate_fee_payer_outflow(
            &self.fee_payer_pubkey,
            transaction_resolved,
            rpc_client,
            config,
        )
        .await?;
        Ok(net.max(0) as u64)
    }

    pub async fn validate_token_payment(
        config: &Config,
        transaction_resolved: &mut VersionedTransactionResolved,
        required_lamports: u64,
        rpc_client: &RpcClient,
        expected_payment_destination: &Pubkey,
    ) -> Result<(), KoraError> {
        if TokenUtil::verify_token_payment(
            config,
            transaction_resolved,
            rpc_client,
            required_lamports,
            expected_payment_destination,
            None,
        )
        .await?
        {
            return Ok(());
        }

        Err(KoraError::InvalidTransaction(format!(
            "Insufficient token payment. Required {required_lamports} lamports"
        )))
    }

    pub fn validate_strict_pricing_with_fee(
        config: &Config,
        fee_calculation: &TotalFeeCalculation,
    ) -> Result<(), KoraError> {
        if !matches!(&config.validation.price.model, PriceModel::Fixed { strict: true, .. }) {
            return Ok(());
        }

        let fixed_price_lamports = fee_calculation.total_fee_lamports;
        let total_fee_lamports = fee_calculation.get_total_fee_lamports()?;

        if fixed_price_lamports < total_fee_lamports {
            log::error!(
                "Strict pricing violation: fixed_price_lamports={} < total_fee_lamports={}",
                fixed_price_lamports,
                total_fee_lamports
            );
            return Err(KoraError::ValidationError(format!(
                    "Strict pricing violation: total fee ({} lamports) exceeds fixed price ({} lamports)",
                    total_fee_lamports,
                    fixed_price_lamports
                )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{Config, FeePayerPolicy, TransferHookPolicy},
        state::{get_config, update_config},
        tests::{
            account_mock::{AccountMockBuilder, MintAccountMockBuilder, TokenAccountMockBuilder},
            config_mock::{mock_state::setup_config_mock, ConfigMockBuilder},
            rpc_mock::RpcMockBuilder,
        },
        transaction::TransactionUtil,
    };
    use serial_test::serial;

    use super::*;
    use solana_address_lookup_table_interface::{
        instruction as alt_instruction, program::ID as ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
    };
    use solana_compute_budget_interface::ComputeBudgetInstruction;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::Instruction,
        signature::{Keypair, Signer},
    };
    use solana_system_interface::{
        instruction::{
            assign, create_account, create_account_with_seed, transfer, transfer_with_seed,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };

    fn setup_both_configs(config: Config) {
        drop(setup_config_mock(config.clone()));
        update_config(config).unwrap();
    }

    // Helper functions to reduce test duplication and setup config
    fn setup_default_config() {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        setup_both_configs(config);
    }

    fn setup_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_spl_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_token2022_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![spl_token_2022_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_config_with_policy_and_disallowed(
        policy: FeePayerPolicy,
        allowed_programs: Vec<String>,
        disallowed_accounts: Vec<String>,
    ) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(allowed_programs)
            .with_disallowed_accounts(disallowed_accounts)
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    fn setup_alt_config_with_policy(policy: FeePayerPolicy) {
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transaction() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let instruction = transfer(&sender, &recipient, 100_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_transfer_amount_limits() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test transaction with amount over limit
        let instruction = transfer(&sender, &recipient, 2_000_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test multiple transfers
        let instructions =
            vec![transfer(&sender, &recipient, 500_000), transfer(&sender, &recipient, 500_000)];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_programs() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test allowed program (system program)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test disallowed program
        let fake_program = Pubkey::new_unique();
        // Create a no-op instruction for the fake program
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_empty_no_restriction() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_require_one_of_programs(vec![])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_only_cu_blocked() {
        let fee_payer = Pubkey::new_unique();
        let compute_budget_id = solana_compute_budget_interface::id().to_string();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![compute_budget_id.clone(), system_id.clone()])
            .with_require_one_of_programs(vec![system_id])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let cu_ix =
            Instruction::new_with_bincode(solana_compute_budget_interface::id(), &[0u8], vec![]);
        let message = VersionedMessage::Legacy(Message::new(&[cu_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_required_program_called() {
        let fee_payer = Pubkey::new_unique();
        let compute_budget_id = solana_compute_budget_interface::id().to_string();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![compute_budget_id.clone(), system_id.clone()])
            .with_require_one_of_programs(vec![system_id])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let cu_ix =
            Instruction::new_with_bincode(solana_compute_budget_interface::id(), &[0u8], vec![]);
        let transfer_ix = transfer(&sender, &recipient, 1000);
        let message =
            VersionedMessage::Legacy(Message::new(&[cu_ix, transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_no_required_program_fails() {
        let fee_payer = Pubkey::new_unique();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let other_program = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![system_id.clone()])
            .with_require_one_of_programs(vec![other_program.to_string()])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_require_one_of_programs_or_semantics() {
        let fee_payer = Pubkey::new_unique();
        let system_id = SYSTEM_PROGRAM_ID.to_string();
        let other_program = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![system_id.clone()])
            .with_require_one_of_programs(vec![system_id, other_program.to_string()])
            .build();
        setup_both_configs(config);
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Only calls system program (one of two required) — should pass
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(get_config().unwrap(), &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_signatures() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_max_signatures(2)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test too many signatures
        let instructions = vec![
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
            transfer(&sender, &recipient, 1000),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        transaction.transaction.signatures = vec![Default::default(); 3]; // Add 3 dummy signatures
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_sign_and_send_transaction_mode() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test SignAndSend mode with fee payer already set should not error
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test SignAndSend mode without fee payer (should succeed)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], None)); // No fee payer specified
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_transaction() {
        let fee_payer = Pubkey::new_unique();
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Create an empty message using Message::new with empty instructions
        let message = VersionedMessage::Legacy(Message::new(&[], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_reject_compute_budget_only_transaction() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![solana_compute_budget_interface::id().to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[compute_budget_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("only ComputeBudget instructions"));
    }

    #[tokio::test]
    #[serial]
    async fn test_allow_transaction_with_compute_budget_and_non_compute_instruction() {
        let fee_payer = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                SYSTEM_PROGRAM_ID.to_string(),
                solana_compute_budget_interface::id().to_string(),
            ])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let transfer_ix = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(
            &[compute_budget_ix, transfer_ix],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_accounts() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_disallowed_accounts(vec![
                "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek".to_string()
            ])
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = transfer(
            &Pubkey::from_str("hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek").unwrap(),
            &fee_payer,
            1000,
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_set_authority_new_authority() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &token_account,
            Some(&disallowed_account),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_token2022_set_authority_new_authority() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &token_account,
            Some(&disallowed_account),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_nonce_authorize_new_authority() {
        use solana_system_interface::instruction::authorize_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &disallowed_account);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_nonce_initialize_nonce_authority() {
        use solana_system_interface::instruction::create_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![SYSTEM_PROGRAM_ID.to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instructions =
            create_nonce_account(&fee_payer, &nonce_account, &disallowed_account, 1_000_000);
        // InitializeNonceAccount is the second instruction.
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_initialize_account2_owner() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::initialize_account2(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &disallowed_account,
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_disallowed_instruction_data_spl_initialize_mint2_freeze_authority() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let disallowed_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = true;
        setup_config_with_policy_and_disallowed(
            policy,
            vec![spl_token_interface::id().to_string()],
            vec![disallowed_account.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = spl_token_interface::instruction::initialize_mint2(
            &spl_token_interface::id(),
            &mint,
            &fee_payer,
            Some(&disallowed_account),
            6,
        )
        .unwrap();
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_sol_transfers() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test with allow_sol_transfers = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_sol_transfers = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = transfer(&fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_assign() {
        let fee_payer = Pubkey::new_unique();
        let new_owner = Pubkey::new_unique();

        // Test with allow_assign = true

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_assign = false

        let rpc_client = RpcMockBuilder::new().build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_assign = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let instruction = assign(&fee_payer, &new_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_spl_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let source_token_account =
            TokenAccountMockBuilder::new().with_mint(&mint).with_owner(&fee_payer).build();
        let mint_account = MintAccountMockBuilder::new().with_decimals(6).build();

        // Test with allow_spl_transfers = true (plain Transfer, mint resolved from source account)
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_spl_transfers = false
        let rpc_client = RpcMockBuilder::new()
            .build_with_sequential_accounts(vec![&source_token_account, &mint_account]);

        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_transfer = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with other account as source - should always pass
        let rpc_client = RpcMockBuilder::new().build();
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &recipient_token_account,
            &other_signer, // other account is the signer
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_transfers() {
        let fee_payer = Pubkey::new_unique();

        let fee_payer_token_account = Pubkey::new_unique();
        let recipient_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_token2022_transfers = true
        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        // Test with token_2022.allow_transfer = true
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_token2022_transfers = false
        let rpc_client = RpcMockBuilder::new()
            .with_mint_account(2) // Mock mint with 2 decimals for SPL outflow calculation
            .build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_transfer = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &fee_payer, // fee payer is the signer
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer is not allowed to be source
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with other account as source - should always pass
        let other_signer = Pubkey::new_unique();
        let transfer_ix = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &recipient_token_account,
            &other_signer, // other account is the signer
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[transfer_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass because fee payer is not the source
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_freeze_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        // Test with allow_freeze = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_freeze = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_freeze = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_freeze = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with another authority - should pass even when fee payer freeze is disabled
        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let freeze_ix = alt_instruction::freeze_lookup_table(lookup_table, other_authority);
        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_create_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        // Test with allow_create = false and fee payer as payer -> should fail
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_create = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let (create_ix, _table_address) =
            alt_instruction::create_lookup_table(authority, fee_payer, 42);
        let message = VersionedMessage::Legacy(Message::new(&[create_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with allow_create = true and fee payer as payer -> should pass
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_create = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let (create_ix, _table_address) =
            alt_instruction::create_lookup_table(authority, fee_payer, 42);
        let message = VersionedMessage::Legacy(Message::new(&[create_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_extend_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        // Test with allow_extend = false and fee payer as authority -> should fail
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            fee_payer,
            None,
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test optional payer branch: allow_extend = false and fee payer as optional payer -> should fail
        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(fee_payer),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with allow_extend = true and fee payer as optional payer -> should pass
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(fee_payer),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with no fee payer involvement - should pass even when allow_extend is disabled
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_extend = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let extend_ix = alt_instruction::extend_lookup_table(
            lookup_table,
            other_authority,
            Some(Pubkey::new_unique()),
            vec![Pubkey::new_unique()],
        );
        let message = VersionedMessage::Legacy(Message::new(&[extend_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_deactivate_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();

        // Test with allow_deactivate = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_deactivate = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_deactivate = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_deactivate = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with another authority - should pass when fee payer is not authority
        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let deactivate_ix = alt_instruction::deactivate_lookup_table(lookup_table, other_authority);
        let message = VersionedMessage::Legacy(Message::new(&[deactivate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_alt_close_lookup_table() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(500_000)
            .build();

        // Test with allow_close = true
        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_close = false
        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = false;
        setup_alt_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test with another authority - should pass when fee payer is not authority
        let rpc_client = RpcMockBuilder::new().build();
        let other_authority = Pubkey::new_unique();
        let close_ix =
            alt_instruction::close_lookup_table(lookup_table, other_authority, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transfer_amounts_rejects_alt_close_outflow_above_max_allowed_lamports() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(2_000_000)
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, recipient);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(matches!(
            result,
            Err(KoraError::InvalidTransaction(message))
                if message.contains("Total transfer amount 2000000 exceeds maximum allowed 1000000")
        ));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_transfer_amounts_allows_alt_close_to_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let lookup_table = Pubkey::new_unique();
        let alt_account = AccountMockBuilder::new()
            .with_owner(ADDRESS_LOOKUP_TABLE_PROGRAM_ID)
            .with_lamports(2_000_000)
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.alt.allow_close = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let rpc_client = RpcMockBuilder::new().with_account_info(&alt_account).build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = alt_instruction::close_lookup_table(lookup_table, fee_payer, fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_calculate_total_outflow() {
        let fee_payer = Pubkey::new_unique();
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .build();
        update_config(config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Test 1: Fee payer as sender in Transfer - should add to outflow
        let recipient = Pubkey::new_unique();
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient in Transfer - should subtract from outflow (account closure)
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow"); // 0 - 50_000 = 0 (saturating_sub)

        // Test 3: Fee payer as funding account in CreateAccount - should add to outflow
        let new_account = Pubkey::new_unique();
        let create_instruction = create_account(
            &fee_payer,
            &new_account,
            200_000, // lamports
            100,     // space
            &SYSTEM_PROGRAM_ID,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        // Test 4: Fee payer as funding account in CreateAccountWithSeed - should add to outflow
        let create_with_seed_instruction = create_account_with_seed(
            &fee_payer,
            &new_account,
            &fee_payer,
            "test_seed",
            300_000, // lamports
            100,     // space
            &SYSTEM_PROGRAM_ID,
        );
        let message = VersionedMessage::Legacy(Message::new(
            &[create_with_seed_instruction],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );

        // Test 5: TransferWithSeed from fee payer - should add to outflow
        let transfer_with_seed_instruction = transfer_with_seed(
            &fee_payer,
            &fee_payer,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &recipient,
            150_000,
        );
        let message = VersionedMessage::Legacy(Message::new(
            &[transfer_with_seed_instruction],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        // Test 6: Multiple instructions - should sum correctly
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100_000
            transfer(&sender, &fee_payer, 30_000),     // -30_000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50_000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );

        // Test 7: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");

        // Test 8: Other account funding CreateAccount - should not affect outflow
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");

        // Test 9: Self-withdraw from a fee-payer-controlled nonce account is neutral.
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let nonce_account = Pubkey::new_unique();
        let withdraw_instruction = solana_system_interface::instruction::withdraw_nonce_account(
            &nonce_account,
            &fee_payer,
            &fee_payer,
            25_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let outflow =
            validator.calculate_total_outflow(config, &mut transaction, &rpc_client).await.unwrap();
        assert_eq!(
            outflow, 0,
            "WithdrawNonceAccount from a fee-payer-controlled nonce account back to fee payer should be neutral"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_burn = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should pass because allow_burn is true by default
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_burn = false

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_burn = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer cannot burn tokens when allow_burn is false
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test burn_checked instruction
        let burn_checked_ix = spl_token_interface::instruction::burn_checked(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_checked_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should also fail for burn_checked
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_interface::instruction::close_account(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should pass because allow_close_account is true by default
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_close_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_close_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_interface::instruction::close_account(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer cannot close accounts when allow_close_account is false
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_interface::instruction::approve(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should pass because allow_approve is true by default
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_approve = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_approve = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_interface::instruction::approve(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test approve_checked instruction
        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token_interface::instruction::approve_checked(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &delegate,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message =
            VersionedMessage::Legacy(Message::new(&[approve_checked_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should also fail for approve_checked
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_burn() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_burn = false for Token2022

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_burn = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let burn_ix = spl_token_2022_interface::instruction::burn(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[burn_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should fail for Token2022 burn
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_close_account() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let destination = Pubkey::new_unique();

        // Test with allow_close_account = false for Token2022

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_close_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let close_ix = spl_token_2022_interface::instruction::close_account(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &destination,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[close_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should fail for Token2022 close account
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_approve() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();

        // Test with allow_approve = true

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        // Should pass because allow_approve is true by default
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_approve = false

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_approve = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let approve_ix = spl_token_2022_interface::instruction::approve(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &delegate,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[approve_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should fail because fee payer cannot approve when allow_approve is false
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());

        // Test approve_checked instruction
        let mint = Pubkey::new_unique();
        let approve_checked_ix = spl_token_2022_interface::instruction::approve_checked(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &mint,
            &delegate,
            &fee_payer,
            &[],
            1000,
            2,
        )
        .unwrap();

        let message =
            VersionedMessage::Legacy(Message::new(&[approve_checked_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should also fail for approve_checked
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        // Use System Program as owner since it's in allowed_programs
        let owner = SYSTEM_PROGRAM_ID;

        // Test with allow_create_account = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_create_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_rejects_disallowed_owner() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let disallowed_owner = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &disallowed_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed programs list"));
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_create_account_allows_valid_owner() {
        use solana_system_interface::instruction::create_account;

        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let allowed_owner = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string(), allowed_owner.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = create_account(&fee_payer, &new_account, 1000, 100, &allowed_owner);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_ata_create_idempotent() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        let rpc_client = RpcMockBuilder::new()
            .with_custom_mock(
                solana_client::rpc_request::RpcRequest::GetMinimumBalanceForRentExemption,
                serde_json::json!(2_039_280),
            )
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = false;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ata_program_id.to_string()])
            .with_max_allowed_lamports(10_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let ata_ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &fee_payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );
        let message = VersionedMessage::Legacy(Message::new(&[ata_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;

        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("ATA creation"));
            }
            _ => panic!("Expected ATA create policy violation"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_ata_create_idempotent_charged_in_outflow_without_inner_create() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata_program_id = spl_associated_token_account_interface::program::id();

        let rpc_client = RpcMockBuilder::new()
            .with_custom_mock(
                solana_client::rpc_request::RpcRequest::GetMinimumBalanceForRentExemption,
                serde_json::json!(2_039_280),
            )
            .build();

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![ata_program_id.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let ata_ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &fee_payer,
                &owner,
                &mint,
                &spl_token_interface::id(),
            );
        let message = VersionedMessage::Legacy(Message::new(&[ata_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;

        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("Total transfer amount"));
                assert!(msg.contains("exceeds maximum allowed"));
            }
            _ => panic!("Expected outflow limit violation for ATA creation"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_allocate() {
        use solana_system_interface::instruction::allocate;

        let fee_payer = Pubkey::new_unique();

        // Test with allow_allocate = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_allocate = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_allocate = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = allocate(&fee_payer, 100);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_initialize() {
        use solana_system_interface::instruction::create_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        // Test with allow_initialize = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        // Only test the InitializeNonceAccount instruction (second one)
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_initialize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = create_nonce_account(&fee_payer, &nonce_account, &fee_payer, 1_000_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[instructions[1].clone()], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_advance() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();

        // Test with allow_advance = true (must also enable durable transactions)
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_advance = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_advance = false (durable txs enabled but policy blocks it)
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_advance = false;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = advance_nonce_account(&nonce_account, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_withdraw() {
        use solana_system_interface::instruction::withdraw_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test with allow_withdraw = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_withdraw = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_withdraw = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_nonce_self_withdraw_does_not_hide_excess_fee_payer_outflow() {
        use solana_system_interface::instruction::withdraw_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.allow_transfer = true;
        policy.system.nonce.allow_withdraw = true;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(800)
            .with_fee_payer_policy(policy)
            .with_allow_durable_transactions(true)
            .build();
        update_config(config).unwrap();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instructions = vec![
            withdraw_nonce_account(&nonce_account, &fee_payer, &fee_payer, 1_000),
            transfer(&fee_payer, &recipient, 900),
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        match result {
            Err(KoraError::InvalidTransaction(msg)) => {
                assert!(msg.contains("Total transfer amount"));
                assert!(msg.contains("exceeds maximum allowed"));
            }
            _ => panic!("Expected self-withdraw not to mask oversized fee-payer transfer"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_nonce_authorize() {
        use solana_system_interface::instruction::authorize_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        // Test with allow_authorize = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = true;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_authorize = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.system.nonce.allow_authorize = false;
        setup_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();
        let instruction = authorize_nonce_account(&nonce_account, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_total_exceeds_fixed() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Fixed price = 5000, but total = 3000 + 2000 + 5000 = 10000 > 5000
        let fee_calc = TotalFeeCalculation::new(5000, 3000, 2000, 5000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_err());
        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("Strict pricing violation"));
            assert!(msg.contains("exceeds fixed price"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    #[serial]
    fn test_strict_pricing_total_within_fixed() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Fixed price = 5000, total = 1000 + 1000 + 1000 = 3000 < 5000
        let fee_calc = TotalFeeCalculation::new(5000, 1000, 1000, 1000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_disabled() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: false, // Disabled
        };
        let _ = update_config(config);

        let fee_calc = TotalFeeCalculation::new(5000, 10000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok(), "Should pass when strict=false");
    }

    #[test]
    #[serial]
    fn test_strict_pricing_with_margin_pricing() {
        use crate::{
            fee::price::PriceModel, state::update_config, tests::config_mock::ConfigMockBuilder,
        };

        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Margin { margin: 0.1 };
        let _ = update_config(config);

        let fee_calc = TotalFeeCalculation::new(5000, 10000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_strict_pricing_exact_match() {
        use crate::{
            fee::price::PriceModel, state::update_config, tests::config_mock::ConfigMockBuilder,
        };

        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 5000,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Total exactly equals fixed price (5000 = 5000)
        let fee_calc = TotalFeeCalculation::new(5000, 2000, 1000, 2000, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_ok(), "Should pass when total equals fixed price");
    }

    #[test]
    #[serial]
    fn test_strict_pricing_sub_lamport_quote_rejected() {
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 1,
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            strict: true,
        };
        let _ = update_config(config);

        // Sub-lamport configured price floors to 0 but real cost is positive (base_fee = 5000).
        let fee_calc = TotalFeeCalculation::new(0, 5000, 0, 0, 0, 0);

        let config = get_config().unwrap();
        let result = TransactionValidator::validate_strict_pricing_with_fee(config, &fee_calc);

        assert!(result.is_err(), "Strict mode must reject a zero quote with positive real cost");
        if let Err(KoraError::ValidationError(msg)) = result {
            assert!(msg.contains("Strict pricing violation"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transaction_rejected_by_default() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique(); // Different from fee payer

        // Default config has allow_durable_transactions = false
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_err());
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Durable transactions"));
            assert!(msg.contains("not allowed"));
        } else {
            panic!("Expected InvalidTransaction error");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_durable_transaction_allowed_when_enabled() {
        use solana_system_interface::instruction::advance_nonce_account;

        let fee_payer = Pubkey::new_unique();
        let nonce_account = Pubkey::new_unique();
        let nonce_authority = Pubkey::new_unique(); // Different from fee payer

        // Enable durable transactions
        let mock_config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![SYSTEM_PROGRAM_ID.to_string()])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(FeePayerPolicy::default())
            .with_allow_durable_transactions(true)
            .build();
        update_config(mock_config).unwrap();

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Transaction with AdvanceNonceAccount (authority is NOT fee payer)
        let instruction = advance_nonce_account(&nonce_account, &nonce_authority);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass because durable transactions are allowed
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_non_durable_transaction_passes() {
        let fee_payer = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Default config has allow_durable_transactions = false
        setup_default_config();
        let rpc_client = RpcMockBuilder::new().build();

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // Regular transfer (no nonce instruction)
        let instruction = transfer(&sender, &recipient, 1000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        // Should pass - no AdvanceNonceAccount instruction
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_revoke() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();

        // Test with allow_revoke = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_revoke = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for revoke policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_revoke() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();

        // Test with allow_revoke = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_revoke = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_2022_interface::instruction::revoke(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_revoke = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_revoke = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let revoke_ix = spl_token_2022_interface::instruction::revoke(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[revoke_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_revoke policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_set_authority() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        // Test with allow_set_authority = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_set_authority = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_set_authority = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for set_authority policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_set_authority() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        // Test with allow_set_authority = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_set_authority = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_set_authority = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let set_authority_ix = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            &fee_payer_token_account,
            Some(&new_authority),
            spl_token_2022_interface::instruction::AuthorityType::AccountOwner,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[set_authority_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_set_authority policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_mint_to() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();

        // Test with allow_mint_to = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_mint_to = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_interface::instruction::mint_to(
            &spl_token_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_mint_to = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_mint_to = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_interface::instruction::mint_to(
            &spl_token_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for mint_to policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_mint_to() {
        let fee_payer = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();

        // Test with allow_mint_to = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_mint_to = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_2022_interface::instruction::mint_to(
            &spl_token_2022_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_mint_to = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_mint_to = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let mint_to_ix = spl_token_2022_interface::instruction::mint_to(
            &spl_token_2022_interface::id(),
            &mint,
            &destination_token_account,
            &fee_payer,
            &[],
            1000,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[mint_to_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_mint_to policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_mint() {
        let fee_payer = Pubkey::new_unique();
        let mint_account = Pubkey::new_unique();

        // Test with allow_initialize_mint = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        // fee_payer is the mint_authority (encoded in instruction data)
        let init_mint_ix = spl_token_interface::instruction::initialize_mint(
            &spl_token_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_mint = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_mint = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_interface::instruction::initialize_mint(
            &spl_token_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_mint policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_mint() {
        let fee_payer = Pubkey::new_unique();
        let mint_account = Pubkey::new_unique();

        // Test with allow_initialize_mint = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_mint = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_2022_interface::instruction::initialize_mint(
            &spl_token_2022_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_mint = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_mint = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_mint_ix = spl_token_2022_interface::instruction::initialize_mint(
            &spl_token_2022_interface::id(),
            &mint_account,
            &fee_payer,
            None,
            6,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_mint_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_mint policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_initialize_account = true
        // initialize_account puts owner at account index 2 (token_account, mint, owner, rent_sysvar)
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_interface::instruction::initialize_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_interface::instruction::initialize_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_initialize_account = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_2022_interface::instruction::initialize_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_account_ix = spl_token_2022_interface::instruction::initialize_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_account_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_initialize_multisig() {
        let fee_payer = Pubkey::new_unique();
        let multisig_account = Pubkey::new_unique();
        let other_signer = Pubkey::new_unique();

        // Test with allow_initialize_multisig = true
        // fee_payer is one of the signers (parsed from accounts[2..])
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_multisig = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_interface::instruction::initialize_multisig(
            &spl_token_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_multisig = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_initialize_multisig = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_interface::instruction::initialize_multisig(
            &spl_token_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for initialize_multisig policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_initialize_multisig() {
        let fee_payer = Pubkey::new_unique();
        let multisig_account = Pubkey::new_unique();
        let other_signer = Pubkey::new_unique();

        // Test with allow_initialize_multisig = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_multisig = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_2022_interface::instruction::initialize_multisig(
            &spl_token_2022_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_initialize_multisig = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_initialize_multisig = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let init_multisig_ix = spl_token_2022_interface::instruction::initialize_multisig(
            &spl_token_2022_interface::id(),
            &multisig_account,
            &[&fee_payer, &other_signer],
            2,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[init_multisig_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_initialize_multisig policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_freeze_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_freeze_account = true
        // freeze_account(program_id, account, mint, freeze_authority, signers) — freeze_authority at index 2
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_freeze_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_interface::instruction::freeze_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_freeze_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_freeze_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_interface::instruction::freeze_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for freeze_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_freeze_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_freeze_account = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_freeze_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_2022_interface::instruction::freeze_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_freeze_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_freeze_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let freeze_ix = spl_token_2022_interface::instruction::freeze_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[freeze_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_freeze_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_thaw_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_thaw_account = true
        // thaw_account(program_id, account, mint, freeze_authority, signers) — freeze_authority at index 2
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_thaw_account = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_interface::instruction::thaw_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_thaw_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_thaw_account = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_interface::instruction::thaw_account(
            &spl_token_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for thaw_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_thaw_account() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Test with allow_thaw_account = true
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_2022_interface::instruction::thaw_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // Test with allow_thaw_account = false
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = false;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let thaw_ix = spl_token_2022_interface::instruction::thaw_account(
            &spl_token_2022_interface::id(),
            &token_account,
            &mint,
            &fee_payer,
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[thaw_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for token2022_thaw_account policy");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_token2022_reallocate_rejected_for_fee_payer() {
        let fee_payer = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();

        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let reallocate_ix = spl_token_2022_interface::instruction::reallocate(
            &spl_token_2022_interface::id(),
            &token_account,
            &fee_payer,
            &fee_payer,
            &[],
            &[spl_token_2022_interface::extension::ExtensionType::MemoTransfer],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[reallocate_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Token2022 Reallocate is not allowed"));
        } else {
            panic!("Expected InvalidTransaction error for token2022 reallocate");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_pause_with_fee_payer_rejected() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::pause(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("Token2022 Pause")),
            "Expected rejection when fee payer is the pausable authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_resume_allowed_when_policy_explicitly_enabled() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_thaw_account = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::resume(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(result.is_ok(), "Explicit thaw opt-in should allow Token2022 Resume: {result:?}");
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_pausable_initialize_rejects_fee_payer_authority_by_default() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::pausable::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("Token2022 InitializePausable authority")),
            "Expected rejection when fee payer is assigned as pausable authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_update_rejected_when_policy_denies() {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let mut config = get_config().unwrap().clone();
        config.validation.token_2022.transfer_hook_policy = TransferHookPolicy::DenyAll;
        config.validation.fee_payer_policy.token_2022.allow_update_extension_authority = true;
        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        validator.validate_transaction(&config, &mut transaction, &rpc_client).await.unwrap();

        let result = validator.validate_token2022_transfer_hook_signing_policies(
            &config,
            &mut transaction,
            TransferHookValidationFlow::DelayedSigning,
        );
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("TransferHook")),
            "Expected transfer-hook policy rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_update_allowed_for_immediate_send_when_policy_is_delayed_only(
    ) {
        let fee_payer = Keypair::new();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let mut config = get_config().unwrap().clone();
        config.validation.token_2022.transfer_hook_policy =
            TransferHookPolicy::DenyMutableForDelayedSigning;
        config.validation.fee_payer_policy.token_2022.allow_update_extension_authority = true;
        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();
        let mint = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        validator.validate_transaction(&config, &mut transaction, &rpc_client).await.unwrap();

        let result = validator.validate_token2022_transfer_hook_signing_policies(
            &config,
            &mut transaction,
            TransferHookValidationFlow::ImmediateSignAndSend,
        );
        assert!(
            result.is_ok(),
            "Immediate-sign flow should be allowed under delayed-only policy: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_transfer_hook_initialize_disallowed_program_id_rejected() {
        let fee_payer = Keypair::new();
        let disallowed_program_id = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_program_id.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_hook::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            Some(authority),
            Some(disallowed_program_id),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferHook program_id")),
            "Expected disallowed transfer-hook program id rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_transfer_fee_config_rejects_fee_payer_authorities_by_default(
    ) {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&fee_payer.pubkey()),
            Some(&Pubkey::new_unique()),
            25,
            100,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferFeeConfig transferFeeConfigAuthority")),
            "Expected rejection when fee payer is planted as transfer-fee authority, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_transfer_fee_config_disallowed_withdraw_authority_rejected()
    {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let disallowed_authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_authority.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&Pubkey::new_unique()),
            Some(&disallowed_authority),
            25,
            100,
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeTransferFeeConfig withdrawWithheldAuthority")),
            "Expected disallowed withdraw authority rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_update_metadata_pointer_rejects_fee_payer_authority_by_default() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("current Token2022 extension authority")),
            "Expected rejection when fee payer updates metadata pointer, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_update_metadata_pointer_allowed_when_extension_update_policy_enabled() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.token_2022.allow_update_extension_authority = true;
        setup_token2022_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::update(
            &spl_token_2022_interface::id(),
            &mint,
            &fee_payer.pubkey(),
            &[],
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            result.is_ok(),
            "Explicit extension update opt-in should allow metadata pointer updates: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_mint_close_authority_rejects_disallowed_new_authority() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let disallowed_authority = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_config_with_policy_and_disallowed(
            FeePayerPolicy::default(),
            vec![spl_token_2022_interface::id().to_string()],
            vec![disallowed_authority.to_string()],
        );

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::instruction::initialize_mint_close_authority(
            &spl_token_2022_interface::id(),
            &mint,
            Some(&disallowed_authority),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("InitializeMintCloseAuthority newAuthority")),
            "Expected disallowed mint close authority rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_initialize_metadata_pointer_rejects_blocked_extension() {
        let fee_payer = Keypair::new();
        let mint = Pubkey::new_unique();
        let metadata_address = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        let mut config = ConfigMockBuilder::new().build();
        config.validation.allowed_programs.push(spl_token_2022_interface::id().to_string());
        config.validation.token_2022.blocked_mint_extensions = vec!["metadata_pointer".to_string()];
        config.validation.token_2022.initialize().unwrap();
        let _config_guard = setup_config_mock(config.clone());

        let validator = TransactionValidator::new(&config, fee_payer.pubkey()).unwrap();

        let ix = spl_token_2022_interface::extension::metadata_pointer::instruction::initialize(
            &spl_token_2022_interface::id(),
            &mint,
            Some(Pubkey::new_unique()),
            Some(metadata_address),
        )
        .unwrap();

        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer.pubkey())));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(&config, &mut transaction, &rpc_client).await;
        assert!(
            matches!(result, Err(KoraError::InvalidTransaction(ref msg)) if msg.contains("extension 'MetadataPointer' is blocked")),
            "Expected blocked metadata pointer extension rejection, got: {result:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_token2022_confidential_extension_instructions_rejected() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = RpcMockBuilder::new().build();
        setup_token2022_config_with_policy(FeePayerPolicy::default());

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let confidential_ix = Instruction {
            program_id: spl_token_2022_interface::id(),
            accounts: vec![],
            data: spl_token_2022_interface::instruction::TokenInstruction::ConfidentialTransferExtension
                .pack(),
        };

        let message = VersionedMessage::Legacy(Message::new(&[confidential_ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Confidential Token-2022 instructions are not supported"));
        } else {
            panic!("Expected InvalidTransaction error for confidential token2022 instruction");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_fee_payer_policy_mixed_instructions() {
        let fee_payer = Pubkey::new_unique();
        let fee_payer_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let revoke_ix = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &fee_payer,
            &[],
        )
        .unwrap();

        let burn_ix = spl_token_interface::instruction::burn(
            &spl_token_interface::id(),
            &fee_payer_token_account,
            &mint,
            &fee_payer,
            &[],
            500,
        )
        .unwrap();

        // --- Test 1: revoke=true, burn=true → is_ok() ---
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(
            validator.validate_transaction(config, &mut transaction, &rpc_client).await.is_ok(),
            "Both policies true should pass"
        );

        // --- Test 2: revoke=true, burn=false → is_err() ---
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = true;
        policy.spl_token.allow_burn = false;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for burn policy");
        }

        // --- Test 3: revoke=false, burn=true → is_err() ---
        let rpc_client = RpcMockBuilder::new().build();
        let mut policy = FeePayerPolicy::default();
        policy.spl_token.allow_revoke = false;
        policy.spl_token.allow_burn = true;
        setup_spl_config_with_policy(policy);

        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let message = VersionedMessage::Legacy(Message::new(
            &[revoke_ix.clone(), burn_ix.clone()],
            Some(&fee_payer),
        ));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let result = validator.validate_transaction(config, &mut transaction, &rpc_client).await;
        if let Err(KoraError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Fee payer cannot be used for"));
        } else {
            panic!("Expected InvalidTransaction error for revoke policy");
        }
    }

    // ----------------------------------------------------------------------------
    // Loader-v4 tests
    // ----------------------------------------------------------------------------

    fn setup_loader_v4_config_with_policy(policy: FeePayerPolicy) {
        use crate::constant::LOADER_V4_PROGRAM_ID;
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                LOADER_V4_PROGRAM_ID.to_string(),
                SYSTEM_PROGRAM_ID.to_string(),
            ])
            .with_max_allowed_lamports(10_000_000_000)
            .with_fee_payer_policy(policy)
            .build();
        setup_both_configs(config);
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_write_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        // allow_write = true -> accepted
        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_write = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::write(&program, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // default (allow_write = false) -> rejected
        setup_loader_v4_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::write(&program, &fee_payer, 0, vec![1, 2, 3]);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_deploy_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_deploy = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::deploy(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());

        // disabled -> rejected
        setup_loader_v4_config_with_policy(FeePayerPolicy::default());
        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::deploy(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_retract_requires_policy() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_retract = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::retract(&program, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_accepts_self_recipient() {
        // When the fee payer is authority AND recipient, freed lamports return to Kora.
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_set_program_length = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 1024, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_rejects_foreign_recipient() {
        // Drainage guard: when Kora is authority, any recipient other than Kora is rejected.
        // Shrinking to zero with a user recipient would drain Kora's rent lamports.
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let user_recipient = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        let mut policy = FeePayerPolicy::default();
        policy.loader_v4.allow_set_program_length = true;
        setup_loader_v4_config_with_policy(policy);

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 0, &user_recipient);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        let err = validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .expect_err("shrink to foreign recipient must be rejected");
        if let KoraError::InvalidTransaction(msg) = err {
            assert!(msg.contains("drainage guard"));
        } else {
            panic!("expected InvalidTransaction, got {err:?}");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_set_program_length_rejects_when_policy_disabled() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::set_program_length(&program, &fee_payer, 1024, &fee_payer);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_transfer_authority_denied_by_default() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::transfer_authority(&program, &fee_payer, &new_authority);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_loader_v4_finalize_denied_by_default() {
        use solana_loader_v4_interface::instruction as loader_v4;
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let next_version = Pubkey::new_unique();

        setup_loader_v4_config_with_policy(FeePayerPolicy::default());

        let rpc_client = RpcMockBuilder::new().build();
        let config = get_config().unwrap();
        let validator = TransactionValidator::new(config, fee_payer).unwrap();

        let ix = loader_v4::finalize(&program, &fee_payer, &next_version);
        let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&fee_payer)));
        let mut transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        assert!(validator
            .validate_transaction(config, &mut transaction, &rpc_client)
            .await
            .is_err());
    }
}
