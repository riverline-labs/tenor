//! Conformance test suite for `TenorStorage` implementations.
//!
//! This module provides a backend-agnostic test suite that any `TenorStorage`
//! implementation can run to verify correctness. The suite covers:
//!
//! - **Initialization**: entity creation, duplicate detection
//! - **Snapshot isolation**: uncommitted writes invisible, committed writes visible
//! - **Atomic commit**: all-or-nothing semantics for multi-record snapshots
//! - **Version validation / OCC**: optimistic concurrency conflict detection
//! - **Provenance coupling**: provenance records tied to operation executions
//! - **Error handling**: correct error variants for invalid operations
//!
//! # Usage
//!
//! Backend crates call [`run_conformance_suite`] with a factory function that
//! creates a fresh, empty storage instance for each test:
//!
//! ```ignore
//! use tenor_storage::conformance::{run_conformance_suite, ConformanceReport};
//!
//! #[tokio::test]
//! async fn postgres_conformance() {
//!     let report = run_conformance_suite(|| async {
//!         create_test_postgres_storage().await
//!     }).await;
//!     assert!(report.failed == 0, "{report}");
//! }
//! ```

mod commit;
mod concurrent;
mod error;
mod init;
mod provenance;
mod snapshot;
mod version;

use std::fmt;
use std::future::Future;

use crate::record::{
    EntityTransitionRecord, FlowExecutionRecord, OperationExecutionRecord, ProvenanceRecord,
};
use crate::TenorStorage;

/// Result of a single conformance test.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test category (e.g. "init", "snapshot", "commit").
    pub category: String,
    /// Test name (e.g. "initialize_creates_entity_at_version_0").
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Error message if the test failed.
    pub message: Option<String>,
}

impl TestResult {
    fn pass(category: &str, name: &str) -> Self {
        Self {
            category: category.to_string(),
            name: name.to_string(),
            passed: true,
            message: None,
        }
    }

    fn fail(category: &str, name: &str, msg: String) -> Self {
        Self {
            category: category.to_string(),
            name: name.to_string(),
            passed: false,
            message: Some(msg),
        }
    }

    fn from_result(category: &str, name: &str, result: Result<(), String>) -> Self {
        match result {
            Ok(()) => Self::pass(category, name),
            Err(msg) => Self::fail(category, name, msg),
        }
    }
}

/// Aggregated report from a full conformance suite run.
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    pub results: Vec<TestResult>,
    pub passed: usize,
    pub failed: usize,
    pub total: usize,
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Conformance: {}/{} passed ({} failed)",
            self.passed, self.total, self.failed
        )?;
        for r in &self.results {
            if !r.passed {
                writeln!(
                    f,
                    "  FAIL [{}/{}]: {}",
                    r.category,
                    r.name,
                    r.message.as_deref().unwrap_or("(no message)")
                )?;
            }
        }
        Ok(())
    }
}

/// Run the full conformance suite against a storage backend.
///
/// The `factory` function is called once per test to create a fresh, empty
/// storage instance, ensuring test isolation.
pub async fn run_conformance_suite<S, F, Fut>(factory: F) -> ConformanceReport
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    results.extend(init::run_init_tests(&factory).await);
    results.extend(error::run_error_tests(&factory).await);
    results.extend(snapshot::run_snapshot_tests(&factory).await);
    results.extend(commit::run_commit_tests(&factory).await);
    results.extend(version::run_version_tests(&factory).await);
    results.extend(provenance::run_provenance_tests(&factory).await);
    results.extend(concurrent::run_concurrent_tests(&factory).await);

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    ConformanceReport {
        results,
        passed,
        failed: total - passed,
        total,
    }
}

// ── Helpers: record constructors with sensible defaults ──────────────────────

fn make_flow_execution(id: &str, flow_id: &str) -> FlowExecutionRecord {
    FlowExecutionRecord {
        id: id.to_string(),
        flow_id: flow_id.to_string(),
        contract_id: "test-contract".to_string(),
        persona_id: "test-persona".to_string(),
        started_at: "2025-01-01T00:00:00Z".to_string(),
        completed_at: Some("2025-01-01T00:01:00Z".to_string()),
        outcome: "success".to_string(),
        snapshot_facts: serde_json::json!({"test": true}),
        snapshot_verdicts: serde_json::json!({"approved": true}),
    }
}

fn make_operation_execution(
    id: &str,
    flow_execution_id: &str,
    operation_id: &str,
) -> OperationExecutionRecord {
    OperationExecutionRecord {
        id: id.to_string(),
        flow_execution_id: flow_execution_id.to_string(),
        operation_id: operation_id.to_string(),
        persona_id: "test-persona".to_string(),
        outcome: "success".to_string(),
        executed_at: "2025-01-01T00:00:30Z".to_string(),
        step_id: "step-1".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_entity_transition(
    id: &str,
    operation_execution_id: &str,
    entity_id: &str,
    instance_id: &str,
    from_state: &str,
    to_state: &str,
    from_version: i64,
    to_version: i64,
) -> EntityTransitionRecord {
    EntityTransitionRecord {
        id: id.to_string(),
        operation_execution_id: operation_execution_id.to_string(),
        entity_id: entity_id.to_string(),
        instance_id: instance_id.to_string(),
        from_state: from_state.to_string(),
        to_state: to_state.to_string(),
        from_version,
        to_version,
    }
}

fn make_provenance_record(id: &str, operation_execution_id: &str) -> ProvenanceRecord {
    ProvenanceRecord {
        id: id.to_string(),
        operation_execution_id: operation_execution_id.to_string(),
        facts_used: serde_json::json!(["fact_a", "fact_b"]),
        verdicts_used: serde_json::json!(["verdict_x"]),
        verdict_set_snapshot: serde_json::json!({"verdict_x": true}),
    }
}
