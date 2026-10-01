use crate::{
    config::Config,
    transaction::{
        set_signature_at, sign_with_signer_pool, VersionedTransactionOps,
        VersionedTransactionResolved,
    },
    KoraError,
};
use solana_keychain::SolanaSigner;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

pub struct BundleSigner {}

impl BundleSigner {
    pub async fn sign_transaction_for_bundle(
        resolved: &mut VersionedTransactionResolved,
        selected_signer: &Arc<solana_keychain::Signer>,
        fee_payer: &Pubkey,
        blockhash: &Option<solana_sdk::hash::Hash>,
        config: &Config,
    ) -> Result<(), KoraError> {
        if resolved.transaction.signatures.is_empty() {
            if let Some(blockhash) = blockhash {
                resolved.transaction.message.set_recent_blockhash(*blockhash);
            }
        }

        let signature = sign_with_signer_pool(
            config,
            &selected_signer.pubkey(),
            &resolved.transaction.message.serialize(),
            "bundle signing",
            "Bundle signing",
        )
        .await?;

        let fee_payer_position = resolved.find_signer_position(fee_payer)?;
        set_signature_at(&mut resolved.transaction, fee_payer_position, signature)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        signer::{pool::SignerWithMetadata, SignerPool},
        state::{update_config, update_signer_pool},
        tests::config_mock::ConfigMockBuilder,
    };
    use serial_test::serial;
    use solana_keychain::Signer;
    use solana_message::Message;
    use solana_sdk::{
        hash::Hash, signature::Keypair, signer::Signer as SdkSigner, transaction::Transaction,
    };
    use solana_system_interface::instruction::transfer;

    fn setup_bundle_signer_state(
        fee_payer_keypair: &Keypair,
        config: &Config,
    ) -> Arc<solana_keychain::Signer> {
        let external_signer = Signer::from_memory(&fee_payer_keypair.to_base58_string()).unwrap();
        let signer = Arc::new(external_signer);
        let pool = SignerPool::new(vec![SignerWithMetadata::new(
            "bundle_test_signer".to_string(),
            Arc::clone(&signer),
            1,
        )]);

        update_config(config.clone()).unwrap();
        update_signer_pool(pool).unwrap();

        signer
    }

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
    #[serial]
    async fn test_sign_transaction_for_bundle_success() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();
        let signer = setup_bundle_signer_state(&fee_payer_keypair, &config);

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
    #[serial]
    async fn test_sign_transaction_for_bundle_invalid_fee_payer() {
        let fee_payer_keypair = Keypair::new();
        let wrong_fee_payer = Pubkey::new_unique();

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();
        let signer = setup_bundle_signer_state(&fee_payer_keypair, &config);

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
    #[serial]
    async fn test_sign_transaction_for_bundle_rejects_unsigned_fee_payer_occurrence() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();
        let signer = setup_bundle_signer_state(&fee_payer_keypair, &config);

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
    #[serial]
    async fn test_sign_transaction_for_bundle_signature_position() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();
        let signer = setup_bundle_signer_state(&fee_payer_keypair, &config);

        let mut resolved = create_test_resolved_with_fee_payer(&fee_payer_keypair);

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
        assert_ne!(resolved.transaction.signatures[0], original_sig);
    }

    #[tokio::test]
    #[serial]
    async fn test_sign_transaction_for_bundle_verifies_signature() {
        let fee_payer_keypair = Keypair::new();
        let fee_payer = fee_payer_keypair.pubkey();

        let blockhash = Some(Hash::new_unique());
        let config = ConfigMockBuilder::new().build();
        let signer = setup_bundle_signer_state(&fee_payer_keypair, &config);

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

        let signature = &resolved.transaction.signatures[0];
        let message_bytes = resolved.transaction.message.serialize();
        assert!(
            signature.verify(fee_payer.as_ref(), &message_bytes),
            "Signature should be valid for the transaction message"
        );
    }
}
