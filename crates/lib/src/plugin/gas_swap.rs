use solana_sdk::pubkey::Pubkey;
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};
use std::str::FromStr;

use crate::{
    config::Config, error::KoraError, fee::price::PriceModel,
    transaction::VersionedTransactionResolved,
};

use super::TransactionPlugin;

const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SPL_TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

const SPL_TOKEN_TRANSFER_DISCRIMINANT: u8 = 3;
const SPL_TOKEN_TRANSFER_CHECKED_DISCRIMINANT: u8 = 12;

pub struct GasSwapPlugin;

impl TransactionPlugin for GasSwapPlugin {
    fn name(&self) -> &str {
        "GasSwap"
    }

    fn validate(&self, resolved: &VersionedTransactionResolved) -> Result<(), KoraError> {
        let outer_ixs = resolved.transaction.message.instructions();

        if outer_ixs.len() != 2 {
            return Err(KoraError::ValidationError(format!(
                "GasSwap plugin requires exactly 2 instructions (got {}): ix[0]=SplTokenTransfer, ix[1]=SystemTransfer",
                outer_ixs.len()
            )));
        }

        let spl_token_id =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("valid SPL Token program ID");
        let spl_token_2022_id =
            Pubkey::from_str(SPL_TOKEN_2022_PROGRAM_ID).expect("valid SPL Token 2022 program ID");

        let ix0_program = resolved.all_account_keys[outer_ixs[0].program_id_index as usize];

        if ix0_program != spl_token_id && ix0_program != spl_token_2022_id {
            return Err(KoraError::ValidationError(format!(
                "GasSwap plugin requires instruction 0 to be an SPL token transfer (got program {})",
                ix0_program
            )));
        }

        let ix0_discriminant = outer_ixs[0].data.first().copied().unwrap_or(0xFF);
        if ix0_discriminant != SPL_TOKEN_TRANSFER_DISCRIMINANT
            && ix0_discriminant != SPL_TOKEN_TRANSFER_CHECKED_DISCRIMINANT
        {
            return Err(KoraError::ValidationError(
                "GasSwap plugin requires instruction 0 to be an SPL token Transfer or TransferChecked"
                    .to_string(),
            ));
        }

        let ix1_program = resolved.all_account_keys[outer_ixs[1].program_id_index as usize];

        if ix1_program != SYSTEM_PROGRAM_ID {
            return Err(KoraError::ValidationError(format!(
                "GasSwap plugin requires instruction 1 to be a System program transfer (got program {})",
                ix1_program
            )));
        }

        let system_ix =
            bincode::deserialize::<SystemInstruction>(&outer_ixs[1].data).map_err(|e| {
                KoraError::ValidationError(format!(
                    "GasSwap plugin: failed to parse instruction 1 as SystemInstruction: {e}"
                ))
            })?;

        if !matches!(system_ix, SystemInstruction::Transfer { .. }) {
            return Err(KoraError::ValidationError(
                "GasSwap plugin requires instruction 1 to be SystemInstruction::Transfer"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn validate_config(&self, config: &Config) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();

        if matches!(config.validation.price.model, PriceModel::Free) {
            errors.push(
                "GasSwap plugin cannot be used with Free pricing; set a margin or fixed price"
                    .to_string(),
            );
        }

        let mut warnings = Vec::new();
        if matches!(config.validation.price.model, PriceModel::Fixed { .. }) {
            warnings.push(
                "GasSwap plugin with Fixed pricing: ensure the fixed token fee is worth at least \
                 max_allowed_lamports in SOL to avoid a drain condition."
                    .to_string(),
            );
        }

        (errors, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fee::price::PriceModel, tests::config_mock::ConfigMockBuilder};

    #[test]
    fn test_validate_config_free_pricing_errors() {
        let plugin = GasSwapPlugin;
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Free;
        let (errors, warnings) = plugin.validate_config(&config);
        assert!(!errors.is_empty(), "expected error for Free pricing");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_config_fixed_pricing_warns() {
        let plugin = GasSwapPlugin;
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Fixed {
            amount: 1_000_000,
            token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
            strict: false,
        };
        let (errors, warnings) = plugin.validate_config(&config);
        assert!(errors.is_empty());
        assert!(!warnings.is_empty(), "expected warning for Fixed pricing");
        assert!(warnings[0].contains("drain condition"));
    }

    #[test]
    fn test_validate_config_margin_pricing_clean() {
        let plugin = GasSwapPlugin;
        let mut config = ConfigMockBuilder::new().build();
        config.validation.price.model = PriceModel::Margin { margin: 0.1 };
        let (errors, warnings) = plugin.validate_config(&config);
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }
}
