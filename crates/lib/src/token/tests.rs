#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_token_program_spl() {
        let program = TokenProgram::new(TokenType::Spl);
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_get_associated_token_address() {
        let program = TokenProgram::new(TokenType::Spl);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata, wallet);
        assert_ne!(ata, mint);
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

        assert_eq!(ix.program_id, program.program_id());
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

        assert_eq!(ix.program_id, program.program_id());
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
    }

    #[test]
    fn test_get_mint_decimals() {
        let program = TokenProgram::new(TokenType::Spl);
        let mint_data = vec![0; Mint::LEN];
        let result = program.get_mint_decimals(&mint_data);
        assert!(result.is_ok());
    }
} 