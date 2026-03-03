//! tenor-interchange: Shared interchange JSON types and deserialization.
//!
//! Provides typed structs for all Tenor interchange construct kinds
//! (Fact, Entity, Rule, Operation, Flow, Persona, System, TypeDecl)
//! and a single `from_interchange()` entry point that deserializes
//! a `serde_json::Value` bundle into an `InterchangeBundle`.
//!
//! This crate eliminates triplicated deserialization code in
//! tenor-eval, tenor-analyze, and tenor-codegen. Each consumer
//! depends on this crate for initial JSON parsing, then converts
//! shared types to its own domain-specific representations.

pub mod deserialize;
pub mod types;

pub use deserialize::{from_interchange, InterchangeError};
pub use types::*;
