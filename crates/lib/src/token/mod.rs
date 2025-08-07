use crate::{
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::{str::FromStr, time::Duration};
pub mod interface;
pub mod token;
pub mod token22;

pub use interface::{TokenInterface, TokenState};
pub use token::{TokenAccount, TokenProgram};
pub use token22::{Token2022Account, Token2022Program};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Spl,
    Token2022,
}

impl TokenType {
    pub fn program_id(&self, token_interface: &impl TokenInterface) -> Pubkey {
        match self {
            TokenType::Spl => token_interface.program_id(),
            TokenType::Token2022 => token_interface.program_id(),
        }
    }
}

pub fn check_valid_tokens(tokens: &[String]) -> Result<Vec<Pubkey>, KoraError> {
    tokens
        .iter()
        .map(|token| {
            Pubkey::from_str(token)
                .map_err(|_| KoraError::ValidationError(format!("Invalid token address: {token}")))
        })
        .collect()
}

pub async fn get_token_price_and_decimals(
    mint: &Pubkey,
    price_source: PriceSource,
    rpc_client: &RpcClient,
) -> Result<(TokenPrice, u8), KoraError> {
    let mint_account = rpc_client.get_account(mint).await?;

    let is_token2022 = mint_account.owner == spl_token_2022::id();
    let token_program =
        TokenProgram::new(if is_token2022 { TokenType::Token2022 } else { TokenType::Spl });

    let decimals = token_program.get_mint_decimals(&mint_account.data)?;

    let oracle =
        RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source));

    // Get token price in SOL directly
    let token_price = oracle
        .get_token_price(&mint.to_string())
        .await
        .map_err(|e| KoraError::RpcError(format!("Failed to fetch token price: {e}")))?;

    Ok((token_price, decimals))
}

pub async fn calculate_token_value_in_lamports(
    amount: u64,
    mint: &Pubkey,
    price_source: PriceSource,
    rpc_client: &RpcClient,
) -> Result<u64, KoraError> {
    let (token_price, decimals) =
        get_token_price_and_decimals(mint, price_source, rpc_client).await?;

    // Convert token amount to its real value based on decimals and multiply by SOL price
    let token_amount = amount as f64 / 10f64.powi(decimals as i32);
    let sol_amount = token_amount * token_price.price;

    // Convert SOL to lamports and round down
    let lamports = (sol_amount * LAMPORTS_PER_SOL as f64).floor() as u64;

    Ok(lamports)
}

pub async fn calculate_lamports_value_in_token(
    lamports: u64,
    mint: &Pubkey,
    price_source: &PriceSource,
    rpc_client: &RpcClient,
) -> Result<f64, KoraError> {
    let (token_price, decimals) =
        get_token_price_and_decimals(mint, price_source.clone(), rpc_client).await?;

    // Convert lamports to SOL, then to token amount
    let fee_in_sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
    let fee_in_token_base_units = fee_in_sol / token_price.price;
    let fee_in_token = fee_in_token_base_units * 10f64.powi(decimals as i32);

    Ok(fee_in_token)
}

#[cfg(test)]
mod tests_token {
    use super::*;
    use base64::Engine;
    use serde_json::json;
    use solana_client::rpc_request::RpcRequest;
    use solana_program::program_pack::Pack;
    use solana_sdk::{account::Account, program_option::COption};
    use spl_token::state::Mint;
    use spl_token_2022::state::Mint as Mint2022;
    use std::{collections::HashMap, sync::Arc};

    fn get_mock_rpc_client(account: &Account) -> Arc<RpcClient> {
        let mut mocks = HashMap::new();
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(&account.data);
        mocks.insert(
            RpcRequest::GetAccountInfo,
            json!({
                "context": {
                    "slot": 1
                },
                "value": {
                    "data": [encoded_data, "base64"],
                    "executable": account.executable,
                    "lamports": account.lamports,
                    "owner": account.owner.to_string(),
                    "rentEpoch": account.rent_epoch
                }
            }),
        );
        Arc::new(RpcClient::new_mock_with_mocks("http://localhost:8899".to_string(), mocks))
    }

    fn create_mock_spl_mint_account(decimals: u8) -> Account {
        let mint_data = Mint {
            mint_authority: COption::Some(Pubkey::new_unique()),
            supply: 1_000_000_000_000,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        let mut data = vec![0u8; Mint::LEN];
        mint_data.pack_into_slice(&mut data);

        Account { lamports: 0, data, owner: spl_token::id(), executable: false, rent_epoch: 0 }
    }

    fn create_mock_token2022_mint_account(decimals: u8) -> Account {
        let mint_data = Mint2022 {
            mint_authority: COption::Some(Pubkey::new_unique()),
            supply: 1_000_000_000_000,
            decimals,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        let mut data = vec![0u8; Mint2022::LEN];
        mint_data.pack_into_slice(&mut data);

        Account { lamports: 0, data, owner: spl_token_2022::id(), executable: false, rent_epoch: 0 }
    }

    #[tokio::test]
    async fn test_check_valid_tokens() {
        let valid_tokens = vec![
            "So11111111111111111111111111111111111111112".to_string(),
            "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
        ];
        let result = check_valid_tokens(&valid_tokens).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].to_string(), "So11111111111111111111111111111111111111112");
        assert_eq!(result[1].to_string(), "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
    }

    #[tokio::test]
    async fn test_check_valid_tokens_invalid() {
        let invalid_tokens = vec!["invalid_token_address".to_string()];
        let result = check_valid_tokens(&invalid_tokens);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_spl() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

        let (token_price, decimals) =
            get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client).await.unwrap();

        assert_eq!(decimals, 9);
        assert_eq!(token_price.price, 1.0);
    }

    #[tokio::test]
    async fn test_get_token_price_and_decimals_token2022() {
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();

        let account = create_mock_token2022_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

        let (token_price, decimals) =
            get_token_price_and_decimals(&mint, PriceSource::Mock, &rpc_client).await.unwrap();

        assert_eq!(decimals, 6);
        assert_eq!(token_price.price, 0.0001);
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

        let amount = 1_000_000_000; // 1 SOL in lamports
        let result =
            calculate_token_value_in_lamports(amount, &mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(result, 1_000_000_000); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_token_value_in_lamports_usdc() {
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let account = create_mock_spl_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

        let amount = 1_000_000; // 1 USDC (6 decimals)
        let result =
            calculate_token_value_in_lamports(amount, &mint, PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        // 1 USDC * 0.0001 SOL/USDC = 0.0001 SOL = 100,000 lamports
        assert_eq!(result, 100_000);
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token() {
        let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let account = create_mock_spl_mint_account(9);
        let rpc_client = get_mock_rpc_client(&account);

        let lamports = 1_000_000_000; // 1 SOL
        let result =
            calculate_lamports_value_in_token(lamports, &mint, &PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        assert_eq!(result, 1_000_000_000.0); // Should equal input since SOL price is 1.0
    }

    #[tokio::test]
    async fn test_calculate_lamports_value_in_token_usdc() {
        let mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
        let account = create_mock_spl_mint_account(6);
        let rpc_client = get_mock_rpc_client(&account);

        let lamports = 100_000; // 0.0001 SOL
        let result =
            calculate_lamports_value_in_token(lamports, &mint, &PriceSource::Mock, &rpc_client)
                .await
                .unwrap();

        // 0.0001 SOL / 0.0001 SOL/USDC = 1 USDC = 1,000,000 base units
        assert_eq!(result, 1_000_000.0);
    }
}
