use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    config::Config,
    constant::{BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LOADER_V4_PROGRAM_ID},
    error::KoraError,
    transaction::{
        ParsedBpfLoaderUpgradeableInstructionData, ParsedLoaderV4InstructionData,
        VersionedTransactionResolved,
    },
};

use super::{PluginExecutionContext, TransactionPlugin};

/// Enforces that the fee payer is the authority on every program-loader instruction we sign,
/// covering both BPF Loader Upgradeable (loader-v3) and Loader-v4. The core fee-payer policies
/// gate whether Kora is *willing* to participate as authority; this plugin requires Kora to
/// *be* the authority and rejects any tx that would hand control elsewhere.
///
/// The plugin is inert for whichever loader has all `allow_*` flags off — operators flip the
/// flags for the loader(s) they want to subsidize.
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
        // ---- Loader-v4 ----
        let loader_v4 = transaction.get_or_parse_loader_v4_instructions()?.clone();
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

        // ---- BPF Loader Upgradeable (loader-v3) ----
        let bpf_v3 = transaction.get_or_parse_bpf_loader_upgradeable_instructions()?.clone();
        for data in bpf_v3.values().flatten() {
            match data {
                ParsedBpfLoaderUpgradeableInstructionData::InitializeBuffer {
                    authority, ..
                } => {
                    // Authority is optional. If set, must be fee payer.
                    if let Some(a) = authority {
                        if a != fee_payer {
                            return Err(KoraError::InvalidTransaction(format!(
                                "DeployAuthority plugin: BPF Loader InitializeBuffer authority \
                                 must be the fee payer ({fee_payer}), got {a} in {}",
                                context.method_name()
                            )));
                        }
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::Write { authority, .. } => {
                    if authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: BPF Loader Write authority must be the \
                             fee payer ({fee_payer}), got {authority} in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::DeployWithMaxDataLen {
                    upgrade_authority,
                    ..
                } => {
                    if upgrade_authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: DeployWithMaxDataLen upgrade_authority \
                             must be the fee payer ({fee_payer}), got {upgrade_authority} in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::Upgrade {
                    upgrade_authority, ..
                } => {
                    if upgrade_authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: Upgrade upgrade_authority must be the \
                             fee payer ({fee_payer}), got {upgrade_authority} in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::SetAuthority {
                    new_authority, ..
                } => {
                    let new_is_fee_payer = new_authority.is_some_and(|n| n == *fee_payer);
                    if !new_is_fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: SetAuthority must keep authority with the \
                             fee payer ({fee_payer}); rejecting handoff in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::SetAuthorityChecked {
                    new_authority,
                    ..
                } => {
                    if new_authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: SetAuthorityChecked must keep authority \
                             with the fee payer ({fee_payer}), got new_authority={new_authority} \
                             in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::Close {
                    authority, recipient, ..
                } => {
                    // Plugin-level rule: when Kora is the authority, recipient must be Kora too.
                    // (Same drainage shape as v4 SetProgramLength.)
                    if authority.is_some_and(|a| a == *fee_payer) && recipient != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: BPF Loader Close: when fee payer is the \
                             authority, recipient must be the fee payer (got {recipient}) in {}",
                            context.method_name()
                        )));
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::ExtendProgram { payer, .. } => {
                    if let Some(p) = payer {
                        if p != fee_payer {
                            return Err(KoraError::InvalidTransaction(format!(
                                "DeployAuthority plugin: ExtendProgram payer must be the fee \
                                 payer ({fee_payer}), got {p} in {}",
                                context.method_name()
                            )));
                        }
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::ExtendProgramChecked {
                    authority,
                    payer,
                    ..
                } => {
                    // Authority must always be Kora — it's a required signer.
                    if authority != fee_payer {
                        return Err(KoraError::InvalidTransaction(format!(
                            "DeployAuthority plugin: ExtendProgramChecked authority must be \
                             the fee payer ({fee_payer}), got {authority} in {}",
                            context.method_name()
                        )));
                    }
                    // Payer is optional but if present must also be Kora (drainage vector
                    // otherwise: someone funds extension on their own program with our SOL).
                    if let Some(p) = payer {
                        if p != fee_payer {
                            return Err(KoraError::InvalidTransaction(format!(
                                "DeployAuthority plugin: ExtendProgramChecked payer must be \
                                 the fee payer ({fee_payer}), got {p} in {}",
                                context.method_name()
                            )));
                        }
                    }
                }
                ParsedBpfLoaderUpgradeableInstructionData::Migrate { .. } => {
                    return Err(KoraError::InvalidTransaction(format!(
                        "DeployAuthority plugin: Migrate is not allowed; programs must remain \
                         under loader-v3 / loader-v4 control of the fee payer (context: {})",
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

        // The plugin only enforces what the core validator first lets through. If neither
        // loader is allowlisted, the plugin is completely inert.
        let v3_allowed = config
            .validation
            .allowed_programs
            .contains(&BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string());
        let v4_allowed =
            config.validation.allowed_programs.contains(&LOADER_V4_PROGRAM_ID.to_string());
        if !v3_allowed && !v4_allowed {
            errors.push(format!(
                "DeployAuthority plugin requires at least one of BPF Loader Upgradeable \
                 ({BPF_LOADER_UPGRADEABLE_PROGRAM_ID}) or LoaderV4 ({LOADER_V4_PROGRAM_ID}) \
                 in allowed_programs"
            ));
        }

        let v3 = &config.validation.fee_payer_policy.bpf_loader_upgradeable;
        let v3_any = v3.allow_initialize_buffer
            || v3.allow_write
            || v3.allow_deploy_with_max_data_len
            || v3.allow_upgrade
            || v3.allow_set_authority
            || v3.allow_set_authority_checked
            || v3.allow_close
            || v3.allow_extend_program
            || v3.allow_extend_program_checked
            || v3.allow_migrate;
        let v4 = &config.validation.fee_payer_policy.loader_v4;
        let v4_any = v4.allow_write
            || v4.allow_copy
            || v4.allow_set_program_length
            || v4.allow_deploy
            || v4.allow_retract
            || v4.allow_transfer_authority;
        if !v3_any && !v4_any {
            warnings.push(
                "DeployAuthority plugin is enabled but every loader's fee_payer_policy.allow_* \
                 flags are false, so the plugin will never see a loader instruction. Enable the \
                 relevant allow_* flags under [validation.fee_payer_policy.bpf_loader_upgradeable] \
                 or [validation.fee_payer_policy.loader_v4] if you intend to sponsor program \
                 deploys."
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
        config::{ProgramsConfig, TransactionPluginType},
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

    // ----------------------------------------------------------------------------
    // BPF Loader Upgradeable (loader-v3) coverage
    // ----------------------------------------------------------------------------

    fn build_runner_v3() -> (Config, Arc<RpcClient>) {
        let mut config = ConfigMockBuilder::new()
            .with_allowed_programs(vec![BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string()])
            .build();
        enable_deploy_authority_plugin(&mut config);
        let rpc_client = RpcMockBuilder::new().build();
        (config, rpc_client)
    }

    #[tokio::test]
    async fn v3_accepts_write_when_kora_is_authority() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let ix = loader_v3::write(&buffer, &fee_payer, 0, vec![1, 2, 3]);
        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn v3_rejects_write_with_user_authority() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let ix = loader_v3::write(&buffer, &user, 0, vec![1]);
        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("BPF Loader Write")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn v3_rejects_set_authority_handoff() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let ix = loader_v3::set_buffer_authority(&buffer, &fee_payer, &user);
        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("SetAuthority")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn v3_rejects_close_with_foreign_recipient() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let attacker = Pubkey::new_unique();
        let ix = loader_v3::close(&buffer, &attacker, &fee_payer);
        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("recipient must be the fee payer")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn v3_accepts_close_back_to_kora() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let ix = loader_v3::close(&buffer, &fee_payer, &fee_payer);
        assert!(run_plugin(&config, &rpc_client, &fee_payer, ix).await.is_ok());
    }

    #[tokio::test]
    async fn v3_rejects_extend_program_checked_with_attacker_authority_kora_payer() {
        // Regression: ExtendProgramChecked previously hit a `_ => {}` wildcard in the
        // parser, leaving the plugin's policy unreachable. An attacker who owned a program
        // could submit one with their own pubkey as authority and Kora as payer (index 4),
        // forcing Kora to fund their program data extension.
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let attacker = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let ix = loader_v3::extend_program_checked(&program, &attacker, Some(&fee_payer), 64);
        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("ExtendProgramChecked")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn v3_rejects_migrate() {
        use solana_loader_v3_interface::instruction as loader_v3;
        let (config, rpc_client) = build_runner_v3();
        let fee_payer = Pubkey::new_unique();
        let program = Pubkey::new_unique();
        let programdata = Pubkey::new_unique();
        let ix = loader_v3::migrate_program(&programdata, &program, &fee_payer);
        let err = run_plugin(&config, &rpc_client, &fee_payer, ix).await.expect_err("must reject");
        assert!(
            matches!(&err, KoraError::InvalidTransaction(msg) if msg.contains("Migrate")),
            "{err:?}"
        );
    }

    #[test]
    fn validate_config_errors_when_no_loader_in_allowed_programs() {
        let plugin = DeployAuthorityPlugin;
        let mut config = ConfigMockBuilder::new().build();
        enable_deploy_authority_plugin(&mut config);
        // Ensure neither loader is in allowed_programs.
        config.validation.allowed_programs = ProgramsConfig::Allowlist(vec![]);
        config.validation.fee_payer_policy.bpf_loader_upgradeable.allow_write = true;

        let (errors, _) = plugin.validate_config(&config);
        assert!(
            errors.iter().any(|e| e.contains("BPF Loader Upgradeable") && e.contains("LoaderV4")),
            "got errors: {errors:?}"
        );
    }

    #[test]
    fn validate_config_ok_when_only_v3_is_configured() {
        let plugin = DeployAuthorityPlugin;
        let mut config = ConfigMockBuilder::new()
            .with_allowed_programs(vec![BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string()])
            .build();
        enable_deploy_authority_plugin(&mut config);
        config.validation.fee_payer_policy.bpf_loader_upgradeable.allow_write = true;

        let (errors, warnings) = plugin.validate_config(&config);
        assert!(errors.is_empty(), "got errors: {errors:?}");
        assert!(warnings.is_empty(), "got warnings: {warnings:?}");
    }

    #[test]
    fn validate_config_no_warning_when_any_v3_flag_true() {
        // Regression: every v3 allow_* flag must be reachable from `v3_any`. If we forget one,
        // an operator who enabled (e.g.) only allow_close would see a misleading "plugin will
        // never see a loader instruction" warning even though the plugin would actually run.
        let new_config = || {
            let mut config = ConfigMockBuilder::new()
                .with_allowed_programs(vec![BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string()])
                .build();
            enable_deploy_authority_plugin(&mut config);
            config
        };

        type ToggleEntry = (&'static str, fn(&mut Config));
        let toggle_one_v3_flag: [ToggleEntry; 10] = [
            ("allow_initialize_buffer", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_initialize_buffer = true;
            }),
            ("allow_write", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_write = true;
            }),
            ("allow_deploy_with_max_data_len", |c| {
                c.validation
                    .fee_payer_policy
                    .bpf_loader_upgradeable
                    .allow_deploy_with_max_data_len = true;
            }),
            ("allow_upgrade", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_upgrade = true;
            }),
            ("allow_set_authority", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_set_authority = true;
            }),
            ("allow_set_authority_checked", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_set_authority_checked =
                    true;
            }),
            ("allow_close", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_close = true;
            }),
            ("allow_extend_program", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_extend_program = true;
            }),
            ("allow_extend_program_checked", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_extend_program_checked =
                    true;
            }),
            ("allow_migrate", |c| {
                c.validation.fee_payer_policy.bpf_loader_upgradeable.allow_migrate = true;
            }),
        ];

        let plugin = DeployAuthorityPlugin;
        for (label, toggle) in toggle_one_v3_flag {
            let mut config = new_config();
            toggle(&mut config);

            let (errors, warnings) = plugin.validate_config(&config);
            assert!(errors.is_empty(), "{label}: got errors: {errors:?}");
            assert!(warnings.is_empty(), "{label}: expected no warnings, got: {warnings:?}");
        }
    }
}
