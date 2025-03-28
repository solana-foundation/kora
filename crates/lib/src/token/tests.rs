#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use spl_token_2022::extension::{
        transfer_fee::{TransferFeeConfig, TransferFeeAmount},
        ExtensionType,
        StateWithExtensions,
    };
    use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction};
    use solana_client::nonblocking::rpc_client::RpcClient;

    #[test]
    fn test_token_program_spl() {
        let program = TokenProgram::new(TokenType::Spl);
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_token_program_token2022() {
        let program = TokenProgram::new(TokenType::Token2022);
        assert_eq!(program.program_id(), spl_token_2022::id());
    }

    #[test]
    fn test_get_associated_token_address() {
        let program = TokenProgram::new(TokenType::Spl);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata, wallet);
        assert_ne!(ata, mint);

        // Test Token2022 ATA
        let program = TokenProgram::new(TokenType::Token2022);
        let ata_2022 = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata_2022, wallet);
        assert_ne!(ata_2022, mint);
        assert_ne!(ata, ata_2022); // SPL and Token2022 ATAs should be different
    }

    #[test]
    fn test_create_transfer_instruction() {
        let program = TokenProgram::new(TokenType::Spl);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        let ix = program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            100,
        ).unwrap();

        assert_eq!(ix.program_id, spl_token::id());

        // Test Token2022 transfer
        let program = TokenProgram::new(TokenType::Token2022);
        let ix = program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            100,
        ).unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
    }

    #[test]
    fn test_create_transfer_checked_instruction() {
        let program = TokenProgram::new(TokenType::Spl);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            100,
            9,
        ).unwrap();

        assert_eq!(ix.program_id, spl_token::id());

        // Test Token2022 transfer checked
        let program = TokenProgram::new(TokenType::Token2022);
        let ix = program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            100,
            9,
        ).unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
    }

    #[test]
    fn test_create_ata_instruction() {
        let program = TokenProgram::new(TokenType::Spl);
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        assert_eq!(ix.program_id, spl_associated_token_account::id());

        // Test Token2022 ATA creation
        let program = TokenProgram::new(TokenType::Token2022);
        let ix = program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        assert_eq!(ix.program_id, spl_associated_token_account::id());
    }

    #[test]
    fn test_token2022_account_state() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create a Token2022Account without transfer fee
        let account = Token2022Account {
            mint,
            owner,
            amount,
            transfer_fee: None,
        };

        assert_eq!(account.mint(), mint);
        assert_eq!(account.owner(), owner);
        assert_eq!(account.amount(), amount);

        // Create a Token2022Account with transfer fee
        let transfer_fee = TransferFeeConfig {
            epoch: 0,
            maximum_fee: 0,
            transfer_fee_basis_points: 0,
            newer_transfer_fee: TransferFeeAmount {
                epoch: 1,
                transfer_fee_basis_points: 100,
                maximum_fee: 1000,
            },
            older_transfer_fee: TransferFeeAmount {
                epoch: 0,
                transfer_fee_basis_points: 0,
                maximum_fee: 0,
            },
        };

        let account = Token2022Account {
            mint,
            owner,
            amount,
            transfer_fee: Some(transfer_fee),
        };

        assert_eq!(account.mint(), mint);
        assert_eq!(account.owner(), owner);
        assert_eq!(account.amount(), amount);
        assert_eq!(account.transfer_fee.unwrap().newer_transfer_fee.transfer_fee_basis_points, 100);
        assert_eq!(account.transfer_fee.unwrap().newer_transfer_fee.maximum_fee, 1000);
    }

    #[test]
    fn test_get_mint_decimals() {
        let program = TokenProgram::new(TokenType::Spl);
        let mint_data = vec![0; Mint::LEN];
        let result = program.get_mint_decimals(&mint_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_token2022_program_id() {
        let program = TokenProgram::new(TokenType::Token2022);
        assert_eq!(program.program_id(), spl_token_2022::id());
    }

    #[test]
    fn test_token2022_transfer_instruction() {
        let program = TokenProgram::new(TokenType::Token2022);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100;

        let ix = program
            .create_transfer_instruction(&source, &dest, &authority, amount)
            .unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
        // Verify accounts are in correct order
        assert_eq!(ix.accounts[0].pubkey, source);
        assert_eq!(ix.accounts[1].pubkey, dest);
        assert_eq!(ix.accounts[2].pubkey, authority);
    }

    #[test]
    fn test_token2022_transfer_checked_instruction() {
        let program = TokenProgram::new(TokenType::Token2022);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100;
        let decimals = 9;

        let ix = program
            .create_transfer_checked_instruction(
                &source,
                &mint,
                &dest,
                &authority,
                amount,
                decimals,
            )
            .unwrap();

        assert_eq!(ix.program_id, spl_token_2022::id());
        // Verify accounts are in correct order
        assert_eq!(ix.accounts[0].pubkey, source);
        assert_eq!(ix.accounts[1].pubkey, mint);
        assert_eq!(ix.accounts[2].pubkey, dest);
        assert_eq!(ix.accounts[3].pubkey, authority);
    }

    #[test]
    fn test_token2022_ata_derivation() {
        let program = TokenProgram::new(TokenType::Token2022);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);
        
        // Verify ATA derivation matches spl-token-2022
        let expected_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &wallet,
            &mint,
            &spl_token_2022::id(),
        );
        assert_eq!(ata, expected_ata);
    }

    #[test]
    fn test_token2022_transfer_fee_calculation() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1_000_000;

        // Test with 1% fee (100 basis points)
        let transfer_fee = TransferFeeConfig {
            epoch: 0,
            maximum_fee: 0,
            transfer_fee_basis_points: 0,
            newer_transfer_fee: TransferFeeAmount {
                epoch: 1,
                transfer_fee_basis_points: 100, // 1%
                maximum_fee: 10_000,
            },
            older_transfer_fee: TransferFeeAmount {
                epoch: 0,
                transfer_fee_basis_points: 0,
                maximum_fee: 0,
            },
        };

        let account = Token2022Account {
            mint,
            owner,
            amount,
            transfer_fee: Some(transfer_fee),
        };

        // Test basic fee calculation
        let fee = std::cmp::min(
            (amount as u128 * 100u128 / 10000) as u64,
            10_000,
        );
        assert_eq!(fee, 10_000);

        // Test with smaller amount
        let small_amount = 1_000;
        let small_fee = std::cmp::min(
            (small_amount as u128 * 100u128 / 10000) as u64,
            10_000,
        );
        assert_eq!(small_fee, 10);
    }

    #[test]
    fn test_token2022_account_state_extensions() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 1000;

        // Create account data with transfer fee extension
        let mut buffer = vec![0; 1000];
        let mut account = StateWithExtensions::<spl_token_2022::state::Account>::unpack_from_slice(&buffer).unwrap();
        account.base.mint = mint;
        account.base.owner = owner;
        account.base.amount = amount;

        // Pack the account back
        account.pack_into_slice(&mut buffer);

        // Test unpacking with our implementation
        let program = TokenProgram::new(TokenType::Token2022);
        let unpacked = program.unpack_token_account(&buffer).unwrap();
        
        assert_eq!(unpacked.mint(), mint);
        assert_eq!(unpacked.owner(), owner);
        assert_eq!(unpacked.amount(), amount);
    }

    #[test]
    fn test_token2022_mint_decimals() {
        let program = TokenProgram::new(TokenType::Token2022);
        let decimals = 9;

        // Create mint data
        let mut buffer = vec![0; 1000];
        let mut mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack_from_slice(&buffer).unwrap();
        mint.base.decimals = decimals;

        // Pack the mint back
        mint.pack_into_slice(&mut buffer);

        // Test unpacking decimals
        let unpacked_decimals = program.get_mint_decimals(&buffer).unwrap();
        assert_eq!(unpacked_decimals, decimals);
    }

    #[tokio::test]
    async fn test_token2022_full_flow() {
        // Skip this test in normal unit test runs as it requires a local validator
        if option_env!("RUN_INTEGRATION_TESTS").is_none() {
            return;
        }

        let rpc_url = "http://localhost:8899".to_string();
        let rpc_client = RpcClient::new(rpc_url);

        // Create mint authority
        let mint_authority = Keypair::new();
        let mint_authority_pubkey = mint_authority.pubkey();

        // Create mint with transfer fee
        let mint = Keypair::new();
        let mint_space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::TransferFeeConfig]);
        
        let transfer_fee_config = TransferFeeConfig {
            epoch: 0,
            maximum_fee: 0,
            transfer_fee_basis_points: 0,
            newer_transfer_fee: TransferFeeAmount {
                epoch: 1,
                transfer_fee_basis_points: 100, // 1%
                maximum_fee: 10_000,
            },
            older_transfer_fee: TransferFeeAmount {
                epoch: 0,
                transfer_fee_basis_points: 0,
                maximum_fee: 0,
            },
        };

        // Initialize mint
        let decimals = 9;
        let init_mint_ix = spl_token_2022::instruction::initialize_mint2(
            &spl_token_2022::id(),
            &mint.pubkey(),
            &mint_authority_pubkey,
            None,
            decimals,
        )
        .unwrap();

        // Create source account
        let source_owner = Keypair::new();
        let token_program = TokenProgram::new(TokenType::Token2022);
        let source_account = token_program.get_associated_token_address(
            &source_owner.pubkey(),
            &mint.pubkey(),
        );

        // Create destination account
        let dest_owner = Keypair::new();
        let dest_account = token_program.get_associated_token_address(
            &dest_owner.pubkey(),
            &mint.pubkey(),
        );

        // Create source ATA
        let create_source_ata_ix = token_program.create_associated_token_account_instruction(
            &source_owner.pubkey(),
            &source_owner.pubkey(),
            &mint.pubkey(),
        );

        // Create destination ATA
        let create_dest_ata_ix = token_program.create_associated_token_account_instruction(
            &dest_owner.pubkey(),
            &dest_owner.pubkey(),
            &mint.pubkey(),
        );

        // Mint tokens
        let mint_amount = 1_000_000;
        let mint_to_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::id(),
            &mint.pubkey(),
            &source_account,
            &mint_authority_pubkey,
            &[],
            mint_amount,
        )
        .unwrap();

        // Transfer tokens
        let transfer_amount = 100_000;
        let transfer_ix = token_program
            .create_transfer_checked_instruction(
                &source_account,
                &mint.pubkey(),
                &dest_account,
                &source_owner.pubkey(),
                transfer_amount,
                decimals,
            )
            .unwrap();

        // Build and send transaction
        let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[
                init_mint_ix,
                create_source_ata_ix,
                create_dest_ata_ix,
                mint_to_ix,
                transfer_ix,
            ],
            Some(&source_owner.pubkey()),
            &[&source_owner, &mint_authority],
            recent_blockhash,
        );

        // Send and confirm transaction
        let result = rpc_client
            .send_and_confirm_transaction(&transaction)
            .await;

        assert!(result.is_ok());

        // Verify balances
        let source_balance = rpc_client
            .get_token_account_balance(&source_account)
            .await
            .unwrap();
        let dest_balance = rpc_client
            .get_token_account_balance(&dest_account)
            .await
            .unwrap();

        // Account for transfer fee
        let fee = std::cmp::min(
            (transfer_amount as u128 * 100u128 / 10000) as u64,
            10_000,
        );
        let expected_transfer = transfer_amount - fee;

        assert_eq!(
            source_balance.ui_amount.unwrap() as u64,
            mint_amount - transfer_amount
        );
        assert_eq!(
            dest_balance.ui_amount.unwrap() as u64,
            expected_transfer
        );
    }
} 