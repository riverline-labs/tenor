//! Runtime value types and JSON parsing helpers.

use rust_decimal::Decimal;
use std::collections::BTreeMap;

use super::{EvalError, TypeSpec};

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
// Interchange JSON parsing helpers
// ──────────────────────────────────────────────

pub(crate) fn get_str(obj: &serde_json::Value, field: &str) -> Result<String, EvalError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| EvalError::DeserializeError {
            message: format!("missing string field '{}'", field),
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
            // Handle two formats:
            // 1. Plain string: "200.75"
            // 2. Structured decimal_value: {"kind": "decimal_value", "value": "200.75", ...}
            let s = if let Some(plain) = v.as_str() {
                plain.to_string()
            } else if let Some(inner) = v.get("value").and_then(|val| val.as_str()) {
                inner.to_string()
            } else {
                return Err(EvalError::DeserializeError {
                    message: "Decimal must be a string or structured decimal_value".to_string(),
                });
            };
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

/// Parse a literal value from interchange JSON.
pub(crate) fn parse_literal_value(
    v: &serde_json::Value,
    type_spec: &TypeSpec,
) -> Result<Value, EvalError> {
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

/// Infer a Value and TypeSpec from a JSON literal when the "type" field is absent.
pub(crate) fn infer_literal(v: &serde_json::Value) -> Result<(Value, TypeSpec), EvalError> {
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
