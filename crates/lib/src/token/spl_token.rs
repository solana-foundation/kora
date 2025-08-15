use crate::token::interface::TokenMint;

use super::interface::{
    ParsedSplInstruction, SplInstructionCommand, SplInstructionType, TokenInterface, TokenState,
};
use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::instruction::{CompiledInstruction, Instruction};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token::{
    self,
    state::{Account as TokenAccountState, AccountState, Mint as MintState},
};

#[derive(Debug)]
pub struct TokenAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: u8,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
}

impl TokenState for TokenAccount {
    fn mint(&self) -> Pubkey {
        self.mint
    }
    fn owner(&self) -> Pubkey {
        self.owner
    }
    fn amount(&self) -> u64 {
        self.amount
    }
    fn decimals(&self) -> u8 {
        0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct SplMint {
    pub mint: Pubkey,
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}

impl TokenMint for SplMint {
    fn address(&self) -> Pubkey {
        self.mint
    }

    fn decimals(&self) -> u8 {
        self.decimals
    }

    fn mint_authority(&self) -> Option<Pubkey> {
        self.mint_authority
    }

    fn supply(&self) -> u64 {
        self.supply
    }

    fn freeze_authority(&self) -> Option<Pubkey> {
        self.freeze_authority
    }

    fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    fn get_token_program(&self) -> Box<dyn TokenInterface> {
        Box::new(TokenProgram::new())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct TokenProgram;

impl Default for TokenProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenProgram {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TokenInterface for TokenProgram {
    fn program_id(&self) -> Pubkey {
        spl_token::id()
    }

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let account = TokenAccountState::unpack(data)?;

        Ok(Box::new(TokenAccount {
            mint: account.mint,
            owner: account.owner,
            amount: account.amount,
            delegate: account.delegate.into(),
            state: match account.state {
                AccountState::Uninitialized => 0,
                AccountState::Initialized => 1,
                AccountState::Frozen => 2,
            },
            is_native: account.is_native.into(),
            delegated_amount: account.delegated_amount,
            close_authority: account.close_authority.into(),
        }))
    }

    fn create_initialize_account_instruction(
        &self,
        account: &Pubkey,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token::instruction::initialize_account(&self.program_id(), account, mint, owner)?)
    }

    fn create_transfer_instruction(
        &self,
        source: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token::instruction::transfer(
            &self.program_id(),
            source,
            destination,
            authority,
            &[],
            amount,
        )?)
    }

    fn create_transfer_checked_instruction(
        &self,
        source: &Pubkey,
        mint: &Pubkey,
        destination: &Pubkey,
        authority: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<Instruction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(spl_token::instruction::transfer_checked(
            &self.program_id(),
            source,
            mint,
            destination,
            authority,
            &[],
            amount,
            decimals,
        )?)
    }

    fn get_associated_token_address(&self, wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(wallet, mint, &self.program_id())
    }

    fn create_associated_token_account_instruction(
        &self,
        funding_account: &Pubkey,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        create_associated_token_account(funding_account, wallet, mint, &self.program_id())
    }

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        let mint_state = MintState::unpack(mint_data)?;

        Ok(Box::new(SplMint {
            mint: *mint,
            mint_authority: mint_state.mint_authority.into(),
            supply: mint_state.supply,
            decimals: mint_state.decimals,
            is_initialized: mint_state.is_initialized,
            freeze_authority: mint_state.freeze_authority.into(),
        }))
    }

    fn decode_transfer_instruction(
        &self,
        data: &[u8],
    ) -> Result<(u64, Option<usize>), Box<dyn std::error::Error + Send + Sync>> {
        let instruction = spl_token::instruction::TokenInstruction::unpack(data)?;
        match instruction {
            spl_token::instruction::TokenInstruction::Transfer { amount } => Ok((amount, None)),
            spl_token::instruction::TokenInstruction::TransferChecked { amount, .. } => {
                Ok((amount, Some(1)))
            }
            _ => Err("Not a transfer instruction".into()),
        }
    }

    fn process_spl_instruction(
        &self,
        instruction_data: &[u8],
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
        fee_payer_pubkey: &Pubkey,
        command: SplInstructionCommand,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parsed) = self.parse_spl_instruction(instruction_data)? {
            command.execute(&parsed, ix, account_keys, fee_payer_pubkey)
        } else {
            Ok(false)
        }
    }

    async fn get_and_validate_amount_for_payment<'a>(
        &self,
        _rpc_client: &'a RpcClient,
        _token_account: Option<&'a dyn TokenState>,
        _mint_account: Option<&'a dyn TokenMint>,
        amount: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(amount)
    }
}

impl TokenProgram {
    fn parse_spl_instruction(
        &self,
        instruction_data: &[u8],
    ) -> Result<Option<ParsedSplInstruction>, Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(spl_ix) = spl_token::instruction::TokenInstruction::unpack(instruction_data) {
            let parsed = match spl_ix {
                spl_token::instruction::TokenInstruction::Transfer { amount } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::Transfer,
                        authority_index: 2,
                        amount: Some(amount),
                        program_id: self.program_id(),
                    }
                }
                spl_token::instruction::TokenInstruction::TransferChecked { amount, .. } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::TransferChecked,
                        authority_index: 3,
                        amount: Some(amount),
                        program_id: self.program_id(),
                    }
                }
                spl_token::instruction::TokenInstruction::Burn { amount } => ParsedSplInstruction {
                    instruction_type: SplInstructionType::Burn,
                    authority_index: 2,
                    amount: Some(amount),
                    program_id: self.program_id(),
                },
                spl_token::instruction::TokenInstruction::BurnChecked { amount, .. } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::BurnChecked,
                        authority_index: 2,
                        amount: Some(amount),
                        program_id: self.program_id(),
                    }
                }
                spl_token::instruction::TokenInstruction::CloseAccount { .. } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::CloseAccount,
                        authority_index: 2,
                        amount: None,
                        program_id: self.program_id(),
                    }
                }
                spl_token::instruction::TokenInstruction::Approve { amount } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::Approve,
                        authority_index: 2,
                        amount: Some(amount),
                        program_id: self.program_id(),
                    }
                }
                spl_token::instruction::TokenInstruction::ApproveChecked { amount, .. } => {
                    ParsedSplInstruction {
                        instruction_type: SplInstructionType::ApproveChecked,
                        authority_index: 3,
                        amount: Some(amount),
                        program_id: self.program_id(),
                    }
                }
                _ => return Ok(None), // Unknown instruction
            };
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_pack::Pack;
    use solana_sdk::pubkey::Pubkey;
    use spl_token::state::Account as SplTokenAccount;

    #[test]
    fn test_token_program_spl() {
        let program = TokenProgram::new();
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_token_program_creation() {
        let program = TokenProgram::new();
        assert_eq!(program.program_id(), spl_token::id());
    }

    #[test]
    fn test_account_from_bytes() {
        let mut bytes = vec![0u8; SplTokenAccount::LEN];
        // Pack a dummy account to make it valid
        let dummy_account = SplTokenAccount {
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            amount: 0,
            state: spl_token::state::AccountState::Initialized,
            ..Default::default()
        };
        dummy_account.pack_into_slice(&mut bytes);

        let account = TokenProgram::new().unpack_token_account(&bytes).unwrap();
        let token_account = account.as_any().downcast_ref::<TokenAccount>().unwrap();
        assert_eq!(token_account.amount, 0);
    }

    #[test]
    fn test_create_transfer_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();

        // Create the instruction directly for testing
        let ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &source,
            &dest,
            &authority,
            &[],
            100,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token::id());
    }

    #[test]
    fn test_create_transfer_checked_instruction() {
        let source = Pubkey::new_unique();
        let dest = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Create the instruction directly for testing
        let ix = spl_token::instruction::transfer_checked(
            &spl_token::id(),
            &source,
            &mint,
            &dest,
            &authority,
            &[],
            100,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token::id());
    }

    #[test]
    fn test_get_associated_token_address() {
        let program = TokenProgram::new();
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ata = program.get_associated_token_address(&wallet, &mint);
        assert_ne!(ata, wallet);
        assert_ne!(ata, mint);
    }

    #[test]
    fn test_create_ata_instruction() {
        let program = TokenProgram::new();
        let funder = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = program.create_associated_token_account_instruction(&funder, &owner, &mint);

        assert_eq!(ix.program_id, spl_associated_token_account::id());
    }
}
