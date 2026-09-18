use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program_pack::Pack;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_2022_interface::instruction as token_2022_instruction;
use spl_token_interface::instruction as token_instruction;
use std::{str::FromStr, sync::Arc};

use crate::common::{
    FeePayerPolicyMintTestHelper, FeePayerTestHelper, RecipientTestHelper, SenderTestHelper,
    USDCMint2022TestHelper, USDCMintTestHelper, TS_AUTH_WALLET_PUBKEY, TS_FREE_WALLET_PUBKEY,
};

/// Keypairs and helpers for state a test creates for itself, on top of what
/// the harness already seeded.
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
    pub async fn new(rpc_client: Arc<RpcClient>) -> Self {
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
