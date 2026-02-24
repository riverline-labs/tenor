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
