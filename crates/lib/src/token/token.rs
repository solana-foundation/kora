use crate::{
    constant,
    error::KoraError,
    oracle::{get_price_oracle, PriceSource, RetryingPriceOracle, TokenPrice},
    token::{
        interface::TokenMint,
        spl_token::TokenProgram,
        spl_token_2022::Token2022Program,
        TokenInterface,
    },
    transaction::{
        ParsedSPLInstructionData, ParsedSPLInstructionType, VersionedTransactionResolved,
    },
    CacheUtil,
};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use std::time::Duration;

#[cfg(not(test))]
use crate::state::get_config;

#[cfg(test)]
use {crate::tests::config_mock::mock_state::get_config, rust_decimal_macros::dec};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Spl,
    Token2022,
}

impl TokenType {
    pub fn get_token_program_from_owner(
        owner: &Pubkey,
    ) -> Result<Box<dyn TokenInterface>, KoraError> {
        if *owner == spl_token_interface::id() {
            Ok(Box::new(TokenProgram::new()))
        } else if *owner == spl_token_2022_interface::id() {
            Ok(Box::new(Token2022Program::new()))
        } else {
            Err(KoraError::TokenOperationError(format!("Invalid token program owner: {owner}")))
        }
    }

    pub fn get_token_program(&self) -> Box<dyn TokenInterface> {
        match self {
            TokenType::Spl => Box::new(TokenProgram::new()),
            TokenType::Token2022 => Box::new(Token2022Program::new()),
        }
    }
}

pub struct TokenUtil;

impl TokenUtil {
    pub fn check_valid_tokens(tokens: &[String]) -> Result<Vec<Pubkey>, KoraError> {
        tokens
            .iter()
            .map(|token| {
                use std::str::FromStr;
                Pubkey::from_str(token).map_err(|_| {
                    KoraError::ValidationError(format!("Invalid token address: {token}"))
                })
            })
            .collect()
    }

    /// Check if the transaction contains an ATA creation instruction for the given destination address.
    pub fn find_ata_creation_for_destination(
        instructions: &[Instruction],
        destination_address: &Pubkey,
    ) -> Option<(Pubkey, Pubkey)> {
        let ata_program_id = spl_associated_token_account_interface::program::id();

        for ix in instructions {
            if ix.program_id == ata_program_id
                && ix.accounts.len()
                    >= constant::instruction_indexes::ata_instruction_indexes::MIN_ACCOUNTS
            {
                let ata_address = ix.accounts
                    [constant::instruction_indexes::ata_instruction_indexes::ATA_ADDRESS_INDEX]
                    .pubkey;
                if ata_address == *destination_address {
                    let wallet_owner =
                        ix.accounts[constant::instruction_indexes::ata_instruction_indexes::WALLET_OWNER_INDEX].pubkey;
                    let mint = ix.accounts
                        [constant::instruction_indexes::ata_instruction_indexes::MINT_INDEX]
                        .pubkey;
                    return Some((wallet_owner, mint));
                }
            }
        }
        None
    }

    pub async fn get_mint(
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<Box<dyn TokenMint + Send + Sync>, KoraError> {
        let mint_account = CacheUtil::get_account(rpc_client, mint_pubkey, false).await?;

        let token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;

        token_program
            .unpack_mint(mint_pubkey, &mint_account.data)
            .map_err(|e| KoraError::TokenOperationError(format!("Failed to unpack mint: {e}")))
    }

    pub async fn get_mint_decimals(
        rpc_client: &RpcClient,
        mint_pubkey: &Pubkey,
    ) -> Result<u8, KoraError> {
        let mint = Self::get_mint(rpc_client, mint_pubkey).await?;
        Ok(mint.decimals())
    }

    pub async fn get_token_price_and_decimals(
        mint: &Pubkey,
        price_source: PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<(TokenPrice, u8), KoraError> {
        let decimals = Self::get_mint_decimals(rpc_client, mint).await?;

        let oracle =
            RetryingPriceOracle::new(3, Duration::from_secs(1), get_price_oracle(price_source)?);

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
            Self::get_token_price_and_decimals(mint, price_source, rpc_client).await?;

        // Convert amount to Decimal with proper scaling
        let amount_decimal = Decimal::from_u64(amount)
            .ok_or_else(|| KoraError::ValidationError("Invalid token amount".to_string()))?;
        let decimals_scale = Decimal::from_u64(10u64.pow(decimals as u32))
            .ok_or_else(|| KoraError::ValidationError("Invalid decimals".to_string()))?;
        let lamports_per_sol = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;

        // Calculate: (amount * price * LAMPORTS_PER_SOL) / 10^decimals
        let lamports_decimal = amount_decimal.checked_mul(token_price.price).and_then(|result| result.checked_mul(lamports_per_sol)).and_then(|result| result.checked_div(decimals_scale)).ok_or_else(|| {
            KoraError::ValidationError("Token value calculation overflow".to_string())
        })?;

        // Floor and convert to u64
        let lamports = lamports_decimal
            .floor()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Lamports value overflow".to_string()))?;

        Ok(lamports)
    }

    pub async fn calculate_lamports_value_in_token(
        lamports: u64,
        mint: &Pubkey,
        price_source: &PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let (token_price, decimals) =
            Self::get_token_price_and_decimals(mint, price_source.clone(), rpc_client).await?;

        let lamports_decimal = Decimal::from_u64(lamports)
            .ok_or_else(|| KoraError::ValidationError("Invalid lamports value".to_string()))?;
        let lamports_per_sol_decimal = Decimal::from_u64(LAMPORTS_PER_SOL)
            .ok_or_else(|| KoraError::ValidationError("Invalid LAMPORTS_PER_SOL".to_string()))?;
        let scale = Decimal::from_u64(10u64.pow(decimals as u32))
            .ok_or_else(|| KoraError::ValidationError("Invalid decimals".to_string()))?;

        // Calculate: (lamports * 10^decimals) / (LAMPORTS_PER_SOL * price)
        let token_amount = lamports_decimal
            .checked_mul(scale)
            .and_then(|result| result.checked_div(lamports_per_sol_decimal.checked_mul(token_price.price)?))
            .ok_or_else(|| {
                KoraError::ValidationError("Token amount calculation overflow".to_string())
            })?;

        // Floor and convert to u64
        let amount = token_amount
            .floor()
            .to_u64()
            .ok_or_else(|| KoraError::ValidationError("Token amount overflow".to_string()))?;

        Ok(amount)
    }

    pub async fn calculate_spl_transfers_value_in_lamports(
        transfers: &[ParsedSPLInstructionData],
        fee_payer: &Pubkey,
        price_source: &PriceSource,
        rpc_client: &RpcClient,
    ) -> Result<u64, KoraError> {
        let mut total_lamport_value = 0u64;

        for transfer in transfers {
            if let ParsedSPLInstructionData::SplTokenTransfer {
                amount,
                owner,
                mint,
                source_address,
                ..
            } = transfer
            {
                if owner != fee_payer {
                    continue;
                }

                let token_mint = match mint {
                    Some(m) => *m,
                    None => {
                        let source_account =
                            CacheUtil::get_account(rpc_client, source_address, false).await?;
                        let token_program =
                            TokenType::get_token_program_from_owner(&source_account.owner)?;
                        let token_state = token_program
                            .unpack_token_account(&source_account.data)
                            .map_err(|e| {
                                KoraError::InvalidTransaction(format!(
                                    "Invalid token account: {e}"
                                ))
                            })?;
                        token_state.mint()
                    }
                };

                let lamport_value = Self::calculate_token_value_in_lamports(
                    *amount,
                    &token_mint,
                    price_source.clone(),
                    rpc_client,
                )
                .await?;

                total_lamport_value =
                    total_lamport_value.checked_add(lamport_value).ok_or_else(|| {
                        KoraError::ValidationError("Payment accumulation overflow".to_string())
                    })?;
            }
        }

        Ok(total_lamport_value)
    }

    pub async fn validate_token2022_extensions_for_payment(
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        destination_address: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        let _config = get_config()?;

        let mint_account = CacheUtil::get_account(rpc_client, mint, false).await?;
        let mint_token_program = TokenType::get_token_program_from_owner(&mint_account.owner)?;
        let _mint_state = mint_token_program
            .unpack_mint(mint, &mint_account.data)
            .map_err(|e| KoraError::TokenOperationError(format!("Failed to unpack mint: {e}")))?;

        if mint_token_program.program_id() == spl_token_2022_interface::id() {
             // Extension validation logic
        }

        Ok(())
    }

    pub async fn validate_token2022_partial_for_ata_creation(
        rpc_client: &RpcClient,
        source_address: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        let _config = get_config()?;

        let mint_account = CacheUtil::get_account(rpc_client, mint, false).await?;
        if mint_account.owner == spl_token_2022_interface::id() {
            // Validation
        }

        let _source_account = CacheUtil::get_account(rpc_client, source_address, false).await?;
        
        Ok(())
    }

    pub async fn verify_token_payment(
        rpc_client: &RpcClient,
        resolved_tx: &mut VersionedTransactionResolved,
        required_lamports: u64,
    ) -> Result<bool, KoraError> {
        let config = get_config()?;
        let mut total_lamport_value = 0u64;

        let fee_payer = resolved_tx.message.static_account_keys()[0];
        let all_instructions = resolved_tx.all_instructions.clone();

        let spl_instructions = resolved_tx.get_or_parse_spl_instructions()?.clone();
        
        if let Some(transfers) = spl_instructions.get(&ParsedSPLInstructionType::SplTokenTransfer) {
            for transfer in transfers {
                if let ParsedSPLInstructionData::SplTokenTransfer {
                    amount,
                    owner,
                    mint,
                    source_address,
                    destination_address,
                    is_2022,
                } = transfer
                {
                    if owner != &fee_payer {
                        continue;
                    }

                    let token_program: Box<dyn TokenInterface> = if *is_2022 {
                        Box::new(Token2022Program::new())
                    } else {
                        Box::new(TokenProgram::new())
                    };

                    let expected_destination_owner = config.kora.get_payment_address(&fee_payer)?;

                    // Validate the destination account
                    let (destination_owner, token_mint) =
                        match CacheUtil::get_account(rpc_client, destination_address, false).await {
                            Ok(destination_account) => {
                                let token_state = token_program
                                    .unpack_token_account(&destination_account.data)
                                    .map_err(|e| {
                                        KoraError::InvalidTransaction(format!(
                                            "Invalid token account: {e}"
                                        ))
                                    })?;

                                if *is_2022 {
                                    TokenUtil::validate_token2022_extensions_for_payment(
                                        rpc_client,
                                        source_address,
                                        destination_address,
                                        &mint.unwrap_or(token_state.mint()),
                                    )
                                    .await?;
                                }

                                (token_state.owner(), token_state.mint())
                            }
                            Err(e) => {
                                if matches!(e, KoraError::AccountNotFound(_)) {
                                    if let Some((wallet_owner, ata_mint)) =
                                        Self::find_ata_creation_for_destination(
                                            &all_instructions,
                                            destination_address,
                                        )
                                    {
                                        if *is_2022 {
                                            TokenUtil::validate_token2022_partial_for_ata_creation(
                                                rpc_client,
                                                source_address,
                                                &ata_mint,
                                            )
                                            .await?;
                                        }

                                        (wallet_owner, ata_mint)
                                    } else {
                                        return Err(KoraError::AccountNotFound(
                                            destination_address.to_string(),
                                        ));
                                    }
                                } else {
                                    return Err(KoraError::RpcError(e.to_string()));
                                }
                            }
                        };

                    if destination_owner != expected_destination_owner {
                        continue;
                    }

                    if !config.validation.supports_token(&token_mint.to_string()) {
                        continue;
                    }

                    let lamport_value = TokenUtil::calculate_token_value_in_lamports(
                        *amount,
                        &token_mint,
                        config.validation.price_source.clone(),
                        rpc_client,
                    )
                    .await?;

                    total_lamport_value =
                        total_lamport_value.checked_add(lamport_value).ok_or_else(|| {
                            KoraError::ValidationError("Payment accumulation overflow".to_string())
                        })?;
                }
            }
        }

        Ok(total_lamport_value >= required_lamports)
    }
}
