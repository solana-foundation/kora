use anyhow::Result;
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use kora_lib::{
    signer::KeypairUtil,
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::encode_b64_transaction,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::Message,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;
use std::{str::FromStr, sync::Arc};

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";
const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

// DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY

// First way of specifying a keypair, as a U8Array (this is the key in local-keys/fee-payer-local.json)
const FEE_PAYER_DEFAULT: &str = "[83, 95, 208, 191, 240, 53, 167, 97, 136, 84, 201, 6, 227, 219, 127, 205, 196, 136, 233, 5, 11, 57, 78, 218, 238, 120, 63, 214, 215, 201, 170, 33, 91, 171, 141, 1, 35, 128, 88, 51, 169, 136, 73, 240, 133, 201, 121, 40, 56, 112, 147, 245, 143, 88, 54, 8, 155, 45, 57, 4, 195, 114, 19, 138]";

// Second way of specifying a keypair, as a file
const SENDER_SIGNER_DEFAULT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/local-keys/sender-local.json");

// Third way of specifying a keypair, as a base58 string
const TEST_USDC_MINT_KEYPAIR: &str =
    "59kKmXphL5UJANqpFFjtH17emEq3oRNmYsx6a3P3vSGJRmhMgVdzH77bkNEi9bArRViT45e8L2TsuPxKNFoc3Qfg"; // Pub: 9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ
const TEST_USDC_MINT_DECIMALS: u8 = 6;

const RECIPIENT_DEFAULT: &str = "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM";

/// Helper function to parse a private key string in multiple formats:
/// - Base58 encoded string (current format)
/// - U8Array format: "[0, 1, 2, ...]"
/// - File path to a JSON keypair file
pub fn parse_private_key_string(private_key: &str) -> Result<Keypair, String> {
    KeypairUtil::from_private_key_string(private_key).map_err(|e| e.to_string())
}

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
        let rpc_client = Arc::new(RpcClient::new(get_rpc_url().await));
        let sender_keypair = get_test_sender_keypair();
        let recipient_pubkey = get_recipient_pubkey();
        let fee_payer_keypair = get_fee_payer_keypair();

        let usdc_mint = get_test_usdc_mint_keypair();

        Self { rpc_client, sender_keypair, fee_payer_keypair, recipient_pubkey, usdc_mint }
    }

    pub async fn setup_all_accounts(&self) -> Result<TestAccountInfo> {
        self.fund_sol_accounts().await?;

        self.create_usdc_mint().await?;

        let account_info = self.setup_token_accounts().await?;

        Ok(account_info)
    }

    pub async fn airdrop_sol(&self, receiver: &Pubkey, amount: u64) -> Result<()> {
        let signature = self.rpc_client.request_airdrop(receiver, amount).await?;

        loop {
            let confirmed = self.rpc_client.confirm_transaction(&signature).await?;
            if confirmed {
                break;
            }
        }

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

        println!("Creating associated token accounts...");

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
}

/// Test account information for outputting to the user
#[derive(Debug)]
pub struct TestAccountInfo {
    pub fee_payer_pubkey: Pubkey,
    pub sender_pubkey: Pubkey,
    pub recipient_pubkey: Pubkey,
    pub usdc_mint_pubkey: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipient_token_account: Pubkey,
    pub fee_payer_token_account: Pubkey,
}

pub async fn setup_test_accounts() -> Result<TestAccountInfo> {
    let setup = TestAccountSetup::new().await;
    setup.setup_all_accounts().await
}

pub async fn check_test_validator() -> bool {
    let client = RpcClient::new(DEFAULT_RPC_URL.to_string());
    client.get_health().await.is_ok()
}

pub async fn get_rpc_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
}

pub fn get_test_sender_keypair() -> Keypair {
    dotenv::dotenv().ok();
    let private_key =
        std::env::var("TEST_SENDER_KEYPAIR").unwrap_or(SENDER_SIGNER_DEFAULT.to_string());
    parse_private_key_string(&private_key).expect("Failed to parse test sender private key")
}

/// Create a fresh HTTP client for each test (no shared state)
pub async fn get_test_client() -> jsonrpsee::http_client::HttpClient {
    HttpClientBuilder::default().build(get_test_server_url()).expect("Failed to create HTTP client")
}

/// Create a fresh RPC client for each test (better for concurrency and isolation)
pub async fn get_rpc_client() -> Arc<RpcClient> {
    Arc::new(RpcClient::new(get_rpc_url().await))
}

/// Create a test SOL transfer transaction
pub async fn create_test_transaction() -> Result<String> {
    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();
    let amount = 10;
    let rpc_client = get_rpc_client().await;

    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    let message = Message::new_with_blockhash(&[instruction], None, &blockhash.0);
    let transaction = Transaction { signatures: vec![Default::default()], message };

    Ok(encode_b64_transaction(&transaction)?)
}

/// Create a test SPL token transfer transaction
pub async fn create_test_spl_transaction() -> Result<String> {
    let rpc_client = get_rpc_client().await;
    let client = get_test_client().await;

    // Get fee payer from config
    let response: serde_json::Value = client
        .request("getConfig", rpc_params![])
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get config: {}", e))?;
    let fee_payer = Pubkey::from_str(response["fee_payer"].as_str().unwrap())?;

    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();

    // Setup token accounts
    let token_mint = get_test_usdc_mint_pubkey();
    let sender_token_account = get_associated_token_address(&sender.pubkey(), &token_mint);
    let recipient_token_account = get_associated_token_address(&recipient, &token_mint);

    // Create token transfer instruction
    let token_interface = TokenProgram::new(TokenType::Spl);
    let amount = 1000; // Transfer 1000 token units
    let instruction = token_interface
        .create_transfer_instruction(
            &sender_token_account,
            &recipient_token_account,
            &sender.pubkey(),
            amount,
        )
        .expect("Failed to create transfer instruction");

    // Get recent blockhash
    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    // Create message and transaction
    let message = Message::new_with_blockhash(&[instruction], Some(&fee_payer), &blockhash.0);
    let transaction = Transaction::new_unsigned(message);

    Ok(encode_b64_transaction(&transaction)?)
}

// Getter functions that check environment variables first, then fall back to defaults

/// Get the test server URL, checking TEST_SERVER_URL env var first
pub fn get_test_server_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| TEST_SERVER_URL.to_string())
}

/// Get the fee payer keypair, checking KORA_PRIVATE_KEY env var first
pub fn get_fee_payer_keypair() -> Keypair {
    dotenv::dotenv().ok();
    let private_key =
        std::env::var("KORA_PRIVATE_KEY").unwrap_or_else(|_| FEE_PAYER_DEFAULT.to_string());
    parse_private_key_string(&private_key).expect("Failed to parse fee payer private key")
}

/// Get the fee payer public key, derived from the fee payer keypair
pub fn get_fee_payer_pubkey() -> Pubkey {
    get_fee_payer_keypair().pubkey()
}

/// Get the recipient pubkey, checking TEST_RECIPIENT_PUBKEY env var first
pub fn get_recipient_pubkey() -> Pubkey {
    dotenv::dotenv().ok();
    let recipient_str =
        std::env::var("TEST_RECIPIENT_PUBKEY").unwrap_or_else(|_| RECIPIENT_DEFAULT.to_string());
    Pubkey::from_str(&recipient_str).expect("Invalid recipient pubkey")
}

/// Get the test USDC mint keypair, checking TEST_USDC_MINT_KEYPAIR env var first
pub fn get_test_usdc_mint_keypair() -> Keypair {
    dotenv::dotenv().ok();
    let mint_keypair = std::env::var("TEST_USDC_MINT_KEYPAIR")
        .unwrap_or_else(|_| TEST_USDC_MINT_KEYPAIR.to_string());
    parse_private_key_string(&mint_keypair).expect("Failed to parse test USDC mint private key")
}

/// Get the test USDC mint pubkey, derived from the mint keypair
pub fn get_test_usdc_mint_pubkey() -> Pubkey {
    get_test_usdc_mint_keypair().pubkey()
}

/// Get the test USDC mint decimals, checking TEST_USDC_MINT_DECIMALS env var first
pub fn get_test_usdc_mint_decimals() -> u8 {
    dotenv::dotenv().ok();
    std::env::var("TEST_USDC_MINT_DECIMALS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TEST_USDC_MINT_DECIMALS)
}
