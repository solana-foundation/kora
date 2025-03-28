#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use spl_token_2022::extension::transfer_fee::{TransferFeeConfig, TransferFeeAmount};

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
} 