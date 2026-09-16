use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account_interface::address::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token_2022_interface::instruction as token_2022_instruction;
use spl_token_interface::instruction as token_instruction;
use std::{str::FromStr, sync::Arc};

use crate::common::{
    FeePayerPolicyMintTestHelper, FeePayerTestHelper, RecipientTestHelper, SenderTestHelper,
    USDCMint2022TestHelper, USDCMintTestHelper, DEFAULT_RPC_URL, TS_AUTH_WALLET_PUBKEY,
    TS_FREE_WALLET_PUBKEY,
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
    // Fee payer policy mint fields
    pub fee_payer_policy_mint_pubkey: Pubkey,
    pub fee_payer_policy_sender_token_account: Pubkey,
    pub fee_payer_policy_recipient_token_account: Pubkey,
    pub fee_payer_policy_fee_payer_token_account: Pubkey,
    // Fee payer policy Token 2022 fields
    pub fee_payer_policy_mint_2022_pubkey: Pubkey,
    pub fee_payer_policy_sender_token_2022_account: Pubkey,
    pub fee_payer_policy_recipient_token_2022_account: Pubkey,
    pub fee_payer_policy_fee_payer_token_2022_account: Pubkey,
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
    pub fee_payer_policy_mint: Keypair,
    pub fee_payer_policy_mint_2022: Keypair,
}

impl TestAccountSetup {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();
        let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        let rpc_client =
            Arc::new(RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()));
        Self::new_with_client(rpc_client).await
    }

    async fn new_with_client(rpc_client: Arc<RpcClient>) -> Self {
        let sender_keypair = SenderTestHelper::get_test_sender_keypair();
        let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();
        let fee_payer_keypair = FeePayerTestHelper::get_fee_payer_keypair();

        let usdc_mint = USDCMintTestHelper::get_test_usdc_mint_keypair();
        let usdc_mint_2022 = USDCMint2022TestHelper::get_test_usdc_mint_2022_keypair();
        let fee_payer_policy_mint =
            FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_keypair();
        let fee_payer_policy_mint_2022 =
            FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_2022_keypair();

        Self {
            rpc_client,
            sender_keypair,
            fee_payer_keypair,
            recipient_pubkey,
            usdc_mint,
            usdc_mint_2022,
            fee_payer_policy_mint,
            fee_payer_policy_mint_2022,
        }
    }

    async fn all_token_accounts_exist(&self, accounts: &[Pubkey]) -> Result<bool> {
        let fetched = self.rpc_client.get_multiple_accounts(accounts).await?;
        Ok(fetched.iter().all(Option::is_some))
    }

    pub async fn mint_tokens_to_account(&self, token_account: &Pubkey, amount: u64) -> Result<()> {
        let instruction = token_instruction::mint_to(
            &spl_token_interface::id(),
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
            &spl_token_2022_interface::id(),
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

    pub async fn setup_fee_payer_policy_token_accounts(
        &self,
    ) -> Result<(Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey)> {
        // SPL Token accounts
        let sender_token_account = get_associated_token_address(
            &self.sender_keypair.pubkey(),
            &self.fee_payer_policy_mint.pubkey(),
        );
        let recipient_token_account = get_associated_token_address(
            &self.recipient_pubkey,
            &self.fee_payer_policy_mint.pubkey(),
        );
        let fee_payer_token_account = get_associated_token_address(
            &self.fee_payer_keypair.pubkey(),
            &self.fee_payer_policy_mint.pubkey(),
        );

        // Token 2022 accounts
        let sender_token_2022_account = get_associated_token_address_with_program_id(
            &self.sender_keypair.pubkey(),
            &self.fee_payer_policy_mint_2022.pubkey(),
            &spl_token_2022_interface::id(),
        );
        let recipient_token_2022_account = get_associated_token_address_with_program_id(
            &self.recipient_pubkey,
            &self.fee_payer_policy_mint_2022.pubkey(),
            &spl_token_2022_interface::id(),
        );
        let fee_payer_token_2022_account = get_associated_token_address_with_program_id(
            &self.fee_payer_keypair.pubkey(),
            &self.fee_payer_policy_mint_2022.pubkey(),
            &spl_token_2022_interface::id(),
        );

        let all_accounts = (
            sender_token_account,
            recipient_token_account,
            fee_payer_token_account,
            sender_token_2022_account,
            recipient_token_2022_account,
            fee_payer_token_2022_account,
        );
        if self
            .all_token_accounts_exist(&[
                sender_token_account,
                recipient_token_account,
                fee_payer_token_account,
                sender_token_2022_account,
                recipient_token_2022_account,
                fee_payer_token_2022_account,
            ])
            .await?
        {
            return Ok(all_accounts);
        }

        // Create regular SPL Token accounts
        let create_associated_token_account_instruction =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.sender_keypair.pubkey(),
                &self.fee_payer_policy_mint.pubkey(),
                &spl_token_interface::id(),
            );

        let create_associated_token_account_instruction_recipient =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.recipient_pubkey,
                &self.fee_payer_policy_mint.pubkey(),
                &spl_token_interface::id(),
            );

        let create_associated_token_account_instruction_fee_payer =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.fee_payer_keypair.pubkey(),
                &self.fee_payer_policy_mint.pubkey(),
                &spl_token_interface::id(),
            );

        // Create Token 2022 accounts
        let create_token_2022_account_instruction_sender =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.sender_keypair.pubkey(),
                &self.fee_payer_policy_mint_2022.pubkey(),
                &spl_token_2022_interface::id(),
            );

        let create_token_2022_account_instruction_recipient =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.recipient_pubkey,
                &self.fee_payer_policy_mint_2022.pubkey(),
                &spl_token_2022_interface::id(),
            );

        let create_token_2022_account_instruction_fee_payer =
            spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
                &self.fee_payer_keypair.pubkey(),
                &self.fee_payer_keypair.pubkey(),
                &self.fee_payer_policy_mint_2022.pubkey(),
                &spl_token_2022_interface::id(),
            );

        let mint_amount =
            1_000_000 * 10_u64.pow(USDCMintTestHelper::get_test_usdc_mint_decimals() as u32);

        let all_instructions = vec![
            create_associated_token_account_instruction,
            create_associated_token_account_instruction_recipient,
            create_associated_token_account_instruction_fee_payer,
            create_token_2022_account_instruction_sender,
            create_token_2022_account_instruction_recipient,
            create_token_2022_account_instruction_fee_payer,
            token_instruction::mint_to(
                &spl_token_interface::id(),
                &self.fee_payer_policy_mint.pubkey(),
                &sender_token_account,
                &self.fee_payer_keypair.pubkey(),
                &[],
                mint_amount,
            )?,
            token_2022_instruction::mint_to(
                &spl_token_2022_interface::id(),
                &self.fee_payer_policy_mint_2022.pubkey(),
                &sender_token_2022_account,
                &self.fee_payer_keypair.pubkey(),
                &[],
                mint_amount,
            )?,
        ];

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&self.fee_payer_keypair.pubkey()),
            &[&self.fee_payer_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;

        Ok((
            sender_token_account,
            recipient_token_account,
            fee_payer_token_account,
            sender_token_2022_account,
            recipient_token_2022_account,
            fee_payer_token_2022_account,
        ))
    }

    pub async fn mint_fee_payer_policy_tokens_to_account(
        &self,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let instruction = token_instruction::mint_to(
            &spl_token_interface::id(),
            &self.fee_payer_policy_mint.pubkey(),
            token_account,
            &self.fee_payer_keypair.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer_keypair.pubkey()),
            &[&self.fee_payer_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    pub async fn mint_fee_payer_policy_tokens_2022_to_account(
        &self,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let instruction = token_2022_instruction::mint_to(
            &spl_token_2022_interface::id(),
            &self.fee_payer_policy_mint_2022.pubkey(),
            token_account,
            &self.fee_payer_keypair.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer_keypair.pubkey()),
            &[&self.fee_payer_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    /// Create a new unique token account for the fee payer (not an ATA)
    pub async fn create_fee_payer_token_account_spl(&self, mint: &Pubkey) -> Result<Keypair> {
        let token_account = Keypair::new();

        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
            .await?;

        let create_account_ix = solana_system_interface::instruction::create_account(
            &self.fee_payer_keypair.pubkey(),
            &token_account.pubkey(),
            rent,
            spl_token_interface::state::Account::LEN as u64,
            &spl_token_interface::id(),
        );

        let initialize_account_ix = spl_token_interface::instruction::initialize_account(
            &spl_token_interface::id(),
            &token_account.pubkey(),
            mint,
            &self.fee_payer_keypair.pubkey(),
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_account_ix],
            Some(&self.fee_payer_keypair.pubkey()),
            &[&self.fee_payer_keypair, &token_account],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(token_account)
    }

    /// Create a new unique token account for the fee payer (Token2022, not an ATA)
    pub async fn create_fee_payer_token_account_2022(&self, mint: &Pubkey) -> Result<Keypair> {
        let token_account = Keypair::new();

        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token_2022_interface::state::Account::LEN)
            .await?;

        let create_account_ix = solana_system_interface::instruction::create_account(
            &self.fee_payer_keypair.pubkey(),
            &token_account.pubkey(),
            rent,
            spl_token_2022_interface::state::Account::LEN as u64,
            &spl_token_2022_interface::id(),
        );

        let initialize_account_ix = spl_token_2022_interface::instruction::initialize_account(
            &spl_token_2022_interface::id(),
            &token_account.pubkey(),
            mint,
            &self.fee_payer_keypair.pubkey(),
        )?;

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, initialize_account_ix],
            Some(&self.fee_payer_keypair.pubkey()),
            &[&self.fee_payer_keypair, &token_account],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(token_account)
    }
}

pub fn ts_auth_wallet() -> Pubkey {
    Pubkey::from_str(TS_AUTH_WALLET_PUBKEY).expect("Invalid TS auth wallet pubkey")
}

pub fn ts_free_wallet() -> Pubkey {
    Pubkey::from_str(TS_FREE_WALLET_PUBKEY).expect("Invalid TS free wallet pubkey")
}
