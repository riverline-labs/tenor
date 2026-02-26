//! Migration analysis module.
//!
//! Provides diffing, classification, analysis, and planning for contract
//! version migrations per spec Section 18.

pub mod analysis;
pub mod classify;
pub mod compatibility;
pub mod diff;
pub mod error;
pub mod plan;

pub use analysis::{
    analyze_migration, BreakingChange, EntityAction, EntityMigrationAction, MigrationAnalysis,
    MigrationSeverity,
};
pub use classify::{
    classify_diff, ChangeClassification, ChangeSeverity, ClassificationSummary, ClassifiedChange,
    ClassifiedConstruct, ClassifiedDiff, ClassifiedFieldDiff,
};
pub use compatibility::{check_flow_compatibility, check_flow_compatibility_static};
pub use diff::{diff_bundles, BundleDiff, ConstructChange, ConstructSummary, DiffError, FieldDiff};
pub use error::MigrationError;
pub use plan::{
    build_migration_plan, EntityStateMapping, FlowCompatibilityResult, IncompatibilityReason,
    LayerResults, MigrationPlan, MigrationPolicy,
};
