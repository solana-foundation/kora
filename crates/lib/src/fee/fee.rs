use crate::{
    constant::{ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION, LAMPORTS_PER_SIGNATURE},
    error::KoraError,
    get_signer,
    state::get_config,
    token::token::TokenType,
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, ParsedSystemInstructionData,
        ParsedSystemInstructionType, VersionedTransactionResolved,
    },
    CacheUtil,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_program::program_pack::Pack;
use solana_sdk::{pubkey::Pubkey, rent::Rent};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account as SplTokenAccountState;

pub struct FeeConfigUtil {}

impl FeeConfigUtil {
    fn is_fee_payer_in_signers(
        transaction: &VersionedTransactionResolved,
    ) -> Result<bool, KoraError> {
        let fee_payer = get_signer()
            .map(|signer| signer.solana_pubkey())
            .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer: {e}")))?;

        let all_account_keys = &transaction.all_account_keys;
        let transaction_inner = &transaction.transaction;

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

    async fn get_associated_token_account_creation_fees(
        rpc_client: &RpcClient,
        transaction: &VersionedTransactionResolved,
    ) -> Result<u64, KoraError> {
        let mut total_lamports = 0u64;

        // Check each instruction in the transaction for ATA creation
        for instruction in &transaction.all_instructions {
            // Skip if not an ATA program instruction
            if instruction.program_id != spl_associated_token_account::id() {
                continue;
            }

            let ata = instruction.accounts[1].pubkey;
            let owner = instruction.accounts[2].pubkey;
            let mint = instruction.accounts[3].pubkey;

            let expected_ata = get_associated_token_address(&owner, &mint);

            // Force refresh in case extensions are modified
            if ata == expected_ata
                && CacheUtil::get_account_from_cache(rpc_client, &ata, true).await.is_err()
            {
                // Determine the appropriate token program and get account size
                let account_size =
                    match CacheUtil::get_account_from_cache(rpc_client, &mint, true).await {
                        Ok(mint_account) => {
                            match TokenType::get_token_program_from_owner(&mint_account.owner) {
                                Ok(token_program) => token_program
                                    .get_ata_account_size(&mint, &mint_account)
                                    .await
                                    .unwrap_or(SplTokenAccountState::LEN),
                                Err(_) => {
                                    return Err(KoraError::InternalServerError(
                                        "Unknown token program".to_string(),
                                    ));
                                }
                            }
                        }
                        Err(_) => {
                            return Err(KoraError::InternalServerError(
                                "Failed to fetch mint account".to_string(),
                            ));
                        }
                    };

                // Get rent cost in lamports for ATA creation with the determined size
                let rent = Rent::default();
                let exempt_min = rent.minimum_balance(account_size);
                total_lamports += exempt_min;
            }
        }

        Ok(total_lamports)
    }

    async fn has_payment_instruction(
        resolved_transaction: &mut VersionedTransactionResolved,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let payment_destination = &get_config()?.kora.get_payment_address()?;

        for instruction in resolved_transaction
            .get_or_parse_spl_instructions()?
            .get(&ParsedSPLInstructionType::SplTokenTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSPLInstructionData::SplTokenTransfer { destination_address, .. } =
                instruction
            {
                let destination_account =
                    CacheUtil::get_account_from_cache(rpc_client, destination_address, false)
                        .await?;

                let token_program =
                    TokenType::get_token_program_from_owner(&destination_account.owner)?;

                let token_account =
                    token_program.unpack_token_account(&destination_account.data)?;

                if token_account.owner() == *payment_destination {
                    return Ok(0);
                }
            }
        }

        // For now we estimate the fee for a payment instruction to be hardcoded, simulation / estimation isn't support yet
        Ok(ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION)
    }

    pub async fn estimate_transaction_fee(
        rpc_client: &RpcClient,
        transaction: &mut VersionedTransactionResolved,
        fee_payer: Option<&Pubkey>,
        is_payment_required: bool,
    ) -> Result<u64, KoraError> {
        let inner_transaction = &transaction.transaction;

        // Get base transaction fee
        let base_fee =
            TransactionFeeUtil::get_estimate_fee(rpc_client, &inner_transaction.message).await?;

        // Get account creation fees (for ATA creation)
        let account_creation_fee =
            FeeConfigUtil::get_associated_token_account_creation_fees(rpc_client, transaction)
                .await
                .map_err(|e| KoraError::RpcError(e.to_string()))?;

        // Priority fees are now included in the calculate done by the RPC getFeeForMessage

        // If the Kora signer is not inclded in the signers, we add another base fee, since each transaction will be 5000 lamports
        let mut kora_signature_fee = 0u64;
        if !FeeConfigUtil::is_fee_payer_in_signers(transaction)? {
            kora_signature_fee = LAMPORTS_PER_SIGNATURE;
        }

        // Calculate fee payer outflow if fee payer is provided, to better estimate the potential fee
        let fee_payer_outflow = if let Some(fee_payer_pubkey) = fee_payer {
            FeeConfigUtil::calculate_fee_payer_outflow(fee_payer_pubkey, transaction).await?
        } else {
            0
        };

        // If the transaction for paying the gasless relayer is not included, but we expect a payment, we need to add the fee for the payment instruction
        // for a better approximation of the fee
        let fee_for_payment_instruction = if is_payment_required {
            FeeConfigUtil::has_payment_instruction(transaction, rpc_client).await?
        } else {
            0
        };

        Ok(base_fee
            + account_creation_fee
            + kora_signature_fee
            + fee_payer_outflow
            + fee_for_payment_instruction)
    }

    /// Calculate the total outflow (SOL spending) that could occur for a fee payer account in a transaction.
    /// This includes transfers, account creation, and other operations that could drain the fee payer's balance.
    pub async fn calculate_fee_payer_outflow(
        fee_payer_pubkey: &Pubkey,
        transaction: &mut VersionedTransactionResolved,
    ) -> Result<u64, KoraError> {
        let mut total = 0u64;

        let parsed_system_instructions = transaction.get_or_parse_system_instructions()?;

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemTransfer)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemTransfer { lamports, sender, receiver } =
                instruction
            {
                if *sender == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
                if *receiver == *fee_payer_pubkey {
                    total = total.saturating_sub(*lamports);
                }
            }
        }

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemCreateAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemCreateAccount { lamports, payer } =
                instruction
            {
                if *payer == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
            }
        }

        for instruction in parsed_system_instructions
            .get(&ParsedSystemInstructionType::SystemWithdrawNonceAccount)
            .unwrap_or(&vec![])
        {
            if let ParsedSystemInstructionData::SystemWithdrawNonceAccount {
                lamports,
                nonce_authority,
                recipient,
            } = instruction
            {
                if *nonce_authority == *fee_payer_pubkey {
                    total = total.saturating_add(*lamports);
                }
                if *recipient == *fee_payer_pubkey {
                    total = total.saturating_sub(*lamports);
                }
            }
        }

        Ok(total)
    }
}

pub struct TransactionFeeUtil {}

impl TransactionFeeUtil {
    pub async fn get_estimate_fee(
        rpc_client: &RpcClient,
        message: &VersionedMessage,
    ) -> Result<u64, KoraError> {
        match message {
            VersionedMessage::Legacy(message) => rpc_client.get_fee_for_message(message).await,
            VersionedMessage::V0(message) => rpc_client.get_fee_for_message(message).await,
        }
        .map_err(|e| KoraError::RpcError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constant::ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION,
        fee::fee::FeeConfigUtil,
        tests::common::{
            create_mock_token_account, get_mock_rpc_client, setup_or_get_test_config,
            setup_or_get_test_signer,
        },
        token::{interface::TokenInterface, TokenProgram},
        transaction::TransactionUtil,
    };
    use solana_message::{v0, Message, VersionedMessage};

    use solana_sdk::{
        account::Account,
        hash::Hash,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
    };
    use solana_system_interface::{
        instruction::{
            create_account, create_account_with_seed, transfer, transfer_with_seed,
            withdraw_nonce_account,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };
    use spl_associated_token_account::get_associated_token_address;

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));

        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        assert!(FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_not_signer() {
        setup_or_get_test_signer();
        let sender = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&sender.pubkey(), &recipient.pubkey(), 1000);

        let message =
            VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));

        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        assert!(!FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction).unwrap());
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
        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        assert!(FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction).unwrap());
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
        let resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        assert!(!FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as sender - should add to outflow
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient - should subtract from outflow
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow (saturating)");

        // Test 3: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer_with_seed() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as sender (index 1 for TransferWithSeed)
        let transfer_instruction = transfer_with_seed(
            &fee_payer,
            &fee_payer,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &recipient,
            150_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 150_000, "TransferWithSeed from fee payer should add to outflow");

        // Test 2: Fee payer as recipient (index 2 for TransferWithSeed)
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer_with_seed(
            &other_sender,
            &other_sender,
            "test_seed".to_string(),
            &SYSTEM_PROGRAM_ID,
            &fee_payer,
            75_000,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(
            outflow, 0,
            "TransferWithSeed to fee payer should subtract from outflow (saturating)"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_create_account() {
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Test 1: Fee payer funding CreateAccount
        let create_instruction =
            create_account(&fee_payer, &new_account, 200_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        // Test 2: Other account funding CreateAccount
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_create_account_with_seed() {
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Test: Fee payer funding CreateAccountWithSeed
        let create_instruction = create_account_with_seed(
            &fee_payer,
            &new_account,
            &fee_payer,
            "test_seed",
            300_000,
            100,
            &SYSTEM_PROGRAM_ID,
        );
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_nonce_withdraw() {
        let nonce_account = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();

        // Test 1: Fee payer as nonce account (outflow)
        let withdraw_instruction =
            withdraw_nonce_account(&nonce_account, &fee_payer, &recipient, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(
            outflow, 50_000,
            "WithdrawNonceAccount from fee payer nonce should add to outflow"
        );

        // Test 2: Fee payer as recipient (inflow)
        let nonce_account = Pubkey::new_unique();
        let withdraw_instruction =
            withdraw_nonce_account(&nonce_account, &fee_payer, &fee_payer, 25_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(
            outflow, 0,
            "WithdrawNonceAccount to fee payer should subtract from outflow (saturating)"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_multiple_instructions() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();

        // Multiple instructions involving fee payer
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100,000
            transfer(&sender, &fee_payer, 30_000),     // -30,000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50,000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(
            outflow, 120_000,
            "Multiple instructions should sum correctly: 100000 - 30000 + 50000 = 120000"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_non_system_program() {
        let fee_payer = Pubkey::new_unique();
        let fake_program = Pubkey::new_unique();

        // Test with non-system program - should not affect outflow
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);
        let outflow =
            FeeConfigUtil::calculate_fee_payer_outflow(&fee_payer, &mut resolved_transaction)
                .await
                .unwrap();
        assert_eq!(outflow, 0, "Non-system program should not affect outflow");
    }

    #[tokio::test]
    async fn test_has_payment_instruction_with_payment() {
        setup_or_get_test_config();
        let signer = setup_or_get_test_signer();
        let mint = Pubkey::new_unique();

        let mocked_account = create_mock_token_account(&signer, &mint);
        let mocked_rpc_client = get_mock_rpc_client(&mocked_account);

        let sender = Keypair::new();

        let sender_token_account = get_associated_token_address(&sender.pubkey(), &mint);
        let payment_token_account = get_associated_token_address(&signer, &mint);

        let transfer_instruction = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &payment_token_account,
                &sender.pubkey(),
                1000,
            )
            .unwrap();

        // Create message with the payment instruction
        let message = VersionedMessage::Legacy(Message::new(&[transfer_instruction], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        // Test: Should return 0 because payment instruction exists
        let result =
            FeeConfigUtil::has_payment_instruction(&mut resolved_transaction, &mocked_rpc_client)
                .await
                .unwrap();

        assert_eq!(result, 0, "Should return 0 when payment instruction exists");
    }

    #[tokio::test]
    async fn test_has_payment_instruction_without_payment() {
        setup_or_get_test_config();
        let mocked_rpc_client = get_mock_rpc_client(&Account::default());

        let sender = Keypair::new();
        let recipient = Pubkey::new_unique();

        // Create SOL transfer instruction (no SPL transfer to payment destination)
        let sol_transfer = transfer(&sender.pubkey(), &recipient, 100_000);

        // Create message without payment instruction
        let message = VersionedMessage::Legacy(Message::new(&[sol_transfer], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        // Test: Should return fee estimate because no payment instruction exists
        let result =
            FeeConfigUtil::has_payment_instruction(&mut resolved_transaction, &mocked_rpc_client)
                .await
                .unwrap();

        assert_eq!(
            result, ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION,
            "Should return fee estimate when no payment instruction exists"
        );
    }

    #[tokio::test]
    async fn test_has_payment_instruction_with_spl_transfer_to_different_destination() {
        setup_or_get_test_config();
        let sender = Keypair::new();
        let mint = Pubkey::new_unique();

        let mocked_account = create_mock_token_account(&sender.pubkey(), &mint);
        let mocked_rpc_client = get_mock_rpc_client(&mocked_account);

        // Create token accounts
        let sender_token_account = get_associated_token_address(&sender.pubkey(), &mint);
        let recipient_token_account = get_associated_token_address(&sender.pubkey(), &mint);

        // Create SPL transfer instruction to DIFFERENT destination (not payment)
        let transfer_instruction = TokenProgram::new()
            .create_transfer_instruction(
                &sender_token_account,
                &recipient_token_account,
                &sender.pubkey(),
                1000,
            )
            .unwrap();

        // Create message with non-payment transfer
        let message = VersionedMessage::Legacy(Message::new(&[transfer_instruction], None));
        let mut resolved_transaction =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message);

        // Test: Should return fee estimate because SPL transfer is to different destination
        let result =
            FeeConfigUtil::has_payment_instruction(&mut resolved_transaction, &mocked_rpc_client)
                .await
                .unwrap();

        assert_eq!(
            result, ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION,
            "Should return fee estimate when SPL transfer is to different destination"
        );
    }
}
