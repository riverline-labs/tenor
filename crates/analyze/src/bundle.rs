//! Interchange JSON deserialization into typed analysis structs.
//!
//! The analyzer consumes interchange JSON (same pattern as tenor-eval),
//! not the raw DSL AST. This module extracts all construct types from
//! the interchange bundle into Rust structs suitable for S1-S7 analysis.

use serde::Serialize;
use std::fmt;
use tenor_interchange::InterchangeConstruct;

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

impl From<tenor_interchange::InterchangeError> for AnalysisError {
    fn from(e: tenor_interchange::InterchangeError) -> Self {
        AnalysisError::InvalidBundle(e.to_string())
    }
}

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
        let parsed = tenor_interchange::from_interchange(bundle)?;

        let mut entities = Vec::new();
        let mut facts = Vec::new();
        let mut rules = Vec::new();
        let mut operations = Vec::new();
        let mut flows = Vec::new();
        let mut personas = Vec::new();
        let mut systems = Vec::new();

        for construct in &parsed.constructs {
            match construct {
                InterchangeConstruct::Entity(e) => {
                    entities.push(AnalysisEntity {
                        id: e.id.clone(),
                        states: e.states.clone(),
                        initial: e.initial.clone(),
                        transitions: e
                            .transitions
                            .iter()
                            .map(|t| Transition {
                                from: t.from.clone(),
                                to: t.to.clone(),
                            })
                            .collect(),
                        parent: e.parent.clone(),
                    });
                }
                InterchangeConstruct::Fact(f) => {
                    facts.push(AnalysisFact {
                        id: f.id.clone(),
                        fact_type: f.fact_type.clone(),
                    });
                }
                InterchangeConstruct::Rule(r) => {
                    let verdict_type =
                        r.verdict_type()
                            .ok_or_else(|| AnalysisError::MissingField {
                                construct: r.id.clone(),
                                field: "body.produce.verdict_type".to_string(),
                            })?;
                    rules.push(AnalysisRule {
                        id: r.id.clone(),
                        stratum: r.stratum,
                        when: r.when().cloned().unwrap_or(serde_json::Value::Null),
                        produce_verdict_type: verdict_type.to_string(),
                        produce_payload: r
                            .produce_payload()
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    });
                }
                InterchangeConstruct::Operation(op) => {
                    operations.push(AnalysisOperation {
                        id: op.id.clone(),
                        allowed_personas: op.allowed_personas.clone(),
                        precondition: op.precondition.clone(),
                        effects: op
                            .effects
                            .iter()
                            .map(|e| Effect {
                                entity_id: e.entity_id.clone(),
                                from_state: e.from.clone(),
                                to_state: e.to.clone(),
                                outcome: e.outcome.clone(),
                            })
                            .collect(),
                        outcomes: op.outcomes.clone(),
                        error_contract: op.error_contract.clone(),
                    });
                }
                InterchangeConstruct::Flow(f) => {
                    flows.push(AnalysisFlow {
                        id: f.id.clone(),
                        entry: f.entry.clone(),
                        steps: f.steps.clone(),
                        snapshot: f.snapshot.clone(),
                    });
                }
                InterchangeConstruct::Persona(p) => {
                    personas.push(AnalysisPersona { id: p.id.clone() });
                }
                InterchangeConstruct::System(s) => {
                    systems.push(AnalysisSystem {
                        id: s.id.clone(),
                        members: s
                            .members
                            .iter()
                            .map(|m| SystemMember {
                                id: m.id.clone(),
                                path: m.path.clone(),
                            })
                            .collect(),
                        shared_personas: s
                            .shared_personas
                            .iter()
                            .map(|sp| SharedPersona {
                                persona: sp.persona.clone(),
                                contracts: sp.contracts.clone(),
                            })
                            .collect(),
                        flow_triggers: s
                            .flow_triggers
                            .iter()
                            .map(|ft| FlowTrigger {
                                source_contract: ft.source_contract.clone(),
                                source_flow: ft.source_flow.clone(),
                                on: ft.on.clone(),
                                target_contract: ft.target_contract.clone(),
                                target_flow: ft.target_flow.clone(),
                                persona: ft.persona.clone(),
                            })
                            .collect(),
                        shared_entities: s
                            .shared_entities
                            .iter()
                            .map(|se| SharedEntity {
                                entity: se.entity.clone(),
                                contracts: se.contracts.clone(),
                            })
                            .collect(),
                    });
                }
                InterchangeConstruct::Source(_) | InterchangeConstruct::TypeDecl(_) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tenor_core::TENOR_BUNDLE_VERSION;

    fn make_bundle(constructs: Vec<serde_json::Value>) -> serde_json::Value {
        json!({
            "constructs": constructs,
            "id": "test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": TENOR_BUNDLE_VERSION
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
