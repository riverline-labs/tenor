//! Stratified rule evaluation.
//!
//! Implements spec Section 7.4: rules are evaluated in stratum order,
//! with higher strata able to reference verdicts from lower strata.
//!
//! Rules at the same stratum are independent -- evaluation order within
//! a stratum does not matter. Higher stratum rules CAN reference verdicts
//! produced by lower strata.

use crate::predicate::{eval_pred, EvalContext};
use crate::provenance::ProvenanceCollector;
use crate::types::{
    Contract, EvalError, FactSet, PayloadValue, Predicate, Value, VerdictInstance, VerdictSet,
};

/// Evaluate all rules in stratum order, producing a VerdictSet.
///
/// For each stratum from 0 to max_stratum:
/// 1. Collect all rules at current stratum
/// 2. Evaluate each rule's condition against facts + accumulated verdicts
/// 3. If condition is true, compute payload and add verdict with provenance
pub fn eval_strata(contract: &Contract, facts: &FactSet) -> Result<VerdictSet, EvalError> {
    let mut verdicts = VerdictSet::new();

    if contract.rules.is_empty() {
        return Ok(verdicts);
    }

    let max_stratum = contract.rules.iter().map(|r| r.stratum).max().unwrap_or(0);

    for n in 0..=max_stratum {
        let stratum_rules: Vec<_> = contract.rules.iter().filter(|r| r.stratum == n).collect();

        for rule in stratum_rules {
            if let Some(verdict) = eval_rule(rule, facts, &verdicts, n)? {
                verdicts.push(verdict);
            }
        }
    }

    Ok(verdicts)
}

/// Evaluate a single rule. Returns Some(VerdictInstance) if the condition
/// is true, None if the condition is false.
fn eval_rule(
    rule: &crate::types::Rule,
    facts: &FactSet,
    verdicts: &VerdictSet,
    stratum: u32,
) -> Result<Option<VerdictInstance>, EvalError> {
    let mut collector = ProvenanceCollector::new();
    let ctx = EvalContext::new();

    // Evaluate the rule's condition
    let condition_result = eval_pred(&rule.condition, facts, verdicts, &ctx, &mut collector)?;
    let is_true = condition_result.as_bool()?;

    if !is_true {
        return Ok(None);
    }

    // Condition is true -- compute payload
    let payload = eval_payload(
        &rule.produce.payload_value,
        facts,
        verdicts,
        &ctx,
        &mut collector,
    )?;

    let provenance = collector.into_provenance(rule.id.clone(), stratum);

    Ok(Some(VerdictInstance {
        verdict_type: rule.produce.verdict_type.clone(),
        payload,
        provenance,
    }))
}

/// Evaluate a payload value expression.
fn eval_payload(
    payload_value: &PayloadValue,
    facts: &FactSet,
    verdicts: &VerdictSet,
    ctx: &EvalContext,
    collector: &mut ProvenanceCollector,
) -> Result<Value, EvalError> {
    match payload_value {
        PayloadValue::Literal(v) => Ok(v.clone()),
        PayloadValue::Mul(mul_expr) => {
            // Build a Predicate::Mul from the MulExpr and evaluate it
            let pred = Predicate::Mul {
                left: Box::new(Predicate::FactRef(mul_expr.fact_ref.clone())),
                literal: mul_expr.literal,
                result_type: mul_expr.result_type.clone(),
            };
            eval_pred(&pred, facts, verdicts, ctx, collector)
        }
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn bool_type() -> TypeSpec {
        TypeSpec {
            base: "Bool".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    fn make_rule(
        id: &str,
        stratum: u32,
        condition: Predicate,
        verdict_type: &str,
        payload: Value,
    ) -> Rule {
        Rule {
            id: id.to_string(),
            stratum,
            condition,
            produce: ProduceClause {
                verdict_type: verdict_type.to_string(),
                payload_type: bool_type(),
                payload_value: PayloadValue::Literal(payload),
            },
        }
    }

    #[test]
    fn eval_single_stratum_simple() {
        // Rule: when is_active = true => produce account_active = true
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(true));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![make_rule(
                "account_active",
                0,
                Predicate::Compare {
                    left: Box::new(Predicate::FactRef("is_active".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Predicate::Literal {
                        value: Value::Bool(true),
                        type_spec: bool_type(),
                    }),
                    comparison_type: None,
                },
                "account_active",
                Value::Bool(true),
            )],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 1);
        assert!(verdicts.has_verdict("account_active"));
        assert_eq!(verdicts.0[0].provenance.rule_id, "account_active");
        assert_eq!(verdicts.0[0].provenance.stratum, 0);
        assert_eq!(verdicts.0[0].provenance.facts_used, vec!["is_active"]);
    }

    #[test]
    fn eval_condition_false_no_verdict() {
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(false));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![make_rule(
                "account_active",
                0,
                Predicate::Compare {
                    left: Box::new(Predicate::FactRef("is_active".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Predicate::Literal {
                        value: Value::Bool(true),
                        type_spec: bool_type(),
                    }),
                    comparison_type: None,
                },
                "account_active",
                Value::Bool(true),
            )],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 0);
        assert!(!verdicts.has_verdict("account_active"));
    }

    #[test]
    fn eval_multi_stratum() {
        // Stratum 0: is_active = true => produce account_active
        // Stratum 1: verdict_present(account_active) => produce order_processable
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(true));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![
                make_rule(
                    "check_active",
                    0,
                    Predicate::Compare {
                        left: Box::new(Predicate::FactRef("is_active".to_string())),
                        op: "=".to_string(),
                        right: Box::new(Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: bool_type(),
                        }),
                        comparison_type: None,
                    },
                    "account_active",
                    Value::Bool(true),
                ),
                make_rule(
                    "can_process",
                    1,
                    Predicate::VerdictPresent("account_active".to_string()),
                    "order_processable",
                    Value::Bool(true),
                ),
            ],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 2);
        assert!(verdicts.has_verdict("account_active"));
        assert!(verdicts.has_verdict("order_processable"));

        // Check provenance of stratum 1 verdict
        let op_verdict = verdicts.get_verdict("order_processable").unwrap();
        assert_eq!(op_verdict.provenance.stratum, 1);
        assert_eq!(op_verdict.provenance.verdicts_used, vec!["account_active"]);
    }

    #[test]
    fn eval_multi_stratum_s1_depends_on_missing_s0() {
        // Stratum 0: condition is false => no verdict
        // Stratum 1: verdict_present(missing_verdict) => should NOT fire
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(false));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![
                make_rule(
                    "check_active",
                    0,
                    Predicate::Compare {
                        left: Box::new(Predicate::FactRef("is_active".to_string())),
                        op: "=".to_string(),
                        right: Box::new(Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: bool_type(),
                        }),
                        comparison_type: None,
                    },
                    "account_active",
                    Value::Bool(true),
                ),
                make_rule(
                    "can_process",
                    1,
                    Predicate::VerdictPresent("account_active".to_string()),
                    "order_processable",
                    Value::Bool(true),
                ),
            ],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 0);
    }

    #[test]
    fn eval_empty_rules() {
        let facts = FactSet::new();
        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };
        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 0);
    }

    #[test]
    fn eval_provenance_tracks_facts_and_verdicts() {
        // Rule at stratum 0 uses a fact and a verdict_present
        let mut facts = FactSet::new();
        facts.insert(
            "balance".to_string(),
            Value::Money {
                amount: rust_decimal::Decimal::new(500000, 2),
                currency: "USD".to_string(),
            },
        );
        facts.insert(
            "limit".to_string(),
            Value::Money {
                amount: rust_decimal::Decimal::new(1000000, 2),
                currency: "USD".to_string(),
            },
        );
        facts.insert("is_active".to_string(), Value::Bool(true));

        let ct = TypeSpec {
            base: "Money".to_string(),
            precision: None,
            scale: None,
            currency: Some("USD".to_string()),
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![
                make_rule(
                    "check_active",
                    0,
                    Predicate::Compare {
                        left: Box::new(Predicate::FactRef("is_active".to_string())),
                        op: "=".to_string(),
                        right: Box::new(Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: bool_type(),
                        }),
                        comparison_type: None,
                    },
                    "account_active",
                    Value::Bool(true),
                ),
                Rule {
                    id: "check_balance".to_string(),
                    stratum: 0,
                    condition: Predicate::Compare {
                        left: Box::new(Predicate::FactRef("balance".to_string())),
                        op: "<=".to_string(),
                        right: Box::new(Predicate::FactRef("limit".to_string())),
                        comparison_type: Some(ct),
                    },
                    produce: ProduceClause {
                        verdict_type: "within_limit".to_string(),
                        payload_type: bool_type(),
                        payload_value: PayloadValue::Literal(Value::Bool(true)),
                    },
                },
                make_rule(
                    "can_process",
                    1,
                    Predicate::And {
                        left: Box::new(Predicate::VerdictPresent("account_active".to_string())),
                        right: Box::new(Predicate::VerdictPresent("within_limit".to_string())),
                    },
                    "order_processable",
                    Value::Bool(true),
                ),
            ],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 3);

        // Check balance verdict provenance
        let balance_v = verdicts.get_verdict("within_limit").unwrap();
        assert!(balance_v
            .provenance
            .facts_used
            .contains(&"balance".to_string()));
        assert!(balance_v
            .provenance
            .facts_used
            .contains(&"limit".to_string()));

        // Check order_processable provenance (stratum 1)
        let order_v = verdicts.get_verdict("order_processable").unwrap();
        assert_eq!(order_v.provenance.stratum, 1);
        assert!(order_v
            .provenance
            .verdicts_used
            .contains(&"account_active".to_string()));
        assert!(order_v
            .provenance
            .verdicts_used
            .contains(&"within_limit".to_string()));
    }

    #[test]
    fn eval_multiple_rules_same_stratum() {
        // Two rules at stratum 0, both should fire
        let mut facts = FactSet::new();
        facts.insert("a".to_string(), Value::Bool(true));
        facts.insert("b".to_string(), Value::Bool(true));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![
                make_rule(
                    "rule_a",
                    0,
                    Predicate::Compare {
                        left: Box::new(Predicate::FactRef("a".to_string())),
                        op: "=".to_string(),
                        right: Box::new(Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: bool_type(),
                        }),
                        comparison_type: None,
                    },
                    "verdict_a",
                    Value::Bool(true),
                ),
                make_rule(
                    "rule_b",
                    0,
                    Predicate::Compare {
                        left: Box::new(Predicate::FactRef("b".to_string())),
                        op: "=".to_string(),
                        right: Box::new(Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: bool_type(),
                        }),
                        comparison_type: None,
                    },
                    "verdict_b",
                    Value::Bool(true),
                ),
            ],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 2);
        assert!(verdicts.has_verdict("verdict_a"));
        assert!(verdicts.has_verdict("verdict_b"));
    }

    #[test]
    fn eval_verdict_json_serialization() {
        let mut facts = FactSet::new();
        facts.insert("x".to_string(), Value::Bool(true));

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![make_rule(
                "r1",
                0,
                Predicate::Compare {
                    left: Box::new(Predicate::FactRef("x".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Predicate::Literal {
                        value: Value::Bool(true),
                        type_spec: bool_type(),
                    }),
                    comparison_type: None,
                },
                "test_verdict",
                Value::Bool(true),
            )],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        let json = verdicts.to_json();
        let verdicts_arr = json.get("verdicts").unwrap().as_array().unwrap();
        assert_eq!(verdicts_arr.len(), 1);
        assert_eq!(verdicts_arr[0]["type"], "test_verdict");
        assert_eq!(verdicts_arr[0]["provenance"]["rule"], "r1");
        assert_eq!(verdicts_arr[0]["provenance"]["stratum"], 0);
    }

    #[test]
    fn eval_mul_payload() {
        let mut facts = FactSet::new();
        facts.insert("x".to_string(), Value::Int(5));

        let rt = TypeSpec {
            base: "Int".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: Some(0),
            max: Some(100),
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };

        let contract = Contract {
            facts: vec![],
            entities: vec![],
            rules: vec![Rule {
                id: "mul_rule".to_string(),
                stratum: 0,
                condition: Predicate::Compare {
                    left: Box::new(Predicate::FactRef("x".to_string())),
                    op: ">".to_string(),
                    right: Box::new(Predicate::Literal {
                        value: Value::Int(0),
                        type_spec: TypeSpec {
                            base: "Int".to_string(),
                            precision: None,
                            scale: None,
                            currency: None,
                            min: Some(0),
                            max: Some(0),
                            max_length: None,
                            values: None,
                            fields: None,
                            element_type: None,
                            unit: None,
                            variants: None,
                        },
                    }),
                    comparison_type: None,
                },
                produce: ProduceClause {
                    verdict_type: "scaled_x".to_string(),
                    payload_type: rt.clone(),
                    payload_value: PayloadValue::Mul(crate::types::MulExpr {
                        fact_ref: "x".to_string(),
                        literal: 10,
                        result_type: rt,
                    }),
                },
            }],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        };

        let verdicts = eval_strata(&contract, &facts).unwrap();
        assert_eq!(verdicts.0.len(), 1);
        let v = verdicts.get_verdict("scaled_x").unwrap();
        assert_eq!(v.payload, Value::Int(50));
        assert!(v.provenance.facts_used.contains(&"x".to_string()));
    }
}
