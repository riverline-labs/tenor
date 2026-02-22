//! Integration tests for the S1-S8 static analysis suite.
//!
//! These tests elaborate real .tenor fixtures, then run the full analysis
//! pipeline and verify the results against known expectations.

use std::path::{Path, PathBuf};

/// Locate the workspace root.
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Elaborate a fixture and return the interchange JSON bundle.
fn elaborate_fixture(fixture: &str) -> serde_json::Value {
    let path = workspace_root().join(fixture);
    tenor_core::elaborate::elaborate(&path).unwrap_or_else(|e| {
        panic!("elaboration failed for {}: {:?}", fixture, e);
    })
}

/// Elaborate a fixture and run the full analysis suite.
fn elaborate_and_analyze(fixture: &str) -> tenor_analyze::AnalysisReport {
    let bundle = elaborate_fixture(fixture);
    tenor_analyze::analyze(&bundle).unwrap_or_else(|e| {
        panic!("analysis failed for {}: {}", fixture, e);
    })
}

// ──────────────────────────────────────────────
// S1 — State Space
// ──────────────────────────────────────────────

#[test]
fn test_s1_entity_basic() {
    let report = elaborate_and_analyze("conformance/positive/entity_basic.tenor");
    let s1 = report.s1_state_space.expect("S1 should be populated");

    assert!(s1.entities.contains_key("Order"));
    let order = &s1.entities["Order"];
    assert_eq!(order.state_count, 6);
    assert_eq!(order.initial_state, "draft");
    assert!(order.declared_states.contains(&"draft".to_string()));
    assert!(order.declared_states.contains(&"submitted".to_string()));
    assert!(order.declared_states.contains(&"approved".to_string()));
}

#[test]
fn test_s1_escrow() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let s1 = report.s1_state_space.expect("S1 should be populated");

    // Escrow has DeliveryRecord and EscrowAccount entities
    assert!(s1.entities.len() >= 2);
    assert!(s1.entities.contains_key("DeliveryRecord"));
    assert!(s1.entities.contains_key("EscrowAccount"));
}

// ──────────────────────────────────────────────
// S2 — Reachability
// ──────────────────────────────────────────────

#[test]
fn test_s2_all_reachable() {
    let report = elaborate_and_analyze("conformance/positive/entity_basic.tenor");
    let s2 = report.s2_reachability.expect("S2 should be populated");

    assert!(!s2.has_dead_states, "entity_basic should have no dead states");
    for (_, result) in &s2.entities {
        assert!(
            result.unreachable_states.is_empty(),
            "entity {} should have no unreachable states",
            result.entity_id
        );
    }
}

#[test]
fn test_s2_dead_states() {
    let report = elaborate_and_analyze("conformance/analysis/dead_states.tenor");
    let s2 = report.s2_reachability.expect("S2 should be populated");

    assert!(s2.has_dead_states, "dead_states fixture should detect dead states");
    let order = &s2.entities["order"];
    assert!(
        order.unreachable_states.contains("archived"),
        "'archived' should be unreachable; got: {:?}",
        order.unreachable_states
    );
}

// ──────────────────────────────────────────────
// S3a — Admissibility
// ──────────────────────────────────────────────

#[test]
fn test_s3a_escrow() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let s3a = report.s3a_admissibility.expect("S3a should be populated");

    // Escrow has entities, personas, and operations -- should have some admissible combinations
    assert!(
        s3a.total_combinations_checked > 0,
        "should have checked at least one combination"
    );
}

// ──────────────────────────────────────────────
// S4 — Authority
// ──────────────────────────────────────────────

#[test]
fn test_s4_authority() {
    let report = elaborate_and_analyze("conformance/analysis/authority_basic.tenor");
    let s4 = report.s4_authority.expect("S4 should be populated");

    assert!(
        s4.persona_authority.contains_key("admin"),
        "admin should have authority"
    );
    assert!(
        s4.persona_authority.contains_key("user"),
        "user should have authority"
    );

    // Admin can close from resolved and open states
    let admin = &s4.persona_authority["admin"];
    assert!(admin.by_entity.contains_key("ticket"));

    // User can assign and resolve but not close
    let user = &s4.persona_authority["user"];
    assert!(user.by_entity.contains_key("ticket"));

    // Verify transition authorities
    assert!(
        !s4.transition_authorities.is_empty(),
        "should have transition authorities"
    );
}

// ──────────────────────────────────────────────
// S5 — Verdicts
// ──────────────────────────────────────────────

#[test]
fn test_s5_verdict_types() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let s5 = report.s5_verdicts.expect("S5 should be populated");

    assert!(
        s5.total_verdict_types > 0,
        "escrow should have verdict types from rules"
    );
}

#[test]
fn test_s5_operation_outcomes() {
    let report = elaborate_and_analyze("conformance/positive/operation_outcomes.tenor");
    let s5 = report.s5_verdicts.expect("S5 should be populated");

    assert!(
        s5.total_operations_with_outcomes > 0,
        "operation_outcomes should have operations with outcomes"
    );
}

// ──────────────────────────────────────────────
// S6 — Flow Paths
// ──────────────────────────────────────────────

#[test]
fn test_s6_flow_paths() {
    let report = elaborate_and_analyze("conformance/analysis/flow_branching.tenor");
    let s6 = report.s6_flow_paths.expect("S6 should be populated");

    assert!(!s6.flows.is_empty(), "should have at least one flow");
    let flow = &s6.flows["request_processing"];
    assert!(
        flow.path_count >= 2,
        "branching flow should have at least 2 paths; got {}",
        flow.path_count
    );
}

#[test]
fn test_s6_escrow_flow() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let s6 = report.s6_flow_paths.expect("S6 should be populated");

    assert!(
        s6.total_paths > 0,
        "escrow should have flow paths"
    );
}

// ──────────────────────────────────────────────
// S7 — Complexity
// ──────────────────────────────────────────────

#[test]
fn test_s7_complexity() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let s7 = report.s7_complexity.expect("S7 should be populated");

    // Escrow has rules with predicates
    assert!(
        !s7.predicate_complexities.is_empty(),
        "escrow should have predicate complexity data"
    );
}

#[test]
fn test_s7_flow_depth() {
    let report = elaborate_and_analyze("conformance/analysis/flow_branching.tenor");
    let s7 = report.s7_complexity.expect("S7 should be populated");

    assert!(
        s7.max_flow_depth > 0,
        "flow_branching should have non-zero flow depth"
    );
}

// ──────────────────────────────────────────────
// S8 — Verdict Uniqueness
// ──────────────────────────────────────────────

#[test]
fn test_s8_verdict_uniqueness() {
    let report = elaborate_and_analyze("conformance/positive/entity_basic.tenor");
    let s8 = report
        .s8_verdict_uniqueness
        .expect("S8 should be populated");
    assert!(s8.pre_verified, "S8 should always be pre-verified");
}

// ──────────────────────────────────────────────
// Full Integration
// ──────────────────────────────────────────────

#[test]
fn test_full_analysis_escrow() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");

    assert!(report.s1_state_space.is_some());
    assert!(report.s2_reachability.is_some());
    assert!(report.s3a_admissibility.is_some());
    assert!(report.s4_authority.is_some());
    assert!(report.s5_verdicts.is_some());
    assert!(report.s6_flow_paths.is_some());
    assert!(report.s7_complexity.is_some());
    assert!(report.s8_verdict_uniqueness.is_some());
    assert_eq!(report.analyses_run.len(), 8);
}

#[test]
fn test_analyze_selected() {
    let bundle = elaborate_fixture("conformance/positive/integration_escrow.tenor");
    let report = tenor_analyze::analyze_selected(&bundle, &["s1", "s2"]).unwrap();

    assert!(report.s1_state_space.is_some());
    assert!(report.s2_reachability.is_some());
    assert!(report.s3a_admissibility.is_none());
    assert!(report.s4_authority.is_none());
    assert_eq!(report.analyses_run, vec!["s1", "s2"]);
}

#[test]
fn test_findings_dead_states() {
    let report = elaborate_and_analyze("conformance/analysis/dead_states.tenor");

    // Should have a finding about the dead state
    assert!(
        !report.findings.is_empty(),
        "dead_states fixture should produce findings"
    );
    let s2_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.analysis == "s2")
        .collect();
    assert!(
        !s2_findings.is_empty(),
        "should have S2 findings about dead states"
    );
    assert!(
        s2_findings[0].message.contains("archived"),
        "finding should mention 'archived'; got: {}",
        s2_findings[0].message
    );
}

#[test]
fn test_report_serializable() {
    let report = elaborate_and_analyze("conformance/positive/integration_escrow.tenor");
    let json = serde_json::to_value(&report).unwrap();
    assert!(json.is_object());
    assert!(json.get("analyses_run").unwrap().is_array());
    assert!(json.get("findings").unwrap().is_array());
}
