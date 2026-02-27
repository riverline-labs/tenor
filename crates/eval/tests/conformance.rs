//! Evaluator conformance test suite.
//!
//! Each test case is a fixture triplet:
//! - `<name>.tenor`    -- Tenor source file (elaborated by tenor-core)
//! - `<name>.facts.json` -- fact values for evaluation
//! - `<name>.verdicts.json` -- expected verdict output
//!
//! The runner elaborates the .tenor file, evaluates with the facts,
//! and compares the result against the expected verdicts.

use std::path::{Path, PathBuf};

/// Run an evaluator conformance fixture.
///
/// 1. Elaborate .tenor -> interchange bundle (via tenor-core)
/// 2. Load .facts.json
/// 3. Evaluate bundle + facts -> verdicts
/// 4. Compare verdicts against .verdicts.json
fn run_eval_fixture(fixture_dir: &Path, name: &str) {
    let tenor_path = fixture_dir.join(format!("{}.tenor", name));
    let facts_path = fixture_dir.join(format!("{}.facts.json", name));
    let expected_path = fixture_dir.join(format!("{}.verdicts.json", name));

    // Step 1: Elaborate the .tenor file to get interchange bundle
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));

    // Step 2: Load facts.json
    let facts_str = std::fs::read_to_string(&facts_path)
        .unwrap_or_else(|e| panic!("Failed to read facts for {}: {}", name, e));
    let facts: serde_json::Value = serde_json::from_str(&facts_str)
        .unwrap_or_else(|e| panic!("Invalid facts JSON for {}: {}", name, e));

    // Step 3: Evaluate
    let result = tenor_eval::evaluate(&bundle, &facts)
        .unwrap_or_else(|e| panic!("Evaluation failed for {}: {:?}", name, e));

    // Step 4: Compare with expected verdicts
    let expected_str = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read expected verdicts for {}: {}", name, e));
    let expected: serde_json::Value = serde_json::from_str(&expected_str)
        .unwrap_or_else(|e| panic!("Invalid expected JSON for {}: {}", name, e));

    let actual = result.verdicts.to_json();

    assert_eq!(
        actual,
        expected,
        "Verdict mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}",
        name,
        serde_json::to_string_pretty(&actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    );
}

/// Run an evaluator conformance fixture that expects an evaluation error.
fn run_eval_fixture_error(fixture_dir: &Path, name: &str) {
    let tenor_path = fixture_dir.join(format!("{}.tenor", name));
    let facts_path = fixture_dir.join(format!("{}.facts.json", name));

    // Step 1: Elaborate
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));

    // Step 2: Load facts
    let facts_str = std::fs::read_to_string(&facts_path)
        .unwrap_or_else(|e| panic!("Failed to read facts for {}: {}", name, e));
    let facts: serde_json::Value = serde_json::from_str(&facts_str)
        .unwrap_or_else(|e| panic!("Invalid facts JSON for {}: {}", name, e));

    // Step 3: Evaluate -- should fail
    let result = tenor_eval::evaluate(&bundle, &facts);
    assert!(
        result.is_err(),
        "Expected evaluation error for {}, but got success",
        name
    );
}

/// Run a flow evaluation fixture.
fn run_eval_flow_fixture(fixture_dir: &Path, name: &str, flow_id: &str, persona: &str) {
    let tenor_path = fixture_dir.join(format!("{}.tenor", name));
    run_domain_flow_fixture(fixture_dir, &tenor_path, name, flow_id, persona);
}

/// Run a flow evaluation fixture with a separate contract file path.
/// Used when the contract file name differs from the fixture name
/// (e.g., multi-file contracts where rfp_workflow.tenor is the contract
/// but rfp_approve/rfp_reject/rfp_escalate are fixture names).
fn run_domain_flow_fixture(
    fixture_dir: &Path,
    tenor_path: &Path,
    name: &str,
    flow_id: &str,
    persona: &str,
) {
    let facts_path = fixture_dir.join(format!("{}.facts.json", name));
    let expected_path = fixture_dir.join(format!("{}.verdicts.json", name));

    // Step 1: Elaborate
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));

    // Step 2: Load facts
    let facts_str = std::fs::read_to_string(&facts_path)
        .unwrap_or_else(|e| panic!("Failed to read facts for {}: {}", name, e));
    let facts: serde_json::Value = serde_json::from_str(&facts_str)
        .unwrap_or_else(|e| panic!("Invalid facts JSON for {}: {}", name, e));

    // Step 3: Evaluate flow
    let result = tenor_eval::evaluate_flow(
        &bundle,
        &facts,
        flow_id,
        persona,
        None,
        &tenor_eval::InstanceBindingMap::new(),
    )
    .unwrap_or_else(|e| panic!("Flow evaluation failed for {}: {:?}", name, e));

    // Step 4: Compare verdicts (flow result also includes verdicts from rule eval)
    let expected_str = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read expected verdicts for {}: {}", name, e));
    let expected: serde_json::Value = serde_json::from_str(&expected_str)
        .unwrap_or_else(|e| panic!("Invalid expected JSON for {}: {}", name, e));

    // Build actual output including flow result
    let verdicts_json = result.verdicts.to_json();
    let actual = serde_json::json!({
        "verdicts": verdicts_json["verdicts"],
        "flow_outcome": result.flow_result.outcome,
        "steps_executed": result.flow_result.steps_executed.iter().map(|s| {
            serde_json::json!({
                "step_id": s.step_id,
                "step_type": s.step_type,
                "result": s.result,
            })
        }).collect::<Vec<_>>(),
    });

    assert_eq!(
        actual,
        expected,
        "Verdict mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}",
        name,
        serde_json::to_string_pretty(&actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    );
}

fn positive_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("conformance")
        .join("eval")
        .join("positive")
}

fn frozen_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("conformance")
        .join("eval")
        .join("frozen")
}

fn numeric_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("conformance")
        .join("eval")
        .join("numeric")
}

// ──────────────────────────────────────────────
// Positive evaluation fixtures (15+)
// ──────────────────────────────────────────────

#[test]
fn fact_bool_basic() {
    run_eval_fixture(&positive_dir(), "fact_bool_basic");
}

#[test]
fn fact_int_basic() {
    run_eval_fixture(&positive_dir(), "fact_int_basic");
}

#[test]
fn fact_decimal_basic() {
    run_eval_fixture(&positive_dir(), "fact_decimal_basic");
}

#[test]
fn fact_money_basic() {
    run_eval_fixture(&positive_dir(), "fact_money_basic");
}

#[test]
fn fact_with_default() {
    run_eval_fixture(&positive_dir(), "fact_with_default");
}

#[test]
fn fact_missing_error() {
    run_eval_fixture_error(&positive_dir(), "fact_missing_error");
}

#[test]
fn fact_enum_basic() {
    run_eval_fixture(&positive_dir(), "fact_enum_basic");
}

#[test]
fn fact_text_basic() {
    run_eval_fixture(&positive_dir(), "fact_text_basic");
}

#[test]
fn rule_multi_stratum() {
    run_eval_fixture(&positive_dir(), "rule_multi_stratum");
}

#[test]
fn rule_multiple_same_stratum() {
    run_eval_fixture(&positive_dir(), "rule_multiple_same_stratum");
}

#[test]
fn rule_condition_false() {
    run_eval_fixture(&positive_dir(), "rule_condition_false");
}

#[test]
fn rule_and_or() {
    run_eval_fixture(&positive_dir(), "rule_and_or");
}

#[test]
fn entity_operation_basic() {
    run_eval_flow_fixture(
        &positive_dir(),
        "entity_operation_basic",
        "approval_flow",
        "admin",
    );
}

#[test]
fn operation_persona_check() {
    run_eval_flow_fixture(
        &positive_dir(),
        "operation_persona_check",
        "review_flow",
        "reviewer",
    );
}

#[test]
fn operation_precondition() {
    run_eval_flow_fixture(
        &positive_dir(),
        "operation_precondition",
        "process_flow",
        "admin",
    );
}

#[test]
fn flow_linear_basic() {
    run_eval_flow_fixture(&positive_dir(), "flow_linear_basic", "submit_flow", "buyer");
}

#[test]
fn flow_branch_basic() {
    run_eval_flow_fixture(&positive_dir(), "flow_branch_basic", "check_flow", "system");
}

// ──────────────────────────────────────────────
// Missing flow features (Phase 03.2-03)
// ──────────────────────────────────────────────

#[test]
fn parallel_step() {
    run_eval_flow_fixture(
        &positive_dir(),
        "parallel_step",
        "parallel_process",
        "system",
    );
}

#[test]
fn compensate_handler() {
    run_eval_flow_fixture(
        &positive_dir(),
        "compensate_handler",
        "compensate_flow",
        "admin",
    );
}

#[test]
fn escalate_handler() {
    run_eval_flow_fixture(
        &positive_dir(),
        "escalate_handler",
        "escalate_flow",
        "agent",
    );
}

// ──────────────────────────────────────────────
// Flow error-path fixtures (HARD-20)
// ──────────────────────────────────────────────

#[test]
fn flow_error_escalate() {
    run_eval_flow_fixture(
        &positive_dir(),
        "flow_error_escalate",
        "escalation_flow",
        "reviewer",
    );
}

// ──────────────────────────────────────────────
// Frozen verdict edge cases (EVAL-06)
// ──────────────────────────────────────────────

#[test]
fn flow_frozen_verdicts() {
    run_eval_flow_fixture(
        &frozen_dir(),
        "flow_frozen_verdicts",
        "frozen_test_flow",
        "admin",
    );
}

#[test]
fn flow_frozen_facts() {
    run_eval_flow_fixture(
        &frozen_dir(),
        "flow_frozen_facts",
        "frozen_facts_flow",
        "admin",
    );
}

#[test]
fn flow_subflow_snapshot() {
    run_eval_flow_fixture(
        &frozen_dir(),
        "flow_subflow_snapshot",
        "parent_flow",
        "admin",
    );
}

// ──────────────────────────────────────────────
// Domain validation — Escrow Release (Phase 5)
// ──────────────────────────────────────────────

#[test]
fn escrow_release() {
    run_eval_flow_fixture(
        &positive_dir(),
        "escrow_release",
        "standard_release",
        "seller",
    );
}

#[test]
fn escrow_compliance() {
    run_eval_flow_fixture(
        &positive_dir(),
        "escrow_compliance",
        "standard_release",
        "seller",
    );
}

#[test]
fn escrow_compensate() {
    run_eval_flow_fixture(
        &positive_dir(),
        "escrow_compensate",
        "standard_release",
        "seller",
    );
}

// ──────────────────────────────────────────────
// Numeric precision fixtures (EVAL-07, TEST-09)
// ──────────────────────────────────────────────

#[test]
fn numeric_int_promotion() {
    run_eval_fixture(&numeric_dir(), "int_promotion");
}

#[test]
fn numeric_decimal_rounding() {
    run_eval_fixture(&numeric_dir(), "decimal_rounding");
}

#[test]
fn numeric_money_comparison() {
    run_eval_fixture(&numeric_dir(), "money_comparison");
}

#[test]
fn numeric_decimal_overflow() {
    run_eval_fixture_error(&numeric_dir(), "decimal_overflow");
}

// ──────────────────────────────────────────────
// Domain validation — SaaS Subscription (Phase 5)
// ──────────────────────────────────────────────

fn domains_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("domains")
}

#[test]
fn domain_saas_activate() {
    run_eval_flow_fixture(
        &domains_dir().join("saas"),
        "saas_activate",
        "subscription_lifecycle",
        "billing_system",
    );
}

#[test]
fn domain_saas_suspend() {
    run_eval_flow_fixture(
        &domains_dir().join("saas"),
        "saas_suspend",
        "subscription_lifecycle",
        "billing_system",
    );
}

// ──────────────────────────────────────────────
// Domain validation — Supply Chain Inspection (Phase 5)
// ──────────────────────────────────────────────

#[test]
fn domain_supply_chain_pass() {
    run_eval_flow_fixture(
        &domains_dir().join("supply_chain"),
        "inspection_pass",
        "inspection_flow",
        "customs_officer",
    );
}

#[test]
fn domain_supply_chain_hold() {
    run_eval_flow_fixture(
        &domains_dir().join("supply_chain"),
        "inspection_hold",
        "inspection_flow",
        "customs_officer",
    );
}

// ──────────────────────────────────────────────
// Domain validation — Healthcare Prior Auth (Phase 5)
// ──────────────────────────────────────────────

#[test]
fn domain_healthcare_approve() {
    run_eval_flow_fixture(
        &domains_dir().join("healthcare"),
        "prior_auth_approve",
        "auth_review_flow",
        "requesting_physician",
    );
}

#[test]
fn domain_healthcare_deny() {
    run_eval_flow_fixture(
        &domains_dir().join("healthcare"),
        "prior_auth_deny",
        "auth_review_flow",
        "requesting_physician",
    );
}

#[test]
fn domain_healthcare_appeal() {
    run_eval_flow_fixture(
        &domains_dir().join("healthcare"),
        "prior_auth_appeal",
        "auth_review_flow",
        "requesting_physician",
    );
}

// ──────────────────────────────────────────────
// Domain validation — Trade Finance LC (Phase 5)
// ──────────────────────────────────────────────

#[test]
fn domain_trade_finance_present() {
    run_eval_flow_fixture(
        &domains_dir().join("trade_finance"),
        "lc_present",
        "lc_presentation_flow",
        "beneficiary",
    );
}

#[test]
fn domain_trade_finance_discrepancy() {
    run_eval_flow_fixture(
        &domains_dir().join("trade_finance"),
        "lc_discrepancy",
        "lc_presentation_flow",
        "beneficiary",
    );
}

// ──────────────────────────────────────────────
// Domain validation — Energy Procurement RFP (Phase 5)
// ──────────────────────────────────────────────

#[test]
fn domain_energy_approve() {
    let dir = domains_dir().join("energy_procurement");
    run_domain_flow_fixture(
        &dir,
        &dir.join("rfp_workflow.tenor"),
        "rfp_approve",
        "rfp_approval_flow",
        "procurement_manager",
    );
}

#[test]
fn domain_energy_reject() {
    let dir = domains_dir().join("energy_procurement");
    run_domain_flow_fixture(
        &dir,
        &dir.join("rfp_workflow.tenor"),
        "rfp_reject",
        "rfp_approval_flow",
        "procurement_manager",
    );
}

#[test]
fn domain_energy_escalate() {
    let dir = domains_dir().join("energy_procurement");
    run_domain_flow_fixture(
        &dir,
        &dir.join("rfp_workflow.tenor"),
        "rfp_escalate",
        "rfp_approval_flow",
        "procurement_manager",
    );
}

// ──────────────────────────────────────────────
// Executor conformance — E11, E13 (Phase 5)
// ──────────────────────────────────────────────

/// Collect all fact_ref and verdict_present references from a predicate
/// expression tree (recursive JSON walk).
fn collect_refs_from_predicate(
    expr: &serde_json::Value,
    fact_refs: &mut Vec<String>,
    verdict_refs: &mut Vec<String>,
) {
    if let Some(fr) = expr.get("fact_ref").and_then(|v| v.as_str()) {
        fact_refs.push(fr.to_string());
    }
    if let Some(vr) = expr.get("verdict_present").and_then(|v| v.as_str()) {
        verdict_refs.push(vr.to_string());
    }
    // Recurse into left/right for binary expressions (Compare, And, Or)
    if let Some(left) = expr.get("left") {
        collect_refs_from_predicate(left, fact_refs, verdict_refs);
    }
    if let Some(right) = expr.get("right") {
        collect_refs_from_predicate(right, fact_refs, verdict_refs);
    }
    // Not expressions
    if let Some(inner) = expr.get("inner") {
        collect_refs_from_predicate(inner, fact_refs, verdict_refs);
    }
}

/// E11: Cold-Start Completeness.
/// Validates that a domain contract's interchange bundle is fully
/// self-contained: every fact_ref in rules points to a declared fact,
/// every persona in operations points to a declared persona, every
/// entity in operations points to a declared entity, and every
/// operation in flows points to a declared operation.
#[test]
fn e11_cold_start_completeness() {
    // Use healthcare contract -- highest construct density
    let tenor_path = domains_dir().join("healthcare").join("prior_auth.tenor");

    // Elaborate to interchange bundle
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate healthcare contract: {:?}", e));

    // Parse constructs from the bundle
    let constructs = bundle["constructs"]
        .as_array()
        .expect("bundle should have constructs array");

    // Build index of declared construct IDs by kind
    let mut fact_ids: Vec<String> = Vec::new();
    let mut entity_ids: Vec<String> = Vec::new();
    let mut persona_ids: Vec<String> = Vec::new();
    let mut operation_ids: Vec<String> = Vec::new();
    let mut rule_ids: Vec<String> = Vec::new();

    for construct in constructs {
        let kind = construct["kind"].as_str().unwrap_or("");
        let id = construct["id"].as_str().unwrap_or("").to_string();
        match kind {
            "Fact" => fact_ids.push(id),
            "Entity" => entity_ids.push(id),
            "Persona" => persona_ids.push(id),
            "Operation" => operation_ids.push(id),
            "Rule" => rule_ids.push(id),
            _ => {}
        }
    }

    // Verify we found constructs (sanity check)
    assert!(!fact_ids.is_empty(), "should have facts");
    assert!(!entity_ids.is_empty(), "should have entities");
    assert!(!persona_ids.is_empty(), "should have personas");
    assert!(!operation_ids.is_empty(), "should have operations");
    assert!(!rule_ids.is_empty(), "should have rules");

    // Check 1: Every fact_ref in rules points to a declared fact
    for construct in constructs {
        if construct["kind"].as_str() != Some("Rule") {
            continue;
        }
        let rule_id = construct["id"].as_str().unwrap_or("unknown");
        if let Some(body) = construct.get("body") {
            if let Some(when) = body.get("when") {
                let mut fact_refs = Vec::new();
                let mut verdict_refs = Vec::new();
                collect_refs_from_predicate(when, &mut fact_refs, &mut verdict_refs);

                for fr in &fact_refs {
                    assert!(
                        fact_ids.contains(fr),
                        "Rule '{}' references undeclared fact '{}'",
                        rule_id,
                        fr
                    );
                }
            }
        }
    }

    // Check 2: Every persona in operations points to a declared persona
    for construct in constructs {
        if construct["kind"].as_str() != Some("Operation") {
            continue;
        }
        let op_id = construct["id"].as_str().unwrap_or("unknown");
        if let Some(personas) = construct.get("allowed_personas").and_then(|v| v.as_array()) {
            for p in personas {
                if let Some(persona_str) = p.as_str() {
                    assert!(
                        persona_ids.contains(&persona_str.to_string()),
                        "Operation '{}' references undeclared persona '{}'",
                        op_id,
                        persona_str
                    );
                }
            }
        }
    }

    // Check 3: Every entity in operation effects points to a declared entity
    for construct in constructs {
        if construct["kind"].as_str() != Some("Operation") {
            continue;
        }
        let op_id = construct["id"].as_str().unwrap_or("unknown");
        if let Some(effects) = construct.get("effects").and_then(|v| v.as_array()) {
            for effect in effects {
                if let Some(entity_id) = effect.get("entity_id").and_then(|v| v.as_str()) {
                    assert!(
                        entity_ids.contains(&entity_id.to_string()),
                        "Operation '{}' effect references undeclared entity '{}'",
                        op_id,
                        entity_id
                    );
                }
            }
        }
    }

    // Check 4: Every operation ref in flows points to a declared operation
    for construct in constructs {
        if construct["kind"].as_str() != Some("Flow") {
            continue;
        }
        let flow_id = construct["id"].as_str().unwrap_or("unknown");
        if let Some(steps) = construct.get("steps").and_then(|v| v.as_array()) {
            for step in steps {
                if let Some(op) = step.get("op").and_then(|v| v.as_str()) {
                    assert!(
                        operation_ids.contains(&op.to_string()),
                        "Flow '{}' step references undeclared operation '{}'",
                        flow_id,
                        op
                    );
                }
            }
        }
    }
}

/// E13: Dry-Run Evaluation Semantics.
/// Validates that rule-only evaluation produces verdicts without
/// side effects. The same input must produce the same output
/// (determinism), and no entity state changes occur.
#[test]
fn e13_dry_run_rule_evaluation() {
    // Use SaaS domain contract
    let tenor_path = domains_dir().join("saas").join("saas_subscription.tenor");

    // Elaborate to interchange bundle
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate SaaS contract: {:?}", e));

    // Load facts for the activate scenario
    let facts_str =
        std::fs::read_to_string(domains_dir().join("saas").join("saas_activate.facts.json"))
            .expect("Failed to read SaaS activate facts");
    let facts: serde_json::Value = serde_json::from_str(&facts_str).expect("Invalid facts JSON");

    // First evaluation (rules only -- no flows, no entity mutations)
    let result1 = tenor_eval::evaluate(&bundle, &facts).expect("First evaluation failed");

    // Verify verdicts are produced
    assert!(
        !result1.verdicts.0.is_empty(),
        "dry-run evaluation should produce verdicts"
    );

    // Second evaluation with same inputs
    let result2 = tenor_eval::evaluate(&bundle, &facts).expect("Second evaluation failed");

    // Verify determinism: same input = same output
    assert_eq!(
        result1.verdicts.0.len(),
        result2.verdicts.0.len(),
        "same inputs should produce same number of verdicts"
    );

    // Verify each verdict matches (type and payload)
    let json1 = result1.verdicts.to_json();
    let json2 = result2.verdicts.to_json();
    assert_eq!(
        json1, json2,
        "dry-run evaluation must be deterministic: same input = same output"
    );

    // Verify that evaluate() is a pure function: no entity state tracking
    // occurs in rules-only evaluation (evaluate() does not call any
    // entity state initialization or flow execution code).
    // This is validated by the fact that evaluate() returns EvalResult
    // which contains only VerdictSet -- no EntityStateMap, no FlowResult.
    // The type system enforces that no entity mutations can leak through.
}

/// E13: Dry-Run with healthcare contract (more complex rule set).
/// Validates determinism with a larger, multi-stratum rule set.
#[test]
fn e13_dry_run_healthcare_determinism() {
    let tenor_path = domains_dir().join("healthcare").join("prior_auth.tenor");

    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate healthcare contract: {:?}", e));

    // Load the approve scenario facts
    let facts_str = std::fs::read_to_string(
        domains_dir()
            .join("healthcare")
            .join("prior_auth_approve.facts.json"),
    )
    .expect("Failed to read healthcare facts");
    let facts: serde_json::Value = serde_json::from_str(&facts_str).expect("Invalid facts JSON");

    // Evaluate three times to verify determinism
    let r1 = tenor_eval::evaluate(&bundle, &facts).expect("eval 1 failed");
    let r2 = tenor_eval::evaluate(&bundle, &facts).expect("eval 2 failed");
    let r3 = tenor_eval::evaluate(&bundle, &facts).expect("eval 3 failed");

    let j1 = r1.verdicts.to_json();
    let j2 = r2.verdicts.to_json();
    let j3 = r3.verdicts.to_json();

    assert_eq!(
        j1, j2,
        "healthcare evaluation must be deterministic (run 1 vs 2)"
    );
    assert_eq!(
        j2, j3,
        "healthcare evaluation must be deterministic (run 2 vs 3)"
    );
    assert!(
        !r1.verdicts.0.is_empty(),
        "healthcare contract should produce verdicts"
    );
}
