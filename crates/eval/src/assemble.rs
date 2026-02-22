//! FactSet assembly from facts.json with type checking.
//!
//! Implements spec Section 5.2: assemble_facts(contract, external_inputs).
//! For each declared fact:
//! - If present in facts_json: parse and type-check against declared type
//! - If missing with default: use default value
//! - If missing without default: return MissingFact error

use crate::types::{parse_plain_value, Contract, EvalError, FactSet, TypeSpec, Value};

/// Assemble a FactSet from a contract and a facts JSON object.
///
/// The facts_json is expected to be a JSON object mapping fact IDs to values.
/// Extra facts not declared in the contract are silently ignored.
pub fn assemble_facts(
    contract: &Contract,
    facts_json: &serde_json::Value,
) -> Result<FactSet, EvalError> {
    let facts_obj = facts_json
        .as_object()
        .ok_or_else(|| EvalError::DeserializeError {
            message: "facts must be a JSON object".to_string(),
        })?;

    let mut fact_set = FactSet::new();

    for decl in &contract.facts {
        if let Some(fact_val) = facts_obj.get(&decl.id) {
            // Fact provided -- parse and type-check
            let value = parse_and_typecheck(&decl.id, fact_val, &decl.fact_type)?;
            fact_set.insert(decl.id.clone(), value);
        } else if let Some(ref default) = decl.default {
            // Fact not provided but has default
            fact_set.insert(decl.id.clone(), default.clone());
        } else {
            // Fact not provided and no default -- error
            return Err(EvalError::MissingFact {
                fact_id: decl.id.clone(),
            });
        }
    }

    Ok(fact_set)
}

/// Parse a JSON value and type-check it against the declared type.
fn parse_and_typecheck(
    fact_id: &str,
    value: &serde_json::Value,
    type_spec: &TypeSpec,
) -> Result<Value, EvalError> {
    let parsed = parse_plain_value(value, type_spec).map_err(|_| EvalError::TypeMismatch {
        fact_id: fact_id.to_string(),
        expected: type_spec.base.clone(),
        got: json_type_name(value).to_string(),
    })?;

    // Additional type-specific validation
    validate_value(fact_id, &parsed, type_spec)?;

    Ok(parsed)
}

/// Valid Duration unit values per the spec DurationUnit enum.
const VALID_DURATION_UNITS: &[&str] = &["seconds", "minutes", "hours", "days"];

/// Check if a string matches ISO 8601 date format (YYYY-MM-DD).
///
/// Simple format check: 4 digits, dash, 2 digits, dash, 2 digits.
/// Does NOT validate actual date correctness (e.g., month 13 would pass).
pub fn validate_date_format(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    // YYYY-MM-DD
    bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

/// Check if a string matches ISO 8601 DateTime format (YYYY-MM-DDT...).
///
/// Validates the date prefix is ISO 8601 and that a 'T' separator follows.
pub fn validate_datetime_format(s: &str) -> bool {
    if s.len() < 11 {
        return false;
    }
    validate_date_format(&s[..10]) && s.as_bytes()[10] == b'T'
}

/// Validate a parsed value against additional type constraints.
fn validate_value(fact_id: &str, value: &Value, type_spec: &TypeSpec) -> Result<(), EvalError> {
    match value {
        Value::Enum(s) => {
            if let Some(ref variants) = type_spec.values {
                if !variants.contains(s) {
                    return Err(EvalError::InvalidEnum {
                        fact_id: fact_id.to_string(),
                        value: s.clone(),
                        variants: variants.clone(),
                    });
                }
            }
        }
        Value::List(items) => {
            if let Some(max) = type_spec.max {
                if items.len() > max as usize {
                    return Err(EvalError::ListOverflow {
                        fact_id: fact_id.to_string(),
                        max: max as u32,
                        actual: items.len(),
                    });
                }
            }
        }
        Value::Int(i) => {
            if let (Some(min), Some(max)) = (type_spec.min, type_spec.max) {
                if *i < min || *i > max {
                    return Err(EvalError::TypeMismatch {
                        fact_id: fact_id.to_string(),
                        expected: format!("Int({}, {})", min, max),
                        got: format!("{}", i),
                    });
                }
            }
        }
        Value::Text(s) => {
            if let Some(max_len) = type_spec.max_length {
                if s.len() > max_len as usize {
                    return Err(EvalError::TypeMismatch {
                        fact_id: fact_id.to_string(),
                        expected: format!("Text(max_length={})", max_len),
                        got: format!("text of length {}", s.len()),
                    });
                }
            }
        }
        Value::Date(s) => {
            if !validate_date_format(s) {
                return Err(EvalError::TypeError {
                    message: format!(
                        "fact '{}': invalid Date format '{}', expected ISO 8601 (YYYY-MM-DD)",
                        fact_id, s
                    ),
                });
            }
        }
        Value::DateTime(s) => {
            if !validate_datetime_format(s) {
                return Err(EvalError::TypeError {
                    message: format!(
                        "fact '{}': invalid DateTime format '{}', expected ISO 8601 (YYYY-MM-DDT...)",
                        fact_id, s
                    ),
                });
            }
        }
        Value::Duration { unit, .. } => {
            if !VALID_DURATION_UNITS.contains(&unit.as_str()) {
                return Err(EvalError::TypeError {
                    message: format!(
                        "fact '{}': invalid Duration unit '{}', expected one of: {}",
                        fact_id,
                        unit,
                        VALID_DURATION_UNITS.join(", ")
                    ),
                });
            }
        }
        _ => {}
    }
    Ok(())
}

/// Return a descriptive type name for a JSON value (for error messages).
fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FactDecl;
    use std::str::FromStr;

    fn make_contract(facts: Vec<FactDecl>) -> Contract {
        Contract {
            facts,
            entities: vec![],
            rules: vec![],
            operations: vec![],
            flows: vec![],
            personas: vec![],
        }
    }

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

    fn int_type(min: i64, max: i64) -> TypeSpec {
        TypeSpec {
            base: "Int".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: Some(min),
            max: Some(max),
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    fn money_type(currency: &str) -> TypeSpec {
        TypeSpec {
            base: "Money".to_string(),
            precision: None,
            scale: None,
            currency: Some(currency.to_string()),
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

    fn enum_type(values: Vec<&str>) -> TypeSpec {
        TypeSpec {
            base: "Enum".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: Some(values.into_iter().map(|s| s.to_string()).collect()),
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    fn text_type(max_length: u32) -> TypeSpec {
        TypeSpec {
            base: "Text".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: Some(max_length),
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    #[test]
    fn assemble_basic_bool_fact() {
        let contract = make_contract(vec![FactDecl {
            id: "is_active".to_string(),
            fact_type: bool_type(),
            default: None,
        }]);
        let facts = serde_json::json!({ "is_active": true });
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(fs.get("is_active"), Some(&Value::Bool(true)));
    }

    #[test]
    fn assemble_missing_fact_with_default() {
        let contract = make_contract(vec![FactDecl {
            id: "flag".to_string(),
            fact_type: bool_type(),
            default: Some(Value::Bool(false)),
        }]);
        let facts = serde_json::json!({});
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(fs.get("flag"), Some(&Value::Bool(false)));
    }

    #[test]
    fn assemble_missing_fact_no_default_error() {
        let contract = make_contract(vec![FactDecl {
            id: "required_fact".to_string(),
            fact_type: bool_type(),
            default: None,
        }]);
        let facts = serde_json::json!({});
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
        if let Err(EvalError::MissingFact { fact_id }) = result {
            assert_eq!(fact_id, "required_fact");
        } else {
            panic!("expected MissingFact error");
        }
    }

    #[test]
    fn assemble_type_mismatch_error() {
        let contract = make_contract(vec![FactDecl {
            id: "count".to_string(),
            fact_type: int_type(0, 100),
            default: None,
        }]);
        let facts = serde_json::json!({ "count": "not_a_number" });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_int_out_of_range() {
        let contract = make_contract(vec![FactDecl {
            id: "count".to_string(),
            fact_type: int_type(0, 100),
            default: None,
        }]);
        let facts = serde_json::json!({ "count": 200 });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_enum_valid() {
        let contract = make_contract(vec![FactDecl {
            id: "status".to_string(),
            fact_type: enum_type(vec!["pending", "confirmed", "failed"]),
            default: None,
        }]);
        let facts = serde_json::json!({ "status": "confirmed" });
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(
            fs.get("status"),
            Some(&Value::Enum("confirmed".to_string()))
        );
    }

    #[test]
    fn assemble_enum_invalid() {
        let contract = make_contract(vec![FactDecl {
            id: "status".to_string(),
            fact_type: enum_type(vec!["pending", "confirmed", "failed"]),
            default: None,
        }]);
        let facts = serde_json::json!({ "status": "unknown" });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
        if let Err(EvalError::InvalidEnum { fact_id, value, .. }) = result {
            assert_eq!(fact_id, "status");
            assert_eq!(value, "unknown");
        } else {
            panic!("expected InvalidEnum error");
        }
    }

    #[test]
    fn assemble_money_fact() {
        let contract = make_contract(vec![FactDecl {
            id: "amount".to_string(),
            fact_type: money_type("USD"),
            default: None,
        }]);
        let facts = serde_json::json!({ "amount": { "amount": "5000.00", "currency": "USD" } });
        let fs = assemble_facts(&contract, &facts).unwrap();
        match fs.get("amount") {
            Some(Value::Money { amount, currency }) => {
                assert_eq!(*amount, rust_decimal::Decimal::from_str("5000.00").unwrap());
                assert_eq!(currency, "USD");
            }
            other => panic!("expected Money value, got {:?}", other),
        }
    }

    #[test]
    fn assemble_text_too_long() {
        let contract = make_contract(vec![FactDecl {
            id: "name".to_string(),
            fact_type: text_type(5),
            default: None,
        }]);
        let facts = serde_json::json!({ "name": "way_too_long_string" });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_extra_facts_ignored() {
        let contract = make_contract(vec![FactDecl {
            id: "x".to_string(),
            fact_type: bool_type(),
            default: None,
        }]);
        let facts = serde_json::json!({ "x": true, "y": 42, "z": "extra" });
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(fs.get("x"), Some(&Value::Bool(true)));
        assert_eq!(fs.get("y"), None); // extra fact not in FactSet
    }

    #[test]
    fn assemble_list_fact() {
        use std::collections::BTreeMap;
        let elem_type = TypeSpec {
            base: "Record".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: Some({
                let mut m = BTreeMap::new();
                m.insert("valid".to_string(), bool_type());
                m
            }),
            element_type: None,
            unit: None,
            variants: None,
        };
        let list_type = TypeSpec {
            base: "List".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: Some(10),
            max_length: None,
            values: None,
            fields: None,
            element_type: Some(Box::new(elem_type)),
            unit: None,
            variants: None,
        };
        let contract = make_contract(vec![FactDecl {
            id: "items".to_string(),
            fact_type: list_type,
            default: None,
        }]);
        let facts = serde_json::json!({
            "items": [
                { "valid": true },
                { "valid": false }
            ]
        });
        let fs = assemble_facts(&contract, &facts).unwrap();
        if let Some(Value::List(items)) = fs.get("items") {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected List value");
        }
    }

    // ──────────────────────────────────────
    // C8: Date format validation tests (B4 fix)
    // ──────────────────────────────────────

    #[test]
    fn validate_date_format_valid_iso() {
        assert!(super::validate_date_format("2024-01-15"));
        assert!(super::validate_date_format("2000-12-31"));
        assert!(super::validate_date_format("1999-06-01"));
    }

    #[test]
    fn validate_date_format_invalid() {
        assert!(!super::validate_date_format("1/1/2024"));
        assert!(!super::validate_date_format("24-01-15"));
        assert!(!super::validate_date_format(""));
        assert!(!super::validate_date_format("2024/01/15"));
        assert!(!super::validate_date_format("20240115"));
    }

    #[test]
    fn validate_datetime_format_valid() {
        assert!(super::validate_datetime_format("2024-01-15T10:30:00Z"));
        assert!(super::validate_datetime_format("2024-01-15T00:00:00"));
    }

    #[test]
    fn validate_datetime_format_invalid() {
        assert!(!super::validate_datetime_format("2024-01-15"));
        assert!(!super::validate_datetime_format("2024-01-15 10:30:00"));
        assert!(!super::validate_datetime_format(""));
    }

    #[test]
    fn assemble_date_fact_valid() {
        let date_type = TypeSpec {
            base: "Date".to_string(),
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
        let contract = make_contract(vec![FactDecl {
            id: "start_date".to_string(),
            fact_type: date_type,
            default: None,
        }]);
        let facts = serde_json::json!({ "start_date": "2024-01-15" });
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(
            fs.get("start_date"),
            Some(&Value::Date("2024-01-15".to_string()))
        );
    }

    #[test]
    fn assemble_date_fact_invalid_format_rejected() {
        let date_type = TypeSpec {
            base: "Date".to_string(),
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
        let contract = make_contract(vec![FactDecl {
            id: "start_date".to_string(),
            fact_type: date_type,
            default: None,
        }]);
        let facts = serde_json::json!({ "start_date": "1/1/2024" });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
    }

    // ──────────────────────────────────────
    // Duration unit validation tests (T8 fix)
    // ──────────────────────────────────────

    fn duration_type(unit: &str) -> TypeSpec {
        TypeSpec {
            base: "Duration".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: Some(unit.to_string()),
            variants: None,
        }
    }

    #[test]
    fn assemble_duration_valid_unit() {
        let contract = make_contract(vec![FactDecl {
            id: "timeout".to_string(),
            fact_type: duration_type("seconds"),
            default: None,
        }]);
        let facts = serde_json::json!({ "timeout": { "value": 30, "unit": "seconds" } });
        let fs = assemble_facts(&contract, &facts).unwrap();
        assert_eq!(
            fs.get("timeout"),
            Some(&Value::Duration {
                value: 30,
                unit: "seconds".to_string()
            })
        );
    }

    #[test]
    fn assemble_duration_invalid_unit_rejected() {
        let contract = make_contract(vec![FactDecl {
            id: "timeout".to_string(),
            fact_type: duration_type("weeks"),
            default: None,
        }]);
        let facts = serde_json::json!({ "timeout": { "value": 1, "unit": "weeks" } });
        let result = assemble_facts(&contract, &facts);
        assert!(result.is_err());
        match result {
            Err(EvalError::TypeError { message }) => {
                assert!(
                    message.contains("invalid Duration unit"),
                    "error should mention invalid unit, got: {}",
                    message
                );
            }
            other => panic!("expected TypeError, got {:?}", other),
        }
    }
}
