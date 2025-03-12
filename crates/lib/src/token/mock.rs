#[cfg(test)]
pub struct MockTokenProgram;

#[cfg(test)]
impl TokenInterface for MockTokenProgram {
    fn decode_transfer_instruction(
        &self,
        _data: &[u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(1000)
    }

    fn get_mint_decimals(
        &self,
        _mint_data: &[u8],
    ) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        Ok(9)
    }

    fn program_id(&self) -> Pubkey {
        Pubkey::new_unique()
    }

    fn create_initialize_account_instruction(
        &self,
        _account: &Pubkey,
        _mint: &Pubkey,
        _owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Instruction::new_with_bytes(Pubkey::new_unique(), &[], vec![]))
    }

    fn create_transfer_checked_instruction(
        &self,
        _source: &Pubkey,
        _mint: &Pubkey,
        _destination: &Pubkey,
        _authority: &Pubkey,
        _amount: u64,
        _decimals: u8,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Instruction::new_with_bytes(Pubkey::new_unique(), &[], vec![]))
    }

    fn unpack_token_account(
        &self,
        _data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        // Create a mock TokenState implementation
        struct MockTokenState {
            mint_key: Pubkey,
            owner_key: Pubkey,
        }

        impl TokenState for MockTokenState {
            fn mint(&self) -> Pubkey { self.mint_key }
            fn owner(&self) -> Pubkey { self.owner_key }
            fn amount(&self) -> u64 { 1000000 }
            fn decimals(&self) -> u8 { 9 }
        }

        Ok(Box::new(MockTokenState {
            mint_key: Pubkey::new_unique(),
            owner_key: Pubkey::new_unique(),
        }))
    }

    fn create_transfer_instruction(
        &self,
        _source: &Pubkey,
        _destination: &Pubkey,
        _authority: &Pubkey,
        _amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Instruction::new_with_bytes(Pubkey::new_unique(), &[], vec![]))
    }

    fn get_associated_token_address(&self, _wallet: &Pubkey, _mint: &Pubkey) -> Pubkey {
        Pubkey::new_unique()
    }

    fn create_associated_token_account_instruction(
        &self,
        _funding_account: &Pubkey,
        _wallet: &Pubkey,
        _mint: &Pubkey,
    ) -> Instruction {
        Instruction::new_with_bytes(Pubkey::new_unique(), &[], vec![])
    }
} 