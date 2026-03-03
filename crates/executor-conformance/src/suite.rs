//! The `executor_conformance_tests!` macro.
//!
//! This macro generates 20 `#[tokio::test]` functions — one per executor
//! obligation E1 through E20 — for any type that implements `TestableExecutor`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tenor_executor_conformance::executor_conformance_tests;
//!
//! struct MyExecutor { /* ... */ }
//!
//! impl MyExecutor {
//!     fn new() -> Self { MyExecutor {} }
//! }
//!
//! // Implement TestableExecutor for MyExecutor ...
//!
//! executor_conformance_tests!(MyExecutor::new());
//! ```
//!
//! Each generated test function is named `conformance_eNN_<description>` and
//! can be run with `cargo test conformance_` to execute the full suite.

/// Generate conformance tests for an executor implementation.
///
/// The `$executor_expr` expression is evaluated fresh for each test, so
/// each test gets an independent executor instance. Tests can be run in
/// any order.
#[macro_export]
macro_rules! executor_conformance_tests {
    ($executor_expr:expr) => {
        #[tokio::test]
        async fn conformance_e01_fact_source() {
            let executor = $executor_expr;
            $crate::tests::e01_fact_source::test_e01_fact_source_only(&executor)
                .await
                .expect("E1: fact source conformance failed");
        }

        #[tokio::test]
        async fn conformance_e02_transition_validation() {
            let executor = $executor_expr;
            $crate::tests::e02_transition_validation::test_e02_transition_validation(&executor)
                .await
                .expect("E2: transition validation conformance failed");
        }

        #[tokio::test]
        async fn conformance_e03_atomicity() {
            let executor = $executor_expr;
            $crate::tests::e03_atomicity::test_e03_atomicity(&executor)
                .await
                .expect("E3: atomicity conformance failed");
        }

        #[tokio::test]
        async fn conformance_e04_snapshot_isolation() {
            let executor = $executor_expr;
            $crate::tests::e04_snapshot_isolation::test_e04_snapshot_isolation(&executor)
                .await
                .expect("E4: snapshot isolation conformance failed");
        }

        #[tokio::test]
        async fn conformance_e05_subflow_snapshot() {
            let executor = $executor_expr;
            $crate::tests::e05_subflow_snapshot::test_e05_subflow_snapshot(&executor)
                .await
                .expect("E5: sub-flow snapshot conformance failed");
        }

        #[tokio::test]
        async fn conformance_e06_datetime_utc() {
            let executor = $executor_expr;
            $crate::tests::e06_datetime_utc::test_e06_datetime_utc(&executor)
                .await
                .expect("E6: DateTime UTC normalization conformance failed");
        }

        #[tokio::test]
        async fn conformance_e07_numeric_model() {
            let executor = $executor_expr;
            $crate::tests::e07_numeric_model::test_e07_numeric_model(&executor)
                .await
                .expect("E7: numeric model conformance failed");
        }

        #[tokio::test]
        async fn conformance_e08_branch_isolation() {
            let executor = $executor_expr;
            $crate::tests::e08_branch_isolation::test_e08_branch_isolation(&executor)
                .await
                .expect("E8: branch isolation conformance failed");
        }

        #[tokio::test]
        async fn conformance_e09_join_completion() {
            let executor = $executor_expr;
            $crate::tests::e09_join_completion::test_e09_join_completion(&executor)
                .await
                .expect("E9: join completion conformance failed");
        }

        #[tokio::test]
        async fn conformance_e10_manifest_endpoint() {
            let executor = $executor_expr;
            $crate::tests::e10_manifest_endpoint::test_e10_manifest_endpoint(&executor)
                .await
                .expect("E10: manifest endpoint conformance failed");
        }

        #[tokio::test]
        async fn conformance_e11_manifest_bundle() {
            let executor = $executor_expr;
            $crate::tests::e11_manifest_bundle::test_e11_manifest_bundle_complete(&executor)
                .await
                .expect("E11: manifest bundle completeness conformance failed");
        }

        #[tokio::test]
        async fn conformance_e12_etag_stability() {
            let executor = $executor_expr;
            $crate::tests::e12_etag_stability::test_e12_etag_stability(&executor)
                .await
                .expect("E12: ETag stability conformance failed");
        }

        #[tokio::test]
        async fn conformance_e13_dry_run() {
            let executor = $executor_expr;
            $crate::tests::e13_dry_run::test_e13_dry_run(&executor)
                .await
                .expect("E13: dry-run conformance failed");
        }

        #[tokio::test]
        async fn conformance_e14_capability_advertisement() {
            let executor = $executor_expr;
            $crate::tests::e14_capability_advertisement::test_e14_capability_advertisement(
                &executor,
            )
            .await
            .expect("E14: capability advertisement conformance failed");
        }

        #[tokio::test]
        async fn conformance_e15_instance_creation() {
            let executor = $executor_expr;
            $crate::tests::e15_instance_creation::test_e15_instance_creation(&executor)
                .await
                .expect("E15: instance creation conformance failed");
        }

        #[tokio::test]
        async fn conformance_e16_instance_identity() {
            let executor = $executor_expr;
            $crate::tests::e16_instance_identity::test_e16_instance_identity(&executor)
                .await
                .expect("E16: instance identity conformance failed");
        }

        #[tokio::test]
        async fn conformance_e17_instance_enumeration() {
            let executor = $executor_expr;
            $crate::tests::e17_instance_enumeration::test_e17_instance_enumeration(&executor)
                .await
                .expect("E17: instance enumeration conformance failed");
        }

        #[tokio::test]
        async fn conformance_e18_artifact_integrity() {
            let executor = $executor_expr;
            $crate::tests::e18_artifact_integrity::test_e18_artifact_integrity(&executor)
                .await
                .expect("E18: artifact integrity conformance failed");
        }

        #[tokio::test]
        async fn conformance_e19_provenance_authenticity() {
            let executor = $executor_expr;
            $crate::tests::e19_provenance_authenticity::test_e19_provenance_authenticity(&executor)
                .await
                .expect("E19: provenance authenticity conformance failed");
        }

        #[tokio::test]
        async fn conformance_e20_trust_domain() {
            let executor = $executor_expr;
            $crate::tests::e20_trust_domain::test_e20_trust_domain(&executor)
                .await
                .expect("E20: trust domain conformance failed");
        }
    };
}
