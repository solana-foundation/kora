use anyhow::Result;
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use kora_lib::{
    signer::KeypairUtil,
    token::{TokenInterface, TokenProgram},
    transaction::{TransactionUtil, VersionedTransactionOps, VersionedTransactionResolved},
};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::{
    v0::Message as V0Message, AddressLookupTableAccount, Message, VersionedMessage,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use solana_system_interface::instruction::transfer;
use spl_associated_token_account::get_associated_token_address;
use std::{str::FromStr, sync::Arc};

pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";
const TEST_SERVER_URL: &str = "http://127.0.0.1:8080";

// ****************************************************************
// DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY
// ****************************************************************
pub const FEE_PAYER_DEFAULT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/fee-payer-local.json");

pub const SENDER_SIGNER_DEFAULT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/sender-local.json");

pub const RECIPIENT_DEFAULT: &str = "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM";

// To test lookup tables
pub const TEST_DISALLOWED_ADDRESS: &str = "hndXZGK45hCxfBYvxejAXzCfCujoqkNf7rk4sTB8pek";

// Test with different payment address for the paymaster
pub const TEST_PAYMENT_ADDRESS: &str = "CWvWnVwqAb9HzqwCGkn4purGEUuu27aNsPQM252uLerV";

// Deterministic test USDC mint keypair (for local testing only)
pub const TEST_USDC_MINT_KEYPAIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/common/local-keys/usdc-mint-local.json"); //9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ
pub const TEST_USDC_MINT_DECIMALS: u8 = 6;

// PYUSD token mint on devnet
pub const PYUSD_MINT: &str = "CXk2AMBfi3TwaEL2468s6zP8xq9NxTXjp9gjMgzeUynM";

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

/// Helper function to parse a private key string in multiple formats.
pub fn parse_private_key_string(private_key: &str) -> Result<Keypair, String> {
    KeypairUtil::from_private_key_string(private_key).map_err(|e| e.to_string())
}

pub struct RPCTestHelper;

impl RPCTestHelper {
    pub async fn get_rpc_client() -> Arc<RpcClient> {
        Arc::new(RpcClient::new(Self::get_rpc_url().await))
    }

    pub async fn get_rpc_url() -> String {
        dotenv::dotenv().ok();
        std::env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
    }
}

pub struct ClientTestHelper;

impl ClientTestHelper {
    pub async fn get_test_client() -> jsonrpsee::http_client::HttpClient {
        HttpClientBuilder::default()
            .build(Self::get_test_server_url())
            .expect("Failed to create HTTP client")
    }

    pub fn get_test_server_url() -> String {
        dotenv::dotenv().ok();
        std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| TEST_SERVER_URL.to_string())
    }
}

pub struct FeePayerTestHelper;

impl FeePayerTestHelper {
    pub fn get_fee_payer_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let private_key = match std::env::var("KORA_PRIVATE_KEY") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(FEE_PAYER_DEFAULT)
                .expect("Failed to read fee payer private key file"),
        };
        parse_private_key_string(&private_key).expect("Failed to parse fee payer private key")
    }

    pub fn get_fee_payer_pubkey() -> Pubkey {
        Self::get_fee_payer_keypair().pubkey()
    }
}

pub struct SenderTestHelper;

impl SenderTestHelper {
    pub fn get_test_sender_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let private_key = match std::env::var("TEST_SENDER_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(SENDER_SIGNER_DEFAULT)
                .expect("Failed to read sender private key file"),
        };
        parse_private_key_string(&private_key).expect("Failed to parse test sender private key")
    }
}

pub struct RecipientTestHelper;

impl RecipientTestHelper {
    pub fn get_recipient_pubkey() -> Pubkey {
        dotenv::dotenv().ok();
        let recipient_str = std::env::var("TEST_RECIPIENT_PUBKEY")
            .unwrap_or_else(|_| RECIPIENT_DEFAULT.to_string());
        Pubkey::from_str(&recipient_str).expect("Invalid recipient pubkey")
    }
}

pub struct USDCMintTestHelper;

impl USDCMintTestHelper {
    pub fn get_test_usdc_mint_keypair() -> Keypair {
        dotenv::dotenv().ok();
        let mint_keypair = match std::env::var("TEST_USDC_MINT_KEYPAIR") {
            Ok(key) => key,
            Err(_) => std::fs::read_to_string(TEST_USDC_MINT_KEYPAIR)
                .expect("Failed to read USDC mint private key file"),
        };
        parse_private_key_string(&mint_keypair).expect("Failed to parse test USDC mint private key")
    }

    pub fn get_test_usdc_mint_pubkey() -> Pubkey {
        Self::get_test_usdc_mint_keypair().pubkey()
    }

    pub fn get_test_usdc_mint_decimals() -> u8 {
        dotenv::dotenv().ok();
        std::env::var("TEST_USDC_MINT_DECIMALS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(TEST_USDC_MINT_DECIMALS)
    }
}

pub struct LookupTableTestHelper;

impl LookupTableTestHelper {
    pub fn get_test_disallowed_address() -> Pubkey {
        Pubkey::from_str(TEST_DISALLOWED_ADDRESS).expect("Invalid disallowed address")
    }

    pub async fn get_allowed_lookup_table_address() -> Result<Pubkey> {
        let lookup_tables = Self::get_test_lookup_table_addresses().await?;
        Ok(lookup_tables[0])
    }

    pub async fn get_disallowed_lookup_table_address() -> Result<Pubkey> {
        let lookup_tables = Self::get_test_lookup_table_addresses().await?;
        Ok(lookup_tables[1])
    }

    pub async fn get_test_lookup_table_addresses() -> Result<[Pubkey; 2]> {
        let rpc_client = RPCTestHelper::get_rpc_client().await;
        let sender = SenderTestHelper::get_test_sender_keypair();

        // Get all address lookup table accounts
        let accounts = rpc_client
            .get_program_accounts(&solana_address_lookup_table_interface::program::ID)
            .await?;

        let disallowed_address = Self::get_test_disallowed_address();

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
}

pub struct TransactionTestHelper;

impl TransactionTestHelper {
    pub async fn create_test_transaction() -> Result<String> {
        let sender = SenderTestHelper::get_test_sender_keypair();
        let recipient = RecipientTestHelper::get_recipient_pubkey();
        let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
        let amount = 10;
        let rpc_client = RPCTestHelper::get_rpc_client().await;

        let instruction = transfer(&sender.pubkey(), &recipient, amount);

        let blockhash =
            rpc_client.get_latest_blockhash_with_commitment(CommitmentConfig::finalized()).await?;

        let message = VersionedMessage::Legacy(Message::new_with_blockhash(
            &[instruction],
            Some(&fee_payer),
            &blockhash.0,
        ));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

        let resolved_transaction =
            VersionedTransactionResolved::from_transaction_only(&transaction);

        Ok(resolved_transaction.encode_b64_transaction()?)
    }

    pub async fn create_test_spl_transaction() -> Result<String> {
        let rpc_client = RPCTestHelper::get_rpc_client().await;
        let client = ClientTestHelper::get_test_client().await;

        // Get fee payer from config
        let response: serde_json::Value = client
            .request("getConfig", rpc_params![])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get config: {}", e))?;
        let fee_payer = Pubkey::from_str(response["fee_payer"].as_str().unwrap())?;

        let sender = SenderTestHelper::get_test_sender_keypair();
        let recipient = RecipientTestHelper::get_recipient_pubkey();

        // Setup token accounts
        let token_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
        let sender_token_account = get_associated_token_address(&sender.pubkey(), &token_mint);
        let recipient_token_account = get_associated_token_address(&recipient, &token_mint);

        // Create token transfer instruction
        let token_interface = TokenProgram::new();
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

        let resolved_transaction =
            VersionedTransactionResolved::from_transaction_only(&transaction);

        Ok(resolved_transaction.encode_b64_transaction()?)
    }

    pub async fn create_v0_transaction_with_lookup(
        lookup_table_key: &Pubkey,
        recipient: &Pubkey,
    ) -> Result<String> {
        let rpc_client = RPCTestHelper::get_rpc_client().await;

        let sender = SenderTestHelper::get_test_sender_keypair();
        let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
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
            &fee_payer,
            &[transfer_instruction],
            &[address_lookup_table_account],
            blockhash.0,
        )?);
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(versioned_message);

        let resolved_transaction =
            VersionedTransactionResolved::from_transaction_only(&transaction);

        Ok(resolved_transaction.encode_b64_transaction()?)
    }
}
