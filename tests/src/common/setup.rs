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
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;
use std::sync::Arc;

use crate::common::{
    FeePayerTestHelper, LookupTableHelper, RecipientTestHelper, SenderTestHelper, TestAccountInfo,
    USDCMintTestHelper, DEFAULT_RPC_URL,
};

/// Test account setup utilities for local validator
pub struct TestAccountSetup {
    pub rpc_client: Arc<RpcClient>,
    pub sender_keypair: Keypair,
    pub fee_payer_keypair: Keypair,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint: Keypair,
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

        Self { rpc_client, sender_keypair, fee_payer_keypair, recipient_pubkey, usdc_mint }
    }

    pub async fn setup_all_accounts(&mut self) -> Result<TestAccountInfo> {
        self.fund_sol_accounts().await?;

        self.create_usdc_mint().await?;

        self.create_lookup_tables().await?;

        let account_info = self.setup_token_accounts().await?;

        // Wait for the accounts to be fully initialized (lookup tables, etc.)
        let await_for_slot = self.rpc_client.get_slot().await? + 30;

        while self.rpc_client.get_slot().await? < await_for_slot {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(account_info)
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

    pub async fn fund_sol_accounts(&self) -> Result<()> {
        let sol_to_fund = 10 * LAMPORTS_PER_SOL;

        let sender_pubkey = self.sender_keypair.pubkey();
        let fee_payer_pubkey = self.fee_payer_keypair.pubkey();

        tokio::try_join!(
            self.airdrop_if_required_sol(&sender_pubkey, sol_to_fund),
            self.airdrop_if_required_sol(&self.recipient_pubkey, sol_to_fund),
            self.airdrop_if_required_sol(&fee_payer_pubkey, sol_to_fund)
        )?;

        Ok(())
    }

    pub async fn create_usdc_mint(&self) -> Result<()> {
        if (self.rpc_client.get_account(&self.usdc_mint.pubkey()).await).is_ok() {
            return Ok(());
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

        Ok(())
    }

    pub async fn setup_token_accounts(&self) -> Result<TestAccountInfo> {
        let sender_token_account =
            get_associated_token_address(&self.sender_keypair.pubkey(), &self.usdc_mint.pubkey());
        let recipient_token_account =
            get_associated_token_address(&self.recipient_pubkey, &self.usdc_mint.pubkey());
        let fee_payer_token_account = get_associated_token_address(
            &self.fee_payer_keypair.pubkey(),
            &self.usdc_mint.pubkey(),
        );

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

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[
                create_associated_token_account_instruction,
                create_associated_token_account_instruction_recipient,
                create_associated_token_account_instruction_fee_payer,
            ],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;

        let mint_amount =
            1_000_000 * 10_u64.pow(USDCMintTestHelper::get_test_usdc_mint_decimals() as u32);

        self.mint_tokens_to_account(&sender_token_account, mint_amount).await?;

        Ok(TestAccountInfo {
            fee_payer_pubkey: self.fee_payer_keypair.pubkey(),
            sender_pubkey: self.sender_keypair.pubkey(),
            recipient_pubkey: self.recipient_pubkey,
            usdc_mint_pubkey: self.usdc_mint.pubkey(),
            sender_token_account,
            recipient_token_account,
            fee_payer_token_account,
        })
    }

    async fn mint_tokens_to_account(&self, token_account: &Pubkey, amount: u64) -> Result<()> {
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

    async fn create_lookup_tables(&mut self) -> Result<()> {
        LookupTableHelper::setup_and_save_lookup_tables(self.rpc_client.clone()).await?;

        Ok(())
    }
}
