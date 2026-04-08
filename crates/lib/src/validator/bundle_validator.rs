use crate::{
    bundle::{
        constant::JITO_MAX_BUNDLE_SIZE, BundleError, JitoBundleAccountConfig, JitoBundleClient,
        JitoBundleSimulationConfig, JitoBundleSimulationResult,
        JitoBundleSimulationTransactionResult, JitoError,
    },
    config::Config,
    fee::fee::TransactionFeeUtil,
    transaction::TransactionUtil,
    KoraError,
};
use regex::Regex;
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashSet, str::FromStr, sync::OnceLock};

pub struct BundleValidator {}

impl BundleValidator {
    pub fn validate_jito_bundle_size(transactions: &[String]) -> Result<(), KoraError> {
        if transactions.is_empty() {
            return Err(BundleError::Empty.into());
        }
        if transactions.len() > JITO_MAX_BUNDLE_SIZE {
            return Err(BundleError::Jito(JitoError::BundleTooLarge(JITO_MAX_BUNDLE_SIZE)).into());
        }
        Ok(())
    }

    pub fn signed_indices_for_bundle(
        bundle_size: usize,
        sign_only_indices: Option<&[usize]>,
    ) -> Vec<usize> {
        let mut indices: Vec<usize> =
            sign_only_indices.map(|raw| raw.to_vec()).unwrap_or_else(|| (0..bundle_size).collect());

        indices.retain(|idx| *idx < bundle_size);
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    pub async fn simulate_and_validate_sequential_bundle(
        rpc_client: &RpcClient,
        config: &Config,
        encoded_transactions: &[String],
        signed_indices: &[usize],
        fee_payer: &Pubkey,
        skip_sig_verify: bool,
    ) -> Result<JitoBundleSimulationResult, KoraError> {
        let simulation_config = Self::build_simulation_config_for_signed_indices(
            encoded_transactions.len(),
            signed_indices,
            fee_payer,
            skip_sig_verify,
        );
        let jito_client = JitoBundleClient::new(&config.kora.bundle.jito);
        let simulation_result = jito_client
            .simulate_bundle_with_config(encoded_transactions, simulation_config)
            .await?;

        Self::validate_simulation_policy(
            rpc_client,
            config,
            encoded_transactions,
            signed_indices,
            &simulation_result,
        )
        .await?;

        Ok(simulation_result)
    }

    fn build_simulation_config_for_signed_indices(
        bundle_size: usize,
        signed_indices: &[usize],
        fee_payer: &Pubkey,
        skip_sig_verify: bool,
    ) -> JitoBundleSimulationConfig {
        let signed_set: HashSet<usize> = signed_indices.iter().copied().collect();
        let fee_payer_account = JitoBundleAccountConfig {
            addresses: vec![fee_payer.to_string()],
            encoding: Some("base64".to_string()),
        };

        let pre_execution_accounts_configs =
            (0..bundle_size)
                .map(|tx_idx| {
                    if signed_set.contains(&tx_idx) {
                        Some(fee_payer_account.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<Option<JitoBundleAccountConfig>>>();

        let post_execution_accounts_configs =
            (0..bundle_size)
                .map(|tx_idx| {
                    if signed_set.contains(&tx_idx) {
                        Some(fee_payer_account.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<Option<JitoBundleAccountConfig>>>();

        JitoBundleSimulationConfig {
            pre_execution_accounts_configs: Some(pre_execution_accounts_configs),
            post_execution_accounts_configs: Some(post_execution_accounts_configs),
            transaction_encoding: Some("base64".to_string()),
            skip_sig_verify: Some(skip_sig_verify),
            replace_recent_blockhash: None,
        }
    }

    async fn validate_simulation_policy(
        rpc_client: &RpcClient,
        config: &Config,
        encoded_transactions: &[String],
        signed_indices: &[usize],
        simulation_result: &JitoBundleSimulationResult,
    ) -> Result<(), KoraError> {
        if !signed_indices.is_empty()
            && simulation_result.transaction_results.len() != encoded_transactions.len()
        {
            return Err(KoraError::InvalidTransaction(format!(
                "Bundle simulation returned {} transaction results for {} transactions; \
                 cannot safely validate signed bundle members sequentially",
                simulation_result.transaction_results.len(),
                encoded_transactions.len()
            )));
        }

        let allowed_programs = Self::parse_pubkey_set(&config.validation.allowed_programs)?;
        let disallowed_programs = Self::parse_pubkey_set(&config.validation.disallowed_accounts)?;

        for &signed_idx in signed_indices {
            if signed_idx >= encoded_transactions.len() {
                continue;
            }

            let tx_result =
                simulation_result.transaction_results.get(signed_idx).ok_or_else(|| {
                    KoraError::InvalidTransaction(format!(
                        "Bundle simulation missing result for signed transaction index {}",
                        signed_idx
                    ))
                })?;

            Self::validate_invoked_programs(
                &allowed_programs,
                &disallowed_programs,
                &tx_result.logs,
            )?;

            Self::validate_fee_payer_lamport_outflow(
                rpc_client,
                &encoded_transactions[signed_idx],
                signed_idx,
                tx_result,
                config.validation.max_allowed_lamports,
            )
            .await?;
        }

        Ok(())
    }

    fn validate_invoked_programs(
        allowed_programs: &HashSet<Pubkey>,
        disallowed_programs: &HashSet<Pubkey>,
        logs: &[String],
    ) -> Result<(), KoraError> {
        let invoked_programs = Self::extract_invoked_programs(logs)?;

        for program_id in invoked_programs {
            if disallowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is disallowed",
                    program_id
                )));
            }

            if !allowed_programs.contains(&program_id) {
                return Err(KoraError::InvalidTransaction(format!(
                    "Program {} is not in the allowed list",
                    program_id
                )));
            }
        }

        Ok(())
    }

    async fn validate_fee_payer_lamport_outflow(
        rpc_client: &RpcClient,
        encoded_transaction: &str,
        signed_idx: usize,
        tx_result: &JitoBundleSimulationTransactionResult,
        max_allowed_lamports: u64,
    ) -> Result<(), KoraError> {
        let pre_accounts = tx_result.pre_execution_accounts.as_ref().ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation did not return pre-execution accounts for signed transaction index {}",
                signed_idx
            ))
        })?;
        let post_accounts = tx_result.post_execution_accounts.as_ref().ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation did not return post-execution accounts for signed transaction index {}",
                signed_idx
            ))
        })?;

        let pre_account = pre_accounts.first().ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation returned empty pre-execution accounts for signed transaction index {}",
                signed_idx
            ))
        })?;
        let post_account = post_accounts.first().ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation returned empty post-execution accounts for signed transaction index {}",
                signed_idx
            ))
        })?;

        let pre_lamports = Self::extract_lamports(pre_account).ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation pre-execution lamports missing for signed transaction index {}",
                signed_idx
            ))
        })?;
        let post_lamports = Self::extract_lamports(post_account).ok_or_else(|| {
            KoraError::InvalidTransaction(format!(
                "Bundle simulation post-execution lamports missing for signed transaction index {}",
                signed_idx
            ))
        })?;

        let observed_lamport_outflow = pre_lamports.saturating_sub(post_lamports);

        if observed_lamport_outflow == 0 {
            return Ok(());
        }

        let transaction = TransactionUtil::decode_b64_transaction(encoded_transaction)?;
        let estimated_network_fee =
            TransactionFeeUtil::get_estimate_fee(rpc_client, &transaction.message).await?;

        // Match existing policy semantics by excluding the network signature fee from transfer outflow.
        let transfer_outflow = observed_lamport_outflow.saturating_sub(estimated_network_fee);

        if transfer_outflow > max_allowed_lamports {
            return Err(KoraError::InvalidTransaction(format!(
                "Total transfer amount {} exceeds maximum allowed {}",
                transfer_outflow, max_allowed_lamports
            )));
        }

        Ok(())
    }

    fn parse_pubkey_set(pubkeys: &[String]) -> Result<HashSet<Pubkey>, KoraError> {
        pubkeys
            .iter()
            .map(|pubkey| {
                Pubkey::from_str(pubkey).map_err(|e| {
                    KoraError::InternalServerError(format!(
                        "Invalid public key `{}` in config: {}",
                        pubkey, e
                    ))
                })
            })
            .collect::<Result<HashSet<Pubkey>, KoraError>>()
    }

    fn extract_invoked_programs(logs: &[String]) -> Result<HashSet<Pubkey>, KoraError> {
        static PROGRAM_INVOKE_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = PROGRAM_INVOKE_REGEX.get_or_init(|| {
            Regex::new(r"^Program ([1-9A-HJ-NP-Za-km-z]{32,44}) invoke \[\d+\]$")
                .expect("program invoke regex must be valid")
        });

        let mut invoked = HashSet::new();
        for log in logs {
            if let Some(captures) = regex.captures(log) {
                if let Some(program) = captures.get(1) {
                    let program_id = Pubkey::from_str(program.as_str()).map_err(|e| {
                        KoraError::InvalidTransaction(format!(
                            "Invalid invoked program id in simulation logs: {}",
                            e
                        ))
                    })?;
                    invoked.insert(program_id);
                }
            }
        }

        Ok(invoked)
    }

    fn extract_lamports(account: &Value) -> Option<u64> {
        account.get("lamports").and_then(|lamports| match lamports {
            Value::Number(number) => number.as_u64(),
            Value::String(value) => value.parse::<u64>().ok(),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{config_mock::ConfigMockBuilder, rpc_mock::RpcMockBuilder};
    use serde_json::json;
    use solana_message::{Message, VersionedMessage};
    use solana_system_interface::instruction::transfer;

    fn make_test_transaction(fee_payer: &Pubkey) -> String {
        let instruction = transfer(fee_payer, &Pubkey::new_unique(), 1_000);
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(fee_payer)));
        let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
        TransactionUtil::encode_versioned_transaction(&transaction).unwrap()
    }

    fn make_tx_result(
        logs: Vec<String>,
        pre_lamports: Option<u64>,
        post_lamports: Option<u64>,
    ) -> JitoBundleSimulationTransactionResult {
        let pre_execution_accounts = pre_lamports.map(|lamports| {
            vec![json!({
                "lamports": lamports,
                "owner": "11111111111111111111111111111111",
                "data": ["", "base64"],
                "executable": false,
                "rentEpoch": 0
            })]
        });
        let post_execution_accounts = post_lamports.map(|lamports| {
            vec![json!({
                "lamports": lamports,
                "owner": "11111111111111111111111111111111",
                "data": ["", "base64"],
                "executable": false,
                "rentEpoch": 0
            })]
        });

        JitoBundleSimulationTransactionResult {
            err: None,
            logs,
            context: json!({ "slot": 1 }),
            pre_execution_accounts,
            post_execution_accounts,
            units_consumed: Some(1),
            return_data: None,
        }
    }

    #[test]
    fn test_validate_jito_bundle_size_empty() {
        let result = BundleValidator::validate_jito_bundle_size(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jito_bundle_size_too_large() {
        let result = BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_jito_bundle_size_valid() {
        let result = BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 5]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_jito_bundle_size_boundary() {
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 1]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 4]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 5]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]).is_err());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 7]).is_err());
    }

    #[test]
    fn test_validate_jito_bundle_size_error_types() {
        let empty_err = BundleValidator::validate_jito_bundle_size(&[]).unwrap_err();
        assert!(matches!(empty_err, KoraError::InvalidTransaction(_)));

        let large_err =
            BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]).unwrap_err();
        assert!(matches!(large_err, KoraError::JitoError(_)));
    }

    #[test]
    fn test_signed_indices_for_bundle_defaults_to_all() {
        let indices = BundleValidator::signed_indices_for_bundle(4, None);
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_signed_indices_for_bundle_dedups_and_filters_oob() {
        let indices = BundleValidator::signed_indices_for_bundle(3, Some(&[2, 0, 2, 5]));
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_build_simulation_config_tracks_only_signed_indices() {
        let fee_payer = Pubkey::new_unique();
        let config =
            BundleValidator::build_simulation_config_for_signed_indices(3, &[1], &fee_payer, false);

        let pre = config.pre_execution_accounts_configs.unwrap();
        let post = config.post_execution_accounts_configs.unwrap();

        assert!(pre[0].is_none());
        assert!(pre[2].is_none());
        assert_eq!(pre[1].as_ref().unwrap().addresses, vec![fee_payer.to_string()]);
        assert_eq!(post[1].as_ref().unwrap().addresses, vec![fee_payer.to_string()]);
    }

    #[test]
    fn test_extract_invoked_programs() {
        let system_program = "11111111111111111111111111111111";
        let token_program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        let logs = vec![
            format!("Program {} invoke [1]", system_program),
            format!("Program {} invoke [2]", token_program),
            "Program log: some message".to_string(),
        ];

        let programs = BundleValidator::extract_invoked_programs(&logs).unwrap();
        assert!(programs.contains(&Pubkey::from_str(system_program).unwrap()));
        assert!(programs.contains(&Pubkey::from_str(token_program).unwrap()));
    }

    #[tokio::test]
    async fn test_validate_simulation_policy_rejects_disallowed_program() {
        let fee_payer = Pubkey::new_unique();
        let encoded_transactions = vec![make_test_transaction(&fee_payer)];
        let config = ConfigMockBuilder::new()
            .with_max_allowed_lamports(1_000_000)
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .with_disallowed_accounts(vec![
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()
            ])
            .build();
        let rpc_client = RpcMockBuilder::new().with_fee_estimate(5_000).build();

        let simulation_result = JitoBundleSimulationResult {
            context: json!({ "slot": 1 }),
            summary: Some(json!("succeeded")),
            transaction_results: vec![make_tx_result(
                vec!["Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [1]".to_string()],
                Some(1_000_000_000),
                Some(1_000_000_000),
            )],
        };

        let result = BundleValidator::validate_simulation_policy(
            &rpc_client,
            &config,
            &encoded_transactions,
            &[0],
            &simulation_result,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("disallowed"));
    }

    #[tokio::test]
    async fn test_validate_simulation_policy_rejects_outflow_above_limit() {
        let fee_payer = Pubkey::new_unique();
        let encoded_transactions = vec![make_test_transaction(&fee_payer)];
        let config = ConfigMockBuilder::new()
            .with_max_allowed_lamports(1_000_000)
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .build();
        let rpc_client = RpcMockBuilder::new().with_fee_estimate(5_000).build();

        let simulation_result = JitoBundleSimulationResult {
            context: json!({ "slot": 1 }),
            summary: Some(json!("succeeded")),
            transaction_results: vec![make_tx_result(
                vec!["Program 11111111111111111111111111111111 invoke [1]".to_string()],
                Some(2_000_000),
                Some(890_000),
            )],
        };

        let result = BundleValidator::validate_simulation_policy(
            &rpc_client,
            &config,
            &encoded_transactions,
            &[0],
            &simulation_result,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Total transfer amount"));
        assert!(err.contains("exceeds maximum allowed"));
    }

    #[tokio::test]
    async fn test_validate_simulation_policy_accepts_outflow_within_limit() {
        let fee_payer = Pubkey::new_unique();
        let encoded_transactions = vec![make_test_transaction(&fee_payer)];
        let config = ConfigMockBuilder::new()
            .with_max_allowed_lamports(1_000_000)
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .build();
        let rpc_client = RpcMockBuilder::new().with_fee_estimate(5_000).build();

        let simulation_result = JitoBundleSimulationResult {
            context: json!({ "slot": 1 }),
            summary: Some(json!("succeeded")),
            transaction_results: vec![make_tx_result(
                vec!["Program 11111111111111111111111111111111 invoke [1]".to_string()],
                Some(2_000_000),
                Some(1_010_000),
            )],
        };

        let result = BundleValidator::validate_simulation_policy(
            &rpc_client,
            &config,
            &encoded_transactions,
            &[0],
            &simulation_result,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_simulation_policy_requires_per_tx_results() {
        let fee_payer = Pubkey::new_unique();
        let encoded_transactions =
            vec![make_test_transaction(&fee_payer), make_test_transaction(&fee_payer)];
        let config = ConfigMockBuilder::new()
            .with_max_allowed_lamports(1_000_000)
            .with_allowed_programs(vec!["11111111111111111111111111111111".to_string()])
            .build();
        let rpc_client = RpcMockBuilder::new().with_fee_estimate(5_000).build();

        let simulation_result = JitoBundleSimulationResult {
            context: json!({ "slot": 1 }),
            summary: Some(json!("succeeded")),
            transaction_results: vec![make_tx_result(vec![], Some(1_000_000), Some(1_000_000))],
        };

        let result = BundleValidator::validate_simulation_policy(
            &rpc_client,
            &config,
            &encoded_transactions,
            &[1],
            &simulation_result,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot safely validate signed bundle members sequentially"));
    }
}
