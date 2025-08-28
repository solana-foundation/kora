use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use kora_lib::{
    token::{Token2022Program, TokenInterface, TokenProgram},
    transaction::TransactionUtil,
};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{
    v0::Message as V0Message, AddressLookupTableAccount, Message, VersionedMessage,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::VersionedTransaction,
};
use solana_system_interface::instruction::transfer;
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use std::sync::Arc;

/// Transaction version types
#[derive(Debug, Clone)]
pub enum TransactionVersion {
    Legacy,
    V0,
    V0WithLookup(Vec<Pubkey>),
}

/// Fluent transaction builder for tests
pub struct TransactionBuilder {
    version: TransactionVersion,
    instructions: Vec<Instruction>,
    fee_payer: Option<Pubkey>,
    signers: Vec<Keypair>,
    rpc_client: Option<Arc<RpcClient>>,
}

impl TransactionBuilder {
    /// Create a legacy transaction builder
    pub fn legacy() -> Self {
        Self {
            version: TransactionVersion::Legacy,
            instructions: Vec::new(),
            fee_payer: None,
            signers: Vec::new(),
            rpc_client: None,
        }
    }

    /// Create a V0 transaction builder without lookup tables
    pub fn v0() -> Self {
        Self {
            version: TransactionVersion::V0,
            instructions: Vec::new(),
            fee_payer: None,
            signers: Vec::new(),
            rpc_client: None,
        }
    }

    /// Create a V0 transaction builder with lookup tables
    pub fn v0_with_lookup(lookup_tables: Vec<Pubkey>) -> Self {
        Self {
            version: TransactionVersion::V0WithLookup(lookup_tables),
            instructions: Vec::new(),
            fee_payer: None,
            signers: Vec::new(),
            rpc_client: None,
        }
    }

    /// Set the RPC client for fetching blockhash and lookup tables
    pub fn with_rpc_client(mut self, client: Arc<RpcClient>) -> Self {
        self.rpc_client = Some(client);
        self
    }

    /// Set the fee payer
    pub fn with_fee_payer(mut self, fee_payer: Pubkey) -> Self {
        self.fee_payer = Some(fee_payer);
        self
    }

    /// Add a signer (not the fee payer)
    pub fn with_signer(mut self, signer: &Keypair) -> Self {
        self.signers.push(signer.insecure_clone());
        self
    }

    /// Add a simple SOL transfer instruction
    pub fn with_transfer(mut self, from: &Pubkey, to: &Pubkey, lamports: u64) -> Self {
        self.instructions.push(transfer(from, to, lamports));
        self
    }

    /// Add an SPL token transfer instruction
    pub fn with_spl_transfer(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
    ) -> Self {
        let from_token_account = get_associated_token_address(from_authority, token_mint);
        let to_token_account = get_associated_token_address(to_pubkey, token_mint);

        let token_interface = TokenProgram::new();
        let instruction = token_interface
            .create_transfer_instruction(
                &from_token_account,
                &to_token_account,
                from_authority,
                amount,
            )
            .expect("Failed to create SPL transfer instruction");

        self.instructions.push(instruction);
        self
    }

    /// Add an SPL payment instruction (direct token transfer using spl_token)
    pub fn with_spl_payment(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
    ) -> Self {
        let from_ata = get_associated_token_address(from_authority, token_mint);
        let to_ata = get_associated_token_address(to_pubkey, token_mint);

        let instruction = spl_token::instruction::transfer(
            &spl_token::ID,
            &from_ata,
            &to_ata,
            from_authority,
            &[],
            amount,
        )
        .expect("Failed to create SPL payment instruction");

        self.instructions.push(instruction);
        self
    }

    /// Add an SPL token transfer_checked instruction (includes mint address for lookup table testing)
    pub fn with_spl_transfer_checked(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Self {
        let from_ata = get_associated_token_address(from_authority, token_mint);
        let to_ata = get_associated_token_address(to_pubkey, token_mint);

        let instruction = spl_token::instruction::transfer_checked(
            &spl_token::ID,
            &from_ata,
            token_mint,
            &to_ata,
            from_authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create SPL transfer_checked instruction");

        self.instructions.push(instruction);
        self
    }

    /// Add compute budget instructions (both limit and price)
    pub fn with_compute_budget(mut self, units: u32, price: u64) -> Self {
        self.instructions.insert(0, ComputeBudgetInstruction::set_compute_unit_limit(units));
        self.instructions.insert(1, ComputeBudgetInstruction::set_compute_unit_price(price));
        self
    }

    /// Add a custom instruction
    pub fn with_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add a Token 2022 transfer_checked instruction
    pub fn with_spl_token_2022_transfer_checked(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Self {
        let from_ata = get_associated_token_address_with_program_id(
            from_authority,
            token_mint,
            &spl_token_2022::id(),
        );
        let to_ata = get_associated_token_address_with_program_id(
            to_pubkey,
            token_mint,
            &spl_token_2022::id(),
        );

        let token_interface = Token2022Program::new();
        let instruction = token_interface
            .create_transfer_checked_instruction(
                &from_ata,
                token_mint,
                &to_ata,
                from_authority,
                amount,
                decimals,
            )
            .expect("Failed to create Token 2022 transfer_checked instruction");

        self.instructions.push(instruction);
        self
    }

    /// Add Token 2022 transfer checked instruction with specific token accounts
    pub fn with_spl_token_2022_transfer_checked_with_accounts(
        mut self,
        token_mint: &Pubkey,
        from_token_account: &Pubkey,
        to_token_account: &Pubkey,
        from_authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Self {
        let token_interface = Token2022Program::new();
        let instruction = token_interface
            .create_transfer_checked_instruction(
                from_token_account,
                token_mint,
                to_token_account,
                from_authority,
                amount,
                decimals,
            )
            .expect("Failed to create Token 2022 transfer_checked instruction with accounts");

        self.instructions.push(instruction);
        self
    }

    /// Build the transaction and return as base64-encoded string
    pub async fn build(self) -> Result<String> {
        let rpc_client =
            self.rpc_client.ok_or_else(|| anyhow::anyhow!("RPC client is required"))?;

        let fee_payer = self.fee_payer.ok_or_else(|| anyhow::anyhow!("Fee payer is required"))?;

        let blockhash =
            rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

        let message =
            match self.version {
                TransactionVersion::Legacy => VersionedMessage::Legacy(
                    Message::new_with_blockhash(&self.instructions, Some(&fee_payer), &blockhash.0),
                ),
                TransactionVersion::V0 => {
                    let v0_message = V0Message::try_compile(
                        &fee_payer,
                        &self.instructions,
                        &[], // No lookup tables
                        blockhash.0,
                    )?;
                    VersionedMessage::V0(v0_message)
                }
                TransactionVersion::V0WithLookup(lookup_table_keys) => {
                    // Fetch and deserialize lookup tables
                    let mut lookup_table_accounts = Vec::new();
                    for key in lookup_table_keys {
                        let account = rpc_client.get_account(&key).await?;
                        let lookup_table = AddressLookupTable::deserialize(&account.data)?;
                        lookup_table_accounts.push(AddressLookupTableAccount {
                            key,
                            addresses: lookup_table.addresses.to_vec(),
                        });
                    }

                    let v0_message = V0Message::try_compile(
                        &fee_payer,
                        &self.instructions,
                        &lookup_table_accounts,
                        blockhash.0,
                    )?;
                    VersionedMessage::V0(v0_message)
                }
            };

        let transaction = if self.signers.is_empty() {
            // Unsigned transaction
            TransactionUtil::new_unsigned_versioned_transaction(message)
        } else {
            // Signed transaction - create with proper number of signatures
            let num_required_signatures = message.header().num_required_signatures as usize;
            let mut tx = VersionedTransaction {
                signatures: vec![Signature::default(); num_required_signatures],
                message,
            };

            let message_bytes = tx.message.serialize();
            for signer in &self.signers {
                let account_keys = tx.message.static_account_keys();
                if let Some(position) = account_keys.iter().position(|key| key == &signer.pubkey())
                {
                    let signature = signer.sign_message(&message_bytes);
                    tx.signatures[position] = signature;
                }
            }
            tx
        };

        let serialized = bincode::serialize(&transaction)?;
        Ok(STANDARD.encode(serialized))
    }
}
