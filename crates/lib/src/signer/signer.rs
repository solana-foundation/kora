//! Re-exports for external signer infrastructure
//!
//! Kora uses solana-keychain crate as its signing infrastructure.
//! This module exists only for re-exporting convenience.

pub use solana_keychain::{Signer, SolanaSigner};
