//! Tenor executor conformance test suite (E1-E20).
//!
//! Provides a `TestableExecutor` trait and `executor_conformance_tests!`
//! macro for validating any executor implementation against the full
//! set of executor obligations from TENOR.md Section 17.

pub mod fixtures;
pub mod suite;
pub mod tests;
pub mod traits;

pub use traits::*;
