use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token::instruction as token_instruction;
use spl_token_2022::{
    extension::{transfer_fee::instruction::initialize_transfer_fee_config, ExtensionType},
    instruction as token_2022_instruction,
    state::Mint as Token2022Mint,
};
use std::sync::Arc;

use crate::common::{
    FeePayerTestHelper, LookupTableHelper, RecipientTestHelper, SenderTestHelper,
    USDCMint2022TestHelper, USDCMintTestHelper, DEFAULT_RPC_URL,
};

/// Test account information for outputting to the user
#[derive(Debug, Default, Clone)]
pub struct TestAccountInfo {
    pub fee_payer_pubkey: Pubkey,
    pub sender_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    // USDC mint fields
    pub usdc_mint_pubkey: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipient_token_account: Pubkey,
    pub fee_payer_token_account: Pubkey,
    // Token 2022 fields
    pub usdc_mint_2022_pubkey: Pubkey,
    pub sender_token_2022_account: Pubkey,
    pub recipient_token_2022_account: Pubkey,
    pub fee_payer_token_2022_account: Pubkey,
    // Lookup tables
    pub allowed_lookup_table: Pubkey,
    pub disallowed_lookup_table: Pubkey,
    pub transaction_lookup_table: Pubkey,
}

/// Test account setup utilities for local validator
pub struct TestAccountSetup {
    pub rpc_client: Arc<RpcClient>,
    pub sender_keypair: Keypair,
    pub fee_payer_keypair: Keypair,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint: Keypair,
    pub usdc_mint_2022: Keypair,
}

impl TestAccountSetup {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();
        let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        let rpc_client =
            Arc::new(RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()));
        Self::new_with_client(rpc_client).await
    }

    pub async fn new_with_rpc_url(rpc_url: &str) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        Self::new_with_client(rpc_client).await
    }

    async fn new_with_client(rpc_client: Arc<RpcClient>) -> Self {
        let sender_keypair = SenderTestHelper::get_test_sender_keypair();
        let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();
        let fee_payer_keypair = FeePayerTestHelper::get_fee_payer_keypair();

        let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_keypair();
        let usdc_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_keypair();

        Self {
            rpc_client,
            sender_keypair,
            fee_payer_keypair,
            recipient_pubkey,
            usdc_mint,
            usdc_mint_2022,
        }
    }

    pub async fn setup_all_accounts(&mut self) -> Result<TestAccountInfo> {
        let mut account_infos = TestAccountInfo::default();

        let (sender_pubkey, recipient_pubkey, fee_payer_pubkey) = self.fund_sol_accounts().await?;
        account_infos.sender_pubkey = sender_pubkey;
        account_infos.recipient_pubkey = recipient_pubkey;
        account_infos.fee_payer_pubkey = fee_payer_pubkey;

        let usdc_mint_pubkey = self.create_usdc_mint().await?;
        account_infos.usdc_mint_pubkey = usdc_mint_pubkey;

        let usdc_mint_2022_pubkey = self.create_usdc_mint_2022().await?;
        account_infos.usdc_mint_2022_pubkey = usdc_mint_2022_pubkey;

        let (allowed_lookup_table, disallowed_lookup_table, transaction_lookup_table) =
            self.create_lookup_tables().await?;
        account_infos.allowed_lookup_table = allowed_lookup_table;
        account_infos.disallowed_lookup_table = disallowed_lookup_table;
        account_infos.transaction_lookup_table = transaction_lookup_table;

        let (
            sender_token_account,
            recipient_token_account,
            fee_payer_token_account,
            sender_token_2022_account,
            recipient_token_2022_account,
            fee_payer_token_2022_account,
        ) = self.setup_token_accounts().await?;
        account_infos.sender_token_account = sender_token_account;
        account_infos.recipient_token_account = recipient_token_account;
        account_infos.fee_payer_token_account = fee_payer_token_account;
        account_infos.sender_token_2022_account = sender_token_2022_account;
        account_infos.recipient_token_2022_account = recipient_token_2022_account;
        account_infos.fee_payer_token_2022_account = fee_payer_token_2022_account;

        // Wait for the accounts to be fully initialized (lookup tables, etc.)
        let await_for_slot = self.rpc_client.get_slot().await? + 30;

        while self.rpc_client.get_slot().await? < await_for_slot {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(account_infos)
    }

    pub async fn airdrop_if_required_sol(&self, receiver: &Pubkey, amount: u64) -> Result<()> {
        let balance = self.rpc_client.get_balance(receiver).await?;

        // 80% of the amount is enough to cover the transaction fees
        if balance as f64 >= amount as f64 * 0.8 {
            return Ok(());
        }

        let signature = self.rpc_client.request_airdrop(receiver, amount).await?;

        loop {
            let confirmed = self.rpc_client.confirm_transaction(&signature).await?;

            if confirmed {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(())
    }

    pub async fn fund_sol_accounts(&self) -> Result<(Pubkey, Pubkey, Pubkey)> {
        let sol_to_fund = 10 * LAMPORTS_PER_SOL;

        let sender_pubkey = self.sender_keypair.pubkey();
        let fee_payer_pubkey = self.fee_payer_keypair.pubkey();

        tokio::try_join!(
            self.airdrop_if_required_sol(&sender_pubkey, sol_to_fund),
            self.airdrop_if_required_sol(&self.recipient_pubkey, sol_to_fund),
            self.airdrop_if_required_sol(&fee_payer_pubkey, sol_to_fund)
        )?;

        Ok((self.sender_keypair.pubkey(), self.recipient_pubkey, self.fee_payer_keypair.pubkey()))
    }

    pub async fn create_usdc_mint(&self) -> Result<Pubkey> {
        if (self.rpc_client.get_account(&self.usdc_mint.pubkey()).await).is_ok() {
            return Ok(self.usdc_mint.pubkey());
        }

        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)
            .await?;

        let create_account_instruction = solana_sdk::system_instruction::create_account(
            &self.sender_keypair.pubkey(),
            &self.usdc_mint.pubkey(),
            rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        );

        let initialize_mint_instruction = spl_token::instruction::initialize_mint2(
            &spl_token::id(),
            &self.usdc_mint.pubkey(),
            &self.sender_keypair.pubkey(),
            Some(&self.sender_keypair.pubkey()),
            USDCMintTestHelper::get_test_usdc_mint_decimals(),
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_account_instruction, initialize_mint_instruction],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair, &self.usdc_mint],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;

        Ok(self.usdc_mint.pubkey())
    }

    pub async fn create_usdc_mint_2022(&self) -> Result<Pubkey> {
        if (self.rpc_client.get_account(&self.usdc_mint_2022.pubkey()).await).is_ok() {
            return Ok(self.usdc_mint_2022.pubkey());
        }

        let decimals = USDCMintTestHelper::get_test_usdc_mint_decimals();

        // Calculate space required for mint with transfer fee extension
        let space = spl_token_2022::extension::ExtensionType::try_calculate_account_len::<
            Token2022Mint,
        >(&[ExtensionType::TransferFeeConfig])?;

        let rent = self.rpc_client.get_minimum_balance_for_rent_exemption(space).await?;

        let create_account_instruction = solana_sdk::system_instruction::create_account(
            &self.sender_keypair.pubkey(),
            &self.usdc_mint_2022.pubkey(),
            rent,
            space as u64,
            &spl_token_2022::id(),
        );

        let initialize_transfer_fee_config_instruction = initialize_transfer_fee_config(
            &spl_token_2022::id(),
            &self.usdc_mint_2022.pubkey(),
            Some(&self.sender_keypair.pubkey()),
            Some(&self.sender_keypair.pubkey()),
            100,       // 1% transfer fee basis points
            1_000_000, // 1 USDC max fee (in micro-units)
        )?;

        let initialize_mint_instruction = token_2022_instruction::initialize_mint2(
            &spl_token_2022::id(),
            &self.usdc_mint_2022.pubkey(),
            &self.sender_keypair.pubkey(),
            Some(&self.sender_keypair.pubkey()),
            decimals,
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &[
                create_account_instruction,
                initialize_transfer_fee_config_instruction,
                initialize_mint_instruction,
            ],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair, &self.usdc_mint_2022],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;

        Ok(self.usdc_mint_2022.pubkey())
    }

    pub async fn setup_token_accounts(
        &self,
    ) -> Result<(Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey)> {
        // SPL Token accounts
        let sender_token_account =
            get_associated_token_address(&self.sender_keypair.pubkey(), &self.usdc_mint.pubkey());
        let recipient_token_account =
            get_associated_token_address(&self.recipient_pubkey, &self.usdc_mint.pubkey());
        let fee_payer_token_account = get_associated_token_address(
            &self.fee_payer_keypair.pubkey(),
            &self.usdc_mint.pubkey(),
        );

        // Token 2022 accounts
        let sender_token_2022_account = get_associated_token_address_with_program_id(
            &self.sender_keypair.pubkey(),
            &self.usdc_mint_2022.pubkey(),
            &spl_token_2022::id(),
        );
        let recipient_token_2022_account = get_associated_token_address_with_program_id(
            &self.recipient_pubkey,
            &self.usdc_mint_2022.pubkey(),
            &spl_token_2022::id(),
        );
        let fee_payer_token_2022_account = get_associated_token_address_with_program_id(
            &self.fee_payer_keypair.pubkey(),
            &self.usdc_mint_2022.pubkey(),
            &spl_token_2022::id(),
        );

        // Create regular SPL Token accounts
        let create_associated_token_account_instruction =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.sender_keypair.pubkey(),
                &self.usdc_mint.pubkey(),
                &spl_token::id(),
            );

        let create_associated_token_account_instruction_recipient =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.recipient_pubkey,
                &self.usdc_mint.pubkey(),
                &spl_token::id(),
            );

        let create_associated_token_account_instruction_fee_payer =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.fee_payer_keypair.pubkey(),
                &self.usdc_mint.pubkey(),
                &spl_token::id(),
            );

        // Create Token 2022 accounts using associated token account instructions
        let create_token_2022_account_instruction_sender =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.sender_keypair.pubkey(),
                &self.usdc_mint_2022.pubkey(),
                &spl_token_2022::id(),
            );

        let create_token_2022_account_instruction_recipient =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.recipient_pubkey,
                &self.usdc_mint_2022.pubkey(),
                &spl_token_2022::id(),
            );

        let create_token_2022_account_instruction_fee_payer =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.sender_keypair.pubkey(),
                &self.fee_payer_keypair.pubkey(),
                &self.usdc_mint_2022.pubkey(),
                &spl_token_2022::id(),
            );

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        // Combine all instructions
        let all_instructions = vec![
            create_associated_token_account_instruction,
            create_associated_token_account_instruction_recipient,
            create_associated_token_account_instruction_fee_payer,
            create_token_2022_account_instruction_sender,
            create_token_2022_account_instruction_recipient,
            create_token_2022_account_instruction_fee_payer,
        ];

        let transaction = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;

        let mint_amount =
            1_000_000 * 10_u64.pow(USDCMintTestHelper::get_test_usdc_mint_decimals() as u32);

        // Mint regular SPL tokens
        self.mint_tokens_to_account(&sender_token_account, mint_amount).await?;

        // Mint Token 2022 tokens
        self.mint_tokens_2022_to_account(&sender_token_2022_account, mint_amount).await?;

        Ok((
            sender_token_account,
            recipient_token_account,
            fee_payer_token_account,
            sender_token_2022_account,
            recipient_token_2022_account,
            fee_payer_token_2022_account,
        ))
    }

    pub async fn mint_tokens_to_account(&self, token_account: &Pubkey, amount: u64) -> Result<()> {
        let instruction = token_instruction::mint_to(
            &spl_token::id(),
            &self.usdc_mint.pubkey(),
            token_account,
            &self.sender_keypair.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    pub async fn mint_tokens_2022_to_account(
        &self,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let instruction = token_2022_instruction::mint_to(
            &spl_token_2022::id(),
            &self.usdc_mint_2022.pubkey(),
            token_account,
            &self.sender_keypair.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    async fn create_lookup_tables(&mut self) -> Result<(Pubkey, Pubkey, Pubkey)> {
        let (allowed_lookup_table, disallowed_lookup_table, transaction_lookup_table) =
            LookupTableHelper::setup_and_save_lookup_tables(self.rpc_client.clone()).await?;

        Ok((allowed_lookup_table, disallowed_lookup_table, transaction_lookup_table))
    }
}
