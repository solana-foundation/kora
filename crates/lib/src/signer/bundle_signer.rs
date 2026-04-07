use crate::{
    config::Config,
    sanitize_error, state,
    transaction::{sign_with_retry, VersionedTransactionOps, VersionedTransactionResolved},
    KoraError,
};
use solana_keychain::SolanaSigner;
use solana_sdk::pubkey::Pubkey;
use std::{sync::Arc, time::Duration};

pub struct BundleSigner {}

impl BundleSigner {
    pub async fn sign_transaction_for_bundle(
        resolved: &mut VersionedTransactionResolved,
        signer: &Arc<solana_keychain::Signer>,
        fee_payer: &Pubkey,
        blockhash: &Option<solana_sdk::hash::Hash>,
        config: &Config,
    ) -> Result<(), KoraError> {
        if resolved.transaction.signatures.is_empty() {
            if let Some(blockhash) = blockhash {
                resolved.transaction.message.set_recent_blockhash(*blockhash);
            }
        }

        let message_bytes = resolved.transaction.message.serialize();
        let sign_timeout = Duration::from_secs(config.kora.sign_timeout_seconds);
        let max_retries = config.kora.sign_max_retries;
        let signature = match sign_with_retry(
            sign_timeout,
            max_retries,
            "bundle signing",
            "Bundle signing",
            || async {
                signer
                    .sign_message(&message_bytes)
                    .await
                    .map_err(|e| KoraError::SigningError(sanitize_error!(e)))
            },
        )
        .await
        {
            Ok(sig) => {
                match state::get_signer_pool() {
                    Ok(pool) => pool.record_signing_success(signer),
                    Err(e) => log::warn!(
                        "Could not record bundle signing success to pool: {}",
                        sanitize_error!(e)
                    ),
                }
                sig
            }
            Err(err) => {
                match state::get_signer_pool() {
                    Ok(pool) => pool.record_signing_failure(signer),
                    Err(pool_err) => log::error!(
                        "Bundle signing failed AND pool health tracking unavailable: {}; \
                         signer failure will not be recorded, automatic failover is disabled",
                        sanitize_error!(pool_err)
                    ),
                }
                return Err(err);
            }
        };

        let fee_payer_position = resolved.find_signer_position(fee_payer)?;
        let signatures_len = resolved.transaction.signatures.len();
        let signature_slot = match resolved.transaction.signatures.get_mut(fee_payer_position) {
            Some(slot) => slot,
            None => {
                return Err(KoraError::InvalidTransaction(format!(
                    "Signer position {fee_payer_position} is out of bounds for signatures (len={signatures_len})"
                )));
            }
        };
        *signature_slot = signature;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::config_mock::ConfigMockBuilder;
    use solana_keychain::Signer;
    use solana_message::Message;
    use solana_sdk::{
        hash::Hash, signature::Keypair, signer::Signer as SdkSigner, transaction::Transaction,
    };
    use solana_system_interface::instruction::transfer;

    fn create_test_resolved_with_fee_payer(fee_payer: &Keypair) -> VersionedTransactionResolved {
        let instruction = transfer(&fee_payer.pubkey(), &Pubkey::new_unique(), 1000);
        let message = Message::new(&[instruction], Some(&fee_payer.pubkey()));
        let transaction = Transaction::new_unsigned(message);
        let versioned = solana_sdk::transaction::VersionedTransaction::from(transaction);

        VersionedTransactionResolved::from_kora_built_transaction(&versioned).unwrap()
    }

    fn create_test_resolved_with_unsigned_fee_payer_occurrence(
        signer_like_fee_payer: &Pubkey,
    ) -> VersionedTransactionResolved {
        let attacker_fee_payer = Keypair::new();
        let instruction = transfer(&attacker_fee_payer.pubkey(), signer_like_fee_payer, 1000);
        let message = Message::new(&[instruction], Some(&attacker_fee_payer.pubkey()));
        let transaction = Transaction::new_unsigned(message);
        let versioned = solana_sdk::transaction::VersionedTransaction::from(transaction);

        VersionedTransactionResolved::from_kora_built_transaction(&versioned).unwrap()
    }

    #[tokio::test]
    async fn test_sign_transaction_for_bundle_success() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();

        let mut resolved = create_test_resolved_with_fee_payer(&fee_payer_keypair);

        let result = BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &fee_payer,
            &blockhash,
            &config,
        )
        .await;

        assert!(result.is_ok());
        assert!(!resolved.transaction.signatures[0].to_string().is_empty());
    }

    #[tokio::test]
    async fn test_sign_transaction_for_bundle_invalid_fee_payer() {
        let fee_payer_keypair = Keypair::new();
        let wrong_fee_payer = Pubkey::new_unique();

        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();

        let mut resolved = create_test_resolved_with_fee_payer(&fee_payer_keypair);

        let result = BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &wrong_fee_payer,
            &blockhash,
            &config,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn test_sign_transaction_for_bundle_rejects_unsigned_fee_payer_occurrence() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();

        let mut resolved = create_test_resolved_with_unsigned_fee_payer_occurrence(&fee_payer);
        let result = BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &fee_payer,
            &blockhash,
            &config,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn test_sign_transaction_for_bundle_signature_position() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();

        let mut resolved = create_test_resolved_with_fee_payer(&fee_payer_keypair);

        // Get original default signature
        let original_sig = resolved.transaction.signatures[0];

        let result = BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &fee_payer,
            &blockhash,
            &config,
        )
        .await;

        assert!(result.is_ok());
        // Signature should be different from original default
        assert_ne!(resolved.transaction.signatures[0], original_sig);
    }

    #[tokio::test]
    async fn test_sign_transaction_for_bundle_verifies_signature() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();

        let mut resolved = create_test_resolved_with_fee_payer(&fee_payer_keypair);

        let result = BundleSigner::sign_transaction_for_bundle(
            &mut resolved,
            &signer,
            &fee_payer,
            &blockhash,
            &config,
        )
        .await;

        assert!(result.is_ok());

        // Verify the signature is cryptographically valid
        let signature = &resolved.transaction.signatures[0];
        let message_bytes = resolved.transaction.message.serialize();
        assert!(
            signature.verify(fee_payer.as_ref(), &message_bytes),
            "Signature should be valid for the transaction message"
        );
    }
}
