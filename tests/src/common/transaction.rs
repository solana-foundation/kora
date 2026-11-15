use anyhow::Result;
use kora_lib::{
    constant::instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR,
    transaction::TransactionUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_message::{
    v1::{Message as V1Message, TransactionConfig},
    VersionedMessage,
};
use solana_program_pack::Pack;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
    signature::Keypair,
    signer::Signer,
};
use solana_system_interface::{
    instruction::{create_account, transfer},
    program::ID as SYSTEM_PROGRAM_ID,
};
use spl_associated_token_account_interface::address::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use std::sync::Arc;

/// Fluent V1 transaction builder for tests
#[derive(Default)]
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    fee_payer: Option<Pubkey>,
    signers: Vec<Keypair>,
    rpc_client: Option<Arc<RpcClient>>,
    priority_fee: Option<u64>,
    unset_resource_limits: bool,
}

impl TransactionBuilder {
    pub fn with_rpc_client(mut self, client: Arc<RpcClient>) -> Self {
        self.rpc_client = Some(client);
        self
    }

    pub fn with_fee_payer(mut self, fee_payer: Pubkey) -> Self {
        self.fee_payer = Some(fee_payer);
        self
    }

    /// Add a signer (not the fee payer)
    pub fn with_signer(mut self, signer: &Keypair) -> Self {
        self.signers.push(signer.insecure_clone());
        self
    }

    pub fn with_transfer(mut self, from: &Pubkey, to: &Pubkey, lamports: u64) -> Self {
        self.instructions.push(transfer(from, to, lamports));
        self
    }

    /// Transfer to a fresh recipient key, so two tests submitting otherwise
    /// identical transactions cannot collide on a duplicate signature. The
    /// amount is the rent-exempt minimum because the recipient is a new account.
    pub fn with_unique_transfer(self, from: &Pubkey) -> Self {
        let to = Keypair::new().pubkey();
        self.with_transfer(from, &to, Rent::default().minimum_balance(0))
    }

    pub fn with_system_assign(mut self, account: &Pubkey, owner: &Pubkey) -> Self {
        let instruction = solana_system_interface::instruction::assign(account, owner);
        self.instructions.push(instruction);
        self
    }

    pub fn with_system_create_account(
        mut self,
        from: &Pubkey,
        to: &Pubkey,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> Self {
        let instruction =
            solana_system_interface::instruction::create_account(from, to, lamports, space, owner);
        self.instructions.push(instruction);
        self
    }

    pub fn with_system_allocate(mut self, account: &Pubkey, space: u64) -> Self {
        let instruction = solana_system_interface::instruction::allocate(account, space);
        self.instructions.push(instruction);
        self
    }

    // Tag 13, hand-encoded (no constructor in system-interface 2.0.0); omit funding account if lamports == 0
    pub fn with_create_account_allow_prefund(
        mut self,
        new_account: &Pubkey,
        funder: &Pubkey,
        lamports: u64,
        space: u64,
        owner: &Pubkey,
    ) -> Self {
        let mut accounts = vec![AccountMeta::new(*new_account, true)];
        if lamports > 0 {
            accounts.push(AccountMeta::new(*funder, true));
        }
        self.instructions.push(Instruction {
            program_id: SYSTEM_PROGRAM_ID,
            accounts,
            data: bincode::serialize(&(DISCRIMINATOR, lamports, space, *owner)).unwrap(),
        });
        self
    }

    pub fn with_spl_transfer(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
    ) -> Self {
        let from_token_account = get_associated_token_address(from_authority, token_mint);
        let to_token_account = get_associated_token_address(to_pubkey, token_mint);

        let instruction = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &from_token_account,
            &to_token_account,
            from_authority,
            &[],
            amount,
        )
        .expect("Failed to create SPL transfer instruction");

        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_payment(
        mut self,
        token_mint: &Pubkey,
        from_authority: &Pubkey,
        to_pubkey: &Pubkey,
        amount: u64,
    ) -> Self {
        let from_ata = get_associated_token_address(from_authority, token_mint);
        let to_ata = get_associated_token_address(to_pubkey, token_mint);

        let instruction = spl_token_interface::instruction::transfer(
            &spl_token_interface::ID,
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

    pub fn with_spl_payment_with_accounts(
        mut self,
        from_token_account: &Pubkey,
        to_token_account: &Pubkey,
        from_authority: &Pubkey,
        amount: u64,
    ) -> Self {
        let instruction = spl_token_interface::instruction::transfer(
            &spl_token_interface::ID,
            from_token_account,
            to_token_account,
            from_authority,
            &[],
            amount,
        )
        .expect("Failed to create SPL payment instruction");

        self.instructions.push(instruction);
        self
    }

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

        let instruction = spl_token_interface::instruction::transfer_checked(
            &spl_token_interface::ID,
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

    // Flat lamports amount for priority fee
    pub fn with_priority_fee(mut self, lamports: u64) -> Self {
        self.priority_fee = Some(lamports);
        self
    }

    // Omit resource limits from the config (requests zero compute units and zero data size)
    pub fn with_unset_resource_limits(mut self) -> Self {
        self.unset_resource_limits = true;
        self
    }

    pub fn with_compute_budget(mut self, units: u32, price: u64) -> Self {
        self.instructions.insert(0, ComputeBudgetInstruction::set_compute_unit_limit(units));
        self.instructions.insert(1, ComputeBudgetInstruction::set_compute_unit_price(price));
        self
    }

    pub fn with_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

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
            &spl_token_2022_interface::id(),
        );
        let to_ata = get_associated_token_address_with_program_id(
            to_pubkey,
            token_mint,
            &spl_token_2022_interface::id(),
        );

        let instruction = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            &from_ata,
            token_mint,
            &to_ata,
            from_authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create Token 2022 transfer_checked instruction");

        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_token_2022_transfer_checked_with_accounts(
        mut self,
        token_mint: &Pubkey,
        from_token_account: &Pubkey,
        to_token_account: &Pubkey,
        from_authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::transfer_checked(
            &spl_token_2022_interface::id(),
            from_token_account,
            token_mint,
            to_token_account,
            from_authority,
            &[],
            amount,
            decimals,
        )
        .expect("Failed to create Token 2022 transfer_checked instruction with accounts");

        self.instructions.push(instruction);
        self
    }

    pub fn with_create_ata(mut self, mint: &Pubkey, owner: &Pubkey) -> Self {
        let instruction =
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &self.fee_payer.expect("Fee payer must be set before creating ATA"),
                owner,
                mint,
                &spl_token_interface::id(),
            );
        self.instructions.push(instruction);
        self
    }

    pub fn with_create_ata_idempotent(mut self, mint: &Pubkey, owner: &Pubkey) -> Self {
        let instruction =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer.expect("Fee payer must be set before creating ATA"),
                owner,
                mint,
                &spl_token_interface::id(),
            );
        self.instructions.push(instruction);
        self
    }

    pub fn with_create_token2022_ata(mut self, mint: &Pubkey, owner: &Pubkey) -> Self {
        let instruction =
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &self.fee_payer.expect("Fee payer must be set before creating ATA"),
                owner,
                mint,
                &spl_token_2022_interface::id(),
            );
        self.instructions.push(instruction);
        self
    }

    pub fn with_create_and_init_token_account(
        mut self,
        account: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
        rent_lamports: u64,
    ) -> Self {
        let create_instruction = create_account(
            &self.fee_payer.expect("Fee payer must be set"),
            &account.pubkey(),
            rent_lamports,
            spl_token_interface::state::Account::LEN as u64,
            &spl_token_interface::id(),
        );

        let init_instruction = spl_token_interface::instruction::initialize_account3(
            &spl_token_interface::id(),
            &account.pubkey(),
            mint,
            owner,
        )
        .expect("Failed to create initialize account instruction");

        self.instructions.push(create_instruction);
        self.instructions.push(init_instruction);
        self.signers.push(account.insecure_clone());
        self
    }

    pub fn with_spl_revoke(mut self, token_account: &Pubkey, owner: &Pubkey) -> Self {
        let instruction = spl_token_interface::instruction::revoke(
            &spl_token_interface::id(),
            token_account,
            owner,
            &[],
        )
        .expect("Failed to create revoke instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_revoke(mut self, token_account: &Pubkey, owner: &Pubkey) -> Self {
        let instruction = spl_token_2022_interface::instruction::revoke(
            &spl_token_2022_interface::id(),
            token_account,
            owner,
            &[],
        )
        .expect("Failed to create Token2022 revoke instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_withdraw_excess_lamports(
        mut self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_interface::instruction::withdraw_excess_lamports(
            &spl_token_interface::id(),
            account,
            destination,
            authority,
            &[],
        )
        .expect("Failed to create withdraw_excess_lamports instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_withdraw_excess_lamports(
        mut self,
        account: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::withdraw_excess_lamports(
            &spl_token_2022_interface::id(),
            account,
            destination,
            authority,
            &[],
        )
        .expect("Failed to create Token2022 withdraw_excess_lamports instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_batch(mut self, inner: Vec<Instruction>) -> Self {
        let instruction =
            spl_token_interface::instruction::batch(&spl_token_interface::id(), &inner)
                .expect("Failed to create batch instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_set_authority(
        mut self,
        account: &Pubkey,
        new_authority: Option<&Pubkey>,
        authority_type: spl_token_interface::instruction::AuthorityType,
        current_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_interface::instruction::set_authority(
            &spl_token_interface::id(),
            account,
            new_authority,
            authority_type,
            current_authority,
            &[],
        )
        .expect("Failed to create set_authority instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_set_authority(
        mut self,
        account: &Pubkey,
        new_authority: Option<&Pubkey>,
        authority_type: spl_token_2022_interface::instruction::AuthorityType,
        current_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::id(),
            account,
            new_authority,
            authority_type,
            current_authority,
            &[],
        )
        .expect("Failed to create Token2022 set_authority instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_mint_to(
        mut self,
        mint: &Pubkey,
        destination: &Pubkey,
        mint_authority: &Pubkey,
        amount: u64,
    ) -> Self {
        let instruction = spl_token_interface::instruction::mint_to(
            &spl_token_interface::id(),
            mint,
            destination,
            mint_authority,
            &[],
            amount,
        )
        .expect("Failed to create mint_to instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_mint_to(
        mut self,
        mint: &Pubkey,
        destination: &Pubkey,
        mint_authority: &Pubkey,
        amount: u64,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::mint_to(
            &spl_token_2022_interface::id(),
            mint,
            destination,
            mint_authority,
            &[],
            amount,
        )
        .expect("Failed to create Token2022 mint_to instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_freeze_account(
        mut self,
        token_account: &Pubkey,
        mint: &Pubkey,
        freeze_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_interface::instruction::freeze_account(
            &spl_token_interface::id(),
            token_account,
            mint,
            freeze_authority,
            &[],
        )
        .expect("Failed to create freeze_account instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_freeze_account(
        mut self,
        token_account: &Pubkey,
        mint: &Pubkey,
        freeze_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::freeze_account(
            &spl_token_2022_interface::id(),
            token_account,
            mint,
            freeze_authority,
            &[],
        )
        .expect("Failed to create Token2022 freeze_account instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_spl_thaw_account(
        mut self,
        token_account: &Pubkey,
        mint: &Pubkey,
        freeze_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_interface::instruction::thaw_account(
            &spl_token_interface::id(),
            token_account,
            mint,
            freeze_authority,
            &[],
        )
        .expect("Failed to create thaw_account instruction");
        self.instructions.push(instruction);
        self
    }

    pub fn with_token2022_thaw_account(
        mut self,
        token_account: &Pubkey,
        mint: &Pubkey,
        freeze_authority: &Pubkey,
    ) -> Self {
        let instruction = spl_token_2022_interface::instruction::thaw_account(
            &spl_token_2022_interface::id(),
            token_account,
            mint,
            freeze_authority,
            &[],
        )
        .expect("Failed to create Token2022 thaw_account instruction");
        self.instructions.push(instruction);
        self
    }

    pub async fn build(self) -> Result<String> {
        let rpc_client =
            self.rpc_client.as_ref().ok_or_else(|| anyhow::anyhow!("RPC client is required"))?;
        let blockhash =
            rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).await?;

        self.compile_and_sign(blockhash.0)
    }

    pub async fn build_with_nonce(mut self, nonce_pubkey: &Pubkey) -> Result<String> {
        let rpc_client =
            self.rpc_client.as_ref().ok_or_else(|| anyhow::anyhow!("RPC client is required"))?;

        // Nonce account data has a version prefix; deserialize as Versions
        let nonce_account = rpc_client.get_account(nonce_pubkey).await?;
        let nonce_versions: solana_nonce::versions::Versions =
            bincode::deserialize(&nonce_account.data)?;

        let nonce_data = match nonce_versions.state() {
            solana_nonce::state::State::Initialized(data) => data,
            solana_nonce::state::State::Uninitialized => {
                return Err(anyhow::anyhow!("Nonce account is not initialized"));
            }
        };

        self.instructions.insert(
            0,
            solana_system_interface::instruction::advance_nonce_account(
                nonce_pubkey,
                &nonce_data.authority,
            ),
        );
        self.compile_and_sign(nonce_data.blockhash())
    }

    fn compile_and_sign(&self, lifetime: solana_sdk::hash::Hash) -> Result<String> {
        let message = self.compile(lifetime)?;
        let mut transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let message_bytes = transaction.message.serialize();
        for signer in &self.signers {
            let account_keys = transaction.message.static_account_keys();
            if let Some(position) = account_keys.iter().position(|key| key == &signer.pubkey()) {
                transaction.signatures[position] = signer.sign_message(&message_bytes);
            }
        }

        Ok(TransactionUtil::encode_versioned_transaction(&transaction)?)
    }

    fn compile(&self, lifetime: solana_sdk::hash::Hash) -> Result<VersionedMessage> {
        let fee_payer = self.fee_payer.ok_or_else(|| anyhow::anyhow!("Fee payer is required"))?;
        // An empty config requests 0 compute units and a 0 data cap, so limits are set explicitly
        let mut config = if self.unset_resource_limits {
            TransactionConfig::empty()
        } else {
            TransactionConfig::empty()
                .with_compute_unit_limit(1_400_000)
                .with_loaded_accounts_data_size_limit(64 * 1024 * 1024)
        };
        if let Some(priority_fee) = self.priority_fee {
            config = config.with_priority_fee(priority_fee);
        }
        Ok(VersionedMessage::V1(V1Message::try_compile_with_config(
            &fee_payer,
            &self.instructions,
            lifetime,
            config,
        )?))
    }
}
