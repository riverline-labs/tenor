//! Runtime value types and contract representation for the Tenor evaluator.
//!
//! These types are DISTINCT from tenor-core AST types. The evaluator consumes
//! interchange JSON, not raw DSL. All types here are deserialized from the
//! canonical interchange format.

pub mod contract;
pub mod fact;
pub mod values;

use std::collections::BTreeMap;
use std::fmt;

// Re-export everything at the types:: level for backward compatibility.
pub use contract::{
    parse_predicate, CompStep, Contract, Effect, Entity, FailureHandler, Flow, FlowStep,
    JoinPolicy, MulExpr, Operation, ParallelBranch, PayloadValue, ProduceClause, Rule, StepTarget,
    Transition,
};
pub use fact::{FactDecl, FactSet, VerdictInstance, VerdictSet};
pub use values::{parse_default_value, parse_plain_value, value_to_json, Value};

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

/// Errors that can occur during evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    /// A required fact was not provided and has no default.
    MissingFact { fact_id: String },
    /// A fact value does not match its declared type.
    TypeMismatch {
        fact_id: String,
        expected: String,
        got: String,
    },
    /// Numeric overflow during arithmetic.
    Overflow { message: String },
    /// An invalid comparison operator was used.
    InvalidOperator { op: String },
    /// A referenced fact was not found in the FactSet.
    UnknownFact { fact_id: String },
    /// A referenced verdict was not found in the VerdictSet.
    UnknownVerdict { verdict_type: String },
    /// Error deserializing interchange JSON.
    DeserializeError { message: String },
    /// Type error during predicate evaluation.
    TypeError { message: String },
    /// List exceeds declared max length.
    ListOverflow {
        fact_id: String,
        max: u32,
        actual: usize,
    },
    /// Enum value not in declared variants.
    InvalidEnum {
        fact_id: String,
        value: String,
        variants: Vec<String>,
    },
    /// Field access on non-record value.
    NotARecord { message: String },
    /// Variable not bound in current scope.
    UnboundVariable { name: String },
    /// Error during flow execution (step limit, structural issues).
    FlowError { flow_id: String, message: String },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::MissingFact { fact_id } => {
                write!(f, "missing required fact: {}", fact_id)
            }
            EvalError::TypeMismatch {
                fact_id,
                expected,
                got,
            } => {
                write!(
                    f,
                    "type mismatch for fact '{}': expected {}, got {}",
                    fact_id, expected, got
                )
            }
            EvalError::Overflow { message } => {
                write!(f, "numeric overflow: {}", message)
            }
            EvalError::InvalidOperator { op } => {
                write!(f, "invalid operator: {}", op)
            }
            EvalError::UnknownFact { fact_id } => {
                write!(f, "unknown fact: {}", fact_id)
            }
            EvalError::UnknownVerdict { verdict_type } => {
                write!(f, "unknown verdict: {}", verdict_type)
            }
            EvalError::DeserializeError { message } => {
                write!(f, "deserialization error: {}", message)
            }
            EvalError::TypeError { message } => {
                write!(f, "type error: {}", message)
            }
            EvalError::ListOverflow {
                fact_id,
                max,
                actual,
            } => {
                write!(
                    f,
                    "list fact '{}' has {} elements, max is {}",
                    fact_id, actual, max
                )
            }
            EvalError::InvalidEnum {
                fact_id,
                value,
                variants,
            } => {
                write!(
                    f,
                    "invalid enum value '{}' for fact '{}', valid: {:?}",
                    value, fact_id, variants
                )
            }
            EvalError::NotARecord { message } => {
                write!(f, "not a record: {}", message)
            }
            EvalError::UnboundVariable { name } => {
                write!(f, "unbound variable: {}", name)
            }
            EvalError::FlowError { flow_id, message } => {
                write!(f, "flow error in '{}': {}", flow_id, message)
            }
        }
    }
}

impl std::error::Error for EvalError {}

// ──────────────────────────────────────────────
// Type descriptors (from interchange JSON)
// ──────────────────────────────────────────────

/// Type specification deserialized from interchange JSON BaseType.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSpec {
    pub base: String,
    pub precision: Option<u32>,
    pub scale: Option<u32>,
    pub currency: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub max_length: Option<u32>,
    pub values: Option<Vec<String>>,
    pub fields: Option<BTreeMap<String, TypeSpec>>,
    pub element_type: Option<Box<TypeSpec>>,
    pub unit: Option<String>,
    pub variants: Option<BTreeMap<String, TypeSpec>>,
}

impl TypeSpec {
    /// Parse a TypeSpec from interchange JSON BaseType object.
    pub fn from_json(v: &serde_json::Value) -> Result<TypeSpec, EvalError> {
        let obj = v.as_object().ok_or_else(|| EvalError::DeserializeError {
            message: "TypeSpec must be a JSON object".to_string(),
        })?;
        let base = obj
            .get("base")
            .and_then(|b| b.as_str())
            .ok_or_else(|| EvalError::DeserializeError {
                message: "TypeSpec missing 'base' field".to_string(),
            })?
            .to_string();

        let precision = obj
            .get("precision")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let scale = obj.get("scale").and_then(|v| v.as_u64()).map(|v| v as u32);
        let currency = obj
            .get("currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let min = obj.get("min").and_then(|v| v.as_i64());
        let max = obj.get("max").and_then(|v| v.as_i64());
        let max_length = obj
            .get("max_length")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let unit = obj
            .get("unit")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let values = obj.get("values").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });

        let fields = if let Some(fields_val) = obj.get("fields") {
            if let Some(fields_obj) = fields_val.as_object() {
                let mut map = BTreeMap::new();
                for (k, v) in fields_obj {
                    map.insert(k.clone(), TypeSpec::from_json(v)?);
                }
                Some(map)
            } else {
                None
            }
        } else {
            None
        };

        let element_type = if let Some(et) = obj.get("element_type") {
            Some(Box::new(TypeSpec::from_json(et)?))
        } else {
            None
        };

        let variants = if let Some(variants_val) = obj.get("variants") {
            if let Some(variants_obj) = variants_val.as_object() {
                let mut map = BTreeMap::new();
                for (k, v) in variants_obj {
                    map.insert(k.clone(), TypeSpec::from_json(v)?);
                }
                Some(map)
            } else {
                None
            }
        } else {
            None
        };

        Ok(TypeSpec {
            base,
            precision,
            scale,
            currency,
            min,
            max,
            max_length,
            values,
            fields,
            element_type,
            unit,
            variants,
        })
    }
}

// ──────────────────────────────────────────────
// Predicate expression tree
// ──────────────────────────────────────────────

/// Predicate expression nodes matching interchange JSON structure.
#[derive(Debug, Clone)]
pub enum Predicate {
    /// Binary comparison: left op right, with optional comparison_type for promotion.
    Compare {
        left: Box<Predicate>,
        op: String,
        right: Box<Predicate>,
        comparison_type: Option<TypeSpec>,
    },
    /// Logical AND: left and right.
    And {
        left: Box<Predicate>,
        right: Box<Predicate>,
    },
    /// Logical OR: left or right.
    Or {
        left: Box<Predicate>,
        right: Box<Predicate>,
    },
    /// Logical NOT.
    Not { operand: Box<Predicate> },
    /// Reference to a fact value.
    FactRef(String),
    /// Reference to a field on a bound variable.
    FieldRef { var: String, field: String },
    /// Literal value with type.
    Literal { value: Value, type_spec: TypeSpec },
    /// Verdict presence check.
    VerdictPresent(String),
    /// Bounded universal quantification.
    Forall {
        variable: String,
        variable_type: TypeSpec,
        domain: Box<Predicate>,
        body: Box<Predicate>,
    },
    /// Bounded existential quantification.
    Exists {
        variable: String,
        variable_type: TypeSpec,
        domain: Box<Predicate>,
        body: Box<Predicate>,
    },
    /// Multiplication expression.
    Mul {
        left: Box<Predicate>,
        literal: i64,
        result_type: TypeSpec,
    },
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn value_bool_equality() {
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
    }

    #[test]
    fn value_int_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
    }

    #[test]
    fn value_decimal_equality() {
        assert_eq!(Value::Decimal(dec("100.50")), Value::Decimal(dec("100.50")));
        assert_ne!(Value::Decimal(dec("100.50")), Value::Decimal(dec("100.51")));
    }

    #[test]
    fn value_money_equality() {
        let m1 = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let m2 = Value::Money {
            amount: dec("100.00"),
            currency: "USD".to_string(),
        };
        let m3 = Value::Money {
            amount: dec("200.00"),
            currency: "USD".to_string(),
        };
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    #[test]
    fn value_enum_equality() {
        assert_eq!(
            Value::Enum("active".to_string()),
            Value::Enum("active".to_string())
        );
        assert_ne!(
            Value::Enum("active".to_string()),
            Value::Enum("inactive".to_string())
        );
    }

    #[test]
    fn value_record_equality() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), Value::Text("Alice".to_string()));
        fields.insert("age".to_string(), Value::Int(30));
        let r1 = Value::Record(fields.clone());
        let r2 = Value::Record(fields);
        assert_eq!(r1, r2);
    }

    #[test]
    fn value_list_equality() {
        let l1 = Value::List(vec![Value::Int(1), Value::Int(2)]);
        let l2 = Value::List(vec![Value::Int(1), Value::Int(2)]);
        let l3 = Value::List(vec![Value::Int(1), Value::Int(3)]);
        assert_eq!(l1, l2);
        assert_ne!(l1, l3);
    }

    #[test]
    fn value_type_name() {
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Int(1).type_name(), "Int");
        assert_eq!(Value::Decimal(dec("1.0")).type_name(), "Decimal");
        assert_eq!(
            Value::Money {
                amount: dec("1.0"),
                currency: "USD".to_string()
            }
            .type_name(),
            "Money"
        );
    }

    #[test]
    fn value_as_bool() {
        assert_eq!(Value::Bool(true).as_bool().unwrap(), true);
        assert!(Value::Int(1).as_bool().is_err());
    }

    #[test]
    fn typespec_from_json_bool() {
        let json = serde_json::json!({"base": "Bool"});
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "Bool");
    }

    #[test]
    fn typespec_from_json_decimal() {
        let json = serde_json::json!({"base": "Decimal", "precision": 10, "scale": 2});
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "Decimal");
        assert_eq!(ts.precision, Some(10));
        assert_eq!(ts.scale, Some(2));
    }

    #[test]
    fn typespec_from_json_money() {
        let json = serde_json::json!({"base": "Money", "currency": "USD"});
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "Money");
        assert_eq!(ts.currency, Some("USD".to_string()));
    }

    #[test]
    fn typespec_from_json_enum() {
        let json = serde_json::json!({"base": "Enum", "values": ["a", "b", "c"]});
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "Enum");
        assert_eq!(
            ts.values,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn typespec_from_json_list() {
        let json = serde_json::json!({
            "base": "List",
            "element_type": {"base": "Int", "min": 0, "max": 100},
            "max": 50
        });
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "List");
        assert_eq!(ts.max, Some(50));
        let et = ts.element_type.as_ref().unwrap();
        assert_eq!(et.base, "Int");
    }

    #[test]
    fn typespec_from_json_record() {
        let json = serde_json::json!({
            "base": "Record",
            "fields": {
                "name": {"base": "Text", "max_length": 100},
                "active": {"base": "Bool"}
            }
        });
        let ts = TypeSpec::from_json(&json).unwrap();
        assert_eq!(ts.base, "Record");
        let fields = ts.fields.as_ref().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields["name"].base, "Text");
        assert_eq!(fields["active"].base, "Bool");
    }

    #[test]
    fn fact_set_operations() {
        let mut fs = FactSet::new();
        fs.insert("x".to_string(), Value::Int(42));
        assert_eq!(fs.get("x"), Some(&Value::Int(42)));
        assert_eq!(fs.get("y"), None);
    }

    #[test]
    fn verdict_set_operations() {
        let mut vs = VerdictSet::new();
        assert!(!vs.has_verdict("test"));
        vs.push(VerdictInstance {
            verdict_type: "test".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "r1".to_string(),
                stratum: 0,
                facts_used: vec!["f1".to_string()],
                verdicts_used: vec![],
            },
        });
        assert!(vs.has_verdict("test"));
        assert!(!vs.has_verdict("other"));
        assert_eq!(vs.get_verdict("test").unwrap().verdict_type, "test");
    }

    /// §5A.5: Removing all Source declarations from a contract produces
    /// identical evaluation behavior.
    #[test]
    fn source_declarations_do_not_affect_evaluation() {
        use serde_json::json;

        // Bundle WITH Source constructs
        let bundle_with_sources = json!({
            "id": "source-eval-test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [
                {
                    "id": "admin",
                    "kind": "Persona",
                    "provenance": {"file": "test.tenor", "line": 1},
                    "tenor": "1.0"
                },
                {
                    "id": "order_service",
                    "kind": "Source",
                    "protocol": "http",
                    "fields": {
                        "base_url": "https://api.orders.com/v2"
                    },
                    "description": "Order management REST API",
                    "provenance": {"file": "test.tenor", "line": 3},
                    "tenor": "1.0"
                },
                {
                    "id": "compliance_db",
                    "kind": "Source",
                    "protocol": "database",
                    "fields": {
                        "dialect": "postgres"
                    },
                    "provenance": {"file": "test.tenor", "line": 10},
                    "tenor": "1.0"
                },
                {
                    "id": "amount",
                    "kind": "Fact",
                    "type": {"base": "Int"},
                    "source": {"source_id": "order_service", "path": "orders.balance"},
                    "provenance": {"file": "test.tenor", "line": 17},
                    "tenor": "1.0"
                },
                {
                    "id": "is_compliant",
                    "kind": "Fact",
                    "type": {"base": "Bool"},
                    "source": "compliance.flag",
                    "provenance": {"file": "test.tenor", "line": 22},
                    "tenor": "1.0"
                },
                {
                    "id": "check_amount",
                    "kind": "Rule",
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": {"fact_ref": "amount"},
                            "op": ">",
                            "right": {"literal": 100, "type": {"base": "Int"}}
                        },
                        "produce": {
                            "verdict_type": "high_value",
                            "payload": {"type": {"base": "Bool"}, "value": true}
                        }
                    },
                    "provenance": {"file": "test.tenor", "line": 27},
                    "tenor": "1.0"
                }
            ]
        });

        // Bundle WITHOUT Source constructs (everything else identical)
        let bundle_without_sources = json!({
            "id": "source-eval-test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [
                {
                    "id": "admin",
                    "kind": "Persona",
                    "provenance": {"file": "test.tenor", "line": 1},
                    "tenor": "1.0"
                },
                {
                    "id": "amount",
                    "kind": "Fact",
                    "type": {"base": "Int"},
                    "source": {"source_id": "order_service", "path": "orders.balance"},
                    "provenance": {"file": "test.tenor", "line": 17},
                    "tenor": "1.0"
                },
                {
                    "id": "is_compliant",
                    "kind": "Fact",
                    "type": {"base": "Bool"},
                    "source": "compliance.flag",
                    "provenance": {"file": "test.tenor", "line": 22},
                    "tenor": "1.0"
                },
                {
                    "id": "check_amount",
                    "kind": "Rule",
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": {"fact_ref": "amount"},
                            "op": ">",
                            "right": {"literal": 100, "type": {"base": "Int"}}
                        },
                        "produce": {
                            "verdict_type": "high_value",
                            "payload": {"type": {"base": "Bool"}, "value": true}
                        }
                    },
                    "provenance": {"file": "test.tenor", "line": 27},
                    "tenor": "1.0"
                }
            ]
        });

        let facts = json!({"amount": 500, "is_compliant": true});

        // Evaluate both
        let contract_with =
            Contract::from_interchange(&bundle_with_sources).expect("with-sources contract");
        let contract_without =
            Contract::from_interchange(&bundle_without_sources).expect("without-sources contract");

        let fact_set_with =
            crate::assemble::assemble_facts(&contract_with, &facts).expect("assemble with");
        let fact_set_without =
            crate::assemble::assemble_facts(&contract_without, &facts).expect("assemble without");

        let verdicts_with =
            crate::rules::eval_strata(&contract_with, &fact_set_with).expect("eval with");
        let verdicts_without =
            crate::rules::eval_strata(&contract_without, &fact_set_without).expect("eval without");

        // Verdicts must be identical
        assert_eq!(
            verdicts_with.to_json(),
            verdicts_without.to_json(),
            "Source declarations must not affect evaluation results"
        );
    }
}
