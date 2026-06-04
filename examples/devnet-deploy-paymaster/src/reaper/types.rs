use std::{sync::Arc, time::Duration};

use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    V3,
    V4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Program,
    Buffer,
}

#[derive(Debug, Clone)]
pub struct OwnedProgram {
    pub loader: Loader,
    pub kind: AccountKind,
    pub program: Pubkey,
    /// `None` for v4 and for buffers — state and code share one account.
    pub program_data: Option<Pubkey>,
    pub last_state_slot: u64,
}

#[derive(Clone)]
pub struct ReaperConfig {
    pub fee_payer: Pubkey,
    pub signer: Arc<solana_keychain::Signer>,
    pub threshold: Duration,
    pub dry_run: bool,
    pub max_closes: Option<usize>,
}

#[derive(Debug, Default)]
pub struct ReaperReport {
    pub discovered: usize,
    pub skipped_recent: usize,
    pub closed: Vec<ClosedProgram>,
    pub failed: Vec<FailedClose>,
}

#[derive(Debug)]
pub struct ClosedProgram {
    pub program: Pubkey,
    pub loader: Loader,
    pub signature: String,
    pub reclaimed_lamports: u64,
}

#[derive(Debug)]
pub struct FailedClose {
    pub program: Pubkey,
    pub loader: Loader,
    pub error: String,
}
