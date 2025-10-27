use crate::{
    config::{EnabledMethods, ValidationConfig},
    signer::SelectionStrategy,
    state::{self, get_signer_pool},
    KoraError,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SignerPoolInfo {
    pub strategy: SelectionStrategy,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetConfigResponse {
    pub fee_payers: Vec<String>,
    pub validation_config: ValidationConfig,
    pub enabled_methods: EnabledMethods,
}

pub async fn get_config() -> Result<GetConfigResponse, KoraError> {
    let config = state::get_config()?;

    // Get signer pool information (required in multi-signer mode)
    let pool = get_signer_pool()
        .map_err(|e| KoraError::InternalServerError(format!("Signer pool not initialized: {e}")))?;

    // Get all fee payer public keys from the signer pool
    let fee_payers: Vec<String> =
        pool.get_signers_info().iter().map(|signer| signer.public_key.clone()).collect();

    Ok(GetConfigResponse {
        fee_payers,
        validation_config: config.validation.clone(),
        enabled_methods: config.kora.enabled_methods.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{common::setup_or_get_test_signer, config_mock::ConfigMockBuilder};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_get_config_success() {
        let config = ConfigMockBuilder::new().build();
        state::update_config(config).expect("Failed to update config");

        let _ = setup_or_get_test_signer();

        let result = get_config().await;

        let response = result.unwrap();

        // Assert fee payers
        assert!(!response.fee_payers.is_empty(), "Should have at least one fee payer");
        assert!(!response.fee_payers[0].is_empty(), "Fee payer pubkey should not be empty");

        // Assert ValidationConfig defaults
        assert_eq!(response.validation_config.max_allowed_lamports, 1_000_000_000);
        assert_eq!(response.validation_config.max_signatures, 10);
        assert_eq!(response.validation_config.allowed_programs.len(), 3);
        assert_eq!(
            response.validation_config.allowed_programs[0],
            "11111111111111111111111111111111"
        ); // System Program
        assert_eq!(
            response.validation_config.allowed_programs[1],
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        ); // Token Program
        assert_eq!(
            response.validation_config.allowed_programs[2],
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        ); // ATA Program
        assert_eq!(response.validation_config.allowed_tokens.len(), 1);
        assert_eq!(
            response.validation_config.allowed_tokens[0],
            "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
        ); // USDC devnet
        assert_eq!(response.validation_config.allowed_spl_paid_tokens.as_slice().len(), 1);
        assert_eq!(
            response.validation_config.allowed_spl_paid_tokens.as_slice()[0],
            "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
        ); // USDC devnet
        assert_eq!(response.validation_config.disallowed_accounts.len(), 0);
        assert_eq!(response.validation_config.price_source, crate::oracle::PriceSource::Mock);

        // Assert FeePayerPolicy defaults - System (secure by default - all false)
        assert!(!response.validation_config.fee_payer_policy.system.allow_transfer);
        assert!(!response.validation_config.fee_payer_policy.system.allow_assign);
        assert!(!response.validation_config.fee_payer_policy.system.allow_create_account);
        assert!(!response.validation_config.fee_payer_policy.system.allow_allocate);
        assert!(!response.validation_config.fee_payer_policy.system.nonce.allow_initialize);
        assert!(!response.validation_config.fee_payer_policy.system.nonce.allow_advance);
        assert!(!response.validation_config.fee_payer_policy.system.nonce.allow_withdraw);
        assert!(!response.validation_config.fee_payer_policy.system.nonce.allow_authorize);
        // Note: allow_upgrade removed - no authority parameter to validate

        // Assert FeePayerPolicy defaults - SPL Token (secure by default - all false)
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_transfer);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_burn);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_close_account);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_approve);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_revoke);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_set_authority);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_mint_to);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_freeze_account);
        assert!(!response.validation_config.fee_payer_policy.spl_token.allow_thaw_account);

        // Assert FeePayerPolicy defaults - Token2022 (secure by default - all false)
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_transfer);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_burn);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_close_account);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_approve);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_revoke);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_set_authority);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_mint_to);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_freeze_account);
        assert!(!response.validation_config.fee_payer_policy.token_2022.allow_thaw_account);
        // Assert PriceConfig default (check margin value)
        match response.validation_config.price.model {
            crate::fee::price::PriceModel::Margin { margin } => assert_eq!(margin, 0.0),
            _ => panic!("Expected Margin price model"),
        }

        // Assert Token2022Config defaults (only public fields)
        assert_eq!(response.validation_config.token_2022.blocked_mint_extensions.len(), 0);
        assert_eq!(response.validation_config.token_2022.blocked_account_extensions.len(), 0);

        // Assert EnabledMethods defaults
        assert!(response.enabled_methods.liveness);
        assert!(response.enabled_methods.estimate_transaction_fee);
        assert!(response.enabled_methods.get_supported_tokens);
        assert!(response.enabled_methods.get_payer_signer);
        assert!(response.enabled_methods.sign_transaction);
        assert!(response.enabled_methods.sign_and_send_transaction);
        assert!(response.enabled_methods.transfer_transaction);
        assert!(response.enabled_methods.get_blockhash);
        assert!(response.enabled_methods.get_config);
        assert!(response.enabled_methods.sign_transaction_if_paid);
    }
}
