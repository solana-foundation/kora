//! Re-exports for external signer infrastructure
//!
//! Kora uses solana-signers crate as its signing infrastructure.
//! This module exists only for re-exporting convenience.

// Re-export the external signer for use throughout Kora
pub use solana_signers::{Signer, SolanaSigner};
