pub mod account_validator;
pub mod bundle_validator;
pub mod cache_validator;
pub mod config_validator;
pub mod cross_cluster;
pub mod math_validator;
#[macro_use]
pub mod macros;
pub mod signer_validator;
pub mod transaction_validator;

use crate::error::KoraError;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashSet, str::FromStr};

pub(crate) fn parse_pubkey_set(pubkeys: &[String]) -> Result<HashSet<Pubkey>, KoraError> {
    pubkeys
        .iter()
        .map(|pubkey| {
            Pubkey::from_str(pubkey).map_err(|e| {
                KoraError::InternalServerError(format!(
                    "Invalid public key `{}` in config: {}",
                    pubkey, e
                ))
            })
        })
        .collect()
}
