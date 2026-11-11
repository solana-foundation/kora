use solana_sdk::pubkey::Pubkey;
use std::any::Any;

pub trait TokenState: Any + Send + Sync {
    fn mint(&self) -> Pubkey;
    fn owner(&self) -> Pubkey;

    fn as_any(&self) -> &dyn Any;
}

pub trait TokenMint: Any + Send + Sync {
    fn decimals(&self) -> u8;

    fn as_any(&self) -> &dyn Any;
}

pub trait TokenInterface: Send + Sync {
    fn program_id(&self) -> Pubkey;

    fn unpack_token_account(
        &self,
        data: &[u8],
    ) -> Result<Box<dyn TokenState + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>;

    fn unpack_mint(
        &self,
        mint: &Pubkey,
        mint_data: &[u8],
    ) -> Result<Box<dyn TokenMint + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>;
}
