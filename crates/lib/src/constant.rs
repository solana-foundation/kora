pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";
pub const LAMPORTS_PER_SIGNATURE: u64 = 5000;
pub const ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION: u64 = 50;
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

// Metrics
pub const DEFAULT_METRICS_ENDPOINT: &str = "/metrics";
pub const DEFAULT_METRICS_PORT: u16 = 8080;
pub const DEFAULT_METRICS_SCRAPE_INTERVAL: u64 = 60;

// Cache
pub const DEFAULT_CACHE_DEFAULT_TTL: u64 = 300; // 5 minutes
pub const DEFAULT_CACHE_ACCOUNT_TTL: u64 = 60; // 1 minute for account data
pub const DEFAULT_FEE_PAYER_BALANCE_METRICS_EXPIRY_SECONDS: u64 = 30; // 30 seconds

pub const DEFAULT_USAGE_LIMIT_MAX_TRANSACTIONS: u64 = 0; // 0 = unlimited
pub const DEFAULT_USAGE_LIMIT_FALLBACK_IF_UNAVAILABLE: bool = false;

// Request body size limit
pub const DEFAULT_MAX_REQUEST_BODY_SIZE: usize = 2 * 1024 * 1024; // 2 MB

// Account Indexes within instructions
// Instruction indexes for the instructions that we support to parse from the transaction
pub mod instruction_indexes {
    pub mod system_create_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1;
        pub const PAYER_INDEX: usize = 0;
    }

    pub mod system_transfer {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const SENDER_INDEX: usize = 0;
        pub const RECEIVER_INDEX: usize = 1;
    }

    pub mod system_transfer_with_seed {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const SENDER_INDEX: usize = 1;
        pub const RECEIVER_INDEX: usize = 2;
    }

    pub mod system_withdraw_nonce_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 5;
        pub const NONCE_AUTHORITY_INDEX: usize = 4;
        pub const RECIPIENT_INDEX: usize = 1;
    }

    pub mod system_assign {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1;
        pub const AUTHORITY_INDEX: usize = 0;
    }

    pub mod system_assign_with_seed {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const AUTHORITY_INDEX: usize = 1;
    }

    pub mod system_allocate {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1;
        pub const ACCOUNT_INDEX: usize = 0;
    }

    pub mod system_allocate_with_seed {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const ACCOUNT_INDEX: usize = 1;
    }

    pub mod system_initialize_nonce_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const NONCE_ACCOUNT_INDEX: usize = 0;
        // Authority is in instruction data, not accounts
    }

    pub mod system_advance_nonce_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const NONCE_ACCOUNT_INDEX: usize = 0;
        pub const NONCE_AUTHORITY_INDEX: usize = 2;
    }

    pub mod system_authorize_nonce_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const NONCE_ACCOUNT_INDEX: usize = 0;
        pub const NONCE_AUTHORITY_INDEX: usize = 1;
    }

    // Note: system_upgrade_nonce_account not included - no authority parameter, cannot validate

    pub mod spl_token_transfer {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const OWNER_INDEX: usize = 2;
        pub const SOURCE_ADDRESS_INDEX: usize = 0;
        pub const DESTINATION_ADDRESS_INDEX: usize = 1;
    }

    pub mod spl_token_transfer_checked {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 4;
        pub const OWNER_INDEX: usize = 3;
        pub const MINT_INDEX: usize = 1;
        pub const SOURCE_ADDRESS_INDEX: usize = 0;
        pub const DESTINATION_ADDRESS_INDEX: usize = 2;
    }

    pub mod spl_token_burn {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const OWNER_INDEX: usize = 2;
    }

    pub mod spl_token_close_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const OWNER_INDEX: usize = 2;
    }

    pub mod spl_token_approve {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const OWNER_INDEX: usize = 2;
    }

    pub mod spl_token_approve_checked {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 4;
        pub const OWNER_INDEX: usize = 3;
    }

    pub mod spl_token_revoke {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const OWNER_INDEX: usize = 1;
    }

    pub mod spl_token_set_authority {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const CURRENT_AUTHORITY_INDEX: usize = 1;
    }

    pub mod spl_token_mint_to {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const MINT_AUTHORITY_INDEX: usize = 2;
    }

    pub mod spl_token_mint_to_checked {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const MINT_AUTHORITY_INDEX: usize = 2;
    }

    pub mod spl_token_initialize_mint {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        // Authority is in instruction data, not accounts
    }

    pub mod spl_token_initialize_mint2 {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1;
        // Authority is in instruction data, not accounts
    }

    pub mod spl_token_initialize_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 4;
        // Owner is in account data at index 2
        pub const OWNER_INDEX: usize = 2;
    }

    pub mod spl_token_initialize_account2 {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        // Owner is in instruction data, not accounts
    }

    pub mod spl_token_initialize_account3 {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        // Owner is in instruction data, not accounts
    }

    pub mod spl_token_initialize_multisig {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2; // Minimum
                                                          // Signers are accounts from index 2 onwards (after multisig account and rent sysvar)
    }

    pub mod spl_token_initialize_multisig2 {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1; // Minimum
                                                          // Signers are accounts from index 1 onwards (after multisig account)
    }

    pub mod spl_token_freeze_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const FREEZE_AUTHORITY_INDEX: usize = 2;
    }

    pub mod spl_token_thaw_account {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const FREEZE_AUTHORITY_INDEX: usize = 2;
    }
}
