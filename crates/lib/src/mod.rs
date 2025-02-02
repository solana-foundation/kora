pub mod account;
pub mod cache;
pub mod config;
pub mod error;
pub mod jup;
pub mod log;
pub mod rpc;
pub mod signer;
pub mod state;
pub mod token;
pub mod transaction;
pub mod validation;
pub mod solana;

pub use config::{load_config, Config};
pub use error::KoraError;
pub use signer::{Signature, Signer};
pub use state::{get_signer, init_signer};

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";
pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";
pub const LAMPORTS_PER_SIGNATURE: u64 = 5000;
pub const MIN_BALANCE_FOR_RENT_EXEMPTION: u64 = 2_039_280;
