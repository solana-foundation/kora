pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";
pub const LAMPORTS_PER_SIGNATURE: u64 = 5000;
pub const MIN_BALANCE_FOR_RENT_EXEMPTION: u64 = 2_039_280;
pub const DEFAULT_INTEREST_MULTIPLIER: u128 = 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60);

// HTTP Headers
pub const X_API_KEY: &str = "x-api-key";
pub const X_HMAC_SIGNATURE: &str = "x-hmac-signature";
pub const X_TIMESTAMP: &str = "x-timestamp";
pub const DEFAULT_MAX_TIMESTAMP_AGE: i64 = 300;

// External Services
pub const JUPITER_API_LITE_URL: &str = "https://lite-api.jup.ag";
pub const JUPITER_API_PRO_URL: &str = "https://api.jup.ag";
