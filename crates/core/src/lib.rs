#![allow(clippy::result_large_err)]
//! tenor-core: Tenor elaborator core library.
//!
//! Provides the six-pass elaboration pipeline from `.tenor` source files
//! to TenorInterchange JSON bundles.
//!
//! # Public API
//!
//! Key types are re-exported at the crate root for convenience:
//!
//! - [`elaborate()`] -- run the full 6-pass pipeline
//! - [`Index`] -- construct lookup index (Pass 2 output)
//! - [`TypeEnv`] -- name-to-concrete-type map (Pass 3 output)
//! - [`ElabError`] -- elaboration error type
//! - AST types: [`RawConstruct`], [`RawType`], [`RawExpr`], [`RawTerm`],
//!   [`RawLiteral`], [`Provenance`]
//!
//! Individual pass entry functions are also re-exported for selective
//! pipeline execution.

/// Tenor spec version used in per-construct `"tenor"` fields (e.g., "1.0").
pub const TENOR_VERSION: &str = "1.0";
/// Tenor interchange bundle version (semver, e.g., "1.1.0").
pub const TENOR_BUNDLE_VERSION: &str = "1.1.0";

pub mod ast;
pub mod elaborate;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod pass1_bundle;
pub mod pass2_index;
pub mod pass3_types;
pub mod pass4_typecheck;
pub mod pass5_validate;
pub mod pass6_serialize;

// ── Convenience re-exports: key types ────────────────────────────────

pub use ast::{Provenance, RawConstruct, RawExpr, RawLiteral, RawTerm, RawType};
pub use error::ElabError;
pub use pass2_index::Index;
pub use pass3_types::TypeEnv;

// ── Convenience re-exports: pipeline entry points ────────────────────

pub use elaborate::elaborate;
pub use pass1_bundle::load_bundle;
pub use pass2_index::build_index;
pub use pass3_types::build_type_env;
pub use pass4_typecheck::resolve_types;
