//! Interchange JSON deserialization into codegen-internal typed structs.
//!
//! The codegen crate consumes interchange JSON (same pattern as tenor-eval
//! and tenor-analyze), not the raw DSL AST. This module extracts all
//! construct types from the interchange bundle into Rust structs suitable
//! for TypeScript code generation.

use std::collections::BTreeMap;
use std::fmt;

/// Error type for code generation operations.
#[derive(Debug, Clone)]
pub enum CodegenError {
    /// The bundle JSON is invalid or missing required fields.
    InvalidBundle(String),
    /// An I/O error occurred while writing generated files.
    IoError(String),
    /// An error occurred during code emission.
    EmitError(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::InvalidBundle(msg) => write!(f, "invalid bundle: {}", msg),
            CodegenError::IoError(msg) => write!(f, "I/O error: {}", msg),
            CodegenError::EmitError(msg) => write!(f, "emit error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Type information from interchange JSON BaseType.
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Bool,
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    Money {
        currency: Option<String>,
    },
    Text {
        max_length: Option<u32>,
    },
    Date,
    DateTime,
    Duration {
        unit: Option<String>,
        min: Option<i64>,
        max: Option<i64>,
    },
    Enum {
        values: Vec<String>,
    },
    List {
        element_type: Box<TypeInfo>,
        max: Option<u32>,
    },
    Record {
        fields: BTreeMap<String, TypeInfo>,
    },
    TaggedUnion {
        variants: BTreeMap<String, TypeInfo>,
    },
}

/// A Fact construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenFact {
    pub id: String,
    pub type_info: TypeInfo,
}

/// An Entity construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenEntity {
    pub id: String,
    pub states: Vec<String>,
}

/// An Operation construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenOperation {
    pub id: String,
    pub allowed_personas: Vec<String>,
}

/// A Rule construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenRule {
    pub id: String,
    pub verdict_type: String,
}

/// A Flow construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenFlow {
    pub id: String,
}

/// A Persona construct extracted from interchange JSON.
#[derive(Debug, Clone)]
pub struct CodegenPersona {
    pub id: String,
}

/// All constructs extracted from an interchange bundle, ready for code generation.
#[derive(Debug, Clone)]
pub struct CodegenBundle {
    pub id: String,
    pub facts: Vec<CodegenFact>,
    pub entities: Vec<CodegenEntity>,
    pub operations: Vec<CodegenOperation>,
    pub rules: Vec<CodegenRule>,
    pub flows: Vec<CodegenFlow>,
    pub personas: Vec<CodegenPersona>,
}

impl CodegenBundle {
    /// Deserialize an interchange JSON bundle into typed codegen structs.
    pub fn from_interchange(bundle: &serde_json::Value) -> Result<Self, CodegenError> {
        let id = bundle
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CodegenError::InvalidBundle("missing 'id' field".to_string()))?
            .to_string();

        let constructs = bundle
            .get("constructs")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                CodegenError::InvalidBundle("missing or invalid 'constructs' array".to_string())
            })?;

        let mut facts = Vec::new();
        let mut entities = Vec::new();
        let mut operations = Vec::new();
        let mut rules = Vec::new();
        let mut flows = Vec::new();
        let mut personas = Vec::new();

        for construct in constructs {
            let kind = construct.get("kind").and_then(|k| k.as_str()).unwrap_or("");

            match kind {
                "Fact" => facts.push(parse_fact(construct)?),
                "Entity" => entities.push(parse_entity(construct)?),
                "Operation" => operations.push(parse_operation(construct)?),
                "Rule" => rules.push(parse_rule(construct)?),
                "Flow" => flows.push(parse_flow(construct)?),
                "Persona" => personas.push(parse_persona(construct)?),
                "System" => {} // System constructs are not relevant for TypeScript codegen
                _ => {}        // Skip unknown kinds for forward compatibility
            }
        }

        // Extract personas from operation allowed_personas if none found
        if personas.is_empty() {
            let mut seen = std::collections::BTreeSet::new();
            for op in &operations {
                for p in &op.allowed_personas {
                    if seen.insert(p.clone()) {
                        personas.push(CodegenPersona { id: p.clone() });
                    }
                }
            }
        }

        Ok(CodegenBundle {
            id,
            facts,
            entities,
            operations,
            rules,
            flows,
            personas,
        })
    }
}

/// Parse a TypeInfo from interchange JSON BaseType object.
fn parse_type_info(v: &serde_json::Value) -> Result<TypeInfo, CodegenError> {
    let base = v
        .get("base")
        .and_then(|b| b.as_str())
        .ok_or_else(|| CodegenError::InvalidBundle("type missing 'base' field".to_string()))?;

    match base {
        "Bool" => Ok(TypeInfo::Bool),
        "Int" => Ok(TypeInfo::Int {
            min: v.get("min").and_then(|v| v.as_i64()),
            max: v.get("max").and_then(|v| v.as_i64()),
        }),
        "Decimal" => Ok(TypeInfo::Decimal {
            precision: v
                .get("precision")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            scale: v.get("scale").and_then(|v| v.as_u64()).map(|v| v as u32),
        }),
        "Money" => Ok(TypeInfo::Money {
            currency: v
                .get("currency")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }),
        "Text" => Ok(TypeInfo::Text {
            max_length: v
                .get("max_length")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        }),
        "Date" => Ok(TypeInfo::Date),
        "DateTime" => Ok(TypeInfo::DateTime),
        "Duration" => Ok(TypeInfo::Duration {
            unit: v
                .get("unit")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            min: v.get("min").and_then(|v| v.as_i64()),
            max: v.get("max").and_then(|v| v.as_i64()),
        }),
        "Enum" => {
            let values = v
                .get("values")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            Ok(TypeInfo::Enum { values })
        }
        "List" => {
            let element_type = v
                .get("element_type")
                .ok_or_else(|| {
                    CodegenError::InvalidBundle("List type missing 'element_type'".to_string())
                })
                .and_then(parse_type_info)?;
            let max = v.get("max").and_then(|v| v.as_u64()).map(|v| v as u32);
            Ok(TypeInfo::List {
                element_type: Box::new(element_type),
                max,
            })
        }
        "Record" => {
            let fields = if let Some(fields_val) = v.get("fields") {
                if let Some(fields_obj) = fields_val.as_object() {
                    let mut map = BTreeMap::new();
                    for (k, fv) in fields_obj {
                        map.insert(k.clone(), parse_type_info(fv)?);
                    }
                    map
                } else {
                    BTreeMap::new()
                }
            } else {
                BTreeMap::new()
            };
            Ok(TypeInfo::Record { fields })
        }
        "TaggedUnion" => {
            let variants = if let Some(variants_val) = v.get("variants") {
                if let Some(variants_obj) = variants_val.as_object() {
                    let mut map = BTreeMap::new();
                    for (k, vv) in variants_obj {
                        map.insert(k.clone(), parse_type_info(vv)?);
                    }
                    map
                } else {
                    BTreeMap::new()
                }
            } else {
                BTreeMap::new()
            };
            Ok(TypeInfo::TaggedUnion { variants })
        }
        other => Err(CodegenError::InvalidBundle(format!(
            "unsupported type base: {}",
            other
        ))),
    }
}

fn parse_fact(obj: &serde_json::Value) -> Result<CodegenFact, CodegenError> {
    let id = required_str(obj, "id")?;
    let type_val = obj.get("type").ok_or_else(|| {
        CodegenError::InvalidBundle(format!("Fact '{}' missing 'type' field", id))
    })?;
    let type_info = parse_type_info(type_val)?;
    Ok(CodegenFact { id, type_info })
}

fn parse_entity(obj: &serde_json::Value) -> Result<CodegenEntity, CodegenError> {
    let id = required_str(obj, "id")?;
    let states = obj
        .get("states")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(CodegenEntity { id, states })
}

fn parse_operation(obj: &serde_json::Value) -> Result<CodegenOperation, CodegenError> {
    let id = required_str(obj, "id")?;
    let allowed_personas = obj
        .get("allowed_personas")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(CodegenOperation {
        id,
        allowed_personas,
    })
}

fn parse_rule(obj: &serde_json::Value) -> Result<CodegenRule, CodegenError> {
    let id = required_str(obj, "id")?;
    let verdict_type = obj
        .get("body")
        .and_then(|b| b.get("produce"))
        .and_then(|p| p.get("verdict_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CodegenError::InvalidBundle(format!("Rule '{}' missing body.produce.verdict_type", id))
        })?
        .to_string();
    Ok(CodegenRule { id, verdict_type })
}

fn parse_flow(obj: &serde_json::Value) -> Result<CodegenFlow, CodegenError> {
    let id = required_str(obj, "id")?;
    Ok(CodegenFlow { id })
}

fn parse_persona(obj: &serde_json::Value) -> Result<CodegenPersona, CodegenError> {
    let id = required_str(obj, "id")?;
    Ok(CodegenPersona { id })
}

fn required_str(obj: &serde_json::Value, field: &str) -> Result<String, CodegenError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CodegenError::InvalidBundle(format!("missing '{}' field", field)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bundle(id: &str, constructs: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.1.0",
            "constructs": constructs
        })
    }

    #[test]
    fn test_parse_empty_bundle() {
        let bundle = make_bundle("test", vec![]);
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.id, "test");
        assert!(result.facts.is_empty());
        assert!(result.entities.is_empty());
    }

    #[test]
    fn test_parse_fact_bool() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "is_active",
                "kind": "Fact",
                "type": {"base": "Bool"},
                "source": {"field": "active", "system": "s"},
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.facts.len(), 1);
        assert_eq!(result.facts[0].id, "is_active");
        assert!(matches!(result.facts[0].type_info, TypeInfo::Bool));
    }

    #[test]
    fn test_parse_fact_money() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "amount",
                "kind": "Fact",
                "type": {"base": "Money", "currency": "USD"},
                "source": {"field": "amt", "system": "s"},
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.facts.len(), 1);
        match &result.facts[0].type_info {
            TypeInfo::Money { currency } => {
                assert_eq!(currency, &Some("USD".to_string()));
            }
            _ => panic!("expected Money type"),
        }
    }

    #[test]
    fn test_parse_entity() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "Order",
                "kind": "Entity",
                "initial": "draft",
                "states": ["draft", "submitted", "approved"],
                "transitions": [{"from": "draft", "to": "submitted"}],
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].id, "Order");
        assert_eq!(
            result.entities[0].states,
            vec!["draft", "submitted", "approved"]
        );
    }

    #[test]
    fn test_parse_operation() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "approve_order",
                "kind": "Operation",
                "allowed_personas": ["reviewer", "admin"],
                "effects": [],
                "precondition": null,
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.operations[0].id, "approve_order");
        assert_eq!(
            result.operations[0].allowed_personas,
            vec!["reviewer", "admin"]
        );
    }

    #[test]
    fn test_parse_rule_verdict_type() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "check",
                "kind": "Rule",
                "stratum": 0,
                "body": {
                    "when": {"fact_ref": "is_active"},
                    "produce": {
                        "verdict_type": "account_active",
                        "payload": {"type": {"base": "Bool"}, "value": true}
                    }
                },
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.rules.len(), 1);
        assert_eq!(result.rules[0].verdict_type, "account_active");
    }

    #[test]
    fn test_parse_list_type() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "items",
                "kind": "Fact",
                "type": {
                    "base": "List",
                    "element_type": {
                        "base": "Record",
                        "fields": {
                            "id": {"base": "Text", "max_length": 64},
                            "amount": {"base": "Money", "currency": "USD"}
                        }
                    },
                    "max": 100
                },
                "source": {"field": "items", "system": "s"},
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.facts.len(), 1);
        match &result.facts[0].type_info {
            TypeInfo::List { element_type, max } => {
                assert_eq!(*max, Some(100));
                match element_type.as_ref() {
                    TypeInfo::Record { fields } => {
                        assert_eq!(fields.len(), 2);
                        assert!(fields.contains_key("id"));
                        assert!(fields.contains_key("amount"));
                    }
                    _ => panic!("expected Record element type"),
                }
            }
            _ => panic!("expected List type"),
        }
    }

    #[test]
    fn test_missing_constructs() {
        let bundle = serde_json::json!({"id": "test", "kind": "Bundle"});
        let result = CodegenBundle::from_interchange(&bundle);
        assert!(result.is_err());
    }

    #[test]
    fn test_personas_extracted_from_operations() {
        let bundle = make_bundle(
            "test",
            vec![serde_json::json!({
                "id": "op1",
                "kind": "Operation",
                "allowed_personas": ["admin", "user"],
                "effects": [],
                "precondition": null,
                "provenance": {"file": "t.tenor", "line": 1},
                "tenor": "1.0"
            })],
        );
        let result = CodegenBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.personas.len(), 2);
        assert_eq!(result.personas[0].id, "admin");
        assert_eq!(result.personas[1].id, "user");
    }
}
