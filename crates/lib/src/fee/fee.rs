use crate::{
    constant::LAMPORTS_PER_SIGNATURE,
    error::KoraError,
    get_signer,
    transaction::{get_estimate_fee, VersionedTransactionExt},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_message::VersionedMessage;
use solana_sdk::{
    instruction::CompiledInstruction, pubkey::Pubkey, rent::Rent, transaction::VersionedTransaction,
};
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};
use spl_associated_token_account::get_associated_token_address;

pub struct FeeConfigUtil {}

impl FeeConfigUtil {
    fn is_fee_payer_in_signers(
        transaction: &impl VersionedTransactionExt,
    ) -> Result<bool, KoraError> {
        let fee_payer = get_signer()
            .map(|signer| signer.solana_pubkey())
            .map_err(|e| KoraError::InternalServerError(format!("Failed to get signer: {e}")))?;

        let all_account_keys = transaction.get_all_account_keys();
        let transaction_inner = transaction.get_transaction();

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
        transaction: &VersionedTransaction,
    ) -> Result<u64, KoraError> {
        const ATA_ACCOUNT_SIZE: usize = 165; // Standard ATA size
        let mut ata_count = 0u64;

        // Check each instruction in the transaction for ATA creation
        for instruction in transaction.message.instructions() {
            let account_keys = transaction.message.static_account_keys();
            let program_id = account_keys[instruction.program_id_index as usize];

            // Skip if not an ATA program instruction
            if program_id != spl_associated_token_account::id() {
                continue;
            }

            let ata = account_keys[instruction.accounts[1] as usize];
            let owner = account_keys[instruction.accounts[2] as usize];
            let mint = account_keys[instruction.accounts[3] as usize];

            let expected_ata = get_associated_token_address(&owner, &mint);

            if ata == expected_ata && rpc_client.get_account(&ata).await.is_err() {
                ata_count += 1;
            }
        }

        // Get rent cost in lamports for ATA creation
        let rent = Rent::default();
        let exempt_min = rent.minimum_balance(ATA_ACCOUNT_SIZE);

        Ok(exempt_min * ata_count)
    }

    pub async fn estimate_transaction_fee(
        rpc_client: &RpcClient,
        // Should have resolved addresses for lookup tables
        resolved_transaction: &impl VersionedTransactionExt,
        fee_payer: Option<&Pubkey>,
    ) -> Result<u64, KoraError> {
        let transaction = resolved_transaction.get_transaction();

        // Get base transaction fee
        let base_fee = get_estimate_fee(rpc_client, &transaction.message).await?;

        // Get account creation fees (for ATA creation)
        let account_creation_fee =
            FeeConfigUtil::get_associated_token_account_creation_fees(rpc_client, transaction)
                .await
                .map_err(|e| KoraError::RpcError(e.to_string()))?;

        // Priority fees are now included in the calculate done by the RPC getFeeForMessage

        // If the Kora signer is not inclded in the signers, we add another base fee, since each transaction will be 5000 lamports
        let mut kora_signature_fee = 0u64;
        if !FeeConfigUtil::is_fee_payer_in_signers(resolved_transaction)? {
            kora_signature_fee = LAMPORTS_PER_SIGNATURE;
        }

        // Calculate fee payer outflow if fee payer is provided, to better estimate the potential fee
        let fee_payer_outflow = if let Some(fee_payer_pubkey) = fee_payer {
            FeeConfigUtil::calculate_fee_payer_outflow(
                rpc_client,
                fee_payer_pubkey,
                &transaction.message,
                &transaction.get_all_account_keys(),
            )
            .await?
        } else {
            0
        };

        Ok(base_fee + account_creation_fee + kora_signature_fee + fee_payer_outflow)
    }

    /// Calculate the total outflow (SOL spending) that could occur for a fee payer account in a transaction.
    /// This includes transfers, account creation, and other operations that could drain the fee payer's balance.
    pub async fn calculate_fee_payer_outflow(
        rpc_client: &RpcClient,
        fee_payer_pubkey: &Pubkey,
        message: &VersionedMessage,
        account_keys: &[Pubkey],
    ) -> Result<u64, KoraError> {
        let mut total = 0u64;

        // Helper function to check if the fee payer is at a specific account index in an instruction
        let is_fee_payer =
            |instruction: &CompiledInstruction, account_index: usize| -> Result<bool, KoraError> {
                if account_index >= instruction.accounts.len() {
                    return Ok(false); // If account index is invalid, fee payer can't be the source
                }
                let account_key_index = instruction.accounts[account_index];
                if (account_key_index as usize) >= account_keys.len() {
                    return Ok(false); // If account key index is invalid, fee payer can't be the source
                }
                let account_pubkey = account_keys[account_key_index as usize];
                Ok(account_pubkey == *fee_payer_pubkey)
            };

        let get_current_balance = async |account_pubkey: &Pubkey| -> Result<u64, KoraError> {
            if let Ok(account_balance) = rpc_client
                .get_account_with_commitment(account_pubkey, rpc_client.commitment())
                .await
            {
                if let Some(account_balance) = account_balance.value {
                    return Ok(account_balance.lamports);
                }
            }

            Ok(0)
        };

        for instruction in message.instructions() {
            let program_idx = instruction.program_id_index as usize;
            if program_idx >= account_keys.len() {
                continue; // Skip invalid program ID index
            }
            let program_id = account_keys[program_idx];

            // Handle System Program transfers and account creation
            if program_id == SYSTEM_PROGRAM_ID {
                match bincode::deserialize::<SystemInstruction>(&instruction.data) {
                    // Account creation instructions - funding account pays lamports
                    Ok(SystemInstruction::CreateAccount { lamports, .. })
                    | Ok(SystemInstruction::CreateAccountWithSeed { lamports, .. }) => {
                        if is_fee_payer(instruction, 0)? {
                            total = total.saturating_add(lamports);
                        }
                    }
                    // Transfer instructions
                    Ok(SystemInstruction::Transfer { lamports }) => {
                        // Check if fee payer is sender (outflow)
                        if is_fee_payer(instruction, 0)? {
                            total = total.saturating_add(lamports);
                        }
                        // Check if fee payer is receiver (inflow, e.g., from account closure)
                        else if is_fee_payer(instruction, 1)? {
                            total = total.saturating_sub(lamports);
                        }
                    }
                    Ok(SystemInstruction::TransferWithSeed { lamports, .. }) => {
                        // Check if fee payer is sender (outflow). With seeds sender is at index 1
                        if is_fee_payer(instruction, 1)? {
                            total = total.saturating_add(lamports);
                        }
                        // Check if fee payer is receiver (inflow)
                        else if is_fee_payer(instruction, 2)? {
                            total = total.saturating_sub(lamports);
                        }
                    }
                    // Nonce account withdrawal - can drain entire nonce account balance
                    Ok(SystemInstruction::WithdrawNonceAccount(lamports)) => {
                        // Check if fee payer is the nonce account (outflow) - index 0
                        if is_fee_payer(instruction, 0)? {
                            total = total.saturating_add(lamports);
                        }
                        // Check if fee payer is recipient (inflow) - index 1
                        else if is_fee_payer(instruction, 1)? {
                            total = total.saturating_sub(lamports);
                        }
                    }
                    // Account space allocation - may require additional rent
                    Ok(SystemInstruction::Allocate { space }) => {
                        if is_fee_payer(instruction, 0)? {
                            // Get the account being allocated (at index 0)
                            let account_key_index = instruction.accounts[0];
                            let account_being_allocated = account_keys[account_key_index as usize];

                            // Calculate potential rent increase for space allocation
                            let rent = solana_sdk::rent::Rent::default();
                            let current_balance =
                                get_current_balance(&account_being_allocated).await?;
                            let required_balance = rent.minimum_balance(space as usize);
                            if required_balance > current_balance {
                                total = total.saturating_add(required_balance - current_balance);
                            }
                        }
                    }
                    Ok(SystemInstruction::AllocateWithSeed { space, .. }) => {
                        if is_fee_payer(instruction, 1)? {
                            // Get the account being allocated (at index 0, but fee payer funds it at index 1)
                            let account_key_index = instruction.accounts[0];
                            let account_being_allocated = account_keys[account_key_index as usize];

                            // Calculate potential rent increase for space allocation with seed
                            let rent = solana_sdk::rent::Rent::default();
                            let current_balance =
                                get_current_balance(&account_being_allocated).await?;
                            let required_balance = rent.minimum_balance(space as usize);
                            if required_balance > current_balance {
                                total = total.saturating_add(required_balance - current_balance);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        fee::fee::FeeConfigUtil,
        tests::common::{get_mock_rpc_client, setup_or_get_test_signer},
        transaction::{new_unsigned_versioned_transaction, VersionedTransactionResolved},
    };
    use solana_message::{v0, Message, VersionedMessage};
    use solana_sdk::{
        account::Account,
        hash::Hash,
        instruction::{CompiledInstruction, Instruction},
        pubkey::Pubkey,
        rent::Rent,
        signature::{Keypair, Signer},
        system_instruction::SystemInstruction,
    };
    use solana_system_interface::{
        instruction::{
            allocate, allocate_with_seed, create_account, create_account_with_seed, transfer,
            transfer_with_seed, withdraw_nonce_account,
        },
        program::ID as SYSTEM_PROGRAM_ID,
    };

    #[test]
    fn test_is_fee_payer_in_signers_legacy_fee_payer_is_signer() {
        let fee_payer = setup_or_get_test_signer();
        let other_signer = Keypair::new();
        let recipient = Keypair::new();

        let instruction = transfer(&other_signer.pubkey(), &recipient.pubkey(), 1000);

        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

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

        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

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
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

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
        let transaction = new_unsigned_versioned_transaction(message);
        let resolved_transaction = VersionedTransactionResolved::new(&transaction);

        assert!(!FeeConfigUtil::is_fee_payer_in_signers(&resolved_transaction).unwrap());
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test 1: Fee payer as sender - should add to outflow
        let transfer_instruction = transfer(&fee_payer, &recipient, 100_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 100_000, "Transfer from fee payer should add to outflow");

        // Test 2: Fee payer as recipient - should subtract from outflow
        let sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&sender, &fee_payer, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Transfer to fee payer should subtract from outflow (saturating)");

        // Test 3: Other account as sender - should not affect outflow
        let other_sender = Pubkey::new_unique();
        let transfer_instruction = transfer(&other_sender, &recipient, 500_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[transfer_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Transfer from other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_transfer_with_seed() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

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
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
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
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
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
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test 1: Fee payer funding CreateAccount
        let create_instruction =
            create_account(&fee_payer, &new_account, 200_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 200_000, "CreateAccount funded by fee payer should add to outflow");

        // Test 2: Other account funding CreateAccount
        let other_funder = Pubkey::new_unique();
        let create_instruction =
            create_account(&other_funder, &new_account, 1_000_000, 100, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[create_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "CreateAccount funded by other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_create_account_with_seed() {
        let fee_payer = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

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
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 300_000,
            "CreateAccountWithSeed funded by fee payer should add to outflow"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_nonce_withdraw() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test 1: Fee payer as nonce account (outflow)
        let withdraw_instruction =
            withdraw_nonce_account(&fee_payer, &authority, &recipient, 50_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 50_000,
            "WithdrawNonceAccount from fee payer nonce should add to outflow"
        );

        // Test 2: Fee payer as recipient (inflow)
        let nonce_account = Pubkey::new_unique();
        let withdraw_instruction =
            withdraw_nonce_account(&nonce_account, &authority, &fee_payer, 25_000);
        let message =
            VersionedMessage::Legacy(Message::new(&[withdraw_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(
            outflow, 0,
            "WithdrawNonceAccount to fee payer should subtract from outflow (saturating)"
        );
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_allocate() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account {
            lamports: 0, // Start with 0 balance to test rent calculation
            ..Account::default()
        });

        // Test 1: Fee payer allocating space
        let allocate_instruction = allocate(&fee_payer, 1000);
        let message =
            VersionedMessage::Legacy(Message::new(&[allocate_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();

        // Calculate expected rent
        let rent = Rent::default();
        let expected_rent = rent.minimum_balance(1000);
        assert_eq!(outflow, expected_rent, "Allocate should add rent exemption cost");

        // Test 2: Other account allocating
        let other_account = Pubkey::new_unique();
        let allocate_instruction = allocate(&other_account, 1000);
        let message =
            VersionedMessage::Legacy(Message::new(&[allocate_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Allocate by other account should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_allocate_with_seed() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account {
            lamports: 0, // Start with 0 balance to test rent calculation
            ..Account::default()
        });

        // Test: Fee payer allocating with seed (fee payer at index 1)
        let allocate_instruction =
            allocate_with_seed(&fee_payer, &fee_payer, "test_seed", 2000, &SYSTEM_PROGRAM_ID);
        let message =
            VersionedMessage::Legacy(Message::new(&[allocate_instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();

        // Calculate expected rent
        let rent = Rent::default();
        let expected_rent = rent.minimum_balance(2000);
        assert_eq!(outflow, expected_rent, "AllocateWithSeed should add rent exemption cost");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_multiple_instructions() {
        let fee_payer = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let sender = Pubkey::new_unique();
        let new_account = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Multiple instructions involving fee payer
        let instructions = vec![
            transfer(&fee_payer, &recipient, 100_000), // +100,000
            transfer(&sender, &fee_payer, 30_000),     // -30,000
            create_account(&fee_payer, &new_account, 50_000, 100, &SYSTEM_PROGRAM_ID), // +50,000
        ];
        let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
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
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Test with non-system program - should not affect outflow
        let instruction = Instruction::new_with_bincode(
            fake_program,
            &[0u8],
            vec![], // no accounts needed for this test
        );
        let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&fee_payer)));
        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Non-system program should not affect outflow");
    }

    #[tokio::test]
    async fn test_calculate_fee_payer_outflow_invalid_account_indices() {
        let fee_payer = Pubkey::new_unique();
        let rpc_client = get_mock_rpc_client(&Account::default());

        // Create a message with invalid account indices by directly constructing CompiledInstruction
        let compiled_instruction = CompiledInstruction {
            program_id_index: 0,     // System program
            accounts: vec![99, 100], // Invalid indices that exceed account_keys length
            data: bincode::serialize(&SystemInstruction::Transfer { lamports: 1000 }).unwrap(),
        };

        let message = VersionedMessage::Legacy(solana_message::legacy::Message {
            header: solana_message::MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![SYSTEM_PROGRAM_ID, fee_payer], // Only 2 accounts
            recent_blockhash: solana_sdk::hash::Hash::default(),
            instructions: vec![compiled_instruction],
        });

        let outflow = FeeConfigUtil::calculate_fee_payer_outflow(
            &rpc_client,
            &fee_payer,
            &message,
            message.static_account_keys(),
        )
        .await
        .unwrap();
        assert_eq!(outflow, 0, "Invalid account indices should not cause outflow");
    }
}
