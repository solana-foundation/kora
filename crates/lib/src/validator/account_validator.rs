use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program_pack::Pack;
use solana_sdk::{account::Account, pubkey::Pubkey};
use solana_system_interface::program::ID as SYSTEM_PROGRAM_ID;
use spl_token_2022_interface::{
    state::{Account as Token2022Account, Mint as Token2022Mint},
    ID as TOKEN_2022_PROGRAM_ID,
};
use spl_token_interface::{
    state::{Account as SplTokenAccount, Mint},
    ID as SPL_TOKEN_PROGRAM_ID,
};

use crate::{CacheUtil, KoraError};

#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    Mint,
    TokenAccount,
    System,
    Program,
}

impl AccountType {
    pub fn validate_account_type(
        self,
        account: &Account,
        account_pubkey: &Pubkey,
    ) -> Result<(), KoraError> {
        let mut should_be_executable: Option<bool> = None;
        let mut should_be_owned_by: Option<Pubkey> = None;

        match self {
            AccountType::Mint => match account.owner {
                ref owner if *owner == SPL_TOKEN_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    if account.data.len() < Mint::LEN {
                        return Err(KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a Mint account: data too short"
                        )));
                    }
                    Mint::unpack_from_slice(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a Mint account: {e}"
                        ))
                    })?;
                }
                ref owner if *owner == TOKEN_2022_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    if account.data.len() < Token2022Mint::LEN {
                        return Err(KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a Mint account: data too short"
                        )));
                    }
                    Token2022Mint::unpack_from_slice(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a Mint account: {e}"
                        ))
                    })?;
                }
                _ => {
                    return Err(KoraError::InternalServerError(format!(
                            "Account {account_pubkey} is not owned by a token program, cannot be a Mint"
                        )));
                }
            },
            AccountType::TokenAccount => match account.owner {
                ref owner if *owner == SPL_TOKEN_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    if account.data.len() < SplTokenAccount::LEN {
                        return Err(KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a TokenAccount account: data too short"
                        )));
                    }
                    SplTokenAccount::unpack_from_slice(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a TokenAccount account: {e}"
                        ))
                    })?;
                }
                ref owner if *owner == TOKEN_2022_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    if account.data.len() < Token2022Account::LEN {
                        return Err(KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a TokenAccount account: data too short"
                        )));
                    }
                    Token2022Account::unpack_from_slice(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a TokenAccount account: {e}"
                        ))
                    })?;
                }
                _ => {
                    return Err(KoraError::InternalServerError(format!(
                                "Account {account_pubkey} is not owned by a token program, cannot be a TokenAccount"
                            )));
                }
            },
            AccountType::System => {
                should_be_owned_by = Some(SYSTEM_PROGRAM_ID);
            }
            AccountType::Program => {
                should_be_executable = Some(true);
            }
        }

        if let Some(should_be_executable) = should_be_executable {
            if account.executable != should_be_executable {
                return Err(KoraError::InternalServerError(format!(
                    "Account {account_pubkey} executable flag mismatch: expected {should_be_executable}, found {}",
                    account.executable
                )));
            }
        }

        if let Some(should_be_owned_by) = should_be_owned_by {
            if account.owner != should_be_owned_by {
                return Err(KoraError::InternalServerError(format!(
                    "Account {account_pubkey} is not owned by {should_be_owned_by}, found owner: {}",
                    account.owner
                )));
            }
        }

        Ok(())
    }
}

pub async fn validate_account(
    config: &crate::config::Config,
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    expected_account_type: Option<AccountType>,
) -> Result<(), KoraError> {
    let account = CacheUtil::get_account(config, rpc_client, account_pubkey, false).await?;

    if let Some(expected_type) = expected_account_type {
        expected_type.validate_account_type(&account, account_pubkey)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        account_mock::{
            create_mock_account, create_mock_account_with_owner,
            create_mock_non_executable_account, create_mock_program_account,
            create_mock_spl_mint_account, create_mock_token2022_mint_account,
            create_mock_token_account, AccountMockBuilder,
        },
        common::{MintAccountMockBuilder, TokenAccountMockBuilder},
        config_mock::{mock_state::get_config, ConfigMockBuilder},
        rpc_mock::{create_mock_rpc_client_account_not_found, create_mock_rpc_client_with_account},
    };

    #[test]
    fn test_account_type_validate_spl_mint_success() {
        let mint_account = create_mock_spl_mint_account(6);
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&mint_account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_token2022_mint_success() {
        let mint_account = create_mock_token2022_mint_account(9);
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&mint_account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_mint_wrong_owner() {
        let account = AccountMockBuilder::new()
            .with_owner(Pubkey::new_unique()) // Wrong owner, not a token program
            .with_executable(false)
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not owned by a token program"));
    }

    #[test]
    fn test_account_type_validate_mint_executable() {
        let account = AccountMockBuilder::new()
            .with_owner(SPL_TOKEN_PROGRAM_ID)
            .with_executable(true) // Mints should not be executable
            .with_data(MintAccountMockBuilder::new().build().data)
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("executable flag mismatch"));
    }

    #[test]
    fn test_account_type_validate_mint_invalid_data() {
        let account = AccountMockBuilder::new()
            .with_owner(SPL_TOKEN_PROGRAM_ID)
            .with_executable(false)
            .with_data(vec![0u8; 10]) // Too short for mint data
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has invalid data for a Mint account"));
    }

    #[test]
    fn test_account_type_validate_spl_token_account_success() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_account = create_mock_token_account(&owner, &mint);
        let account_pubkey = Pubkey::new_unique();

        let result =
            AccountType::TokenAccount.validate_account_type(&token_account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_token2022_account_success() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_account =
            TokenAccountMockBuilder::new().with_owner(&owner).with_mint(&mint).build_token2022();
        let account_pubkey = Pubkey::new_unique();

        let result =
            AccountType::TokenAccount.validate_account_type(&token_account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_token_account_wrong_owner() {
        let account = AccountMockBuilder::new()
            .with_owner(Pubkey::new_unique()) // Wrong owner, not a token program
            .with_executable(false)
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::TokenAccount.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not owned by a token program"));
    }

    #[test]
    fn test_account_type_validate_token_account_executable() {
        let account = AccountMockBuilder::new()
            .with_owner(SPL_TOKEN_PROGRAM_ID)
            .with_executable(true) // Token accounts should not be executable
            .with_data(TokenAccountMockBuilder::new().build().data)
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::TokenAccount.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("executable flag mismatch"));
    }

    #[test]
    fn test_account_type_validate_token_account_invalid_data() {
        let account = AccountMockBuilder::new()
            .with_owner(SPL_TOKEN_PROGRAM_ID)
            .with_executable(false)
            .with_data(vec![0u8; 10]) // Too short for token account data
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::TokenAccount.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has invalid data for a TokenAccount account"));
    }

    #[test]
    fn test_account_type_validate_system_account_success() {
        let account = create_mock_account_with_owner(SYSTEM_PROGRAM_ID);
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::System.validate_account_type(&account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_system_account_wrong_owner() {
        let account = create_mock_account_with_owner(Pubkey::new_unique());
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::System.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not owned by"));
    }

    #[test]
    fn test_account_type_validate_program_account_success() {
        let account = create_mock_program_account();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Program.validate_account_type(&account, &account_pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_type_validate_program_account_not_executable() {
        let account = create_mock_non_executable_account();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Program.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("executable flag mismatch"));
    }

    #[test]
    fn test_account_type_validate_mint_with_token2022_invalid_data() {
        let account = AccountMockBuilder::new()
            .with_owner(TOKEN_2022_PROGRAM_ID)
            .with_executable(false)
            .with_data(vec![0u8; 10]) // Too short for Token2022 mint data
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::Mint.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has invalid data for a Mint account"));
    }

    #[test]
    fn test_account_type_validate_token_account_with_token2022_invalid_data() {
        let account = AccountMockBuilder::new()
            .with_owner(TOKEN_2022_PROGRAM_ID)
            .with_executable(false)
            .with_data(vec![0u8; 10]) // Too short for Token2022 account data
            .build();
        let account_pubkey = Pubkey::new_unique();

        let result = AccountType::TokenAccount.validate_account_type(&account, &account_pubkey);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has invalid data for a TokenAccount account"));
    }

    #[tokio::test]
    async fn test_validate_account_success_with_type() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let mint_account = create_mock_spl_mint_account(6);
        let rpc_client = create_mock_rpc_client_with_account(&mint_account);
        let account_pubkey = Pubkey::new_unique();

        let config = get_config().unwrap();
        let result =
            validate_account(&config, &rpc_client, &account_pubkey, Some(AccountType::Mint)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_account_success_without_type() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let account = create_mock_account();
        let rpc_client = create_mock_rpc_client_with_account(&account);
        let account_pubkey = Pubkey::new_unique();

        let config = get_config().unwrap();
        let result = validate_account(&config, &rpc_client, &account_pubkey, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_account_rpc_error() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let rpc_client = create_mock_rpc_client_account_not_found();
        let account_pubkey = Pubkey::new_unique();

        let config = get_config().unwrap();
        let result =
            validate_account(&config, &rpc_client, &account_pubkey, Some(AccountType::System))
                .await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Account") && error_msg.contains("not found"));
    }

    #[tokio::test]
    async fn test_validate_account_type_validation_failure() {
        let _m = ConfigMockBuilder::new().with_cache_enabled(false).build_and_setup();

        let account = create_mock_account_with_owner(Pubkey::new_unique()); // Wrong owner for system
        let rpc_client = create_mock_rpc_client_with_account(&account);
        let account_pubkey = Pubkey::new_unique();

        let config = get_config().unwrap();
        let result =
            validate_account(&config, &rpc_client, &account_pubkey, Some(AccountType::System))
                .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not owned by"));
    }
}
