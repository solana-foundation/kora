use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token_2022::{
    extension::{interest_bearing_mint::instruction::initialize, ExtensionType},
    instruction as token_2022_instruction,
    state::{Account as Token2022Account, Mint as Token2022Mint},
};
use std::sync::Arc;

use crate::common::USDCMintTestHelper;

/// Helper functions for creating Token 2022 accounts with specific extensions for testing
pub struct ExtensionHelpers;

impl ExtensionHelpers {
    /// Create a mint with InterestBearingConfig extension
    pub async fn create_mint_with_interest_bearing(
        rpc_client: &Arc<RpcClient>,
        payer: &Keypair,
        mint_keypair: &Keypair,
    ) -> Result<()> {
        if (rpc_client.get_account(&mint_keypair.pubkey()).await).is_ok() {
            return Ok(());
        }

        let decimals = USDCMintTestHelper::get_test_usdc_mint_decimals();

        let space = ExtensionType::try_calculate_account_len::<Token2022Mint>(&[
            ExtensionType::InterestBearingConfig,
        ])?;

        let rent = rpc_client.get_minimum_balance_for_rent_exemption(space).await?;

        let create_account_instruction = solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &mint_keypair.pubkey(),
            rent,
            space as u64,
            &spl_token_2022::id(),
        );

        let initialize_interest_bearing_instruction =
            initialize(&spl_token_2022::id(), &mint_keypair.pubkey(), Some(payer.pubkey()), 10)?;

        let initialize_mint_instruction = token_2022_instruction::initialize_mint2(
            &spl_token_2022::id(),
            &mint_keypair.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            decimals,
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &[
                create_account_instruction,
                initialize_interest_bearing_instruction,
                initialize_mint_instruction,
            ],
            Some(&payer.pubkey()),
            &[payer, mint_keypair],
            recent_blockhash,
        );

        rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    /// Create a manual token account with MemoTransfer extension
    pub async fn create_token_account_with_memo_transfer(
        rpc_client: &Arc<RpcClient>,
        payer: &Keypair,
        token_account_keypair: &Keypair,
        mint: &Pubkey,
        owner: &Keypair,
    ) -> Result<()> {
        if (rpc_client.get_account(&token_account_keypair.pubkey()).await).is_ok() {
            return Ok(());
        }

        // Calculate space for token accounts with MemoTransfer extension
        // Also include TransferFeeAmount if the mint has TransferFeeConfig
        // (The USDC mint 2022 has TransferFeeConfig, so we need to account for it)
        let account_space = ExtensionType::try_calculate_account_len::<Token2022Account>(&[
            ExtensionType::MemoTransfer,
            ExtensionType::TransferFeeAmount,
        ])?;
        let rent = rpc_client.get_minimum_balance_for_rent_exemption(account_space).await?;

        let create_account_instruction = solana_sdk::system_instruction::create_account(
            &payer.pubkey(),
            &token_account_keypair.pubkey(),
            rent,
            account_space as u64,
            &spl_token_2022::id(),
        );

        // Initialize MemoTransfer account extension (requires memo for transfers)
        let initialize_memo_transfer_instruction =
            spl_token_2022::extension::memo_transfer::instruction::enable_required_transfer_memos(
                &spl_token_2022::id(),
                &token_account_keypair.pubkey(),
                &owner.pubkey(),
                &[&owner.pubkey()],
            )?;

        let initialize_account_instruction = token_2022_instruction::initialize_account3(
            &spl_token_2022::id(),
            &token_account_keypair.pubkey(),
            mint,
            &owner.pubkey(),
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[
                create_account_instruction,
                initialize_account_instruction,
                initialize_memo_transfer_instruction,
            ],
            Some(&payer.pubkey()),
            &[payer, token_account_keypair, owner],
            recent_blockhash,
        );

        rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    pub async fn mint_tokens_to_account(
        rpc_client: &Arc<RpcClient>,
        payer: &Keypair,
        mint: &Pubkey,
        token_account: &Pubkey,
        mint_authority: &Keypair,
        amount: Option<u64>,
    ) -> Result<()> {
        let amount = amount.unwrap_or_else(|| {
            1_000_000 * 10_u64.pow(USDCMintTestHelper::get_test_usdc_mint_decimals() as u32)
        });

        let instruction = token_2022_instruction::mint_to(
            &spl_token_2022::id(),
            mint,
            token_account,
            &mint_authority.pubkey(),
            &[],
            amount,
        )?;

        let recent_blockhash = rpc_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[payer, mint_authority],
            recent_blockhash,
        );

        rpc_client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }
}
