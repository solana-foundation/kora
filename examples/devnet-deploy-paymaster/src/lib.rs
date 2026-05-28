//! Devnet deploy-paymaster helpers.
//!
//! This crate hosts the binaries that live next to the devnet
//! deploy-paymaster's config (`kora.toml`, `signers.toml`, Dockerfile). The
//! production Kora library and CLI deliberately stay free of devnet-only
//! behavior, so the reaper that closes idle programs lives here instead of in
//! `crates/lib`.

pub mod reaper;
