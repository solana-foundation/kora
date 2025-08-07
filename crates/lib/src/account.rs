use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    account::Account, program_pack::Pack, pubkey::Pubkey, system_program::ID as SYSTEM_PROGRAM_ID,
};
use spl_token::{
    state::{Account as SplTokenAccount, Mint},
    ID as SPL_TOKEN_PROGRAM_ID,
};
use spl_token_2022::{
    state::{Account as Token2022Account, Mint as Token2022Mint},
    ID as TOKEN_2022_PROGRAM_ID,
};

use crate::KoraError;

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
                SPL_TOKEN_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    Mint::unpack(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a Mint account: {e}"
                        ))
                    })?;
                }
                TOKEN_2022_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    Token2022Mint::unpack(&account.data).map_err(|e| {
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
                SPL_TOKEN_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    SplTokenAccount::unpack(&account.data).map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Account {account_pubkey} has invalid data for a TokenAccount account: {e}"
                        ))
                    })?;
                }
                TOKEN_2022_PROGRAM_ID => {
                    should_be_executable = Some(false);

                    Token2022Account::unpack(&account.data).map_err(|e| {
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
                    "Account {account_pubkey} is not executable, cannot be a Program"
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
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    expected_account_type: Option<AccountType>,
) -> Result<(), KoraError> {
    let account = rpc_client.get_account(account_pubkey).await.map_err(|e| {
        KoraError::InternalServerError(format!("Failed to get account {account_pubkey}: {e}"))
    })?;

    if let Some(expected_type) = expected_account_type {
        expected_type.validate_account_type(&account, account_pubkey)?;
    }

    Ok(())
}
