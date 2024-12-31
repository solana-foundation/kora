pub mod account;
pub mod config;
pub mod error;
pub mod feature;
pub mod jup;
pub mod rpc;
pub mod signer;
pub mod solana;
pub mod solana_signer;
pub mod state;
pub mod token;
pub mod transaction;
pub mod types;
pub mod validation;

pub use config::{load_config, Config};
pub use error::KoraError;
pub use feature::Feature;
pub use signer::{Signature, Signer};
pub use solana_signer::SolanaMemorySigner;
pub use state::{get_signer, init_signer};

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";
pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";
pub const LAMPORTS_PER_SIGNATURE: u64 = 5000;
