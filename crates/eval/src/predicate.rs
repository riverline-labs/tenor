//! PredicateExpression evaluator.
//!
//! Implements spec Section 10: evaluation of predicate expression trees
//! over ground terms (facts and verdicts).
//!
//! The predicate tree is a recursive structure. Each node type evaluates
//! to a `Value` (usually Bool for logical nodes, but can be other types
//! for fact refs, field refs, and literals).

use std::collections::BTreeMap;

use crate::numeric;
use crate::provenance::ProvenanceCollector;
#[cfg(test)]
use crate::types::TypeSpec;
use crate::types::{EvalError, FactSet, Predicate, Value, VerdictSet};

/// Evaluation context for bound variables (forall quantification).
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Bound variables from enclosing forall quantifiers.
    pub bindings: BTreeMap<String, Value>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalContext {
    pub fn new() -> Self {
        EvalContext {
            bindings: BTreeMap::new(),
        }
    }
}

/// Evaluate a predicate expression against facts and verdicts.
///
/// Returns the result value. For comparison/logical expressions this is
/// always `Value::Bool`. For fact refs, literals, etc. it returns the
/// value directly.
///
/// The `collector` tracks which facts and verdicts are accessed for
/// provenance construction.
pub fn eval_pred(
    pred: &Predicate,
    facts: &FactSet,
    verdicts: &VerdictSet,
    ctx: &EvalContext,
    collector: &mut ProvenanceCollector,
) -> Result<Value, EvalError> {
    match pred {
        Predicate::FactRef(id) => {
            collector.record_fact(id);
            facts
                .get(id)
                .cloned()
                .ok_or_else(|| EvalError::UnknownFact {
                    fact_id: id.clone(),
                })
        }

        Predicate::FieldRef { var, field } => {
            // Look up bound variable in context, then fall back to facts
            // (Record-typed facts are accessed via field_ref in interchange JSON)
            let val = ctx
                .bindings
                .get(var)
                .or_else(|| {
                    collector.record_fact(var);
                    facts.get(var)
                })
                .ok_or_else(|| EvalError::UnboundVariable { name: var.clone() })?;
            match val {
                Value::Record(fields) => {
                    fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| EvalError::NotARecord {
                            message: format!(
                                "field '{}' not found in record variable '{}'",
                                field, var
                            ),
                        })
                }
                _ => Err(EvalError::NotARecord {
                    message: format!(
                        "variable '{}' is not a Record, got {}",
                        var,
                        val.type_name()
                    ),
                }),
            }
        }

        Predicate::Literal { value, .. } => Ok(value.clone()),

        Predicate::VerdictPresent(id) => {
            collector.record_verdict(id);
            Ok(Value::Bool(verdicts.has_verdict(id)))
        }

        Predicate::Compare {
            left,
            op,
            right,
            comparison_type,
        } => {
            let left_val = eval_pred(left, facts, verdicts, ctx, collector)?;
            let right_val = eval_pred(right, facts, verdicts, ctx, collector)?;
            let result =
                numeric::compare_values(&left_val, &right_val, op, comparison_type.as_ref())?;
            Ok(Value::Bool(result))
        }

        Predicate::And { left, right } => {
            let left_val = eval_pred(left, facts, verdicts, ctx, collector)?;
            let left_bool = left_val.as_bool()?;
            if !left_bool {
                // Short-circuit: left is false, skip right
                return Ok(Value::Bool(false));
            }
            let right_val = eval_pred(right, facts, verdicts, ctx, collector)?;
            let right_bool = right_val.as_bool()?;
            Ok(Value::Bool(right_bool))
        }

        Predicate::Or { left, right } => {
            let left_val = eval_pred(left, facts, verdicts, ctx, collector)?;
            let left_bool = left_val.as_bool()?;
            if left_bool {
                // Short-circuit: left is true, skip right
                return Ok(Value::Bool(true));
            }
            let right_val = eval_pred(right, facts, verdicts, ctx, collector)?;
            let right_bool = right_val.as_bool()?;
            Ok(Value::Bool(right_bool))
        }

        Predicate::Not { operand } => {
            let val = eval_pred(operand, facts, verdicts, ctx, collector)?;
            let b = val.as_bool()?;
            Ok(Value::Bool(!b))
        }

        Predicate::Forall {
            variable,
            variable_type: _,
            domain,
            body,
        } => {
            // Evaluate domain (must be a List-typed fact)
            let domain_val = eval_pred(domain, facts, verdicts, ctx, collector)?;
            let elements = match domain_val {
                Value::List(items) => items,
                _ => {
                    return Err(EvalError::TypeError {
                        message: format!(
                            "forall domain must be a List, got {}",
                            domain_val.type_name()
                        ),
                    });
                }
            };

            // Check body predicate for each element
            for elem in &elements {
                let mut inner_ctx = ctx.clone();
                inner_ctx.bindings.insert(variable.clone(), elem.clone());
                let result = eval_pred(body, facts, verdicts, &inner_ctx, collector)?;
                let b = result.as_bool()?;
                if !b {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }

        Predicate::Exists {
            variable,
            variable_type: _,
            domain,
            body,
        } => {
            // Evaluate domain (must be a List-typed fact)
            let domain_val = eval_pred(domain, facts, verdicts, ctx, collector)?;
            let elements = match domain_val {
                Value::List(items) => items,
                _ => {
                    return Err(EvalError::TypeError {
                        message: format!(
                            "exists domain must be a List, got {}",
                            domain_val.type_name()
                        ),
                    });
                }
            };

            // Check body predicate for each element -- short-circuit on first true
            for elem in &elements {
                let mut inner_ctx = ctx.clone();
                inner_ctx.bindings.insert(variable.clone(), elem.clone());
                let result = eval_pred(body, facts, verdicts, &inner_ctx, collector)?;
                let b = result.as_bool()?;
                if b {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }

        Predicate::Mul {
            left,
            literal,
            result_type,
        } => {
            let left_val = eval_pred(left, facts, verdicts, ctx, collector)?;
            match left_val {
                Value::Int(i) => numeric::eval_int_mul(i, *literal, result_type),
                Value::Decimal(d) => {
                    let lit_decimal = rust_decimal::Decimal::from(*literal);
                    let result = numeric::eval_mul(
                        d,
                        lit_decimal,
                        result_type.precision.unwrap_or(28),
                        result_type.scale.unwrap_or(0),
                    )?;
                    Ok(Value::Decimal(result))
                }
                _ => Err(EvalError::TypeError {
                    message: format!(
                        "multiplication requires numeric operand, got {}",
                        left_val.type_name()
                    ),
                }),
            }
        }
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::ProvenanceCollector;

    fn empty_verdicts() -> VerdictSet {
        VerdictSet::new()
    }

    fn make_verdict_set(types: &[&str]) -> VerdictSet {
        let mut vs = VerdictSet::new();
        for t in types {
            vs.push(crate::types::VerdictInstance {
                verdict_type: t.to_string(),
                payload: Value::Bool(true),
                provenance: crate::provenance::VerdictProvenance {
                    rule_id: "test".to_string(),
                    stratum: 0,
                    facts_used: vec![],
                    verdicts_used: vec![],
                },
            });
        }
        vs
    }

    #[test]
    fn eval_fact_ref() {
        let mut facts = FactSet::new();
        facts.insert("x".to_string(), Value::Int(42));
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(
            &Predicate::FactRef("x".to_string()),
            &facts,
            &empty_verdicts(),
            &ctx,
            &mut collector,
        )
        .unwrap();
        assert_eq!(result, Value::Int(42));
        assert_eq!(collector.facts_used, vec!["x"]);
    }

    #[test]
    fn eval_fact_ref_missing() {
        let facts = FactSet::new();
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(
            &Predicate::FactRef("missing".to_string()),
            &facts,
            &empty_verdicts(),
            &ctx,
            &mut collector,
        );
        assert!(result.is_err());
    }

    #[test]
    fn eval_literal() {
        let facts = FactSet::new();
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let ts = TypeSpec {
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
        };
        let result = eval_pred(
            &Predicate::Literal {
                value: Value::Bool(true),
                type_spec: ts,
            },
            &facts,
            &empty_verdicts(),
            &ctx,
            &mut collector,
        )
        .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_verdict_present_true() {
        let facts = FactSet::new();
        let verdicts = make_verdict_set(&["account_active"]);
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(
            &Predicate::VerdictPresent("account_active".to_string()),
            &facts,
            &verdicts,
            &ctx,
            &mut collector,
        )
        .unwrap();
        assert_eq!(result, Value::Bool(true));
        assert_eq!(collector.verdicts_used, vec!["account_active"]);
    }

    #[test]
    fn eval_verdict_present_false() {
        let facts = FactSet::new();
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(
            &Predicate::VerdictPresent("missing".to_string()),
            &facts,
            &empty_verdicts(),
            &ctx,
            &mut collector,
        )
        .unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn eval_compare_equal() {
        let mut facts = FactSet::new();
        facts.insert("x".to_string(), Value::Bool(true));
        let ts = TypeSpec {
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
        };
        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("x".to_string())),
            op: "=".to_string(),
            right: Box::new(Predicate::Literal {
                value: Value::Bool(true),
                type_spec: ts,
            }),
            comparison_type: None,
        };
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_and_both_true() {
        let pred = Predicate::And {
            left: Box::new(Predicate::VerdictPresent("a".to_string())),
            right: Box::new(Predicate::VerdictPresent("b".to_string())),
        };
        let verdicts = make_verdict_set(&["a", "b"]);
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &FactSet::new(), &verdicts, &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_and_short_circuit() {
        let pred = Predicate::And {
            left: Box::new(Predicate::VerdictPresent("missing".to_string())),
            right: Box::new(Predicate::VerdictPresent("b".to_string())),
        };
        let verdicts = make_verdict_set(&["b"]);
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &FactSet::new(), &verdicts, &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(false));
        // Right side not evaluated due to short-circuit, so "b" not in verdicts_used
        assert_eq!(collector.verdicts_used, vec!["missing"]);
    }

    #[test]
    fn eval_or_first_true() {
        let pred = Predicate::Or {
            left: Box::new(Predicate::VerdictPresent("a".to_string())),
            right: Box::new(Predicate::VerdictPresent("b".to_string())),
        };
        let verdicts = make_verdict_set(&["a"]);
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &FactSet::new(), &verdicts, &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_not() {
        let pred = Predicate::Not {
            operand: Box::new(Predicate::VerdictPresent("missing".to_string())),
        };
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(
            &pred,
            &FactSet::new(),
            &empty_verdicts(),
            &ctx,
            &mut collector,
        )
        .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_nested_and_or() {
        // (a AND b) OR c -- a=true, b=false, c=true => should be true
        let pred = Predicate::Or {
            left: Box::new(Predicate::And {
                left: Box::new(Predicate::VerdictPresent("a".to_string())),
                right: Box::new(Predicate::VerdictPresent("b".to_string())),
            }),
            right: Box::new(Predicate::VerdictPresent("c".to_string())),
        };
        let verdicts = make_verdict_set(&["a", "c"]);
        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &FactSet::new(), &verdicts, &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_forall_all_true() {
        let mut facts = FactSet::new();
        facts.insert(
            "items".to_string(),
            Value::List(vec![
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("valid".to_string(), Value::Bool(true));
                    m
                }),
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("valid".to_string(), Value::Bool(true));
                    m
                }),
            ]),
        );

        let ts = TypeSpec {
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
        };
        let vt = TypeSpec {
            base: "Record".to_string(),
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
        };

        let pred = Predicate::Forall {
            variable: "item".to_string(),
            variable_type: vt,
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "valid".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: ts,
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_forall_one_false() {
        let mut facts = FactSet::new();
        facts.insert(
            "items".to_string(),
            Value::List(vec![
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("valid".to_string(), Value::Bool(true));
                    m
                }),
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("valid".to_string(), Value::Bool(false));
                    m
                }),
            ]),
        );

        let ts = TypeSpec {
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
        };
        let vt = TypeSpec {
            base: "Record".to_string(),
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
        };

        let pred = Predicate::Forall {
            variable: "item".to_string(),
            variable_type: vt,
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "valid".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: ts,
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn eval_money_comparison_with_promotion() {
        let mut facts = FactSet::new();
        facts.insert(
            "balance".to_string(),
            Value::Money {
                amount: rust_decimal::Decimal::new(500000, 2), // 5000.00
                currency: "USD".to_string(),
            },
        );
        facts.insert(
            "limit".to_string(),
            Value::Money {
                amount: rust_decimal::Decimal::new(1000000, 2), // 10000.00
                currency: "USD".to_string(),
            },
        );

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

        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("balance".to_string())),
            op: "<=".to_string(),
            right: Box::new(Predicate::FactRef("limit".to_string())),
            comparison_type: Some(ct),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_enum_comparison() {
        let mut facts = FactSet::new();
        facts.insert("status".to_string(), Value::Enum("confirmed".to_string()));

        let ts = TypeSpec {
            base: "Enum".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: Some(vec![
                "pending".to_string(),
                "confirmed".to_string(),
                "failed".to_string(),
            ]),
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        };

        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("status".to_string())),
            op: "=".to_string(),
            right: Box::new(Predicate::Literal {
                value: Value::Enum("confirmed".to_string()),
                type_spec: ts,
            }),
            comparison_type: None,
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_mul_int() {
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

        let pred = Predicate::Mul {
            left: Box::new(Predicate::FactRef("x".to_string())),
            literal: 10,
            result_type: rt,
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Int(50));
    }

    #[test]
    fn eval_exists_one_match() {
        let mut facts = FactSet::new();
        facts.insert(
            "items".to_string(),
            Value::List(vec![
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("compliant".to_string(), Value::Bool(true));
                    m
                }),
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("compliant".to_string(), Value::Bool(false));
                    m
                }),
            ]),
        );

        let ts = TypeSpec {
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
        };
        let vt = TypeSpec {
            base: "Record".to_string(),
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
        };

        let pred = Predicate::Exists {
            variable: "item".to_string(),
            variable_type: vt,
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "compliant".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(false),
                    type_spec: ts,
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn eval_exists_no_match() {
        let mut facts = FactSet::new();
        facts.insert(
            "items".to_string(),
            Value::List(vec![
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("compliant".to_string(), Value::Bool(true));
                    m
                }),
                Value::Record({
                    let mut m = BTreeMap::new();
                    m.insert("compliant".to_string(), Value::Bool(true));
                    m
                }),
            ]),
        );

        let ts = TypeSpec {
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
        };
        let vt = TypeSpec {
            base: "Record".to_string(),
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
        };

        let pred = Predicate::Exists {
            variable: "item".to_string(),
            variable_type: vt,
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "compliant".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(false),
                    type_spec: ts,
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();
        assert_eq!(result, Value::Bool(false));
    }
}

// ──────────────────────────────────────────────
// Wave 2B: Failure / edge-case tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::provenance::ProvenanceCollector;
    use crate::types::{EvalError, FactSet, TypeSpec, Value, VerdictSet};
    use std::collections::BTreeMap;

    fn empty_verdicts() -> VerdictSet {
        VerdictSet::new()
    }

    fn make_type_spec(base: &str) -> TypeSpec {
        TypeSpec {
            base: base.to_string(),
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

    // ── 1. Type mismatch: Int fact compared to Text literal ──
    // The predicate evaluator delegates to numeric::compare_values,
    // which returns a TypeError when comparing incompatible types.
    #[test]
    fn type_mismatch_int_vs_text_returns_error() {
        let mut facts = FactSet::new();
        facts.insert("age".to_string(), Value::Int(25));

        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("age".to_string())),
            op: "=".to_string(),
            right: Box::new(Predicate::Literal {
                value: Value::Text("twenty-five".to_string()),
                type_spec: make_type_spec("Text"),
            }),
            comparison_type: None,
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        // Int vs Text comparison produces a TypeError (not false).
        assert!(result.is_err(), "comparing Int to Text should error");
        match result.unwrap_err() {
            EvalError::TypeError { message } => {
                assert!(
                    message.contains("cannot compare"),
                    "error message should mention 'cannot compare', got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got: {:?}", other),
        }
    }

    // ── 2. Undefined verdict reference with empty verdict set ──
    // VerdictPresent on a nonexistent verdict evaluates to false (not error).
    #[test]
    fn verdict_present_nonexistent_returns_false() {
        let facts = FactSet::new();
        let verdicts = empty_verdicts();

        let pred = Predicate::VerdictPresent("nonexistent_verdict".to_string());

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &verdicts, &ctx, &mut collector).unwrap();

        assert_eq!(result, Value::Bool(false));
        // The verdict id is still recorded for provenance tracking.
        assert_eq!(collector.verdicts_used, vec!["nonexistent_verdict"]);
    }

    // ── 3. Overflow: Decimal multiplication near precision bounds ──
    // Multiplying a large Decimal value that exceeds declared precision
    // should produce an Overflow error.
    #[test]
    fn decimal_mul_overflow_at_precision_boundary() {
        let mut facts = FactSet::new();
        // Value of 999.99 in Decimal(5,2) -- max representable
        facts.insert(
            "amount".to_string(),
            Value::Decimal(rust_decimal::Decimal::new(99999, 2)),
        );

        // Multiply by 2, result_type has precision=5, scale=2 (max integer part = 999)
        // 999.99 * 2 = 1999.98 which exceeds precision(5,2) max of 999.99
        let pred = Predicate::Mul {
            left: Box::new(Predicate::FactRef("amount".to_string())),
            literal: 2,
            result_type: TypeSpec {
                base: "Decimal".to_string(),
                precision: Some(5),
                scale: Some(2),
                currency: None,
                min: None,
                max: None,
                max_length: None,
                values: None,
                fields: None,
                element_type: None,
                unit: None,
                variants: None,
            },
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err(), "should overflow at precision boundary");
        match result.unwrap_err() {
            EvalError::Overflow { message } => {
                assert!(
                    message.contains("exceeds declared precision"),
                    "overflow message should mention precision, got: {}",
                    message
                );
            }
            other => panic!("expected Overflow, got: {:?}", other),
        }
    }

    // ── 3b. Integer multiplication overflow (i64 boundary) ──
    #[test]
    fn int_mul_overflow_at_i64_boundary() {
        let mut facts = FactSet::new();
        facts.insert("big".to_string(), Value::Int(i64::MAX));

        let pred = Predicate::Mul {
            left: Box::new(Predicate::FactRef("big".to_string())),
            literal: 2,
            result_type: TypeSpec {
                base: "Int".to_string(),
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
            },
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err(), "i64::MAX * 2 should overflow");
        match result.unwrap_err() {
            EvalError::Overflow { message } => {
                assert!(
                    message.contains("overflow"),
                    "error should mention overflow, got: {}",
                    message
                );
            }
            other => panic!("expected Overflow, got: {:?}", other),
        }
    }

    // ── 4. Empty list quantification: forall over empty list = vacuous truth ──
    #[test]
    fn forall_empty_list_is_vacuous_truth() {
        let mut facts = FactSet::new();
        facts.insert("items".to_string(), Value::List(vec![]));

        let pred = Predicate::Forall {
            variable: "item".to_string(),
            variable_type: make_type_spec("Record"),
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "valid".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: make_type_spec("Bool"),
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();

        // Vacuous truth: forall over empty domain is true.
        assert_eq!(result, Value::Bool(true));
    }

    // ── 4b. Exists over empty list = false ──
    #[test]
    fn exists_empty_list_is_false() {
        let mut facts = FactSet::new();
        facts.insert("items".to_string(), Value::List(vec![]));

        let pred = Predicate::Exists {
            variable: "item".to_string(),
            variable_type: make_type_spec("Record"),
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FieldRef {
                    var: "item".to_string(),
                    field: "valid".to_string(),
                }),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: make_type_spec("Bool"),
                }),
                comparison_type: None,
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector).unwrap();

        // Exists over empty domain is false.
        assert_eq!(result, Value::Bool(false));
    }

    // ── 5. Null/missing fact in predicate: FactRef to nonexistent fact ──
    // When a predicate references a fact that has no value and no default,
    // the evaluator returns an UnknownFact error (not a silent null/false).
    #[test]
    fn missing_fact_returns_unknown_fact_error() {
        let facts = FactSet::new(); // no facts at all

        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("nonexistent_fact".to_string())),
            op: "=".to_string(),
            right: Box::new(Predicate::Literal {
                value: Value::Bool(true),
                type_spec: make_type_spec("Bool"),
            }),
            comparison_type: None,
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err(), "missing fact should produce an error");
        match result.unwrap_err() {
            EvalError::UnknownFact { fact_id } => {
                assert_eq!(fact_id, "nonexistent_fact");
            }
            other => panic!("expected UnknownFact, got: {:?}", other),
        }
    }

    // ── 5b. Missing fact in AND short-circuits: left error propagates ──
    #[test]
    fn missing_fact_in_and_left_propagates_error() {
        let facts = FactSet::new();

        let pred = Predicate::And {
            left: Box::new(Predicate::Compare {
                left: Box::new(Predicate::FactRef("missing".to_string())),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: make_type_spec("Bool"),
                }),
                comparison_type: None,
            }),
            right: Box::new(Predicate::VerdictPresent("something".to_string())),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        // Error from left side propagates (does not short-circuit to false).
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::UnknownFact { fact_id } => {
                assert_eq!(fact_id, "missing");
            }
            other => panic!("expected UnknownFact, got: {:?}", other),
        }
    }

    // ── 5c. Field ref on non-record value ──
    #[test]
    fn field_ref_on_non_record_returns_error() {
        let mut facts = FactSet::new();
        facts.insert("name".to_string(), Value::Text("hello".to_string()));

        let pred = Predicate::FieldRef {
            var: "name".to_string(),
            field: "length".to_string(),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::NotARecord { message } => {
                assert!(
                    message.contains("not a Record"),
                    "error should mention not a Record, got: {}",
                    message
                );
            }
            other => panic!("expected NotARecord, got: {:?}", other),
        }
    }

    // ── 5d. Field ref on record with missing field ──
    #[test]
    fn field_ref_missing_field_on_record_returns_error() {
        let mut facts = FactSet::new();
        let mut record = BTreeMap::new();
        record.insert("name".to_string(), Value::Text("Alice".to_string()));
        facts.insert("person".to_string(), Value::Record(record));

        let pred = Predicate::FieldRef {
            var: "person".to_string(),
            field: "age".to_string(), // field does not exist
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::NotARecord { message } => {
                assert!(
                    message.contains("not found"),
                    "error should mention 'not found', got: {}",
                    message
                );
            }
            other => panic!("expected NotARecord for missing field, got: {:?}", other),
        }
    }

    // ── 5e. Mul on non-numeric type produces TypeError ──
    #[test]
    fn mul_on_text_returns_type_error() {
        let mut facts = FactSet::new();
        facts.insert("label".to_string(), Value::Text("hello".to_string()));

        let pred = Predicate::Mul {
            left: Box::new(Predicate::FactRef("label".to_string())),
            literal: 3,
            result_type: make_type_spec("Int"),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::TypeError { message } => {
                assert!(
                    message.contains("multiplication requires numeric"),
                    "error should mention numeric operand, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got: {:?}", other),
        }
    }

    // ── Forall on non-list domain produces TypeError ──
    #[test]
    fn forall_on_non_list_domain_returns_type_error() {
        let mut facts = FactSet::new();
        facts.insert("count".to_string(), Value::Int(5));

        let pred = Predicate::Forall {
            variable: "item".to_string(),
            variable_type: make_type_spec("Int"),
            domain: Box::new(Predicate::FactRef("count".to_string())),
            body: Box::new(Predicate::Literal {
                value: Value::Bool(true),
                type_spec: make_type_spec("Bool"),
            }),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::TypeError { message } => {
                assert!(
                    message.contains("forall domain must be a List"),
                    "error should mention List domain, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got: {:?}", other),
        }
    }

    // ── Not on non-boolean value produces TypeError ──
    #[test]
    fn not_on_non_bool_returns_type_error() {
        let mut facts = FactSet::new();
        facts.insert("count".to_string(), Value::Int(5));

        let pred = Predicate::Not {
            operand: Box::new(Predicate::FactRef("count".to_string())),
        };

        let mut collector = ProvenanceCollector::new();
        let ctx = EvalContext::new();
        let result = eval_pred(&pred, &facts, &empty_verdicts(), &ctx, &mut collector);

        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::TypeError { message } => {
                assert!(
                    message.contains("expected Bool"),
                    "error should mention expected Bool, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got: {:?}", other),
        }
    }
}
