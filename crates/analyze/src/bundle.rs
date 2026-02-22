//! Interchange JSON deserialization into typed analysis structs.
//!
//! The analyzer consumes interchange JSON (same pattern as tenor-eval),
//! not the raw DSL AST. This module extracts all construct types from
//! the interchange bundle into Rust structs suitable for S1-S7 analysis.

use serde::Serialize;
use std::fmt;

/// Error type for analysis operations.
#[derive(Debug, Clone)]
pub enum AnalysisError {
    /// The bundle JSON is invalid or missing required fields.
    InvalidBundle(String),
    /// A construct is missing a required field.
    MissingField { construct: String, field: String },
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisError::InvalidBundle(msg) => write!(f, "invalid bundle: {}", msg),
            AnalysisError::MissingField { construct, field } => {
                write!(f, "missing field '{}' in construct '{}'", field, construct)
            }
        }
    }
}

impl std::error::Error for AnalysisError {}

/// A state transition in an Entity.
#[derive(Debug, Clone, Serialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
}

/// An Entity construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisEntity {
    pub id: String,
    pub states: Vec<String>,
    pub initial: String,
    pub transitions: Vec<Transition>,
    pub parent: Option<String>,
}

/// A Fact construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisFact {
    pub id: String,
    /// Raw type JSON preserved for S3a type-level analysis.
    pub fact_type: serde_json::Value,
}

/// A Rule construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisRule {
    pub id: String,
    pub stratum: u64,
    /// Raw predicate expression JSON (body.when).
    pub when: serde_json::Value,
    pub produce_verdict_type: String,
    /// Raw produce payload JSON.
    pub produce_payload: serde_json::Value,
}

/// An Operation effect -- entity state transition.
#[derive(Debug, Clone, Serialize)]
pub struct Effect {
    pub entity_id: String,
    pub from_state: String,
    pub to_state: String,
    pub outcome: Option<String>,
}

/// An Operation construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisOperation {
    pub id: String,
    pub allowed_personas: Vec<String>,
    /// Raw precondition predicate expression JSON. None if null.
    pub precondition: Option<serde_json::Value>,
    pub effects: Vec<Effect>,
    pub outcomes: Vec<String>,
    /// Raw error_contract JSON.
    pub error_contract: Option<serde_json::Value>,
}

/// A Flow construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisFlow {
    pub id: String,
    pub entry: String,
    /// Raw step JSON values preserved for S6 path enumeration.
    pub steps: Vec<serde_json::Value>,
    pub snapshot: String,
}

/// A Persona construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisPersona {
    pub id: String,
}

/// A member contract declaration within a System.
#[derive(Debug, Clone, Serialize)]
pub struct SystemMember {
    pub id: String,
    pub path: String,
}

/// A shared persona binding within a System.
#[derive(Debug, Clone, Serialize)]
pub struct SharedPersona {
    pub persona: String,
    pub contracts: Vec<String>,
}

/// A cross-contract flow trigger within a System.
#[derive(Debug, Clone, Serialize)]
pub struct FlowTrigger {
    pub source_contract: String,
    pub source_flow: String,
    pub on: String,
    pub target_contract: String,
    pub target_flow: String,
    pub persona: String,
}

/// A shared entity binding within a System.
#[derive(Debug, Clone, Serialize)]
pub struct SharedEntity {
    pub entity: String,
    pub contracts: Vec<String>,
}

/// A System construct extracted from interchange JSON.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisSystem {
    pub id: String,
    pub members: Vec<SystemMember>,
    pub shared_personas: Vec<SharedPersona>,
    pub flow_triggers: Vec<FlowTrigger>,
    pub shared_entities: Vec<SharedEntity>,
}

/// All constructs extracted from an interchange bundle, ready for analysis.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisBundle {
    pub entities: Vec<AnalysisEntity>,
    pub facts: Vec<AnalysisFact>,
    pub rules: Vec<AnalysisRule>,
    pub operations: Vec<AnalysisOperation>,
    pub flows: Vec<AnalysisFlow>,
    pub personas: Vec<AnalysisPersona>,
    pub systems: Vec<AnalysisSystem>,
}

impl AnalysisBundle {
    /// Deserialize an interchange JSON bundle into typed analysis structs.
    ///
    /// Extracts all constructs from the bundle's `constructs` array,
    /// dispatching on the `kind` field. Unknown kinds are silently skipped
    /// for forward compatibility.
    pub fn from_interchange(bundle: &serde_json::Value) -> Result<Self, AnalysisError> {
        let constructs = bundle
            .get("constructs")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                AnalysisError::InvalidBundle("missing or invalid 'constructs' array".to_string())
            })?;

        let mut entities = Vec::new();
        let mut facts = Vec::new();
        let mut rules = Vec::new();
        let mut operations = Vec::new();
        let mut flows = Vec::new();
        let mut personas = Vec::new();
        let mut systems = Vec::new();

        for construct in constructs {
            let kind = construct.get("kind").and_then(|k| k.as_str()).unwrap_or("");

            match kind {
                "Entity" => entities.push(parse_entity(construct)?),
                "Fact" => facts.push(parse_fact(construct)?),
                "Rule" => rules.push(parse_rule(construct)?),
                "Operation" => operations.push(parse_operation(construct)?),
                "Flow" => flows.push(parse_flow(construct)?),
                "Persona" => personas.push(parse_persona(construct)?),
                "System" => systems.push(parse_system(construct)?),
                _ => {} // Skip unknown kinds for forward compatibility
            }
        }

        // Also extract personas from operation allowed_personas if no Persona
        // constructs were found. The elaborator emits personas within operations
        // rather than as standalone constructs.
        if personas.is_empty() {
            let mut seen = std::collections::BTreeSet::new();
            for op in &operations {
                for p in &op.allowed_personas {
                    if seen.insert(p.clone()) {
                        personas.push(AnalysisPersona { id: p.clone() });
                    }
                }
            }
            // Also check flow steps for persona references
            for flow in &flows {
                for step in &flow.steps {
                    if let Some(persona) = step.get("persona").and_then(|p| p.as_str()) {
                        if seen.insert(persona.to_string()) {
                            personas.push(AnalysisPersona {
                                id: persona.to_string(),
                            });
                        }
                    }
                    if let Some(from) = step.get("from_persona").and_then(|p| p.as_str()) {
                        if seen.insert(from.to_string()) {
                            personas.push(AnalysisPersona {
                                id: from.to_string(),
                            });
                        }
                    }
                    if let Some(to) = step.get("to_persona").and_then(|p| p.as_str()) {
                        if seen.insert(to.to_string()) {
                            personas.push(AnalysisPersona { id: to.to_string() });
                        }
                    }
                }
            }
        }

        Ok(AnalysisBundle {
            entities,
            facts,
            rules,
            operations,
            flows,
            personas,
            systems,
        })
    }
}

/// Extract a required string field from a JSON object.
fn required_str(
    obj: &serde_json::Value,
    field: &str,
    construct_id: &str,
) -> Result<String, AnalysisError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AnalysisError::MissingField {
            construct: construct_id.to_string(),
            field: field.to_string(),
        })
}

/// Extract a required u64 field from a JSON object.
fn required_u64(
    obj: &serde_json::Value,
    field: &str,
    construct_id: &str,
) -> Result<u64, AnalysisError> {
    obj.get(field)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| AnalysisError::MissingField {
            construct: construct_id.to_string(),
            field: field.to_string(),
        })
}

fn parse_entity(obj: &serde_json::Value) -> Result<AnalysisEntity, AnalysisError> {
    let id = required_str(obj, "id", "Entity")?;

    let states = obj
        .get("states")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| AnalysisError::MissingField {
            construct: id.clone(),
            field: "states".to_string(),
        })?;

    let initial = required_str(obj, "initial", &id)?;

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

    Ok(AnalysisEntity {
        id,
        states,
        initial,
        transitions,
        parent,
    })
}

fn parse_fact(obj: &serde_json::Value) -> Result<AnalysisFact, AnalysisError> {
    let id = required_str(obj, "id", "Fact")?;
    let fact_type = obj.get("type").cloned().unwrap_or(serde_json::Value::Null);

    Ok(AnalysisFact { id, fact_type })
}

fn parse_rule(obj: &serde_json::Value) -> Result<AnalysisRule, AnalysisError> {
    let id = required_str(obj, "id", "Rule")?;
    let stratum = required_u64(obj, "stratum", &id)?;

    let body = obj.get("body").ok_or_else(|| AnalysisError::MissingField {
        construct: id.clone(),
        field: "body".to_string(),
    })?;

    let when = body.get("when").cloned().unwrap_or(serde_json::Value::Null);

    let produce = body
        .get("produce")
        .ok_or_else(|| AnalysisError::MissingField {
            construct: id.clone(),
            field: "body.produce".to_string(),
        })?;

    let produce_verdict_type = produce
        .get("verdict_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AnalysisError::MissingField {
            construct: id.clone(),
            field: "body.produce.verdict_type".to_string(),
        })?;

    let produce_payload = produce
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(AnalysisRule {
        id,
        stratum,
        when,
        produce_verdict_type,
        produce_payload,
    })
}

fn parse_operation(obj: &serde_json::Value) -> Result<AnalysisOperation, AnalysisError> {
    let id = required_str(obj, "id", "Operation")?;

    let allowed_personas = obj
        .get("allowed_personas")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // precondition can be null or a predicate expression object
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
                    let from_state = e.get("from")?.as_str()?.to_string();
                    let to_state = e.get("to")?.as_str()?.to_string();
                    let outcome = e
                        .get("outcome")
                        .and_then(|o| o.as_str())
                        .map(|s| s.to_string());
                    Some(Effect {
                        entity_id,
                        from_state,
                        to_state,
                        outcome,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // outcomes can be null or an array of strings
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

    Ok(AnalysisOperation {
        id,
        allowed_personas,
        precondition,
        effects,
        outcomes,
        error_contract,
    })
}

fn parse_flow(obj: &serde_json::Value) -> Result<AnalysisFlow, AnalysisError> {
    let id = required_str(obj, "id", "Flow")?;
    let entry = required_str(obj, "entry", &id)?;

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

    Ok(AnalysisFlow {
        id,
        entry,
        steps,
        snapshot,
    })
}

fn parse_persona(obj: &serde_json::Value) -> Result<AnalysisPersona, AnalysisError> {
    let id = required_str(obj, "id", "Persona")?;
    Ok(AnalysisPersona { id })
}

fn parse_system(obj: &serde_json::Value) -> Result<AnalysisSystem, AnalysisError> {
    let id = required_str(obj, "id", "System")?;

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

    Ok(AnalysisSystem {
        id,
        members,
        shared_personas,
        flow_triggers,
        shared_entities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_bundle(constructs: Vec<serde_json::Value>) -> serde_json::Value {
        json!({
            "constructs": constructs,
            "id": "test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0"
        })
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

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].id, "Order");
        assert_eq!(
            result.entities[0].states,
            vec!["draft", "submitted", "approved"]
        );
        assert_eq!(result.entities[0].initial, "draft");
        assert_eq!(result.entities[0].transitions.len(), 2);
        assert_eq!(result.entities[0].transitions[0].from, "draft");
        assert_eq!(result.entities[0].transitions[0].to, "submitted");
        assert!(result.entities[0].parent.is_none());
    }

    #[test]
    fn test_parse_fact() {
        let bundle = make_bundle(vec![json!({
            "id": "amount",
            "kind": "Fact",
            "type": {"base": "Decimal", "precision": 10, "scale": 2},
            "source": {"field": "amt", "system": "billing"},
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.facts.len(), 1);
        assert_eq!(result.facts[0].id, "amount");
        assert_eq!(result.facts[0].fact_type["base"], "Decimal");
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
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.rules.len(), 1);
        assert_eq!(result.rules[0].id, "check_amount");
        assert_eq!(result.rules[0].stratum, 0);
        assert_eq!(result.rules[0].produce_verdict_type, "high_value");
    }

    #[test]
    fn test_parse_operation() {
        let bundle = make_bundle(vec![json!({
            "id": "approve",
            "kind": "Operation",
            "allowed_personas": ["admin", "manager"],
            "precondition": {"verdict_present": "approved"},
            "effects": [
                {"entity_id": "Order", "from": "pending", "to": "approved"}
            ],
            "outcomes": ["success", "rejected"],
            "error_contract": ["precondition_failed"],
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.operations.len(), 1);
        let op = &result.operations[0];
        assert_eq!(op.id, "approve");
        assert_eq!(op.allowed_personas, vec!["admin", "manager"]);
        assert!(op.precondition.is_some());
        assert_eq!(op.effects.len(), 1);
        assert_eq!(op.effects[0].entity_id, "Order");
        assert_eq!(op.effects[0].from_state, "pending");
        assert_eq!(op.effects[0].to_state, "approved");
        assert_eq!(op.outcomes, vec!["success", "rejected"]);
    }

    #[test]
    fn test_parse_operation_null_precondition_and_outcomes() {
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

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.operations.len(), 1);
        assert!(result.operations[0].precondition.is_none());
        assert!(result.operations[0].outcomes.is_empty());
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
                {"id": "step2", "kind": "HandoffStep", "from_persona": "admin", "to_persona": "user",
                 "next": "step3"}
            ],
            "snapshot": "at_initiation",
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.flows.len(), 1);
        assert_eq!(result.flows[0].id, "main_flow");
        assert_eq!(result.flows[0].entry, "step1");
        assert_eq!(result.flows[0].steps.len(), 2);
        assert_eq!(result.flows[0].snapshot, "at_initiation");
    }

    #[test]
    fn test_parse_persona() {
        let bundle = make_bundle(vec![json!({
            "id": "admin",
            "kind": "Persona",
            "provenance": {"file": "test.tenor", "line": 1},
            "tenor": "1.0"
        })]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.personas.len(), 1);
        assert_eq!(result.personas[0].id, "admin");
    }

    #[test]
    fn test_unknown_kind_skipped() {
        let bundle = make_bundle(vec![
            json!({"id": "admin", "kind": "Persona", "provenance": {"file": "t.tenor", "line": 1}, "tenor": "1.0"}),
            json!({"id": "unknown", "kind": "FutureConstruct", "data": {}}),
        ]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.personas.len(), 1);
        // FutureConstruct was silently skipped
    }

    #[test]
    fn test_missing_constructs_array() {
        let bundle = json!({"id": "test", "kind": "Bundle"});
        let result = AnalysisBundle::from_interchange(&bundle);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AnalysisError::InvalidBundle(_)
        ));
    }

    #[test]
    fn test_multi_construct_bundle() {
        let bundle = make_bundle(vec![
            json!({"id": "admin", "kind": "Persona", "provenance": {"file": "t.tenor", "line": 1}, "tenor": "1.0"}),
            json!({"id": "user", "kind": "Persona", "provenance": {"file": "t.tenor", "line": 2}, "tenor": "1.0"}),
            json!({"id": "amount", "kind": "Fact", "type": {"base": "Int"}, "source": {"field": "amt", "system": "s"}, "provenance": {"file": "t.tenor", "line": 3}, "tenor": "1.0"}),
            json!({"id": "Order", "kind": "Entity", "initial": "draft", "states": ["draft", "done"], "transitions": [{"from": "draft", "to": "done"}], "provenance": {"file": "t.tenor", "line": 4}, "tenor": "1.0"}),
        ]);

        let result = AnalysisBundle::from_interchange(&bundle).unwrap();
        assert_eq!(result.personas.len(), 2);
        assert_eq!(result.facts.len(), 1);
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.rules.len(), 0);
        assert_eq!(result.operations.len(), 0);
        assert_eq!(result.flows.len(), 0);
    }
}
