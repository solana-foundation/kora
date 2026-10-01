use super::ConfigValidator;
use crate::{
    admin::token_util::find_missing_atas,
    config::Config,
    validator::{
        account_validator::{validate_account, AccountType},
        cross_cluster::check_cross_cluster_mints,
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{BaseStateWithExtensions, StateWithExtensions},
    state::Mint as Token2022MintState,
    ID as TOKEN_2022_PROGRAM_ID,
};
use std::str::FromStr;

impl ConfigValidator {
    /// Check Token2022 mints for risky extensions (PermanentDelegate, TransferHook)
    pub(super) async fn check_token_mint_extensions(
        rpc_client: &RpcClient,
        allowed_tokens: &[String],
        warnings: &mut Vec<String>,
    ) {
        for token_str in allowed_tokens {
            let token_pubkey = match Pubkey::from_str(token_str) {
                Ok(pk) => pk,
                Err(_) => continue,
            };

            let account: Account = match rpc_client.get_account(&token_pubkey).await {
                Ok(acc) => acc,
                Err(_) => continue,
            };

            if account.owner != TOKEN_2022_PROGRAM_ID {
                continue;
            }

            let mint_with_extensions =
                match StateWithExtensions::<Token2022MintState>::unpack(&account.data) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

            if mint_with_extensions
                .get_extension::<spl_token_2022_interface::extension::permanent_delegate::PermanentDelegate>()
                .is_ok()
            {
                warnings.push(format!(
                    "⚠️  SECURITY: Token {} has PermanentDelegate extension. \
                    Risk: The permanent delegate can transfer or burn tokens at any time without owner approval. \
                    This creates significant risks for payment tokens as funds can be seized after payment. \
                    Consider removing this token from allowed_tokens or blocking the extension in [validation.token_2022].",
                    token_str
                ));
            }

            if mint_with_extensions
                .get_extension::<spl_token_2022_interface::extension::transfer_hook::TransferHook>()
                .is_ok()
            {
                warnings.push(format!(
                    "⚠️  SECURITY: Token {} has TransferHook extension. \
                    Risk: A custom program executes on every transfer which can reject transfers  \
                    or introduce external dependencies and attack surface. \
                    Consider removing this token from allowed_tokens or blocking the extension in [validation.token_2022].",
                    token_str
                ));
            }
        }
    }

    pub(super) async fn check_onchain(
        config: &Config,
        rpc_client: &RpcClient,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        let accounts = [
            ("Program", config.validation.allowed_programs.as_slice(), AccountType::Program),
            ("Token", config.validation.allowed_tokens.as_slice(), AccountType::Mint),
            (
                "SPL paid token",
                config.validation.allowed_spl_paid_tokens.as_slice(),
                AccountType::Mint,
            ),
        ];
        for (label, pubkeys, account_type) in accounts {
            for pubkey_str in pubkeys {
                if let Ok(pubkey) = Pubkey::from_str(pubkey_str) {
                    if let Err(e) =
                        validate_account(config, rpc_client, &pubkey, Some(account_type.clone()))
                            .await
                    {
                        errors.push(format!("{label} {pubkey_str} validation failed: {e}"));
                    }
                }
            }
        }

        Self::check_token_mint_extensions(rpc_client, &config.validation.allowed_tokens, warnings)
            .await;

        if let Some(payment_address) = &config.kora.payment_address {
            if let Ok(payment_address) = Pubkey::from_str(payment_address) {
                match find_missing_atas(config, rpc_client, &payment_address).await {
                    Ok(atas_to_create) => {
                        if !atas_to_create.is_empty() {
                            errors.push(format!(
                                "Missing ATAs for payment address: {payment_address}"
                            ));
                        }
                    }
                    Err(e) => errors.push(format!("Failed to find missing ATAs: {e}")),
                }
            } else {
                errors.push(format!("Invalid payment address: {payment_address}"));
            }
        }
    }

    pub(super) async fn check_cross_cluster(
        config: &Config,
        rpc_client: &RpcClient,
        warnings: &mut Vec<String>,
    ) {
        let mut all_tokens: Vec<String> = config
            .validation
            .allowed_tokens
            .iter()
            .chain(config.validation.allowed_spl_paid_tokens.iter())
            .cloned()
            .collect();
        all_tokens.sort_unstable();
        all_tokens.dedup();
        check_cross_cluster_mints(
            rpc_client,
            &all_tokens,
            &config.validation.cross_cluster_endpoints,
            warnings,
        )
        .await;
    }
}
