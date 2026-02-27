//! Deserialization from interchange JSON bundles into typed structs.
//!
//! The main entry point is [`from_interchange`], which takes a
//! `&serde_json::Value` and produces an [`InterchangeBundle`].

use crate::types::*;
use std::fmt;

/// Errors during interchange JSON deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterchangeError {
    /// The bundle is missing a required top-level field.
    MissingField { field: String },
    /// A construct is missing a required field.
    ConstructError {
        kind: String,
        id: String,
        message: String,
    },
    /// The bundle structure is invalid.
    InvalidBundle(String),
}

impl fmt::Display for InterchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterchangeError::MissingField { field } => {
                write!(f, "bundle missing required field: '{}'", field)
            }
            InterchangeError::ConstructError { kind, id, message } => {
                write!(f, "{} '{}': {}", kind, id, message)
            }
            InterchangeError::InvalidBundle(msg) => {
                write!(f, "invalid bundle: {}", msg)
            }
        }
    }
}

impl std::error::Error for InterchangeError {}

/// Deserialize an interchange JSON bundle into typed structs.
///
/// Walks the `constructs` array and dispatches on the `kind` field.
/// Unknown construct kinds are silently skipped for forward compatibility.
pub fn from_interchange(bundle: &serde_json::Value) -> Result<InterchangeBundle, InterchangeError> {
    let id = bundle
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| InterchangeError::MissingField {
            field: "id".to_string(),
        })?
        .to_string();

    let tenor = bundle
        .get("tenor")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tenor_version = bundle
        .get("tenor_version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let constructs_arr = bundle
        .get("constructs")
        .and_then(|c| c.as_array())
        .ok_or_else(|| InterchangeError::MissingField {
            field: "constructs".to_string(),
        })?;

    let mut constructs = Vec::with_capacity(constructs_arr.len());

    for obj in constructs_arr {
        let kind = obj.get("kind").and_then(|k| k.as_str()).unwrap_or("");

        let construct = match kind {
            "Fact" => Some(InterchangeConstruct::Fact(parse_fact(obj)?)),
            "Entity" => Some(InterchangeConstruct::Entity(parse_entity(obj)?)),
            "Rule" => Some(InterchangeConstruct::Rule(parse_rule(obj)?)),
            "Operation" => Some(InterchangeConstruct::Operation(parse_operation(obj)?)),
            "Flow" => Some(InterchangeConstruct::Flow(parse_flow(obj)?)),
            "Persona" => Some(InterchangeConstruct::Persona(parse_persona(obj)?)),
            "Source" => Some(InterchangeConstruct::Source(parse_source(obj)?)),
            "System" => Some(InterchangeConstruct::System(parse_system(obj)?)),
            "TypeDecl" => Some(InterchangeConstruct::TypeDecl(parse_type_decl(obj)?)),
            _ => None, // Skip unknown kinds for forward compatibility
        };

        if let Some(c) = construct {
            constructs.push(c);
        }
    }

    Ok(InterchangeBundle {
        id,
        tenor,
        tenor_version,
        constructs,
    })
}

// ── Parsing helpers ─────────────────────────────────────────────────

fn required_str(obj: &serde_json::Value, field: &str) -> Result<String, InterchangeError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| InterchangeError::InvalidBundle(format!("missing '{}' field", field)))
}

fn parse_provenance(obj: &serde_json::Value) -> Option<Provenance> {
    let prov = obj.get("provenance")?;
    let file = prov.get("file")?.as_str()?.to_string();
    let line = prov.get("line")?.as_u64()?;
    Some(Provenance { file, line })
}

fn parse_tenor(obj: &serde_json::Value) -> Option<String> {
    obj.get("tenor")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn parse_fact(obj: &serde_json::Value) -> Result<FactConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;
    let fact_type = obj.get("type").cloned().unwrap_or(serde_json::Value::Null);
    let source = obj.get("source").cloned();
    let default = obj.get("default").cloned();
    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(FactConstruct {
        id,
        fact_type,
        source,
        default,
        provenance,
        tenor,
    })
}

fn parse_entity(obj: &serde_json::Value) -> Result<EntityConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;

    let states = obj
        .get("states")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| InterchangeError::ConstructError {
            kind: "Entity".to_string(),
            id: id.clone(),
            message: "missing 'states' array".to_string(),
        })?;

    let initial = obj
        .get("initial")
        .and_then(|v| v.as_str())
        .ok_or_else(|| InterchangeError::ConstructError {
            kind: "Entity".to_string(),
            id: id.clone(),
            message: "missing 'initial' field".to_string(),
        })?
        .to_string();

    let transitions = obj
        .get("transitions")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let from = t.get("from")?.as_str()?.to_string();
                    let to = t.get("to")?.as_str()?.to_string();
                    Some(Transition { from, to })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let parent = obj
        .get("parent")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(EntityConstruct {
        id,
        states,
        initial,
        transitions,
        parent,
        provenance,
        tenor,
    })
}

fn parse_rule(obj: &serde_json::Value) -> Result<RuleConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;

    let stratum = obj.get("stratum").and_then(|s| s.as_u64()).ok_or_else(|| {
        InterchangeError::ConstructError {
            kind: "Rule".to_string(),
            id: id.clone(),
            message: "missing 'stratum' field".to_string(),
        }
    })?;

    let body = obj
        .get("body")
        .cloned()
        .ok_or_else(|| InterchangeError::ConstructError {
            kind: "Rule".to_string(),
            id: id.clone(),
            message: "missing 'body' field".to_string(),
        })?;

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(RuleConstruct {
        id,
        stratum,
        body,
        provenance,
        tenor,
    })
}

fn parse_operation(obj: &serde_json::Value) -> Result<OperationConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;

    let allowed_personas = obj
        .get("allowed_personas")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let precondition =
        obj.get("precondition")
            .and_then(|p| if p.is_null() { None } else { Some(p.clone()) });

    let effects = obj
        .get("effects")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let entity_id = e.get("entity_id")?.as_str()?.to_string();
                    let from = e.get("from")?.as_str()?.to_string();
                    let to = e.get("to")?.as_str()?.to_string();
                    let outcome = e
                        .get("outcome")
                        .and_then(|o| o.as_str())
                        .map(|s| s.to_string());
                    Some(Effect {
                        entity_id,
                        from,
                        to,
                        outcome,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let outcomes = obj
        .get("outcomes")
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let error_contract =
        obj.get("error_contract")
            .and_then(|e| if e.is_null() { None } else { Some(e.clone()) });

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(OperationConstruct {
        id,
        allowed_personas,
        precondition,
        effects,
        outcomes,
        error_contract,
        provenance,
        tenor,
    })
}

fn parse_flow(obj: &serde_json::Value) -> Result<FlowConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;

    let entry = obj
        .get("entry")
        .and_then(|v| v.as_str())
        .ok_or_else(|| InterchangeError::ConstructError {
            kind: "Flow".to_string(),
            id: id.clone(),
            message: "missing 'entry' field".to_string(),
        })?
        .to_string();

    let steps = obj
        .get("steps")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();

    let snapshot = obj
        .get("snapshot")
        .and_then(|s| s.as_str())
        .unwrap_or("at_initiation")
        .to_string();

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(FlowConstruct {
        id,
        entry,
        steps,
        snapshot,
        provenance,
        tenor,
    })
}

fn parse_persona(obj: &serde_json::Value) -> Result<PersonaConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;
    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(PersonaConstruct {
        id,
        provenance,
        tenor,
    })
}

fn parse_source(obj: &serde_json::Value) -> Result<SourceConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;
    let protocol = required_str(obj, "protocol")?;

    let fields = obj
        .get("fields")
        .and_then(|f| f.as_object())
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(SourceConstruct {
        id,
        protocol,
        fields,
        description,
        provenance,
        tenor,
    })
}

fn parse_system(obj: &serde_json::Value) -> Result<SystemConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;

    let members = obj
        .get("members")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let mid = m.get("id")?.as_str()?.to_string();
                    let path = m.get("path")?.as_str()?.to_string();
                    Some(SystemMember { id: mid, path })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let shared_personas = obj
        .get("shared_personas")
        .and_then(|sp| sp.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|sp| {
                    let persona = sp.get("persona")?.as_str()?.to_string();
                    let contracts = sp
                        .get("contracts")?
                        .as_array()?
                        .iter()
                        .filter_map(|c| c.as_str().map(|s| s.to_string()))
                        .collect();
                    Some(SharedPersona { persona, contracts })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let flow_triggers = obj
        .get("triggers")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let source_contract = t.get("source_contract")?.as_str()?.to_string();
                    let source_flow = t.get("source_flow")?.as_str()?.to_string();
                    let on = t.get("on")?.as_str()?.to_string();
                    let target_contract = t.get("target_contract")?.as_str()?.to_string();
                    let target_flow = t.get("target_flow")?.as_str()?.to_string();
                    let persona = t.get("persona")?.as_str()?.to_string();
                    Some(FlowTrigger {
                        source_contract,
                        source_flow,
                        on,
                        target_contract,
                        target_flow,
                        persona,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let shared_entities = obj
        .get("shared_entities")
        .and_then(|se| se.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|se| {
                    let entity = se.get("entity")?.as_str()?.to_string();
                    let contracts = se
                        .get("contracts")?
                        .as_array()?
                        .iter()
                        .filter_map(|c| c.as_str().map(|s| s.to_string()))
                        .collect();
                    Some(SharedEntity { entity, contracts })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(SystemConstruct {
        id,
        members,
        shared_personas,
        flow_triggers,
        shared_entities,
        provenance,
        tenor,
    })
}

fn parse_type_decl(obj: &serde_json::Value) -> Result<TypeDeclConstruct, InterchangeError> {
    let id = required_str(obj, "id")?;
    let type_def = obj.get("type").cloned().unwrap_or(serde_json::Value::Null);
    let provenance = parse_provenance(obj);
    let tenor = parse_tenor(obj);

    Ok(TypeDeclConstruct {
        id,
        type_def,
        provenance,
        tenor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_bundle(constructs: Vec<serde_json::Value>) -> serde_json::Value {
        json!({
            "id": "test-bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": constructs
        })
    }

    #[test]
    fn test_empty_bundle() {
        let bundle = make_bundle(vec![]);
        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.id, "test-bundle");
        assert_eq!(result.tenor, "1.0");
        assert_eq!(result.tenor_version, "1.0.0");
        assert!(result.constructs.is_empty());
    }

    #[test]
    fn test_missing_constructs_array() {
        let bundle = json!({"id": "test", "kind": "Bundle"});
        let result = from_interchange(&bundle);
        assert!(result.is_err());
        match result.unwrap_err() {
            InterchangeError::MissingField { field } => assert_eq!(field, "constructs"),
            other => panic!("expected MissingField, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_bundle_id() {
        let bundle = json!({"constructs": []});
        let result = from_interchange(&bundle);
        assert!(result.is_err());
        match result.unwrap_err() {
            InterchangeError::MissingField { field } => assert_eq!(field, "id"),
            other => panic!("expected MissingField, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_fact() {
        let bundle = make_bundle(vec![json!({
            "id": "amount",
            "kind": "Fact",
            "type": {"base": "Decimal", "precision": 10, "scale": 2},
            "source": {"field": "amt", "system": "billing"},
            "provenance": {"file": "test.tenor", "line": 5},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Fact(f) => {
                assert_eq!(f.id, "amount");
                assert_eq!(f.fact_type["base"], "Decimal");
                assert_eq!(f.fact_type["precision"], 10);
                assert!(f.source.is_some());
                assert!(f.default.is_none());
                assert_eq!(f.provenance.as_ref().unwrap().file, "test.tenor");
                assert_eq!(f.provenance.as_ref().unwrap().line, 5);
            }
            other => panic!("expected Fact, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_entity() {
        let bundle = make_bundle(vec![json!({
            "id": "Order",
            "kind": "Entity",
            "initial": "draft",
            "states": ["draft", "submitted", "approved"],
            "transitions": [
                {"from": "draft", "to": "submitted"},
                {"from": "submitted", "to": "approved"}
            ],
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Entity(e) => {
                assert_eq!(e.id, "Order");
                assert_eq!(e.states, vec!["draft", "submitted", "approved"]);
                assert_eq!(e.initial, "draft");
                assert_eq!(e.transitions.len(), 2);
                assert_eq!(e.transitions[0].from, "draft");
                assert_eq!(e.transitions[0].to, "submitted");
                assert!(e.parent.is_none());
            }
            other => panic!("expected Entity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rule() {
        let bundle = make_bundle(vec![json!({
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
            "provenance": {"file": "test.tenor", "line": 10},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Rule(r) => {
                assert_eq!(r.id, "check_amount");
                assert_eq!(r.stratum, 0);
                assert!(r.when().is_some());
                assert_eq!(r.verdict_type(), Some("high_value"));
                assert!(r.produce_payload().is_some());
            }
            other => panic!("expected Rule, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_operation() {
        let bundle = make_bundle(vec![json!({
            "id": "approve",
            "kind": "Operation",
            "allowed_personas": ["admin", "manager"],
            "precondition": {"verdict_present": "reviewed"},
            "effects": [
                {"entity_id": "Order", "from": "pending", "to": "approved", "outcome": "success"}
            ],
            "outcomes": ["success", "rejected"],
            "error_contract": ["precondition_failed"],
            "provenance": {"file": "test.tenor", "line": 15},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Operation(op) => {
                assert_eq!(op.id, "approve");
                assert_eq!(op.allowed_personas, vec!["admin", "manager"]);
                assert!(op.precondition.is_some());
                assert_eq!(op.effects.len(), 1);
                assert_eq!(op.effects[0].entity_id, "Order");
                assert_eq!(op.effects[0].from, "pending");
                assert_eq!(op.effects[0].to, "approved");
                assert_eq!(op.effects[0].outcome, Some("success".to_string()));
                assert_eq!(op.outcomes, vec!["success", "rejected"]);
                assert!(op.error_contract.is_some());
            }
            other => panic!("expected Operation, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_operation_null_precondition() {
        let bundle = make_bundle(vec![json!({
            "id": "simple_op",
            "kind": "Operation",
            "allowed_personas": ["user"],
            "precondition": null,
            "effects": [],
            "outcomes": null,
            "error_contract": [],
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::Operation(op) => {
                assert!(op.precondition.is_none());
                assert!(op.outcomes.is_empty());
            }
            other => panic!("expected Operation, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_flow() {
        let bundle = make_bundle(vec![json!({
            "id": "main_flow",
            "kind": "Flow",
            "entry": "step1",
            "steps": [
                {"id": "step1", "kind": "OperationStep", "op": "approve", "persona": "admin",
                 "outcomes": {"success": "step2"}, "on_failure": {"kind": "Terminate", "outcome": "failure"}},
                {"id": "step2", "kind": "HandoffStep", "from_persona": "admin", "to_persona": "user", "next": "step3"}
            ],
            "snapshot": "at_initiation",
            "provenance": {"file": "test.tenor", "line": 20},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::Flow(f) => {
                assert_eq!(f.id, "main_flow");
                assert_eq!(f.entry, "step1");
                assert_eq!(f.steps.len(), 2);
                assert_eq!(f.snapshot, "at_initiation");
            }
            other => panic!("expected Flow, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_persona() {
        let bundle = make_bundle(vec![json!({
            "id": "admin",
            "kind": "Persona",
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::Persona(p) => {
                assert_eq!(p.id, "admin");
            }
            other => panic!("expected Persona, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_system() {
        let bundle = make_bundle(vec![json!({
            "id": "lending_platform",
            "kind": "System",
            "members": [
                {"id": "loan", "path": "loan.tenor"},
                {"id": "credit", "path": "credit.tenor"}
            ],
            "shared_personas": [
                {"persona": "underwriter", "contracts": ["loan", "credit"]}
            ],
            "triggers": [
                {"source_contract": "loan", "source_flow": "approval",
                 "on": "approved", "target_contract": "credit",
                 "target_flow": "check", "persona": "underwriter"}
            ],
            "shared_entities": [
                {"entity": "Application", "contracts": ["loan", "credit"]}
            ],
            "provenance": {"file": "system.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::System(s) => {
                assert_eq!(s.id, "lending_platform");
                assert_eq!(s.members.len(), 2);
                assert_eq!(s.members[0].id, "loan");
                assert_eq!(s.shared_personas.len(), 1);
                assert_eq!(s.shared_personas[0].persona, "underwriter");
                assert_eq!(s.flow_triggers.len(), 1);
                assert_eq!(s.flow_triggers[0].source_contract, "loan");
                assert_eq!(s.shared_entities.len(), 1);
                assert_eq!(s.shared_entities[0].entity, "Application");
            }
            other => panic!("expected System, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_kind_skipped() {
        let bundle = make_bundle(vec![
            json!({"id": "admin", "kind": "Persona", "provenance": {"file": "t.tenor", "line": 1}, "tenor": "1.0"}),
            json!({"id": "future", "kind": "FutureConstruct", "data": {}}),
        ]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Persona(p) => assert_eq!(p.id, "admin"),
            other => panic!("expected Persona, got {:?}", other),
        }
    }

    #[test]
    fn test_multi_construct_bundle() {
        let bundle = make_bundle(vec![
            json!({"id": "admin", "kind": "Persona", "provenance": {"file": "t.tenor", "line": 1}, "tenor": "1.0"}),
            json!({"id": "amount", "kind": "Fact", "type": {"base": "Int"}, "source": {"field": "amt", "system": "s"}, "provenance": {"file": "t.tenor", "line": 2}, "tenor": "1.0"}),
            json!({"id": "Order", "kind": "Entity", "initial": "draft", "states": ["draft", "done"], "transitions": [{"from": "draft", "to": "done"}], "provenance": {"file": "t.tenor", "line": 3}, "tenor": "1.0"}),
        ]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 3);

        let mut has_persona = false;
        let mut has_fact = false;
        let mut has_entity = false;
        for c in &result.constructs {
            match c {
                InterchangeConstruct::Persona(_) => has_persona = true,
                InterchangeConstruct::Fact(_) => has_fact = true,
                InterchangeConstruct::Entity(_) => has_entity = true,
                _ => {}
            }
        }
        assert!(has_persona);
        assert!(has_fact);
        assert!(has_entity);
    }

    #[test]
    fn test_parse_type_decl() {
        let bundle = make_bundle(vec![json!({
            "id": "Currency",
            "kind": "TypeDecl",
            "type": {"base": "Enum", "values": ["USD", "EUR", "GBP"]},
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::TypeDecl(td) => {
                assert_eq!(td.id, "Currency");
                assert_eq!(td.type_def["base"], "Enum");
            }
            other => panic!("expected TypeDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_source() {
        let bundle = make_bundle(vec![json!({
            "id": "order_service",
            "kind": "Source",
            "protocol": "http",
            "fields": {
                "auth": "bearer_token",
                "base_url": "https://api.orders.com/v2",
                "schema_ref": "https://api.orders.com/v2/openapi.json"
            },
            "description": "Order management REST API",
            "provenance": {"file": "escrow.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        assert_eq!(result.constructs.len(), 1);
        match &result.constructs[0] {
            InterchangeConstruct::Source(s) => {
                assert_eq!(s.id, "order_service");
                assert_eq!(s.protocol, "http");
                assert_eq!(s.fields.len(), 3);
                assert_eq!(s.fields["base_url"], "https://api.orders.com/v2");
                assert_eq!(s.fields["auth"], "bearer_token");
                assert_eq!(s.description, Some("Order management REST API".to_string()));
                assert_eq!(s.provenance.as_ref().unwrap().file, "escrow.tenor");
                assert_eq!(s.provenance.as_ref().unwrap().line, 1);
            }
            other => panic!("expected Source, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_source_minimal() {
        let bundle = make_bundle(vec![json!({
            "id": "config",
            "kind": "Source",
            "protocol": "static",
            "fields": {},
            "provenance": {"file": "test.tenor", "line": 5},
            "tenor": "1.0"
        })]);

        let result = from_interchange(&bundle).unwrap();
        match &result.constructs[0] {
            InterchangeConstruct::Source(s) => {
                assert_eq!(s.id, "config");
                assert_eq!(s.protocol, "static");
                assert!(s.fields.is_empty());
                assert!(s.description.is_none());
            }
            other => panic!("expected Source, got {:?}", other),
        }
    }
}
