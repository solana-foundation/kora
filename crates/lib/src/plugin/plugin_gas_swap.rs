use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::{
    config::Config,
    error::KoraError,
    transaction::{
        ParsedSPLInstructionType, ParsedSystemInstructionType, VersionedTransactionResolved,
    },
};

use super::{PluginExecutionContext, TransactionPlugin};

pub(super) struct GasSwapPlugin;

impl GasSwapPlugin {
    fn validate_total_instruction_count(
        transaction: &VersionedTransactionResolved,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let total_instruction_count = transaction.all_instructions.len();

        if total_instruction_count != 2 {
            return Err(KoraError::InvalidTransaction(format!(
                "Plugin gas_swap requires exactly two total instructions (outer + inner), found {} in {}",
                total_instruction_count,
                context.method_name()
            )));
        }

        Ok(())
    }

    fn validate_parsed_system_transfer(
        transaction: &mut VersionedTransactionResolved,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let system_instructions = transaction.get_or_parse_system_instructions()?;
        let transfer_count = system_instructions
            .get(&ParsedSystemInstructionType::SystemTransfer)
            .map(Vec::len)
            .unwrap_or(0);

        if transfer_count != 1 {
            return Err(KoraError::InvalidTransaction(format!(
                "Plugin gas_swap requires exactly one SystemTransfer, found {} in {}",
                transfer_count,
                context.method_name()
            )));
        }

        Ok(())
    }

    fn validate_parsed_token_transfer(
        transaction: &mut VersionedTransactionResolved,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        let spl_instructions = transaction.get_or_parse_spl_instructions()?;
        let transfer_count = spl_instructions
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .map(Vec::len)
            .unwrap_or(0);

        if transfer_count != 1 {
            return Err(KoraError::InvalidTransaction(format!(
                "Plugin gas_swap requires exactly one SplTokenTransfer, found {} in {}",
                transfer_count,
                context.method_name()
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl TransactionPlugin for GasSwapPlugin {
    async fn validate(
        &self,
        transaction: &mut VersionedTransactionResolved,
        _config: &Config,
        _rpc_client: &RpcClient,
        _fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        Self::validate_total_instruction_count(transaction, context)?;
        Self::validate_parsed_system_transfer(transaction, context)?;
        Self::validate_parsed_token_transfer(transaction, context)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::TransactionPluginRunner, *};
    use crate::{
        config::TransactionPluginType,
        tests::{common::RpcMockBuilder, config_mock::ConfigMockBuilder},
        transaction::TransactionUtil,
    };
    use solana_compute_budget_interface::ComputeBudgetInstruction;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
    use solana_system_interface::instruction::{assign, transfer};
    use std::sync::Arc;

    fn enable_gas_swap_plugin(config: &mut Config) {
        config.kora.plugins.enabled = vec![TransactionPluginType::GasSwap];
    }

    fn build_runner() -> (Config, Arc<RpcClient>) {
        let mut config = ConfigMockBuilder::new().build();
        enable_gas_swap_plugin(&mut config);
        let rpc_client = RpcMockBuilder::new().build();
        (config, rpc_client)
    }

    #[tokio::test]
    async fn gas_swap_accepts_valid_top_level_swap_shape() {
        let (config, rpc_client) = build_runner();

        let fee_payer = Pubkey::new_unique();
        let source_wallet = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let destination_wallet = Pubkey::new_unique();

        let token_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &source_token_account,
            &destination_token_account,
            &source_wallet,
            &[],
            1_500,
        )
        .unwrap();
        let sol_ix = transfer(&fee_payer, &destination_wallet, 20_000);

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[token_ix, sol_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

        let runner = TransactionPluginRunner::from_config(&config);
        let result = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignTransaction,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn gas_swap_rejects_non_swap_programs() {
        let (config, rpc_client) = build_runner();

        let fee_payer = Pubkey::new_unique();
        let source_wallet = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let destination_wallet = Pubkey::new_unique();

        let token_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &source_token_account,
            &destination_token_account,
            &source_wallet,
            &[],
            1_500,
        )
        .unwrap();
        let custom_ix =
            Instruction { program_id: Pubkey::new_unique(), accounts: vec![], data: vec![1, 2, 3] };
        let sol_ix = transfer(&fee_payer, &destination_wallet, 20_000);

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[token_ix, custom_ix, sol_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

        let runner = TransactionPluginRunner::from_config(&config);
        let result = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignAndSendTransaction,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn gas_swap_rejects_non_transfer_system_instruction() {
        let (config, rpc_client) = build_runner();

        let fee_payer = Pubkey::new_unique();
        let source_wallet = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let token_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &source_token_account,
            &destination_token_account,
            &source_wallet,
            &[],
            1_500,
        )
        .unwrap();
        let assign_ix = assign(&fee_payer, &Pubkey::new_unique());

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[token_ix, assign_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

        let runner = TransactionPluginRunner::from_config(&config);
        let result = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignTransaction,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn gas_swap_rejects_when_no_token_transfer() {
        let (config, rpc_client) = build_runner();

        let fee_payer = Pubkey::new_unique();
        let destination_wallet = Pubkey::new_unique();
        let sol_ix = transfer(&fee_payer, &destination_wallet, 20_000);

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[sol_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

        let runner = TransactionPluginRunner::from_config(&config);
        let result = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignAndSendTransaction,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }

    #[tokio::test]
    async fn gas_swap_rejects_extra_outer_instruction() {
        let (config, rpc_client) = build_runner();

        let fee_payer = Pubkey::new_unique();
        let source_wallet = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let destination_wallet = Pubkey::new_unique();

        let token_ix = spl_token_interface::instruction::transfer(
            &spl_token_interface::id(),
            &source_token_account,
            &destination_token_account,
            &source_wallet,
            &[],
            1_500,
        )
        .unwrap();
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let sol_ix = transfer(&fee_payer, &destination_wallet, 20_000);

        let tx = TransactionUtil::new_unsigned_versioned_transaction(VersionedMessage::Legacy(
            Message::new(&[token_ix, compute_budget_ix, sol_ix], Some(&fee_payer)),
        ));
        let mut resolved = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

        let runner = TransactionPluginRunner::from_config(&config);
        let result = runner
            .run(
                &mut resolved,
                &config,
                rpc_client.as_ref(),
                &fee_payer,
                PluginExecutionContext::SignTransaction,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::InvalidTransaction(_)));
    }
}
