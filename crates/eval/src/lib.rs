//! Tenor contract evaluator -- accepts interchange bundle + facts,
//! produces verdicts with provenance.
//!
//! The evaluator consumes interchange JSON (not raw DSL), assembles
//! a FactSet from external inputs, runs stratified rule evaluation,
//! and produces provenance-traced verdicts.
//!
//! Implementation: Phase 3.

pub mod assemble;
pub mod flow;
pub mod numeric;
pub mod operation;
pub mod predicate;
pub mod provenance;
pub mod rules;
pub mod types;

pub use flow::{FlowEvalResult, FlowResult, Snapshot, StepRecord};
pub use operation::{
    EffectRecord, EntityStateMap, OperationError, OperationProvenance, OperationResult,
};
pub use types::{Contract, EvalError, FactSet, Value, VerdictInstance, VerdictSet};

/// Result of evaluating a contract against facts (rules only).
pub struct EvalResult {
    pub verdicts: VerdictSet,
}

/// Evaluate a contract against provided facts (rules only).
///
/// This is the top-level public API for rules-only evaluation.
/// For flow execution, use `evaluate_flow` instead.
///
/// # Arguments
/// * `bundle` - Interchange JSON bundle (serde_json::Value)
/// * `facts` - Facts JSON object mapping fact IDs to values
///
/// # Returns
/// * `EvalResult` containing the verdict set, or an `EvalError`
pub fn evaluate(
    bundle: &serde_json::Value,
    facts: &serde_json::Value,
) -> Result<EvalResult, EvalError> {
    let contract = Contract::from_interchange(bundle)?;
    let fact_set = assemble::assemble_facts(&contract, facts)?;
    let verdict_set = rules::eval_strata(&contract, &fact_set)?;
    Ok(EvalResult {
        verdicts: verdict_set,
    })
}

/// Evaluate a contract and execute a named flow.
///
/// Runs the full evaluation pipeline (spec Section 14):
/// 1. Deserialize contract from interchange JSON
/// 2. Assemble facts
/// 3. Evaluate rules (stratified) to produce verdicts
/// 4. Create Snapshot from current FactSet + VerdictSet
/// 5. Initialize entity states from contract
/// 6. Execute the named flow against the snapshot
///
/// # Arguments
/// * `bundle` - Interchange JSON bundle
/// * `facts` - Facts JSON object
/// * `flow_id` - ID of the flow to execute
/// * `persona` - Persona executing the flow (recorded for provenance; per spec Section 11.4,
///   flow-level persona authorization is delegated to step-level Operation persona checks)
///
/// # Returns
/// * `FlowEvalResult` containing verdicts and flow execution result
pub fn evaluate_flow(
    bundle: &serde_json::Value,
    facts: &serde_json::Value,
    flow_id: &str,
    persona: &str,
) -> Result<FlowEvalResult, EvalError> {
    let contract = Contract::from_interchange(bundle)?;
    let fact_set = assemble::assemble_facts(&contract, facts)?;
    let verdict_set = rules::eval_strata(&contract, &fact_set)?;

    // Create frozen snapshot
    let snapshot = Snapshot {
        facts: fact_set,
        verdicts: verdict_set.clone(),
    };

    // Initialize entity states
    let mut entity_states = operation::init_entity_states(&contract);

    // Find the flow
    let target_flow = contract
        .flows
        .iter()
        .find(|f| f.id == flow_id)
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("flow '{}' not found in contract", flow_id),
        })?;

    // Execute the flow
    let mut flow_result =
        flow::execute_flow(target_flow, &contract, &snapshot, &mut entity_states, None)?;

    // Per spec Section 11.4: initiating_persona is recorded for provenance.
    // Flow-level persona authorization is delegated to step-level Operation
    // persona checks.
    flow_result.initiating_persona = Some(persona.to_string());

    Ok(FlowEvalResult {
        verdicts: verdict_set,
        flow_result,
    })
}

// ──────────────────────────────────────────────
// Integration tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// End-to-end test using a hand-constructed interchange bundle.
    #[test]
    fn evaluate_simple_contract() {
        let bundle = serde_json::json!({
            "id": "test_bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.1.0",
            "constructs": [
                {
                    "id": "is_active",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 1 },
                    "source": { "system": "test", "field": "active" },
                    "type": { "base": "Bool" }
                },
                {
                    "id": "balance",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 5 },
                    "source": { "system": "test", "field": "balance" },
                    "type": { "base": "Money", "currency": "USD" }
                },
                {
                    "id": "limit",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 10 },
                    "source": { "system": "test", "field": "limit" },
                    "type": { "base": "Money", "currency": "USD" },
                    "default": {
                        "kind": "money_value",
                        "currency": "USD",
                        "amount": {
                            "kind": "decimal_value",
                            "precision": 10,
                            "scale": 2,
                            "value": "10000.00"
                        }
                    }
                },
                {
                    "id": "check_active",
                    "kind": "Rule",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 15 },
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": { "fact_ref": "is_active" },
                            "op": "=",
                            "right": { "literal": true, "type": { "base": "Bool" } }
                        },
                        "produce": {
                            "verdict_type": "account_active",
                            "payload": {
                                "type": { "base": "Bool" },
                                "value": true
                            }
                        }
                    }
                },
                {
                    "id": "check_balance",
                    "kind": "Rule",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 25 },
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": { "fact_ref": "balance" },
                            "op": "<=",
                            "right": { "fact_ref": "limit" },
                            "comparison_type": { "base": "Money", "currency": "USD" }
                        },
                        "produce": {
                            "verdict_type": "within_limit",
                            "payload": {
                                "type": { "base": "Bool" },
                                "value": true
                            }
                        }
                    }
                },
                {
                    "id": "can_process",
                    "kind": "Rule",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 35 },
                    "stratum": 1,
                    "body": {
                        "when": {
                            "left": { "verdict_present": "account_active" },
                            "op": "and",
                            "right": { "verdict_present": "within_limit" }
                        },
                        "produce": {
                            "verdict_type": "order_processable",
                            "payload": {
                                "type": { "base": "Bool" },
                                "value": true
                            }
                        }
                    }
                }
            ]
        });

        let facts = serde_json::json!({
            "is_active": true,
            "balance": { "amount": "5000.00", "currency": "USD" }
        });

        let result = evaluate(&bundle, &facts).unwrap();
        assert_eq!(result.verdicts.0.len(), 3);
        assert!(result.verdicts.has_verdict("account_active"));
        assert!(result.verdicts.has_verdict("within_limit"));
        assert!(result.verdicts.has_verdict("order_processable"));

        // Verify provenance
        let op = result.verdicts.get_verdict("order_processable").unwrap();
        assert_eq!(op.provenance.stratum, 1);
        assert!(op
            .provenance
            .verdicts_used
            .contains(&"account_active".to_string()));
        assert!(op
            .provenance
            .verdicts_used
            .contains(&"within_limit".to_string()));

        // Verify JSON output
        let json_output = result.verdicts.to_json();
        let verdicts_arr = json_output["verdicts"].as_array().unwrap();
        assert_eq!(verdicts_arr.len(), 3);
    }

    /// Test with default value used for missing fact.
    #[test]
    fn evaluate_with_default_fact() {
        let bundle = serde_json::json!({
            "id": "test_defaults",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.1.0",
            "constructs": [
                {
                    "id": "flag",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 1 },
                    "source": { "system": "test", "field": "flag" },
                    "type": { "base": "Bool" },
                    "default": { "kind": "bool_literal", "value": false }
                },
                {
                    "id": "check_flag",
                    "kind": "Rule",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 5 },
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": { "fact_ref": "flag" },
                            "op": "=",
                            "right": { "literal": false, "type": { "base": "Bool" } }
                        },
                        "produce": {
                            "verdict_type": "flag_is_false",
                            "payload": {
                                "type": { "base": "Bool" },
                                "value": true
                            }
                        }
                    }
                }
            ]
        });

        let facts = serde_json::json!({});
        let result = evaluate(&bundle, &facts).unwrap();
        // Default is false, rule checks flag = false, should produce verdict
        assert!(result.verdicts.has_verdict("flag_is_false"));
    }

    /// Test missing required fact causes error.
    #[test]
    fn evaluate_missing_required_fact() {
        let bundle = serde_json::json!({
            "id": "test_missing",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.1.0",
            "constructs": [
                {
                    "id": "required",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 1 },
                    "source": { "system": "test", "field": "req" },
                    "type": { "base": "Bool" }
                }
            ]
        });

        let facts = serde_json::json!({});
        let result = evaluate(&bundle, &facts);
        assert!(result.is_err());
        if let Err(EvalError::MissingFact { fact_id }) = result {
            assert_eq!(fact_id, "required");
        } else {
            panic!("expected MissingFact error");
        }
    }

    /// Test with no rules -- should produce empty verdict set.
    #[test]
    fn evaluate_no_rules() {
        let bundle = serde_json::json!({
            "id": "test_empty",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.1.0",
            "constructs": [
                {
                    "id": "x",
                    "kind": "Fact",
                    "tenor": "1.0",
                    "provenance": { "file": "test.tenor", "line": 1 },
                    "source": { "system": "test", "field": "x" },
                    "type": { "base": "Bool" }
                }
            ]
        });

        let facts = serde_json::json!({ "x": true });
        let result = evaluate(&bundle, &facts).unwrap();
        assert_eq!(result.verdicts.0.len(), 0);
    }
}
