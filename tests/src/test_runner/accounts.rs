use crate::common::{
    TestAccountInfo, KORA_PRIVATE_KEY_ENV, PAYMENT_ADDRESS_KEYPAIR_ENV, SIGNER_2_KEYPAIR_ENV,
    TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV, TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
    TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV, TEST_RECIPIENT_PUBKEY_ENV, TEST_SENDER_KEYPAIR_ENV,
    TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV, TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV,
    TEST_USDC_MINT_2022_KEYPAIR_ENV, TEST_USDC_MINT_KEYPAIR_ENV,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{fs, path::Path};

const TEST_ACCOUNTS_DIR: &str = "tests/src/common/fixtures/test-accounts";

#[derive(Debug, Clone, Copy)]
pub enum AccountFile {
    FeePayer,
    Sender,
    Recipient,
    UsdcMint,
    SenderTokenAccount,
    RecipientTokenAccount,
    FeePayerTokenAccount,
    UsdcMint2022,
    SenderToken2022Account,
    RecipientToken2022Account,
    FeePayerToken2022Account,
    AllowedLookupTable,
    DisallowedLookupTable,
    TransactionLookupTable,
    Signer2,
    InterestBearingMint,
    TransferHookMint,
    Payment,
}

impl AccountFile {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::FeePayer => "fee-payer-local.json",
            Self::Sender => "sender-local.json",
            Self::Recipient => "recipient-local.json",
            Self::UsdcMint => "usdc-mint-local.json",
            Self::SenderTokenAccount => "sender-token-account-local.json",
            Self::RecipientTokenAccount => "recipient-token-account-local.json",
            Self::FeePayerTokenAccount => "fee-payer-token-account-local.json",
            Self::UsdcMint2022 => "usdc-mint-2022-local.json",
            Self::SenderToken2022Account => "sender-token-2022-account-local.json",
            Self::RecipientToken2022Account => "recipient-token-2022-account-local.json",
            Self::FeePayerToken2022Account => "fee-payer-token-2022-account-local.json",
            Self::AllowedLookupTable => "allowed-lookup-table-local.json",
            Self::DisallowedLookupTable => "disallowed-lookup-table-local.json",
            Self::TransactionLookupTable => "transaction-lookup-table-local.json",
            Self::Signer2 => "signer2-local.json",
            Self::InterestBearingMint => "mint-2022-interest-bearing.json",
            Self::TransferHookMint => "mint-transfer-hook-local.json",
            Self::Payment => "payment-local.json",
        }
    }

    pub fn local_key_env_var(&self) -> &'static str {
        match self {
            Self::FeePayer => KORA_PRIVATE_KEY_ENV,
            Self::Sender => TEST_SENDER_KEYPAIR_ENV,
            Self::Recipient => TEST_RECIPIENT_PUBKEY_ENV,
            Self::UsdcMint => TEST_USDC_MINT_KEYPAIR_ENV,
            Self::UsdcMint2022 => TEST_USDC_MINT_2022_KEYPAIR_ENV,
            Self::AllowedLookupTable => TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
            Self::DisallowedLookupTable => TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
            Self::TransactionLookupTable => TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV,
            Self::Signer2 => SIGNER_2_KEYPAIR_ENV,
            Self::InterestBearingMint => TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV,
            Self::TransferHookMint => TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV,
            Self::Payment => PAYMENT_ADDRESS_KEYPAIR_ENV,
            _ => panic!("Invalid account env"),
        }
    }

    pub fn local_key_path(&self) -> String {
        format!("tests/src/common/local-keys/{}", self.filename())
    }

    pub fn test_account_path(&self) -> std::path::PathBuf {
        Path::new(TEST_ACCOUNTS_DIR).join(self.filename())
    }

    pub fn required_test_accounts() -> &'static [AccountFile] {
        &[
            Self::FeePayer,
            Self::Sender,
            Self::Recipient,
            Self::UsdcMint,
            Self::SenderTokenAccount,
            Self::RecipientTokenAccount,
            Self::FeePayerTokenAccount,
            Self::UsdcMint2022,
            Self::SenderToken2022Account,
            Self::RecipientToken2022Account,
            Self::FeePayerToken2022Account,
            Self::AllowedLookupTable,
            Self::DisallowedLookupTable,
            Self::TransactionLookupTable,
            Self::Signer2,
            Self::InterestBearingMint,
            Self::TransferHookMint,
            Self::Payment,
        ]
    }

    pub fn required_test_accounts_env_vars() -> &'static [AccountFile] {
        &[
            Self::FeePayer,
            Self::Signer2,
            Self::Sender,
            Self::UsdcMint,
            Self::UsdcMint2022,
            Self::InterestBearingMint,
            Self::TransferHookMint,
            Self::Payment,
        ]
    }

    pub fn set_environment_variable(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::env::set_var(self.local_key_env_var(), fs::read_to_string(self.local_key_path())?);
        Ok(())
    }

    pub fn set_dynamic_environment_variable(
        &self,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::env::set_var(self.local_key_env_var(), value);
        Ok(())
    }

    pub async fn save_account_for_file(
        &self,
        client: &RpcClient,
        address: &Pubkey,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        save_account(client, address, self.test_account_path()).await
    }

    pub fn get_as_env_var(&self) -> (&'static str, String) {
        (self.local_key_env_var(), std::env::var(self.local_key_env_var()).unwrap())
    }
}

pub fn set_environment_variables() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for account_file in AccountFile::required_test_accounts_env_vars() {
        account_file.set_environment_variable()?;
    }

    Ok(())
}

pub async fn set_lookup_table_environment_variables(
    test_accounts: &TestAccountInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    AccountFile::AllowedLookupTable
        .set_dynamic_environment_variable(&test_accounts.allowed_lookup_table.to_string())?;
    AccountFile::DisallowedLookupTable
        .set_dynamic_environment_variable(&test_accounts.disallowed_lookup_table.to_string())?;
    AccountFile::TransactionLookupTable
        .set_dynamic_environment_variable(&test_accounts.transaction_lookup_table.to_string())?;
    Ok(())
}

pub fn get_account_address_from_file(
    account_path: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let account_json = fs::read_to_string(account_path)?;
    let account_data: serde_json::Value = serde_json::from_str(&account_json)?;

    if let Some(pubkey) = account_data["account"]["pubkey"].as_str() {
        return Ok(pubkey.to_string());
    }

    if let Some(pubkey) = account_data["pubkey"].as_str() {
        return Ok(pubkey.to_string());
    }

    Err("Could not find pubkey in account file".into())
}

pub async fn download_accounts(
    client: &RpcClient,
    test_accounts: &TestAccountInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let accounts_dir = Path::new(TEST_ACCOUNTS_DIR);
    fs::create_dir_all(accounts_dir)?;

    AccountFile::FeePayer.save_account_for_file(client, &test_accounts.fee_payer_pubkey).await?;
    AccountFile::Sender.save_account_for_file(client, &test_accounts.sender_pubkey).await?;
    AccountFile::Recipient.save_account_for_file(client, &test_accounts.recipient_pubkey).await?;
    AccountFile::UsdcMint.save_account_for_file(client, &test_accounts.usdc_mint_pubkey).await?;
    AccountFile::SenderTokenAccount
        .save_account_for_file(client, &test_accounts.sender_token_account)
        .await?;
    AccountFile::RecipientTokenAccount
        .save_account_for_file(client, &test_accounts.recipient_token_account)
        .await?;
    AccountFile::FeePayerTokenAccount
        .save_account_for_file(client, &test_accounts.fee_payer_token_account)
        .await?;
    AccountFile::UsdcMint2022
        .save_account_for_file(client, &test_accounts.usdc_mint_2022_pubkey)
        .await?;
    AccountFile::SenderToken2022Account
        .save_account_for_file(client, &test_accounts.sender_token_2022_account)
        .await?;
    AccountFile::RecipientToken2022Account
        .save_account_for_file(client, &test_accounts.recipient_token_2022_account)
        .await?;
    AccountFile::FeePayerToken2022Account
        .save_account_for_file(client, &test_accounts.fee_payer_token_2022_account)
        .await?;
    AccountFile::AllowedLookupTable
        .save_account_for_file(client, &test_accounts.allowed_lookup_table)
        .await?;
    AccountFile::DisallowedLookupTable
        .save_account_for_file(client, &test_accounts.disallowed_lookup_table)
        .await?;
    AccountFile::TransactionLookupTable
        .save_account_for_file(client, &test_accounts.transaction_lookup_table)
        .await?;
    Ok(())
}

async fn save_account(
    client: &RpcClient,
    address: &Pubkey,
    path: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account = client.get_account(address).await?;

    let account_data = serde_json::json!({
        "pubkey": address.to_string(),
        "account": {
            "lamports": account.lamports,
            "data": [STANDARD.encode(&account.data), "base64"],
            "owner": account.owner.to_string(),
            "executable": account.executable,
            "rentEpoch": 0
        }
    });

    std::fs::write(path, serde_json::to_string_pretty(&account_data)?)?;

    Ok(())
}
