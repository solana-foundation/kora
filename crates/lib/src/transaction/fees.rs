use crate::{
    constant::LAMPORTS_PER_SIGNATURE,
    error::KoraError,
    get_signer,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle},
    token::{TokenInterface, TokenProgram, TokenType},
    transaction::{get_estimate_fee, VersionedTransactionExt},
};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, rent::Rent, transaction::VersionedTransaction,
};
use spl_associated_token_account::get_associated_token_address;
use std::{str::FromStr, time::Duration};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPriceInfo {
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PriceModel {
    Margin { margin: f64 },
    Fixed { amount: u64, token: String },
    Free,
}

impl Default for PriceModel {
    fn default() -> Self {
        Self::Margin { margin: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct PriceConfig {
    #[serde(flatten)]
    pub model: PriceModel,
}

impl PriceConfig {
    pub async fn get_required_lamports(
        &self,
        rpc_client: Option<&RpcClient>,
        price_source: Option<PriceSource>,
        min_transaction_fee: u64,
    ) -> Result<u64, KoraError> {
        match &self.model {
            PriceModel::Margin { margin } => {
                Ok((min_transaction_fee as f64 * (1.0 + margin)) as u64)
            }
            PriceModel::Fixed { amount, token } => {
                if let (Some(price_source), Some(rpc_client)) = (price_source, rpc_client) {
                    Ok(calculate_token_value_in_lamports(
                        *amount,
                        &Pubkey::from_str(token).unwrap(),
                        price_source,
                        rpc_client,
                    )
                    .await?)
                } else {
                    Ok(*amount)
                }
            }
            PriceModel::Free => Ok(0),
        }
    }
}

pub fn is_fee_payer_in_signers(
    transaction: &impl VersionedTransactionExt,
) -> Result<bool, KoraError> {
    let fee_payer = get_signer()
        .map(|signer| signer.solana_pubkey())
        .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer: {e}")))?;

    let all_account_keys = transaction.get_all_account_keys();
    let transaction_inner = transaction.get_transaction();

    // In messages, the first num_required_signatures accounts are signers
    Ok(match &transaction_inner.message {
        VersionedMessage::Legacy(legacy_message) => {
            let num_signers = legacy_message.header.num_required_signatures as usize;
            all_account_keys.iter().take(num_signers).any(|key| *key == fee_payer)
        }
        VersionedMessage::V0(v0_message) => {
            let num_signers = v0_message.header.num_required_signatures as usize;
            all_account_keys.iter().take(num_signers).any(|key| *key == fee_payer)
        }
    })
}

pub async fn estimate_transaction_fee(
    rpc_client: &RpcClient,
    // Should have resolved addresses for lookup tables
    resolved_transaction: &impl VersionedTransactionExt,
) -> Result<u64, KoraError> {
    let transaction = resolved_transaction.get_transaction();

    // Get base transaction fee
    let base_fee = get_estimate_fee(rpc_client, &transaction.message).await?;

    // Get account creation fees (for ATA creation)
    let account_creation_fee = get_associated_token_account_creation_fees(rpc_client, transaction)
        .await
        .map_err(|e| KoraError::RpcError(e.to_string()))?;

    // Priority fees are now included in the calculate done by the RPC getFeeForMessage

    // If the Kora signer is not inclded in the signers, we add another base fee, since each transaction will be 5000 lamports
    let mut kora_signature_fee = 0u64;
    if !is_fee_payer_in_signers(resolved_transaction)? {
        kora_signature_fee = LAMPORTS_PER_SIGNATURE;
    }

    Ok(base_fee + account_creation_fee + kora_signature_fee)
}

async fn get_associated_token_account_creation_fees(
    rpc_client: &RpcClient,
    transaction: &VersionedTransaction,
) -> Result<u64, KoraError> {
    const ATA_ACCOUNT_SIZE: usize = 165; // Standard ATA size
    let mut ata_count = 0u64;

    // Check each instruction in the transaction for ATA creation
    for instruction in transaction.message.instructions() {
        let account_keys = transaction.message.static_account_keys();
        let program_id = account_keys[instruction.program_id_index as usize];

        // Skip if not an ATA program instruction
        if program_id != spl_associated_token_account::id() {
            continue;
        }

        let ata = account_keys[instruction.accounts[1] as usize];
        let owner = account_keys[instruction.accounts[2] as usize];
        let mint = account_keys[instruction.accounts[3] as usize];

        let expected_ata = get_associated_token_address(&owner, &mint);

        if ata == expected_ata && rpc_client.get_account(&ata).await.is_err() {
            ata_count += 1;
        }
    }

    // Get rent cost in lamports for ATA creation
    let rent = Rent::default();
    let exempt_min = rent.minimum_balance(ATA_ACCOUNT_SIZE);

    Ok(exempt_min * ata_count)
}

pub async fn calculate_token_value_in_lamports(
    amount: u64,
    mint: &Pubkey,
    price_source: PriceSource,
    rpc_client: &RpcClient,
) -> Result<u64, KoraError> {
    // Fetch mint account data to determine token decimals
    let mint_account =
        rpc_client.get_account(mint).await.map_err(|e| KoraError::RpcError(e.to_string()))?;

    let token_program = TokenProgram::new(TokenType::Spl);
    let decimals = token_program.get_mint_decimals(&mint_account.data)?;

    // Initialize price oracle with retries for reliability
    let oracle =
        RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source));

    // Get token price in SOL directly
    let token_price = oracle
        .get_token_price(&mint.to_string())
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {e}")))?;

    // Convert token amount to its real value based on decimals and multiply by SOL price
    let token_amount = amount as f64 / 10f64.powi(decimals as i32);
    let sol_amount = token_amount * token_price.price;

    // Convert SOL to lamports and round down
    let lamports = (sol_amount * LAMPORTS_PER_SOL as f64).floor() as u64;

    Ok(lamports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        signer::{KoraSigner, SolanaMemorySigner},
        state::init_signer,
        transaction::{new_unsigned_versioned_transaction, VersionedTransactionResolved},
    };
    use base64::Engine;
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use solana_message::{v0, Message};
    use solana_program::program_pack::Pack;
    use solana_sdk::{
        account::Account,
        hash::Hash,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
    };
    use solana_system_interface::instruction::transfer;
    use spl_token::state::Mint;
    use std::{collections::HashMap, sync::Arc};

    fn get_mock_rpc_client_with_mint(mint_decimals: u8) -> Arc<RpcClient> {
        let mut mocks = HashMap::new();

        // Create a mock mint account
        let mut mint_data = vec![0u8; Mint::LEN];
        let mint = Mint {
            mint_authority: Some(Pubkey::new_unique()).into(),
            supply: 1000000u64.into(),
            decimals: mint_decimals,
            is_initialized: true,
            freeze_authority: None.into(),
        };
        Mint::pack(mint, &mut mint_data).unwrap();

        let mint_account = Account {
            lamports: 1000000,
            data: mint_data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: 0,
        };

        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&mint_account.data);
        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": {
                    "slot": 1
                },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": mint_account.executable,
                    "lamports": mint_account.lamports,
                    "owner": mint_account.owner.to_string(),
                    "rentEpoch": mint_account.rent_epoch
                }
            }),
        );

        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    #[tokio::test]
    async fn test_margin_model_get_required_lamports() {
        // Test margin of 0.1 (10%)
        let price_config = PriceConfig { model: PriceModel::Margin { margin: 0.1 } };

        let min_transaction_fee = 5000u64; // 5000 lamports base fee
        let expected_lamports = (5000.0 * 1.1) as u64; // 5500 lamports

        let result =
            price_config.get_required_lamports(None, None, min_transaction_fee).await.unwrap();

        assert_eq!(result, expected_lamports);
    }

    #[tokio::test]
    async fn test_margin_model_get_required_lamports_zero_margin() {
        // Test margin of 0.0 (no margin)
        let price_config = PriceConfig { model: PriceModel::Margin { margin: 0.0 } };

        let min_transaction_fee = 5000u64;

        let result =
            price_config.get_required_lamports(None, None, min_transaction_fee).await.unwrap();

        assert_eq!(result, min_transaction_fee);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_with_oracle() {
        let rpc_client = get_mock_rpc_client_with_mint(6); // USDC has 6 decimals

        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 1_000_000, // 1 USDC (1,000,000 base units with 6 decimals)
                token: usdc_mint.to_string(),
            },
        };

        // Use Mock price source which returns 0.0001 SOL per USDC
        let price_source = Some(PriceSource::Mock);
        let min_transaction_fee = 5000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), price_source, min_transaction_fee)
            .await
            .unwrap();

        // Expected calculation:
        // 1,000,000 base units / 10^6 = 1.0 USDC
        // 1.0 USDC * 0.0001 SOL/USDC = 0.0001 SOL
        // 0.0001 SOL * 1,000,000,000 lamports/SOL = 100,000 lamports
        assert_eq!(result, 100000);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_with_custom_price() {
        let rpc_client = get_mock_rpc_client_with_mint(9); // 9 decimals token

        let custom_token = "So11111111111111111111111111111111111111112"; // SOL mint
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 500000000, // 0.5 tokens (500,000,000 base units with 9 decimals)
                token: custom_token.to_string(),
            },
        };

        // Mock oracle returns 1.0 SOL price for SOL mint
        let price_source = Some(PriceSource::Mock);
        let min_transaction_fee = 5000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), price_source, min_transaction_fee)
            .await
            .unwrap();

        // Expected calculation:
        // 500,000,000 base units / 10^9 = 0.5 tokens
        // 0.5 tokens * 1.0 SOL/token = 0.5 SOL
        // 0.5 SOL * 1,000,000,000 lamports/SOL = 500,000,000 lamports
        assert_eq!(result, 500000000);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_without_oracle() {
        let rpc_client = get_mock_rpc_client_with_mint(6);

        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 25000,
                token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
            },
        };

        // No price source provided - should return amount directly
        let price_source = None;
        let min_transaction_fee = 5000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), price_source, min_transaction_fee)
            .await
            .unwrap();

        assert_eq!(result, 25000);
    }

    #[tokio::test]
    async fn test_fixed_model_get_required_lamports_small_amount() {
        let rpc_client = get_mock_rpc_client_with_mint(6); // USDC has 6 decimals

        let usdc_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
        let price_config = PriceConfig {
            model: PriceModel::Fixed {
                amount: 1000, // 0.001 USDC (1,000 base units with 6 decimals)
                token: usdc_mint.to_string(),
            },
        };

        let price_source = Some(PriceSource::Mock);
        let min_transaction_fee = 5000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), price_source, min_transaction_fee)
            .await
            .unwrap();

        // Expected calculation:
        // 1,000 base units / 10^6 = 0.001 USDC
        // 0.001 USDC * 0.0001 SOL/USDC = 0.0000001 SOL
        // 0.0000001 SOL * 1,000,000,000 lamports/SOL = 100 lamports (rounded down)
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_free_model_get_required_lamports() {
        let rpc_client = get_mock_rpc_client_with_mint(6);

        let price_config = PriceConfig { model: PriceModel::Free };

        let min_transaction_fee = 10000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), Some(PriceSource::Mock), min_transaction_fee)
            .await
            .unwrap();

        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_free_model_get_required_lamports_with_high_base_fee() {
        let rpc_client = get_mock_rpc_client_with_mint(6);

        let price_config = PriceConfig { model: PriceModel::Free };

        let min_transaction_fee = 1000000u64;

        let result = price_config
            .get_required_lamports(Some(&rpc_client), Some(PriceSource::Mock), min_transaction_fee)
            .await
            .unwrap();

        // Free model should always return 0 regardless of base fee
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_default_price_config() {
        // Test that default creates Margin with 0.0 margin
        let default_config = PriceConfig::default();

        match default_config.model {
            PriceModel::Margin { margin } => assert_eq!(margin, 0.0),
            _ => panic!("Default should be Margin with 0.0 margin"),
        }
    }

    fn setup_or_get_test_signer() -> Pubkey {
        if let Ok(signer) = get_signer() {
            return signer.solana_pubkey();
        }

        let test_keypair = Keypair::new();
        let signer = SolanaMemorySigner::new(test_keypair.insecure_clone());
        match init_signer(KoraSigner::Memory(signer)) {
            Ok(_) => test_keypair.pubkey(),
            Err(_) => {
                // Signer already initialized, get it
                get_signer().expect("Signer should be available").solana_pubkey()
            }
        }
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_not_signer() {
        setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&sender.pubkey(), &recipient.pubkey(), 1000);

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(!is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &fee_payer,
            &[transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_v0_fee_payer_not_signer() {
        setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let v0_message = v0::Message::try_compile(
            &sender.pubkey(),
            &[transfer(&sender.pubkey(), &recipient.pubkey(), 1000)],
            &[],
            Hash::default(),
        )
        .expect("Failed to compile V0 message");

        let message = VersionedMessage::V0(v0_message);
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(!is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }
}
