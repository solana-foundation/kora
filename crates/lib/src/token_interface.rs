use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use async_trait::async_trait;
use crate::error::KoraError;
use std::sync::Arc;
use solana_sdk::signer::{keypair::Keypair, Signer};
use mockall::automock;

/// Trait defining the interface for token operations
#[automock]
#[async_trait]
pub trait TokenInterface {
    /// Validates if a given token address is valid for this token type
    async fn validate_token(&self, rpc_client: &RpcClient, token: &str) -> Result<(), KoraError>;

    /// Validates multiple token addresses
    async fn validate_tokens(&self, rpc_client: &RpcClient, tokens: &[String]) -> Result<(), KoraError>;

    /// Gets or creates a token account for a user
    async fn get_or_create_token_account(
        &self,
        rpc_client: &RpcClient,
        user_pubkey: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(Pubkey, Option<Transaction>), KoraError>;

    /// Gets or creates multiple token accounts for a user
    async fn get_or_create_multiple_token_accounts(
        &self,
        rpc_client: &RpcClient,
        user_pubkey: &Pubkey,
        mints: &[Pubkey],
    ) -> Result<(Vec<Pubkey>, Option<Transaction>), KoraError>;

    /// Calculates the value of a token amount in lamports
    async fn calculate_token_value_in_lamports(
        &self,
        amount: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError>;
}

/// Implementation for the tokenkeg token type
pub struct TokenKeg;

#[async_trait]
impl TokenInterface for TokenKeg {
    async fn validate_token(&self, rpc_client: &RpcClient, token: &str) -> Result<(), KoraError> {
        crate::token::check_valid_token(rpc_client, token).await
    }

    async fn validate_tokens(&self, rpc_client: &RpcClient, tokens: &[String]) -> Result<(), KoraError> {
        crate::token::check_valid_tokens(rpc_client, tokens).await
    }

    async fn get_or_create_token_account(
        &self,
        rpc_client: &RpcClient,
        user_pubkey: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(Pubkey, Option<Transaction>), KoraError> {
        use crate::cache::TokenAccountCache;
        
        // Create a temporary cache for this operation
        let cache = TokenAccountCache::new("redis://localhost")?;
        let rpc_arc = Arc::new(RpcClient::new(rpc_client.url().to_string()));
        crate::account::get_or_create_token_account(
            &rpc_arc,
            &cache,
            user_pubkey,
            mint,
        ).await
    }

    async fn get_or_create_multiple_token_accounts(
        &self,
        rpc_client: &RpcClient,
        user_pubkey: &Pubkey,
        mints: &[Pubkey],
    ) -> Result<(Vec<Pubkey>, Option<Transaction>), KoraError> {
        use crate::cache::TokenAccountCache;
        
        // Create a temporary cache for this operation
        let cache = TokenAccountCache::new("redis://localhost")?;
        let rpc_arc = Arc::new(RpcClient::new(rpc_client.url().to_string()));
        crate::account::get_or_create_multiple_token_accounts(
            &rpc_arc,
            &cache,
            user_pubkey,
            mints,
        ).await
    }

    async fn calculate_token_value_in_lamports(
        &self,
        amount: u64,
        mint: &Pubkey,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        crate::transaction::calculate_token_value_in_lamports(amount, mint, rpc_client).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::test_utils::setup_test_rpc_client;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_validate_token() {
        let mut mock_token_interface = MockTokenInterface::new();
        mock_token_interface
            .expect_validate_token()
            .with(always(), eq("test_token"))
            .returning(|_, _| Ok(()));

        let rpc_client = setup_test_rpc_client();
        let result = mock_token_interface.validate_token(&rpc_client, "test_token").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_tokens() {
        let mut mock_token_interface = MockTokenInterface::new();
        mock_token_interface
            .expect_validate_tokens()
            .with(always(), eq(vec!["token1".to_string(), "token2".to_string()]))
            .returning(|_, _| Ok(()));

        let rpc_client = setup_test_rpc_client();
        let result = mock_token_interface
            .validate_tokens(&rpc_client, &["token1".to_string(), "token2".to_string()])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_or_create_token_account() {
        let mut mock_token_interface = MockTokenInterface::new();
        let user = Keypair::new();
        let mint = Keypair::new();
        let token_account = Pubkey::new_unique();

        mock_token_interface
            .expect_get_or_create_token_account()
            .with(always(), always(), always())
            .returning(move |_, _, _| Ok((token_account, None)));

        let rpc_client = setup_test_rpc_client();
        let result = mock_token_interface
            .get_or_create_token_account(&rpc_client, &user.pubkey(), &mint.pubkey())
            .await;
        
        assert!(result.is_ok());
        let (account, tx) = result.unwrap();
        assert_eq!(account, token_account);
        assert!(tx.is_none());
    }

    #[tokio::test]
    async fn test_get_or_create_multiple_token_accounts() {
        let mut mock_token_interface = MockTokenInterface::new();
        let user = Keypair::new();
        let mints = vec![Keypair::new().pubkey(), Keypair::new().pubkey()];
        let token_accounts = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        mock_token_interface
            .expect_get_or_create_multiple_token_accounts()
            .with(always(), always(), always())
            .returning(move |_, _, _| Ok((token_accounts.clone(), None)));

        let rpc_client = setup_test_rpc_client();
        let result = mock_token_interface
            .get_or_create_multiple_token_accounts(&rpc_client, &user.pubkey(), &mints)
            .await;
        
        assert!(result.is_ok());
        let (accounts, tx) = result.unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(tx.is_none());
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports() {
        let mut mock_token_interface = MockTokenInterface::new();
        let mint = Keypair::new();
        let expected_lamports = 1_000_000;

        mock_token_interface
            .expect_calculate_token_value_in_lamports()
            .with(always(), always(), always())
            .returning(move |_, _, _| Ok(expected_lamports));

        let rpc_client = setup_test_rpc_client();
        let result = mock_token_interface
            .calculate_token_value_in_lamports(100, &mint.pubkey(), &rpc_client)
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_lamports);
    }
} 