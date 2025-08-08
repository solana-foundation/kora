use anyhow::Result;
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use kora_lib::{
    signer::KeypairUtil,
    token::{TokenInterface, TokenProgram, token::TokenType},
    transaction::{TransactionUtil, VersionedTransactionUtilExt},
};
use solana_address_lookup_table_interface::{
    instruction::create_lookup_table, state::AddressLookupTable,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{
    AddressLookupTableAccount, Message, VersionedMessage, v0::Message as V0Message,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::transfer;
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

const RECIPIENT_DEFAULT: &str = "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM";

// To test lookup tables
const TEST_DISALLOWED_ADDRESS: &str = "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek";

// Deterministic test USDC mint keypair (for local testing only)
// Third way of specifying a keypair, as a base58 string
const TEST_USDC_MINT_KEYPAIR: &str =
    "59kKmXphL5UJANqpFFjtH17emEq3oRNmYsx6a3P3vSGJRmhMgVdzH77bkNEi9bArRViT45e8L2TsuPxKNFoc3Qfg"; // Pub: 9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ
const TEST_USDC_MINT_DECIMALS: u8 = 6;

// PYUSD token mint on devnet
pub const PYUSD_MINT: &str = "CXk2AMBfi3TwaEL2468s6zP8xq9NxTXjp9gjMgzeUynM";

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
    pub lookup_tables_key: Vec<Pubkey>,
}

impl TestAccountSetup {
    pub async fn new() -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            get_rpc_url().await,
            CommitmentConfig::confirmed(),
        ));
        let sender_keypair = get_test_sender_keypair();
        let recipient_pubkey = get_recipient_pubkey();
        let fee_payer_keypair = get_fee_payer_keypair();

        let usdc_mint = get_test_usdc_mint_keypair();

        Self {
            rpc_client,
            sender_keypair,
            fee_payer_keypair,
            recipient_pubkey,
            usdc_mint,
            lookup_tables_key: vec![],
        }
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
        println!("Checking and funding SOL accounts...");

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

    async fn create_lookup_tables(&mut self) -> Result<()> {
        let allowed_lookup_table =
            self.create_with_addresses(vec![solana_sdk::system_program::ID]).await?;

        let disallowed_address = get_test_disallowed_address();
        let blocked_lookup_table: Pubkey =
            self.create_with_addresses(vec![disallowed_address]).await?;

        self.lookup_tables_key = vec![allowed_lookup_table, blocked_lookup_table];

        Ok(())
    }

    async fn create_with_addresses(&self, addresses: Vec<Pubkey>) -> Result<Pubkey> {
        let recent_slot = self.rpc_client.get_slot().await?;
        let (create_instruction, lookup_table_key) = create_lookup_table(
            self.sender_keypair.pubkey(),
            self.sender_keypair.pubkey(),
            recent_slot - 1,
        );

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let create_transaction = Transaction::new_signed_with_payer(
            &[create_instruction],
            Some(&self.sender_keypair.pubkey()),
            &[&self.sender_keypair],
            recent_blockhash,
        );

        self.rpc_client.send_and_confirm_transaction(&create_transaction).await?;

        // Add addresses to the lookup table if provided
        if !addresses.is_empty() {
            let extend_instruction =
                solana_address_lookup_table_interface::instruction::extend_lookup_table(
                    lookup_table_key,
                    self.sender_keypair.pubkey(),
                    Some(self.sender_keypair.pubkey()),
                    addresses.clone(),
                );

            let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
            let extend_transaction = Transaction::new_signed_with_payer(
                &[extend_instruction],
                Some(&self.sender_keypair.pubkey()),
                &[&self.sender_keypair],
                recent_blockhash,
            );

            self.rpc_client.send_and_confirm_transaction(&extend_transaction).await?;
        }

        Ok(lookup_table_key)
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
    let mut setup = TestAccountSetup::new().await;
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

pub fn get_test_server_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| TEST_SERVER_URL.to_string())
}

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

pub fn get_test_disallowed_address() -> Pubkey {
    Pubkey::from_str(TEST_DISALLOWED_ADDRESS).expect("Invalid disallowed address")
}

/// Get the allowed lookup table address (contains random address)
pub async fn get_allowed_lookup_table_address() -> Result<Pubkey> {
    let lookup_tables = get_test_lookup_table_addresses().await?;
    Ok(lookup_tables[0])
}

/// Get the disallowed lookup table address (contains disallowed address)  
pub async fn get_disallowed_lookup_table_address() -> Result<Pubkey> {
    let lookup_tables = get_test_lookup_table_addresses().await?;
    Ok(lookup_tables[1])
}

/// Get both lookup table addresses by finding them on-chain
pub async fn get_test_lookup_table_addresses() -> Result<[Pubkey; 2]> {
    let rpc_client = get_rpc_client().await;
    let sender = get_test_sender_keypair();

    // Get all address lookup table accounts
    let accounts = rpc_client
        .get_program_accounts(&solana_address_lookup_table_interface::program::ID)
        .await?;

    let disallowed_address = get_test_disallowed_address();

    let mut lookup_addresses = [Pubkey::default(), Pubkey::default()];
    for (pubkey, account) in accounts {
        if let Ok(lookup_table) = AddressLookupTable::deserialize(&account.data) {
            // Check if this lookup table was created by our test sender
            if lookup_table.meta.authority == Some(sender.pubkey()) {
                // Determine which lookup table this is based on its contents
                if lookup_table.addresses.len() == 1 {
                    let address = lookup_table.addresses[0];
                    if address == disallowed_address {
                        lookup_addresses[1] = pubkey;
                    } else {
                        lookup_addresses[0] = pubkey;
                    }
                }
            }
        }
    }

    Ok(lookup_addresses)
}

/// Create a test SOL transfer transaction
pub async fn create_test_transaction() -> Result<String> {
    let sender = get_test_sender_keypair();
    let recipient = get_recipient_pubkey();
    let amount = 10;
    let rpc_client = get_rpc_client().await;

    let instruction = transfer(&sender.pubkey(), &recipient, amount);

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    let message =
        VersionedMessage::Legacy(Message::new_with_blockhash(&[instruction], None, &blockhash.0));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    Ok(transaction.encode_b64_transaction()?)
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
    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[instruction],
        Some(&fee_payer),
        &blockhash.0,
    ));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    Ok(transaction.encode_b64_transaction()?)
}

pub async fn create_v0_transaction_with_lookup(
    lookup_table_key: &Pubkey,
    recipient: &Pubkey,
) -> Result<String> {
    let rpc_client = get_rpc_client().await;

    let sender = get_test_sender_keypair();
    let lookup_table_account = rpc_client.get_account(lookup_table_key).await?;
    let lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data)?;

    let address_lookup_table_account = AddressLookupTableAccount {
        key: *lookup_table_key,
        addresses: lookup_table.addresses.to_vec(),
    };

    let transfer_instruction = transfer(&sender.pubkey(), recipient, 1_000_000);

    let blockhash =
        rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

    let versioned_message = VersionedMessage::V0(V0Message::try_compile(
        &sender.pubkey(),
        &[transfer_instruction],
        &[address_lookup_table_account],
        blockhash.0,
    )?);
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(versioned_message);

    Ok(transaction.encode_b64_transaction()?)
}
