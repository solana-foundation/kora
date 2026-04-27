use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";
pub const LAMPORTS_PER_SIGNATURE: u64 = 5000;
pub const ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION: u64 = 50;
pub const MIN_BALANCE_FOR_RENT_EXEMPTION: u64 = 2_039_280;
pub const DEFAULT_INTEREST_MULTIPLIER: u128 = 100 * 24 * 60 * 60 / 10000 / (365 * 24 * 60 * 60);
pub const MAX_TRANSACTION_SIZE: usize = 1232;

// HTTP Headers
pub const X_RECAPTCHA_TOKEN: &str = "x-recaptcha-token";
pub const X_API_KEY: &str = "x-api-key";
pub const X_HMAC_SIGNATURE: &str = "x-hmac-signature";
pub const X_TIMESTAMP: &str = "x-timestamp";
pub const DEFAULT_MAX_TIMESTAMP_AGE: i64 = 300;
pub const MIN_RECAPTCHA_SCORE: f64 = 0.0;
pub const MAX_RECAPTCHA_SCORE: f64 = 1.0;
pub const DEFAULT_RECAPTCHA_SCORE_THRESHOLD: f64 = 0.5;
pub const DEFAULT_PROTECTED_METHODS: &[&str] =
    &["signTransaction", "signAndSendTransaction", "signBundle", "signAndSendBundle"];

// External Services
pub const JUPITER_API_URL: &str = "https://api.jup.ag";
pub const RECAPTCHA_VERIFY_URL: &str = "https://www.google.com/recaptcha/api/siteverify";
pub const RECAPTCHA_TIMEOUT_SECS: u64 = 5;

// Lighthouse Program ID
pub const LIGHTHOUSE_PROGRAM_ID: Pubkey = pubkey!("L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95");

// High-risk native programs that have no fee-payer instruction parser in Kora.
// These programs can directly control funds and should not be added to
// `allowed_programs` without understanding the implications.
pub const VOTE_PROGRAM_ID: Pubkey = pubkey!("Vote111111111111111111111111111111111111111");
pub const STAKE_PROGRAM_ID: Pubkey = pubkey!("Stake11111111111111111111111111111111111111");
pub const BPF_LOADER_UPGRADEABLE_PROGRAM_ID: Pubkey =
    pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

// Loader-v4 — successor to BPF Loader Upgradeable (loader-v3). Has a dedicated fee-payer parser
// in Kora; policy enforcement lives in `LoaderV4InstructionPolicy`.
pub const LOADER_V4_PROGRAM_ID: Pubkey = pubkey!("LoaderV411111111111111111111111111111111111");

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
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const PAYER_INDEX: usize = 0;
        pub const NEW_ACCOUNT_INDEX: usize = 1;
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

    pub mod spl_token_reallocate {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 4;
        pub const ACCOUNT_INDEX: usize = 0;
        pub const PAYER_INDEX: usize = 1;
        pub const OWNER_INDEX: usize = 3;
    }

    // ATA Create/CreateIdempotent account layout:
    // https://github.com/solana-program/associated-token-account/blob/7af39d84438199e7e488adc379baa8ee0b8085c0/interface/src/instruction.rs#L19-L38
    pub mod ata_instruction_indexes {
        pub const PAYER_INDEX: usize = 0;
        pub const ATA_ADDRESS_INDEX: usize = 1;
        pub const WALLET_OWNER_INDEX: usize = 2;
        pub const MINT_INDEX: usize = 3;
        pub const SYSTEM_PROGRAM_INDEX: usize = 4;
        pub const TOKEN_PROGRAM_INDEX: usize = 5;
        pub const MIN_ACCOUNTS: usize = 6;
    }

    pub mod spl_transfer_instruction_indexes {
        pub const DESTINATION_INDEX: usize = 1;
        pub const MIN_ACCOUNTS: usize = 3;
    }

    // Address Lookup Table (ALT) instruction indexes
    pub mod alt_create_lookup_table {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 4;
        pub const LOOKUP_TABLE_ACCOUNT_INDEX: usize = 0;
        pub const LOOKUP_TABLE_AUTHORITY_INDEX: usize = 1;
        pub const PAYER_ACCOUNT_INDEX: usize = 2;
    }

    pub mod alt_freeze_lookup_table {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const LOOKUP_TABLE_ACCOUNT_INDEX: usize = 0;
        pub const LOOKUP_TABLE_AUTHORITY_INDEX: usize = 1;
    }

    pub mod alt_extend_lookup_table {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_PAYER: usize = 4;
        pub const LOOKUP_TABLE_ACCOUNT_INDEX: usize = 0;
        pub const LOOKUP_TABLE_AUTHORITY_INDEX: usize = 1;
        pub const OPTIONAL_PAYER_ACCOUNT_INDEX: usize = 2;
    }

    pub mod alt_deactivate_lookup_table {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const LOOKUP_TABLE_ACCOUNT_INDEX: usize = 0;
        pub const LOOKUP_TABLE_AUTHORITY_INDEX: usize = 1;
    }

    pub mod alt_close_lookup_table {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const LOOKUP_TABLE_ACCOUNT_INDEX: usize = 0;
        pub const LOOKUP_TABLE_AUTHORITY_INDEX: usize = 1;
        pub const RECIPIENT_INDEX: usize = 2;
    }

    // Loader-v4 instruction layouts.
    // Source: solana-loader-v4-interface::instruction::LoaderV4Instruction
    pub mod loader_v4_write {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const PROGRAM_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
    }

    pub mod loader_v4_copy {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const DESTINATION_PROGRAM_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
        pub const SOURCE_PROGRAM_INDEX: usize = 2;
    }

    pub mod loader_v4_set_program_length {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_RECIPIENT: usize = 3;
        pub const PROGRAM_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
        pub const OPTIONAL_RECIPIENT_INDEX: usize = 2;
    }

    pub mod loader_v4_deploy {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_SOURCE: usize = 3;
        pub const PROGRAM_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
        pub const OPTIONAL_SOURCE_PROGRAM_INDEX: usize = 2;
    }

    pub mod loader_v4_retract {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const PROGRAM_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
    }

    pub mod loader_v4_transfer_authority {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const PROGRAM_INDEX: usize = 0;
        pub const CURRENT_AUTHORITY_INDEX: usize = 1;
        pub const NEW_AUTHORITY_INDEX: usize = 2;
    }

    pub mod loader_v4_finalize {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const PROGRAM_INDEX: usize = 0;
        pub const CURRENT_AUTHORITY_INDEX: usize = 1;
        pub const NEXT_VERSION_INDEX: usize = 2;
    }

    // BPF Loader Upgradeable (loader-v3) instruction layouts.
    // Source: solana-loader-v3-interface::instruction::UpgradeableLoaderInstruction.
    pub mod bpf_loader_upgradeable_initialize_buffer {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 1;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_AUTHORITY: usize = 2;
        pub const BUFFER_INDEX: usize = 0;
        pub const OPTIONAL_AUTHORITY_INDEX: usize = 1;
    }

    pub mod bpf_loader_upgradeable_write {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const BUFFER_INDEX: usize = 0;
        pub const AUTHORITY_INDEX: usize = 1;
    }

    pub mod bpf_loader_upgradeable_deploy_with_max_data_len {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 8;
        pub const PAYER_INDEX: usize = 0;
        pub const PROGRAM_DATA_INDEX: usize = 1;
        pub const PROGRAM_INDEX: usize = 2;
        pub const BUFFER_INDEX: usize = 3;
        // Indexes 4 (rent), 5 (clock), 6 (system program) are sysvars/builtins.
        pub const UPGRADE_AUTHORITY_INDEX: usize = 7;
    }

    pub mod bpf_loader_upgradeable_upgrade {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 7;
        pub const PROGRAM_DATA_INDEX: usize = 0;
        pub const PROGRAM_INDEX: usize = 1;
        pub const BUFFER_INDEX: usize = 2;
        pub const SPILL_INDEX: usize = 3;
        // Indexes 4 (rent) + 5 (clock) are sysvars.
        pub const UPGRADE_AUTHORITY_INDEX: usize = 6;
    }

    pub mod bpf_loader_upgradeable_set_authority {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_NEW_AUTHORITY: usize = 3;
        pub const TARGET_INDEX: usize = 0;
        pub const CURRENT_AUTHORITY_INDEX: usize = 1;
        pub const OPTIONAL_NEW_AUTHORITY_INDEX: usize = 2;
    }

    pub mod bpf_loader_upgradeable_set_authority_checked {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const TARGET_INDEX: usize = 0;
        pub const CURRENT_AUTHORITY_INDEX: usize = 1;
        pub const NEW_AUTHORITY_INDEX: usize = 2;
    }

    pub mod bpf_loader_upgradeable_close {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_AUTHORITY: usize = 3;
        // Closing a ProgramData account requires the associated Program account too.
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_PROGRAM: usize = 4;
        pub const TARGET_INDEX: usize = 0;
        pub const RECIPIENT_INDEX: usize = 1;
        pub const OPTIONAL_AUTHORITY_INDEX: usize = 2;
        pub const OPTIONAL_PROGRAM_INDEX: usize = 3;
    }

    pub mod bpf_loader_upgradeable_extend_program {
        pub const MIN_REQUIRED_NUMBER_OF_ACCOUNTS: usize = 2;
        pub const REQUIRED_NUMBER_OF_ACCOUNTS_WITH_PAYER: usize = 4;
        pub const PROGRAM_DATA_INDEX: usize = 0;
        pub const PROGRAM_INDEX: usize = 1;
        // Index 2 is the System program (optional).
        pub const OPTIONAL_PAYER_INDEX: usize = 3;
    }

    pub mod bpf_loader_upgradeable_migrate {
        pub const REQUIRED_NUMBER_OF_ACCOUNTS: usize = 3;
        pub const PROGRAM_DATA_INDEX: usize = 0;
        pub const PROGRAM_INDEX: usize = 1;
        pub const CURRENT_AUTHORITY_INDEX: usize = 2;
    }
}
