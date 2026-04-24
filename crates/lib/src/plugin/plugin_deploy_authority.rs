use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    config::Config,
    constant::LOADER_V4_PROGRAM_ID,
    error::KoraError,
    transaction::{ParsedLoaderV4InstructionData, VersionedTransactionResolved},
};

use super::{PluginExecutionContext, TransactionPlugin};

/// Enforces that the fee payer is the authority on every loader-v4 instruction we sign.
///
/// Stacks on top of [`LoaderV4InstructionPolicy`](crate::config::LoaderV4InstructionPolicy):
/// the core fee-payer policy gates whether Kora is *willing* to participate as authority;
/// this plugin requires Kora to *be* the authority. The plugin is inert unless an operator
/// also flipped the matching `allow_*` flags.
///
/// Use case: a devnet paymaster that sponsors program deploys and needs to keep control of
/// every deployed program.
pub(super) struct DeployAuthorityPlugin;

#[async_trait]
impl TransactionPlugin for DeployAuthorityPlugin {
    async fn validate(
        &self,
        transaction: &mut VersionedTransactionResolved,
        _config: &Config,
        _rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let loader_v4 = transaction.get_or_parse_loader_v4_instructions()?;

        for data in loader_v4.values().flatten() {
            match data {
                ParsedLoaderV4InstructionData::Write { authority, .. }
                | ParsedLoaderV4InstructionData::Copy { authority, .. }
                | ParsedLoaderV4InstructionData::SetProgramLength { authority, .. }
                | ParsedLoaderV4InstructionData::Deploy { authority, .. }
                | ParsedLoaderV4InstructionData::Retract { authority, .. } => {
                    if authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: loader-v4 authority must be the fee payer \
                             ({fee_payer}), got {authority} in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedLoaderV4InstructionData::TransferAuthority { new_authority, .. } => {
                    if new_authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: TransferAuthority must keep authority with \
                             the fee payer ({fee_payer}), got new_authority={new_authority} in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedLoaderV4InstructionData::Finalize { .. } => {
                    return Err(KoraError::InvalidTransaction(format!(
                        "DeployAuthority plugin: Finalize is not allowed; programs must stay \
                         upgradable so the fee payer can reclaim rent (context: {})",
                        context.method_name()
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_config(&self, config: &Config) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Without LoaderV4 in allowed_programs, every loader-v4 instruction is rejected by
        // the core validator before the plugin ever runs — the plugin is completely inert.
        if !config.validation.allowed_programs.contains(&LOADER_V4_PROGRAM_ID.to_string()) {
            errors.push(format!(
                "DeployAuthority plugin requires LoaderV4 program ({LOADER_V4_PROGRAM_ID}) \
                 in allowed_programs"
            ));
        }

        let loader_v4 = &config.validation.fee_payer_policy.loader_v4;
        let any_allow_set = loader_v4.allow_write
            || loader_v4.allow_copy
            || loader_v4.allow_set_program_length
            || loader_v4.allow_deploy
            || loader_v4.allow_retract
            || loader_v4.allow_transfer_authority;
        if !any_allow_set {
            warnings.push(
                "DeployAuthority plugin is enabled but every fee_payer_policy.loader_v4.allow_* \
                 flag is false, so the plugin will never see a loader-v4 instruction. Enable the \
                 relevant allow_* flags (e.g. allow_write, allow_deploy) if you intend to sponsor \
                 program deploys."
                    .to_string(),
            );
        }

        (errors, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::TransactionPluginRunner, *};
    use crate::{
        config::TransactionPluginType,
        constant::LOADER_V4_PROGRAM_ID,
        tests::{common::RpcMockBuilder, config_mock::ConfigMockBuilder},
        transaction::TransactionUtil,
    };
    use solana_loader_v4_interface::instruction as loader_v4;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::pubkey::Pubkey;
    use solana_system_interface::instruction::transfer as system_transfer;
    use std::sync::Arc;

    fn enable_deploy_authority_plugin(config: &mut Config) {
        config.kora.plugins.enabled = vec![TransactionPluginType::DeployAuthority];
    }

    fn build_runner() -> (Config, Arc<RpcClient>) {
        let mut config = ConfigMockBuilder::new()
            .with_allowed_programs(vec![LOADER_V4_PROGRAM_ID.to_string()])
            .build();
        enable_deploy_authority_plugin(&mut config);
        let rpc_client = RpcMockBuilder::new().build();
        (config, rpc_client)
    }

    async fn run_plugin(
        config: &Config,
        rpc_client: &Arc<RpcClient>,
        fee_payer: &Pubkey,
        ix: solana_sdk::instruction::Instruction,
    ) -> Result<(), KoraError> {
        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[ix], Some(fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();
        let runner = TransactionPluginRunner::from_config(config);
        runner
            .run(
                &mut resolved,
                config,
                rpc_client.as_ref(),
                fee_payer,
                PluginExecutionContext::SignTransaction,
            )
            .await
    }

    #[tokio::test]
    async fn accepts_write_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::write(&program, &fee_payer, 0, vec![1, 2, 3]);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_write_when_user_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::write(&program, &user, 0, vec![1, 2, 3]);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("DeployAuthority")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn accepts_copy_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let ix = loader_v4::copy(&dest, &fee_payer, &source, 0, 0, 64);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_copy_when_user_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let ix = loader_v4::copy(&dest, &user, &source, 0, 0, 64);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn accepts_set_program_length_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::set_program_length(&program, &fee_payer, 1024, &fee_payer);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_set_program_length_when_user_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::set_program_length(&program, &user, 1024, &user);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn accepts_deploy_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::deploy(&program, &fee_payer);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_deploy_when_user_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::deploy(&program, &user);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn accepts_retract_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::retract(&program, &fee_payer);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_retract_when_user_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::retract(&program, &user);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn accepts_transfer_authority_when_new_is_kora() {
        // Attacker hands authority to Kora via TransferAuthority — plugin accepts because the
        // authority lands back with the fee payer. Note the core fee-payer policy would reject
        // this as a drainage vector (see #449); the plugin is additive, not a replacement.
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let old_auth = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::transfer_authority(&program, &old_auth, &fee_payer);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_transfer_authority_to_user() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v4::transfer_authority(&program, &fee_payer, &user);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("TransferAuthority")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn rejects_finalize_even_when_kora_is_authority() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let next_version = Pubkey::new_unique();
        let ix = loader_v4::finalize(&program, &fee_payer, &next_version);

        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("Finalize")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn mixed_tx_rejects_when_loader_v4_authority_wrong() {
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        let write_ix = loader_v4::write(&program, &user, 0, vec![1]);
        let sol_ix = system_transfer(&fee_payer, &recipient, 1);

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[sol_ix, write_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();
        let runner = TransactionPluginRunner::from_config(&config);
        let err = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignAndSendTransaction,
            )
            .await
            .expect_err("must reject");
        assert!(matches!(err, KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn no_loader_v4_instructions_is_noop() {
        // Plugin should not interfere with non-loader-v4 transactions.
        let (config, rpc_client) = build_runner();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let ix = system_transfer(&fee_payer, &recipient, 1);

        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    fn config_with_loader_v4_allowed() -> Config {
        let mut config = ConfigMockBuilder::new()
            .with_allowed_programs(vec![LOADER_V4_PROGRAM_ID.to_string()])
            .build();
        enable_deploy_authority_plugin(&mut config);
        config
    }

    #[test]
    fn validate_config_errors_when_loader_v4_not_in_allowed_programs() {
        let plugin = DeployAuthorityPlugin;
        let mut config = ConfigMockBuilder::new().build();
        enable_deploy_authority_plugin(&mut config);
        config.validation.fee_payer_policy.loader_v4.allow_write = true;

        let (errors, _warnings) = plugin.validate_config(&config);
        assert!(
            errors.iter().any(|e| e.contains("LoaderV4") && e.contains("allowed_programs")),
            "got errors: {errors:?}"
        );
    }

    #[test]
    fn validate_config_warns_when_all_allow_flags_false() {
        let plugin = DeployAuthorityPlugin;
        let config = config_with_loader_v4_allowed();

        let (errors, warnings) = plugin.validate_config(&config);
        assert!(errors.is_empty(), "got errors: {errors:?}");
        assert!(warnings.iter().any(|w| w.contains("allow_*")), "got warnings: {warnings:?}");
    }

    #[test]
    fn validate_config_no_warning_when_any_allow_flag_true() {
        let plugin = DeployAuthorityPlugin;
        let mut config = config_with_loader_v4_allowed();
        config.validation.fee_payer_policy.loader_v4.allow_write = true;

        let (errors, warnings) = plugin.validate_config(&config);
        assert!(errors.is_empty(), "got errors: {errors:?}");
        assert!(warnings.is_empty(), "expected no warnings, got: {warnings:?}");
    }
}
