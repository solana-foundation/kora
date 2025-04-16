#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use spl_token::state::{Account as TokenAccountState, AccountState, Mint};

    fn create_test_token_account(
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        state: u8,
        is_native: Option<u64>,
    ) -> Vec<u8> {
        let account = TokenAccountState {
            mint,
            owner,
            amount,
            delegate: None,
            state,
            is_native,
            delegated_amount: 0,
            close_authority: None,
        };
        let mut data = vec![0; TokenAccountState::LEN];
        TokenAccountState::pack(account, &mut data).unwrap();
        data
    }

    fn create_test_token2022_account(
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        state: u8,
        is_native: Option<u64>,
    ) -> Vec<u8> {
        let account = spl_token_2022::state::Account {
            mint,
            owner,
            amount,
            delegate: None,
            state,
            is_native,
            delegated_amount: 0,
            close_authority: None,
            extension_data: vec![],
        };
        let mut data = vec![0; spl_token_2022::state::Account::LEN];
        spl_token_2022::state::Account::pack(account, &mut data).unwrap();
        data
    }

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
    fn test_unpack_token_account_spl() {
        let program = TokenProgram::new(TokenType::Spl);
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 100;

        // Test initialized account
        let data = create_test_token_account(
            mint,
            owner,
            amount,
            AccountState::Initialized as u8,
            None,
        );
        let account = program.unpack_token_account(&data).unwrap();
        let token_account = account.as_any().downcast_ref::<TokenAccount>().unwrap();
        assert_eq!(token_account.mint, mint);
        assert_eq!(token_account.owner, owner);
        assert_eq!(token_account.amount, amount);
        assert_eq!(token_account.state, AccountState::Initialized as u8);

        // Test uninitialized account
        let data = create_test_token_account(
            mint,
            owner,
            amount,
            AccountState::Uninitialized as u8,
            None,
        );
        assert!(program.unpack_token_account(&data).is_err());

        // Test frozen account
        let data = create_test_token_account(
            mint,
            owner,
            amount,
            AccountState::Frozen as u8,
            None,
        );
        assert!(program.unpack_token_account(&data).is_err());
    }

    #[test]
    fn test_unpack_token_account_token2022() {
        let program = TokenProgram::new(TokenType::Token2022);
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let amount = 100;

        // Test initialized account
        let data = create_test_token2022_account(
            mint,
            owner,
            amount,
            AccountState::Initialized as u8,
            None,
        );
        let account = program.unpack_token_account(&data).unwrap();
        let token_account = account.as_any().downcast_ref::<TokenAccount>().unwrap();
        assert_eq!(token_account.mint, mint);
        assert_eq!(token_account.owner, owner);
        assert_eq!(token_account.amount, amount);
        assert_eq!(token_account.state, AccountState::Initialized as u8);

        // Test uninitialized account
        let data = create_test_token2022_account(
            mint,
            owner,
            amount,
            AccountState::Uninitialized as u8,
            None,
        );
        assert!(program.unpack_token_account(&data).is_err());

        // Test frozen account
        let data = create_test_token2022_account(
            mint,
            owner,
            amount,
            AccountState::Frozen as u8,
            None,
        );
        assert!(program.unpack_token_account(&data).is_err());
    }

    #[test]
    fn test_decode_transfer_instruction() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let amount = 100u64;

        // Test regular transfer
        let spl_ix = spl_program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            amount,
        ).unwrap();
        let decoded_spl_amount = spl_program.decode_transfer_instruction(&spl_ix.data).unwrap();
        assert_eq!(decoded_spl_amount, amount);

        let token2022_ix = token2022_program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            amount,
        ).unwrap();
        let decoded_token2022_amount = token2022_program.decode_transfer_instruction(&token2022_ix.data).unwrap();
        assert_eq!(decoded_token2022_amount, amount);

        // Test transfer checked
        let spl_ix = spl_program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            amount,
            9,
        ).unwrap();
        let decoded_spl_amount = spl_program.decode_transfer_instruction(&spl_ix.data).unwrap();
        assert_eq!(decoded_spl_amount, amount);

        let token2022_ix = token2022_program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            amount,
            9,
        ).unwrap();
        let decoded_token2022_amount = token2022_program.decode_transfer_instruction(&token2022_ix.data).unwrap();
        assert_eq!(decoded_token2022_amount, amount);

        // Test invalid instruction
        let invalid_data = vec![0; 10];
        assert!(spl_program.decode_transfer_instruction(&invalid_data).is_err());
        assert!(token2022_program.decode_transfer_instruction(&invalid_data).is_err());
    }

    #[test]
    fn test_get_associated_token_address() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let spl_ata = spl_program.get_associated_token_address(&wallet, &mint);
        let token2022_ata = token2022_program.get_associated_token_address(&wallet, &mint);

        assert_ne!(spl_ata, wallet);
        assert_ne!(spl_ata, mint);
        assert_ne!(token2022_ata, wallet);
        assert_ne!(token2022_ata, mint);
        assert_ne!(spl_ata, token2022_ata);

        // Verify ATA derivation
        let expected_spl_ata = get_associated_token_address_with_program_id(
            &wallet,
            &mint,
            &spl_token::id(),
        );
        let expected_token2022_ata = get_associated_token_address_with_program_id(
            &wallet,
            &mint,
            &spl_token_2022::id(),
        );

        assert_eq!(spl_ata, expected_spl_ata);
        assert_eq!(token2022_ata, expected_token2022_ata);
    }

    #[test]
    fn test_create_ata_instruction() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let spl_ix = spl_program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        let token2022_ix = token2022_program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        // Both should use the ATA program
        assert_eq!(spl_ix.program_id, spl_associated_token_account::id());
        assert_eq!(token2022_ix.program_id, spl_associated_token_account::id());

        // But should have different data due to different token programs
        assert_ne!(spl_ix.data, token2022_ix.data);

        // Verify accounts are in correct order
        assert_eq!(spl_ix.accounts[0].pubkey, funder); // payer
        assert_eq!(spl_ix.accounts[2].pubkey, owner); // wallet
        assert_eq!(spl_ix.accounts[3].pubkey, mint); // mint
        assert_eq!(spl_ix.accounts[4].pubkey, spl_token::id()); // token program

        assert_eq!(token2022_ix.accounts[0].pubkey, funder); // payer
        assert_eq!(token2022_ix.accounts[2].pubkey, owner); // wallet
        assert_eq!(token2022_ix.accounts[3].pubkey, mint); // mint
        assert_eq!(token2022_ix.accounts[4].pubkey, spl_token_2022::id()); // token program
    }

    #[test]
    fn test_create_transfer_instruction() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        let spl_ix = spl_program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            100,
        ).unwrap();

        let token2022_ix = token2022_program.create_transfer_instruction(
            &source,
            &dest,
            &authority,
            100,
        ).unwrap();

        assert_eq!(spl_ix.program_id, spl_program.program_id());
        assert_eq!(token2022_ix.program_id, token2022_program.program_id());
    }

    #[test]
    fn test_create_transfer_checked_instruction() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let spl_ix = spl_program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            100,
            9,
        ).unwrap();

        let token2022_ix = token2022_program.create_transfer_checked_instruction(
            &source,
            &mint,
            &dest,
            &authority,
            100,
            9,
        ).unwrap();

        assert_eq!(spl_ix.program_id, spl_program.program_id());
        assert_eq!(token2022_ix.program_id, token2022_program.program_id());
    }

    #[test]
    fn test_create_ata_instruction() {
        let spl_program = TokenProgram::new(TokenType::Spl);
        let token2022_program = TokenProgram::new(TokenType::Token2022);
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let spl_ix = spl_program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        let token2022_ix = token2022_program.create_associated_token_account_instruction(
            &funder,
            &owner,
            &mint,
        );

        assert_eq!(spl_ix.program_id, spl_associated_token_account::id());
        assert_eq!(token2022_ix.program_id, spl_associated_token_account::id());
        assert_ne!(spl_ix.data, token2022_ix.data);
    }

    #[test]
    fn test_get_mint_decimals() {
        let program = TokenProgram::new(TokenType::Spl);
        let mint_data = vec![0; Mint::LEN];
        let result = program.get_mint_decimals(&mint_data);
        assert!(result.is_ok());
    }
} 