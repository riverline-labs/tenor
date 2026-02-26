//! Migration analysis module.
//!
//! Provides diffing, classification, analysis, and planning for contract
//! version migrations per spec Section 18.

pub mod analysis;
pub mod classify;
pub mod diff;
pub mod error;
pub mod plan;

pub use classify::{
    classify_diff, ChangeClassification, ChangeSeverity, ClassificationSummary, ClassifiedChange,
    ClassifiedConstruct, ClassifiedDiff, ClassifiedFieldDiff,
};
pub use diff::{diff_bundles, BundleDiff, ConstructChange, ConstructSummary, DiffError, FieldDiff};
pub use error::MigrationError;
