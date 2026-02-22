//! Runtime value types and contract representation for the Tenor evaluator.
//!
//! These types are DISTINCT from tenor-core AST types. The evaluator consumes
//! interchange JSON, not raw DSL. All types here are deserialized from the
//! canonical interchange format.

use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt;

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
// Runtime values
// ──────────────────────────────────────────────

/// Runtime value types from spec Section 4 BaseType.
/// All numeric values use `rust_decimal::Decimal` -- never `f64`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Decimal(Decimal),
    Text(String),
    Date(String),
    DateTime(String),
    Money { amount: Decimal, currency: String },
    Duration { value: i64, unit: String },
    Enum(String),
    Record(BTreeMap<String, Value>),
    List(Vec<Value>),
    TaggedUnion { tag: String, payload: Box<Value> },
}

impl Value {
    /// Returns a human-readable type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Decimal(_) => "Decimal",
            Value::Text(_) => "Text",
            Value::Date(_) => "Date",
            Value::DateTime(_) => "DateTime",
            Value::Money { .. } => "Money",
            Value::Duration { .. } => "Duration",
            Value::Enum(_) => "Enum",
            Value::Record(_) => "Record",
            Value::List(_) => "List",
            Value::TaggedUnion { .. } => "TaggedUnion",
        }
    }

    /// Extracts a boolean or returns a type error.
    pub fn as_bool(&self) -> Result<bool, EvalError> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(EvalError::TypeError {
                message: format!("expected Bool, got {}", other.type_name()),
            }),
        }
    }
}

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
// Contract representation
// ──────────────────────────────────────────────

/// Contract deserialized from interchange JSON bundle.
#[derive(Debug, Clone)]
pub struct Contract {
    pub facts: Vec<FactDecl>,
    pub entities: Vec<Entity>,
    pub rules: Vec<Rule>,
    pub operations: Vec<Operation>,
    pub flows: Vec<Flow>,
    pub personas: Vec<String>,
}

impl Contract {
    /// Deserialize a Contract from interchange JSON bundle.
    ///
    /// Walks the `constructs` array and extracts facts, entities, rules,
    /// operations, flows, and personas by matching on the `kind` field.
    pub fn from_interchange(bundle: &serde_json::Value) -> Result<Contract, EvalError> {
        let constructs = bundle
            .get("constructs")
            .and_then(|c| c.as_array())
            .ok_or_else(|| EvalError::DeserializeError {
                message: "bundle missing 'constructs' array".to_string(),
            })?;

        let mut facts = Vec::new();
        let mut entities = Vec::new();
        let mut rules = Vec::new();
        let mut operations = Vec::new();
        let mut flows = Vec::new();
        let mut personas = Vec::new();

        for construct in constructs {
            let kind = construct
                .get("kind")
                .and_then(|k| k.as_str())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "construct missing 'kind' field".to_string(),
                })?;

            match kind {
                "Fact" => facts.push(parse_fact(construct)?),
                "Entity" => entities.push(parse_entity(construct)?),
                "Rule" => rules.push(parse_rule(construct)?),
                "Operation" => operations.push(parse_operation(construct)?),
                "Flow" => flows.push(parse_flow(construct)?),
                "Persona" => {
                    let id = get_str(construct, "id")?;
                    personas.push(id);
                }
                _ => {
                    // Ignore unknown construct kinds for forward compatibility
                }
            }
        }

        Ok(Contract {
            facts,
            entities,
            rules,
            operations,
            flows,
            personas,
        })
    }
}

// ──────────────────────────────────────────────
// Interchange JSON parsing helpers
// ──────────────────────────────────────────────

fn get_str(obj: &serde_json::Value, field: &str) -> Result<String, EvalError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("missing string field '{}'", field),
        })
}

fn parse_fact(v: &serde_json::Value) -> Result<FactDecl, EvalError> {
    let id = get_str(v, "id")?;
    let type_val = v.get("type").ok_or_else(|| EvalError::DeserializeError {
        message: format!("Fact '{}' missing 'type' field", id),
    })?;
    let fact_type = TypeSpec::from_json(type_val)?;
    let default = if let Some(def) = v.get("default") {
        Some(parse_default_value(def, &fact_type)?)
    } else {
        None
    };
    Ok(FactDecl {
        id,
        fact_type,
        default,
    })
}

/// Parse a default value from interchange JSON format.
/// Defaults use structured representations (bool_literal, decimal_value, money_value, etc.).
pub fn parse_default_value(
    v: &serde_json::Value,
    type_spec: &TypeSpec,
) -> Result<Value, EvalError> {
    // Structured defaults have a "kind" field
    if let Some(kind) = v.get("kind").and_then(|k| k.as_str()) {
        match kind {
            "bool_literal" => {
                let val = v.get("value").and_then(|b| b.as_bool()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "bool_literal missing 'value'".to_string(),
                    }
                })?;
                return Ok(Value::Bool(val));
            }
            "int_literal" => {
                let val = v.get("value").and_then(|i| i.as_i64()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "int_literal missing 'value'".to_string(),
                    }
                })?;
                return Ok(Value::Int(val));
            }
            "decimal_value" => {
                let val_str = v.get("value").and_then(|s| s.as_str()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "decimal_value missing 'value'".to_string(),
                    }
                })?;
                let d = val_str.parse::<rust_decimal::Decimal>().map_err(|e| {
                    EvalError::DeserializeError {
                        message: format!("invalid decimal: {}", e),
                    }
                })?;
                return Ok(Value::Decimal(d));
            }
            "money_value" => {
                let currency = get_str(v, "currency")?;
                let amount_obj = v.get("amount").ok_or_else(|| EvalError::DeserializeError {
                    message: "money_value missing 'amount'".to_string(),
                })?;
                let amount_str = amount_obj
                    .get("value")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "money_value amount missing 'value'".to_string(),
                    })?;
                let amount = amount_str.parse::<rust_decimal::Decimal>().map_err(|e| {
                    EvalError::DeserializeError {
                        message: format!("invalid money amount: {}", e),
                    }
                })?;
                return Ok(Value::Money { amount, currency });
            }
            _ => {}
        }
    }
    // Plain JSON values (no "kind" wrapper)
    parse_plain_value(v, type_spec)
}

/// Parse a plain JSON value (without interchange "kind" wrapper) according to type.
pub fn parse_plain_value(v: &serde_json::Value, type_spec: &TypeSpec) -> Result<Value, EvalError> {
    match type_spec.base.as_str() {
        "Bool" => {
            let b = v.as_bool().ok_or_else(|| EvalError::DeserializeError {
                message: "expected boolean".to_string(),
            })?;
            Ok(Value::Bool(b))
        }
        "Int" => {
            let i = v.as_i64().ok_or_else(|| EvalError::DeserializeError {
                message: "expected integer".to_string(),
            })?;
            Ok(Value::Int(i))
        }
        "Decimal" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected decimal string".to_string(),
            })?;
            let d =
                s.parse::<rust_decimal::Decimal>()
                    .map_err(|e| EvalError::DeserializeError {
                        message: format!("invalid decimal: {}", e),
                    })?;
            Ok(Value::Decimal(d))
        }
        "Money" => {
            // Handle two formats:
            // 1. Facts format: {"amount": "8500.00", "currency": "USD"}
            // 2. Interchange literal format: {"amount": {"kind": "decimal_value", "value": "50000.00", ...}, "currency": "USD"}
            let amount_val = v.get("amount").ok_or_else(|| EvalError::DeserializeError {
                message: "Money value missing 'amount' field".to_string(),
            })?;
            let amount_str = if let Some(s) = amount_val.as_str() {
                // Format 1: plain string
                s.to_string()
            } else if let Some(inner) = amount_val.get("value").and_then(|v| v.as_str()) {
                // Format 2: structured decimal_value with nested "value" string
                inner.to_string()
            } else {
                return Err(EvalError::DeserializeError {
                    message: "Money 'amount' must be a string or structured decimal_value"
                        .to_string(),
                });
            };
            let amount = amount_str.parse::<rust_decimal::Decimal>().map_err(|e| {
                EvalError::DeserializeError {
                    message: format!("invalid money amount: {}", e),
                }
            })?;
            let currency = v
                .get("currency")
                .and_then(|c| c.as_str())
                .unwrap_or_else(|| type_spec.currency.as_deref().unwrap_or(""))
                .to_string();
            Ok(Value::Money { amount, currency })
        }
        "Text" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected text string".to_string(),
            })?;
            Ok(Value::Text(s.to_string()))
        }
        "Date" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected date string".to_string(),
            })?;
            Ok(Value::Date(s.to_string()))
        }
        "DateTime" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected datetime string".to_string(),
            })?;
            Ok(Value::DateTime(s.to_string()))
        }
        "Duration" => {
            let val = v.get("value").and_then(|i| i.as_i64()).ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "Duration missing 'value'".to_string(),
                }
            })?;
            let unit = v
                .get("unit")
                .and_then(|u| u.as_str())
                .unwrap_or_else(|| type_spec.unit.as_deref().unwrap_or("seconds"))
                .to_string();
            Ok(Value::Duration { value: val, unit })
        }
        "Enum" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected enum string".to_string(),
            })?;
            Ok(Value::Enum(s.to_string()))
        }
        "Record" => {
            let obj = v.as_object().ok_or_else(|| EvalError::DeserializeError {
                message: "expected record object".to_string(),
            })?;
            let field_types =
                type_spec
                    .fields
                    .as_ref()
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "Record type missing 'fields'".to_string(),
                    })?;
            let mut fields = BTreeMap::new();
            for (k, ft) in field_types {
                let fv = obj.get(k).ok_or_else(|| EvalError::DeserializeError {
                    message: format!("Record missing field '{}'", k),
                })?;
                fields.insert(k.clone(), parse_plain_value(fv, ft)?);
            }
            Ok(Value::Record(fields))
        }
        "List" => {
            let arr = v.as_array().ok_or_else(|| EvalError::DeserializeError {
                message: "expected list array".to_string(),
            })?;
            let et =
                type_spec
                    .element_type
                    .as_ref()
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "List type missing 'element_type'".to_string(),
                    })?;
            let items: Result<Vec<Value>, _> =
                arr.iter().map(|item| parse_plain_value(item, et)).collect();
            Ok(Value::List(items?))
        }
        "TaggedUnion" => {
            let obj = v.as_object().ok_or_else(|| EvalError::DeserializeError {
                message: "expected tagged union object".to_string(),
            })?;
            let tag = obj
                .get("tag")
                .and_then(|t| t.as_str())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "TaggedUnion missing 'tag'".to_string(),
                })?
                .to_string();
            let payload_val = obj
                .get("payload")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "TaggedUnion missing 'payload'".to_string(),
                })?;
            let variant_types =
                type_spec
                    .variants
                    .as_ref()
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "TaggedUnion type missing 'variants'".to_string(),
                    })?;
            let vt = variant_types
                .get(&tag)
                .ok_or_else(|| EvalError::DeserializeError {
                    message: format!("unknown TaggedUnion variant '{}'", tag),
                })?;
            let payload = parse_plain_value(payload_val, vt)?;
            Ok(Value::TaggedUnion {
                tag,
                payload: Box::new(payload),
            })
        }
        other => Err(EvalError::DeserializeError {
            message: format!("unsupported type base: {}", other),
        }),
    }
}

fn parse_entity(v: &serde_json::Value) -> Result<Entity, EvalError> {
    let id = get_str(v, "id")?;
    let initial = get_str(v, "initial")?;
    let states: Vec<String> = v
        .get("states")
        .and_then(|s| s.as_array())
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("Entity '{}' missing 'states'", id),
        })?
        .iter()
        .filter_map(|s| s.as_str().map(|s| s.to_string()))
        .collect();
    let transitions: Vec<Transition> = v
        .get("transitions")
        .and_then(|t| t.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .map(|t| {
            Ok(Transition {
                from: get_str(t, "from")?,
                to: get_str(t, "to")?,
            })
        })
        .collect::<Result<Vec<_>, EvalError>>()?;
    Ok(Entity {
        id,
        states,
        initial,
        transitions,
    })
}

fn parse_rule(v: &serde_json::Value) -> Result<Rule, EvalError> {
    let id = get_str(v, "id")?;
    let stratum =
        v.get("stratum")
            .and_then(|s| s.as_u64())
            .ok_or_else(|| EvalError::DeserializeError {
                message: format!("Rule '{}' missing 'stratum'", id),
            })? as u32;
    let body = v.get("body").ok_or_else(|| EvalError::DeserializeError {
        message: format!("Rule '{}' missing 'body'", id),
    })?;
    let when = body
        .get("when")
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("Rule '{}' body missing 'when'", id),
        })?;
    let condition = parse_predicate(when)?;
    let produce_obj = body
        .get("produce")
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("Rule '{}' body missing 'produce'", id),
        })?;
    let produce = parse_produce(produce_obj)?;
    Ok(Rule {
        id,
        stratum,
        condition,
        produce,
    })
}

fn parse_produce(v: &serde_json::Value) -> Result<ProduceClause, EvalError> {
    let verdict_type = get_str(v, "verdict_type")?;
    let payload = v
        .get("payload")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce clause missing 'payload'".to_string(),
        })?;
    let type_val = payload
        .get("type")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce payload missing 'type'".to_string(),
        })?;
    let payload_type = TypeSpec::from_json(type_val)?;
    let value = payload
        .get("value")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce payload missing 'value'".to_string(),
        })?;

    let payload_value =
        if value.is_object() && value.get("op").and_then(|o| o.as_str()) == Some("*") {
            // MulExpr
            let left = value
                .get("left")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'left'".to_string(),
                })?;
            let fact_ref = get_str(left, "fact_ref")?;
            let literal = value
                .get("literal")
                .and_then(|l| l.as_i64())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'literal'".to_string(),
                })?;
            let rt = value
                .get("result_type")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'result_type'".to_string(),
                })?;
            let result_type = TypeSpec::from_json(rt)?;
            PayloadValue::Mul(MulExpr {
                fact_ref,
                literal,
                result_type,
            })
        } else {
            // Literal value
            let lit = parse_literal_value(value, &payload_type)?;
            PayloadValue::Literal(lit)
        };

    Ok(ProduceClause {
        verdict_type,
        payload_type,
        payload_value,
    })
}

/// Parse a literal value from interchange JSON.
fn parse_literal_value(v: &serde_json::Value, type_spec: &TypeSpec) -> Result<Value, EvalError> {
    match type_spec.base.as_str() {
        "Bool" => {
            let b = v.as_bool().ok_or_else(|| EvalError::DeserializeError {
                message: "expected boolean literal".to_string(),
            })?;
            Ok(Value::Bool(b))
        }
        "Int" => {
            let i = v.as_i64().ok_or_else(|| EvalError::DeserializeError {
                message: "expected integer literal".to_string(),
            })?;
            Ok(Value::Int(i))
        }
        "Text" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected text literal".to_string(),
            })?;
            Ok(Value::Text(s.to_string()))
        }
        "Enum" => {
            let s = v.as_str().ok_or_else(|| EvalError::DeserializeError {
                message: "expected enum literal".to_string(),
            })?;
            Ok(Value::Enum(s.to_string()))
        }
        _ => {
            // For structured types, try parse_default_value which handles kind-tagged formats
            parse_default_value(v, type_spec)
        }
    }
}

fn parse_operation(v: &serde_json::Value) -> Result<Operation, EvalError> {
    let id = get_str(v, "id")?;
    let allowed_personas: Vec<String> = v
        .get("allowed_personas")
        .and_then(|a| a.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|p| p.as_str().map(|s| s.to_string()))
        .collect();
    let precondition =
        parse_predicate(
            v.get("precondition")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: format!("Operation '{}' missing 'precondition'", id),
                })?,
        )?;
    let effects: Vec<Effect> = v
        .get("effects")
        .and_then(|e| e.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .map(|e| {
            Ok(Effect {
                entity_id: get_str(e, "entity_id")?,
                from: get_str(e, "from")?,
                to: get_str(e, "to")?,
                outcome: e
                    .get("outcome")
                    .and_then(|o| o.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect::<Result<Vec<_>, EvalError>>()?;
    let error_contract: Vec<String> = v
        .get("error_contract")
        .and_then(|e| e.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|e| e.as_str().map(|s| s.to_string()))
        .collect();
    let outcomes: Vec<String> = v
        .get("outcomes")
        .and_then(|o| o.as_array())
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|o| o.as_str().map(|s| s.to_string()))
        .collect();
    Ok(Operation {
        id,
        allowed_personas,
        precondition,
        effects,
        error_contract,
        outcomes,
    })
}

fn parse_flow(v: &serde_json::Value) -> Result<Flow, EvalError> {
    let id = get_str(v, "id")?;
    let snapshot = v
        .get("snapshot")
        .and_then(|s| s.as_str())
        .unwrap_or("at_initiation")
        .to_string();
    let entry = get_str(v, "entry")?;
    let steps_arr =
        v.get("steps")
            .and_then(|s| s.as_array())
            .ok_or_else(|| EvalError::DeserializeError {
                message: format!("Flow '{}' missing 'steps'", id),
            })?;
    let steps: Vec<FlowStep> = steps_arr
        .iter()
        .map(parse_flow_step)
        .collect::<Result<Vec<_>, EvalError>>()?;
    Ok(Flow {
        id,
        snapshot,
        entry,
        steps,
    })
}

fn parse_flow_step(v: &serde_json::Value) -> Result<FlowStep, EvalError> {
    let kind = get_str(v, "kind")?;
    match kind.as_str() {
        "OperationStep" => {
            let id = get_str(v, "id")?;
            let op = get_str(v, "op")?;
            let persona = get_str(v, "persona")?;
            let outcomes_obj = v
                .get("outcomes")
                .and_then(|o| o.as_object())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "OperationStep missing 'outcomes'".to_string(),
                })?;
            let mut outcomes = BTreeMap::new();
            for (k, target_val) in outcomes_obj {
                outcomes.insert(k.clone(), parse_step_target(target_val)?);
            }
            let on_failure = parse_failure_handler(v.get("on_failure").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "OperationStep missing 'on_failure'".to_string(),
                }
            })?)?;
            Ok(FlowStep::OperationStep {
                id,
                op,
                persona,
                outcomes,
                on_failure,
            })
        }
        "BranchStep" => {
            let id = get_str(v, "id")?;
            let persona = get_str(v, "persona")?;
            let condition = parse_predicate(v.get("condition").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'condition'".to_string(),
                }
            })?)?;
            let if_true = parse_step_target(v.get("if_true").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'if_true'".to_string(),
                }
            })?)?;
            let if_false = parse_step_target(v.get("if_false").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'if_false'".to_string(),
                }
            })?)?;
            Ok(FlowStep::BranchStep {
                id,
                condition,
                persona,
                if_true,
                if_false,
            })
        }
        "HandoffStep" => {
            let id = get_str(v, "id")?;
            let from_persona = get_str(v, "from_persona")?;
            let to_persona = get_str(v, "to_persona")?;
            let next = get_str(v, "next")?;
            Ok(FlowStep::HandoffStep {
                id,
                from_persona,
                to_persona,
                next,
            })
        }
        "SubFlowStep" => {
            let id = get_str(v, "id")?;
            let flow = get_str(v, "flow")?;
            let persona = get_str(v, "persona")?;
            let on_success = parse_step_target(v.get("on_success").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "SubFlowStep missing 'on_success'".to_string(),
                }
            })?)?;
            let on_failure = parse_failure_handler(v.get("on_failure").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "SubFlowStep missing 'on_failure'".to_string(),
                }
            })?)?;
            Ok(FlowStep::SubFlowStep {
                id,
                flow,
                persona,
                on_success,
                on_failure,
            })
        }
        "ParallelStep" => {
            let id = get_str(v, "id")?;
            let branches_arr = v
                .get("branches")
                .and_then(|b| b.as_array())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "ParallelStep missing 'branches'".to_string(),
                })?;
            let branches: Vec<ParallelBranch> = branches_arr
                .iter()
                .map(|b| {
                    let bid = get_str(b, "id")?;
                    let entry = get_str(b, "entry")?;
                    let steps: Vec<FlowStep> = b
                        .get("steps")
                        .and_then(|s| s.as_array())
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(parse_flow_step)
                        .collect::<Result<Vec<_>, EvalError>>()?;
                    Ok(ParallelBranch {
                        id: bid,
                        entry,
                        steps,
                    })
                })
                .collect::<Result<Vec<_>, EvalError>>()?;
            let join_obj = v.get("join").ok_or_else(|| EvalError::DeserializeError {
                message: "ParallelStep missing 'join'".to_string(),
            })?;
            let on_all_success = if let Some(t) = join_obj.get("on_all_success") {
                Some(parse_step_target(t)?)
            } else {
                None
            };
            let on_any_failure = if let Some(f) = join_obj.get("on_any_failure") {
                Some(parse_failure_handler(f)?)
            } else {
                None
            };
            let on_all_complete = if let Some(t) = join_obj.get("on_all_complete") {
                Some(parse_step_target(t)?)
            } else {
                None
            };
            let join = JoinPolicy {
                on_all_success,
                on_any_failure,
                on_all_complete,
            };
            Ok(FlowStep::ParallelStep { id, branches, join })
        }
        _ => Err(EvalError::DeserializeError {
            message: format!("unknown step kind: {}", kind),
        }),
    }
}

fn parse_step_target(v: &serde_json::Value) -> Result<StepTarget, EvalError> {
    if let Some(s) = v.as_str() {
        Ok(StepTarget::StepRef(s.to_string()))
    } else if let Some(obj) = v.as_object() {
        let outcome = get_str(v, "outcome")?;
        let _ = obj; // already consumed via get_str on v
        Ok(StepTarget::Terminal { outcome })
    } else {
        Err(EvalError::DeserializeError {
            message: "invalid step target".to_string(),
        })
    }
}

fn parse_failure_handler(v: &serde_json::Value) -> Result<FailureHandler, EvalError> {
    let kind = get_str(v, "kind")?;
    match kind.as_str() {
        "Terminate" => {
            let outcome = get_str(v, "outcome")?;
            Ok(FailureHandler::Terminate { outcome })
        }
        "Compensate" => {
            let steps_arr = v.get("steps").and_then(|s| s.as_array()).ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "Compensate handler missing 'steps'".to_string(),
                }
            })?;
            let steps: Vec<CompStep> = steps_arr
                .iter()
                .map(|s| {
                    let op = get_str(s, "op")?;
                    let persona = get_str(s, "persona")?;
                    let on_failure = parse_step_target(s.get("on_failure").ok_or_else(|| {
                        EvalError::DeserializeError {
                            message: "CompensationStep missing 'on_failure'".to_string(),
                        }
                    })?)?;
                    Ok(CompStep {
                        op,
                        persona,
                        on_failure,
                    })
                })
                .collect::<Result<Vec<_>, EvalError>>()?;
            let then =
                parse_step_target(v.get("then").ok_or_else(|| EvalError::DeserializeError {
                    message: "Compensate handler missing 'then'".to_string(),
                })?)?;
            Ok(FailureHandler::Compensate { steps, then })
        }
        "Escalate" => {
            let to_persona = get_str(v, "to_persona")?;
            let next = get_str(v, "next")?;
            Ok(FailureHandler::Escalate { to_persona, next })
        }
        _ => Err(EvalError::DeserializeError {
            message: format!("unknown failure handler kind: {}", kind),
        }),
    }
}

/// Infer a Value and TypeSpec from a JSON literal when the "type" field is absent.
fn infer_literal(v: &serde_json::Value) -> Result<(Value, TypeSpec), EvalError> {
    let base_type = |base: &str| TypeSpec {
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
    };
    if let Some(b) = v.as_bool() {
        Ok((Value::Bool(b), base_type("Bool")))
    } else if let Some(i) = v.as_i64() {
        Ok((Value::Int(i), base_type("Int")))
    } else if let Some(s) = v.as_str() {
        // Could be Text or Enum -- default to Text for untyped string literals
        Ok((Value::Text(s.to_string()), base_type("Text")))
    } else {
        Err(EvalError::DeserializeError {
            message: format!("cannot infer type for literal: {}", v),
        })
    }
}

/// Parse a predicate expression from interchange JSON.
pub fn parse_predicate(v: &serde_json::Value) -> Result<Predicate, EvalError> {
    // Check for verdict_present
    if let Some(vp) = v.get("verdict_present") {
        let id = vp.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "verdict_present must be a string".to_string(),
        })?;
        return Ok(Predicate::VerdictPresent(id.to_string()));
    }

    // Check for fact_ref
    if let Some(fr) = v.get("fact_ref") {
        let id = fr.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "fact_ref must be a string".to_string(),
        })?;
        return Ok(Predicate::FactRef(id.to_string()));
    }

    // Check for field_ref
    if let Some(fr) = v.get("field_ref") {
        let var = get_str(fr, "var")?;
        let field = get_str(fr, "field")?;
        return Ok(Predicate::FieldRef { var, field });
    }

    // Check for op-based expressions BEFORE literal, because Mul nodes
    // have both "op" and "literal" fields -- the op check must come first.
    if let Some(op_val) = v.get("op") {
        let op = op_val.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "'op' must be a string".to_string(),
        })?;

        match op {
            "and" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "and missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "and missing 'right'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::And {
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
            "or" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "or missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "or missing 'right'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::Or {
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
            "not" => {
                let operand = parse_predicate(v.get("operand").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "not missing 'operand'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::Not {
                    operand: Box::new(operand),
                });
            }
            "*" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "mul missing 'left'".to_string(),
                    })?)?;
                let literal = v.get("literal").and_then(|l| l.as_i64()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "mul missing 'literal'".to_string(),
                    }
                })?;
                let rt = v
                    .get("result_type")
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "mul missing 'result_type'".to_string(),
                    })?;
                let result_type = TypeSpec::from_json(rt)?;
                return Ok(Predicate::Mul {
                    left: Box::new(left),
                    literal,
                    result_type,
                });
            }
            // Comparison operators
            "=" | "!=" | "<" | "<=" | ">" | ">=" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "compare missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "compare missing 'right'".to_string(),
                    }
                })?)?;
                let comparison_type = if let Some(ct) = v.get("comparison_type") {
                    Some(TypeSpec::from_json(ct)?)
                } else {
                    None
                };
                return Ok(Predicate::Compare {
                    left: Box::new(left),
                    op: op.to_string(),
                    right: Box::new(right),
                    comparison_type,
                });
            }
            _ => {
                return Err(EvalError::DeserializeError {
                    message: format!("unknown operator: {}", op),
                });
            }
        }
    }

    // Check for literal (after op check, since Mul nodes also have "literal")
    if v.get("literal").is_some() {
        let literal_val = v.get("literal").unwrap();
        let (value, type_spec) = if let Some(type_val) = v.get("type") {
            let ts = TypeSpec::from_json(type_val)?;
            let val = parse_literal_value(literal_val, &ts)?;
            (val, ts)
        } else {
            // Infer type from the JSON literal value when "type" is absent
            // (the elaborator omits type for some literal nodes like text comparisons)
            infer_literal(literal_val)?
        };
        return Ok(Predicate::Literal { value, type_spec });
    }

    // Check for forall (quantifier)
    if v.get("quantifier").and_then(|q| q.as_str()) == Some("forall") {
        let variable = get_str(v, "variable")?;
        let vt = v
            .get("variable_type")
            .ok_or_else(|| EvalError::DeserializeError {
                message: "forall missing 'variable_type'".to_string(),
            })?;
        let variable_type = TypeSpec::from_json(vt)?;
        let domain_val = v.get("domain").ok_or_else(|| EvalError::DeserializeError {
            message: "forall missing 'domain'".to_string(),
        })?;
        let domain = parse_predicate(domain_val)?;
        let body_val = v.get("body").ok_or_else(|| EvalError::DeserializeError {
            message: "forall missing 'body'".to_string(),
        })?;
        let body = parse_predicate(body_val)?;
        return Ok(Predicate::Forall {
            variable,
            variable_type,
            domain: Box::new(domain),
            body: Box::new(body),
        });
    }

    // Check for exists (quantifier)
    if v.get("quantifier").and_then(|q| q.as_str()) == Some("exists") {
        let variable = get_str(v, "variable")?;
        let vt = v
            .get("variable_type")
            .ok_or_else(|| EvalError::DeserializeError {
                message: "exists missing 'variable_type'".to_string(),
            })?;
        let variable_type = TypeSpec::from_json(vt)?;
        let domain_val = v.get("domain").ok_or_else(|| EvalError::DeserializeError {
            message: "exists missing 'domain'".to_string(),
        })?;
        let domain = parse_predicate(domain_val)?;
        let body_val = v.get("body").ok_or_else(|| EvalError::DeserializeError {
            message: "exists missing 'body'".to_string(),
        })?;
        let body = parse_predicate(body_val)?;
        return Ok(Predicate::Exists {
            variable,
            variable_type,
            domain: Box::new(domain),
            body: Box::new(body),
        });
    }

    Err(EvalError::DeserializeError {
        message: format!("unrecognized predicate expression: {}", v),
    })
}

/// A declared fact with type and optional default.
#[derive(Debug, Clone)]
pub struct FactDecl {
    pub id: String,
    pub fact_type: TypeSpec,
    pub default: Option<Value>,
}

/// An entity state machine.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub states: Vec<String>,
    pub initial: String,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
}

/// A verdict-producing rule with stratification.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub stratum: u32,
    pub condition: Predicate,
    pub produce: ProduceClause,
}

/// A produce clause specifying verdict output.
#[derive(Debug, Clone)]
pub struct ProduceClause {
    pub verdict_type: String,
    pub payload_type: TypeSpec,
    pub payload_value: PayloadValue,
}

/// Payload value: either a literal or a computed expression.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum PayloadValue {
    Literal(Value),
    Mul(MulExpr),
}

/// Multiplication expression in a payload.
#[derive(Debug, Clone)]
pub struct MulExpr {
    pub fact_ref: String,
    pub literal: i64,
    pub result_type: TypeSpec,
}

/// An operation (persona-gated state transition).
#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub allowed_personas: Vec<String>,
    pub precondition: Predicate,
    pub effects: Vec<Effect>,
    pub error_contract: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub entity_id: String,
    pub from: String,
    pub to: String,
    pub outcome: Option<String>,
}

/// A flow (DAG of steps).
#[derive(Debug, Clone)]
pub struct Flow {
    pub id: String,
    pub snapshot: String,
    pub entry: String,
    pub steps: Vec<FlowStep>,
}

/// A step within a flow.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum FlowStep {
    OperationStep {
        id: String,
        op: String,
        persona: String,
        outcomes: BTreeMap<String, StepTarget>,
        on_failure: FailureHandler,
    },
    BranchStep {
        id: String,
        condition: Predicate,
        persona: String,
        if_true: StepTarget,
        if_false: StepTarget,
    },
    HandoffStep {
        id: String,
        from_persona: String,
        to_persona: String,
        next: String,
    },
    SubFlowStep {
        id: String,
        flow: String,
        persona: String,
        on_success: StepTarget,
        on_failure: FailureHandler,
    },
    ParallelStep {
        id: String,
        branches: Vec<ParallelBranch>,
        join: JoinPolicy,
    },
}

#[derive(Debug, Clone)]
pub enum StepTarget {
    StepRef(String),
    Terminal { outcome: String },
}

#[derive(Debug, Clone)]
pub enum FailureHandler {
    Terminate {
        outcome: String,
    },
    Compensate {
        steps: Vec<CompStep>,
        then: StepTarget,
    },
    Escalate {
        to_persona: String,
        next: String,
    },
}

#[derive(Debug, Clone)]
pub struct CompStep {
    pub op: String,
    pub persona: String,
    pub on_failure: StepTarget,
}

#[derive(Debug, Clone)]
pub struct ParallelBranch {
    pub id: String,
    pub entry: String,
    pub steps: Vec<FlowStep>,
}

#[derive(Debug, Clone)]
pub struct JoinPolicy {
    pub on_all_success: Option<StepTarget>,
    pub on_any_failure: Option<FailureHandler>,
    pub on_all_complete: Option<StepTarget>,
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
// Fact and verdict sets
// ──────────────────────────────────────────────

/// A set of fact values keyed by fact id.
#[derive(Debug, Clone)]
pub struct FactSet(pub BTreeMap<String, Value>);

impl Default for FactSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FactSet {
    pub fn new() -> Self {
        FactSet(BTreeMap::new())
    }

    pub fn get(&self, id: &str) -> Option<&Value> {
        self.0.get(id)
    }

    pub fn insert(&mut self, id: String, value: Value) {
        self.0.insert(id, value);
    }
}

/// A set of produced verdicts.
#[derive(Debug, Clone)]
pub struct VerdictSet(pub Vec<VerdictInstance>);

impl Default for VerdictSet {
    fn default() -> Self {
        Self::new()
    }
}

impl VerdictSet {
    pub fn new() -> Self {
        VerdictSet(Vec::new())
    }

    pub fn push(&mut self, verdict: VerdictInstance) {
        self.0.push(verdict);
    }

    /// Check if a verdict of the given type has been produced.
    pub fn has_verdict(&self, verdict_type: &str) -> bool {
        self.0.iter().any(|v| v.verdict_type == verdict_type)
    }

    /// Get the most recent verdict of the given type.
    pub fn get_verdict(&self, verdict_type: &str) -> Option<&VerdictInstance> {
        self.0.iter().rev().find(|v| v.verdict_type == verdict_type)
    }

    /// Serialize to JSON output format.
    pub fn to_json(&self) -> serde_json::Value {
        let verdicts: Vec<serde_json::Value> = self
            .0
            .iter()
            .map(|v| {
                serde_json::json!({
                    "type": v.verdict_type,
                    "payload": value_to_json(&v.payload),
                    "provenance": {
                        "rule": v.provenance.rule_id,
                        "stratum": v.provenance.stratum,
                        "facts_used": v.provenance.facts_used,
                        "verdicts_used": v.provenance.verdicts_used,
                    }
                })
            })
            .collect();
        serde_json::json!({ "verdicts": verdicts })
    }
}

/// A single verdict instance with provenance.
#[derive(Debug, Clone)]
pub struct VerdictInstance {
    pub verdict_type: String,
    pub payload: Value,
    pub provenance: crate::provenance::VerdictProvenance,
}

/// Convert a runtime Value to JSON for output.
pub fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Bool(b) => serde_json::json!({ "kind": "bool_value", "value": b }),
        Value::Int(i) => serde_json::json!({ "kind": "int_value", "value": i }),
        Value::Decimal(d) => serde_json::json!({ "kind": "decimal_value", "value": d.to_string() }),
        Value::Text(t) => serde_json::json!({ "kind": "text_value", "value": t }),
        Value::Date(d) => serde_json::json!({ "kind": "date_value", "value": d }),
        Value::DateTime(dt) => serde_json::json!({ "kind": "datetime_value", "value": dt }),
        Value::Money { amount, currency } => serde_json::json!({
            "kind": "money_value",
            "amount": amount.to_string(),
            "currency": currency,
        }),
        Value::Duration { value, unit } => serde_json::json!({
            "kind": "duration_value",
            "value": value,
            "unit": unit,
        }),
        Value::Enum(e) => serde_json::json!({ "kind": "enum_value", "value": e }),
        Value::Record(fields) => {
            let mut map = serde_json::Map::new();
            map.insert("kind".to_string(), serde_json::json!("record_value"));
            let mut fields_map = serde_json::Map::new();
            for (k, v) in fields {
                fields_map.insert(k.clone(), value_to_json(v));
            }
            map.insert("fields".to_string(), serde_json::Value::Object(fields_map));
            serde_json::Value::Object(map)
        }
        Value::List(items) => {
            let arr: Vec<serde_json::Value> = items.iter().map(value_to_json).collect();
            serde_json::json!({ "kind": "list_value", "elements": arr })
        }
        Value::TaggedUnion { tag, payload } => serde_json::json!({
            "kind": "tagged_union_value",
            "tag": tag,
            "payload": value_to_json(payload),
        }),
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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
}
