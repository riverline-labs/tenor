//! Trust and security utilities for the Tenor CLI.
//!
//! Provides Ed25519 key generation, contract bundle signing/verification,
//! and WASM binary signing/verification.

pub mod keygen;
pub mod sign_wasm;
pub mod verify_wasm;
