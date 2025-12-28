//! Privacy pool extensions for Kora.
//!
//! This module contains all privacy-specific validation logic for CPI-based
//! fee payments, isolated from core Kora code for easier fork maintenance.
//!
//! # Overview
//!
//! Standard Kora validates fee payments via top-level SPL token transfers.
//! Privacy pools need to pay fees via CPI (Cross-Program Invocation) from
//! the vault PDA, because:
//!
//! - **Transfer**: No visible tokens exist—value moves entirely within the shielded pool
//! - **Unshield**: Recipient may be any address and cannot sign the transaction
//!
//! This module adds CPI-based fee validation while preserving standard Kora behavior.

pub mod config;
pub mod cpi_validator;
pub mod instruction_context;

pub use config::PrivacyConfig;
pub use cpi_validator::CpiPaymentValidator;
pub use instruction_context::{InstructionContext, InstructionOrigin};
