use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::str::FromStr;

#[tokio::test]
async fn test_disallowed_memo_program() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let disallowed_program_id = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
        .expect("Failed to parse SPL Memo program ID");

    let malicious_instruction = Instruction::new_with_bincode(disallowed_program_id, &(), vec![]);

    let malicious_tx = ctx
        .transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(malicious_instruction)
        .build()
        .await
        .expect("Failed to create transaction with disallowed program");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            let expected_message =
                format!("Program {disallowed_program_id} is not in the allowed list");
            error.assert_error_type_and_message("Invalid transaction", &expected_message);
        }
        Ok(_) => panic!("Expected error for transaction with disallowed program"),
    }
}

#[tokio::test]
async fn test_disallowed_program_v0_transaction() {
    let ctx = TestContext::new().await.expect("Failed to create test context");

    let disallowed_program_id = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
        .expect("Failed to parse BPF Loader Upgradeable program ID");

    let malicious_instruction = Instruction::new_with_bincode(disallowed_program_id, &(), vec![]);

    let malicious_tx = ctx
        .v0_transaction_builder()
        .with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey())
        .with_instruction(malicious_instruction)
        .build()
        .await
        .expect("Failed to create V0 transaction with disallowed program");

    let result =
        ctx.rpc_call::<serde_json::Value, _>("signTransaction", rpc_params![malicious_tx]).await;

    match result {
        Err(error) => {
            let expected_message =
                format!("Program {disallowed_program_id} is not in the allowed list");
            error.assert_error_type_and_message("Invalid transaction", &expected_message);
        }
        Ok(_) => panic!("Expected error for V0 transaction with disallowed program"),
    }
}
