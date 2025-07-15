use anyhow::Result;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcRequestAirdropConfig};
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
use std::{str::FromStr, sync::Arc};

pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";
pub const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

// DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY
pub const FEE_PAYER_DEFAULT: &str =
    "64TVMxZyYyLHwyfZRJUJMqF8GJsMsZQKk4JhdKkyMB7k3fNUjWZdAF1YmyLd43dWEBLCLYjKDkoRGfMwMoGUrDF6"; // Would be the key used to boot-up Kora --private-key 
pub const FEE_PAYER_PUBKEY: &str = "8vCbKjax96gWzqRsok7eRG9TvxDttm93F3e46MAYQM3n";
pub const SENDER_SIGNER_DEFAULT: &str =
    "3Tdt5TrRGJYPbTo8zZAscNTvgRGnCLM854tCpxapggUazqdYn6VQRQ9DqNz1UkEfoPCYKj6PwSwCNtckGGvAKugb";
pub const RECIPIENT_DEFAULT: &str = "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM";

// Deterministic test USDC mint keypair (for local testing only)
pub const TEST_USDC_MINT_KEYPAIR: &str =
    "59kKmXphL5UJANqpFFjtH17emEq3oRNmYsx6a3P3vSGJRmhMgVdzH77bkNEi9bArRViT45e8L2TsuPxKNFoc3Qfg";
pub const TEST_USDC_MINT_PUBKEY: &str = "9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ";
pub const TEST_USDC_MINT_DECIMALS: u8 = 6;

/// Test account setup utilities for local validator
pub struct TestAccountSetup {
    pub rpc_client: Arc<RpcClient>,
    pub sender_keypair: Keypair,
    pub fee_payer_keypair: Keypair,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint: Keypair,
}

impl TestAccountSetup {
    pub fn new() -> Self {
        let rpc_client = Arc::new(RpcClient::new(DEFAULT_RPC_URL.to_string()));
        let sender_keypair = Keypair::from_base58_string(SENDER_SIGNER_DEFAULT);
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_DEFAULT).unwrap();
        let fee_payer_keypair = Keypair::from_base58_string(FEE_PAYER_DEFAULT);

        let usdc_mint = Keypair::from_base58_string(TEST_USDC_MINT_KEYPAIR);

        Self { rpc_client, sender_keypair, fee_payer_keypair, recipient_pubkey, usdc_mint }
    }

    pub async fn setup_all_accounts(&self) -> Result<TestAccountInfo> {
        self.fund_sol_accounts().await?;

        self.create_usdc_mint().await?;

        let account_info = self.setup_token_accounts().await?;

        Ok(account_info)
    }

    pub async fn airdrop_sol(&self, receiver: &Pubkey, amount: u64) -> Result<()> {
        let config = RpcRequestAirdropConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };

        let _signature =
            self.rpc_client.request_airdrop_with_config(receiver, amount, config).await?;

        Ok(())
    }

    pub async fn fund_sol_accounts(&self) -> Result<()> {
        println!("Checking and funding SOL accounts...");

        let sol_to_fund = 10 * LAMPORTS_PER_SOL;

        let sender_balance = self.rpc_client.get_balance(&self.sender_keypair.pubkey()).await?;
        println!("Sender balance: {} SOL", sender_balance as f64 / LAMPORTS_PER_SOL as f64);
        if sender_balance < sol_to_fund {
            self.airdrop_sol(&self.sender_keypair.pubkey(), sol_to_fund).await?;
        }

        let recipient_balance = self.rpc_client.get_balance(&self.recipient_pubkey).await?;
        println!("Recipient balance: {} SOL", recipient_balance as f64 / LAMPORTS_PER_SOL as f64);
        if recipient_balance < sol_to_fund {
            self.airdrop_sol(&self.recipient_pubkey, sol_to_fund).await?;
        }

        let fee_payer_balance =
            self.rpc_client.get_balance(&self.fee_payer_keypair.pubkey()).await?;
        println!("Fee payer balance: {} SOL", fee_payer_balance as f64 / LAMPORTS_PER_SOL as f64);
        if fee_payer_balance < sol_to_fund {
            self.airdrop_sol(&self.fee_payer_keypair.pubkey(), sol_to_fund).await?;
        }

        Ok(())
    }

    pub async fn create_usdc_mint(&self) -> Result<()> {
        println!("Creating USDC mint: {}", self.usdc_mint.pubkey());

        if (self.rpc_client.get_account(&self.usdc_mint.pubkey()).await).is_ok() {
            println!("USDC mint already exists");
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
            TEST_USDC_MINT_DECIMALS,
        )?;

        println!("Creating and initializing mint...");
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
        println!("Setting up token accounts...");

        let sender_token_account =
            get_associated_token_address(&self.sender_keypair.pubkey(), &self.usdc_mint.pubkey());
        let recipient_token_account =
            get_associated_token_address(&self.recipient_pubkey, &self.usdc_mint.pubkey());

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

        println!("Creating associated token accounts...");

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[
                create_associated_token_account_instruction,
                create_associated_token_account_instruction_recipient,
            ],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).await?;
        println!("Token accounts created");

        let mint_amount = 1_000_000 * 10_u64.pow(TEST_USDC_MINT_DECIMALS as u32);
        println!(
            "Minting {} USDC to sender...",
            mint_amount / 10_u64.pow(TEST_USDC_MINT_DECIMALS as u32)
        );
        self.mint_tokens_to_account(&sender_token_account, mint_amount).await?;

        Ok(TestAccountInfo {
            fee_payer_pubkey: self.fee_payer_keypair.pubkey(),
            sender_pubkey: self.sender_keypair.pubkey(),
            recipient_pubkey: self.recipient_pubkey,
            usdc_mint_pubkey: self.usdc_mint.pubkey(),
            sender_token_account,
            recipient_token_account,
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
}

#[derive(Debug)]
pub struct TestAccountInfo {
    pub fee_payer_pubkey: Pubkey,
    pub sender_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint_pubkey: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipient_token_account: Pubkey,
}

pub async fn setup_test_accounts() -> Result<TestAccountInfo> {
    let setup = TestAccountSetup::new();
    setup.setup_all_accounts().await
}

pub async fn check_test_validator() -> bool {
    let client = RpcClient::new(DEFAULT_RPC_URL.to_string());
    client.get_health().await.is_ok()
}
