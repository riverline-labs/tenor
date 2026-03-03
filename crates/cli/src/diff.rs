//! Thin wrapper around `tenor_eval::migration` for CLI diff commands.
//!
//! All diff and classification logic lives in `tenor_eval::migration`.
//! This module re-exports the public API so that `main.rs` continues
//! to work unchanged.

pub use tenor_eval::migration::classify::classify_diff;
pub use tenor_eval::migration::diff::diff_bundles;
