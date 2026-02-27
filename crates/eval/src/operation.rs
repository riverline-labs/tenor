//! Operation execution engine.
//!
//! Implements spec Section 9: Operation execution with persona authorization,
//! precondition evaluation, entity state transitions, and outcome routing.
//!
//! Operations are persona-gated state transitions. Execution follows:
//! 1. Persona check (authorization)
//! 2. Precondition evaluation
//! 3. Effect execution (entity state transitions)
//! 4. Outcome determination (single or multi-outcome routing)

use std::collections::BTreeMap;

use crate::predicate::{eval_pred, EvalContext};
use crate::provenance::ProvenanceCollector;
use crate::types::{EvalError, FactSet, Operation, VerdictSet};

// ──────────────────────────────────────────────
// Operation execution types
// ──────────────────────────────────────────────

/// Default instance ID for single-instance entities per §6.5 degenerate case.
pub const DEFAULT_INSTANCE_ID: &str = "_default";

/// Map of (entity_id, instance_id) -> current state name.
///
/// Per §6.5: every entity instance is identified by a composite key.
/// Single-instance contracts use `_default` as the instance_id.
pub type EntityStateMap = BTreeMap<(String, String), String>;

/// Maps entity_id → instance_id for instance targeting in operations and flows.
///
/// Per §9.2 and §11.1: the executor provides which specific instance to target
/// for each entity effect. An empty map falls back to DEFAULT_INSTANCE_ID
/// for backward compatibility with single-instance contracts.
pub type InstanceBindingMap = BTreeMap<String, String>;

/// Create a single-instance state map from entity_id -> state (backward compat).
/// Each entity gets the `_default` instance ID per §6.5 degenerate case.
pub fn single_instance(states: BTreeMap<String, String>) -> EntityStateMap {
    states
        .into_iter()
        .map(|(entity_id, state)| ((entity_id, DEFAULT_INSTANCE_ID.to_string()), state))
        .collect()
}

/// Get state for a specific (entity_id, instance_id) pair.
pub fn get_instance_state<'a>(
    states: &'a EntityStateMap,
    entity_id: &str,
    instance_id: &str,
) -> Option<&'a String> {
    states.get(&(entity_id.to_string(), instance_id.to_string()))
}

/// Resolve the target instance_id for a given entity from the binding map.
///
/// Per §11.4: if an entity is not in the bindings, fall back to DEFAULT_INSTANCE_ID
/// for backward compatibility with single-instance contracts.
pub fn resolve_instance_id<'a>(bindings: &'a InstanceBindingMap, entity_id: &str) -> &'a str {
    bindings
        .get(entity_id)
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_INSTANCE_ID)
}

/// Record of a single entity state transition applied by an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRecord {
    pub entity_id: String,
    /// The specific instance that was targeted by this effect.
    /// Per §9.5: provenance records instance_binding.
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
}

/// Provenance record for an operation execution.
#[derive(Debug, Clone)]
pub struct OperationProvenance {
    pub operation_id: String,
    pub persona: String,
    pub effects: Vec<EffectRecord>,
}

/// Result of a successful operation execution.
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub outcome: String,
    pub effects_applied: Vec<EffectRecord>,
    pub provenance: OperationProvenance,
}

/// Errors specific to operation execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// The persona is not authorized to execute this operation.
    PersonaRejected {
        operation_id: String,
        persona: String,
    },
    /// A precondition was not met.
    PreconditionFailed {
        operation_id: String,
        condition_desc: String,
    },
    /// Entity is not in the expected state for the effect.
    InvalidEntityState {
        entity_id: String,
        instance_id: String,
        expected: String,
        actual: String,
    },
    /// Entity referenced by effect not found in entity state map.
    EntityNotFound {
        entity_id: String,
        instance_id: String,
    },
    /// Evaluation error during precondition check.
    EvalError(EvalError),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::PersonaRejected {
                operation_id,
                persona,
            } => {
                write!(
                    f,
                    "persona '{}' not authorized for operation '{}'",
                    persona, operation_id
                )
            }
            OperationError::PreconditionFailed {
                operation_id,
                condition_desc,
            } => {
                write!(
                    f,
                    "precondition failed for operation '{}': {}",
                    operation_id, condition_desc
                )
            }
            OperationError::InvalidEntityState {
                entity_id,
                instance_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "entity '{}' instance '{}' in state '{}', expected '{}'",
                    entity_id, instance_id, actual, expected
                )
            }
            OperationError::EntityNotFound {
                entity_id,
                instance_id,
            } => {
                write!(
                    f,
                    "entity '{}' instance '{}' not found in state map",
                    entity_id, instance_id
                )
            }
            OperationError::EvalError(e) => write!(f, "evaluation error: {}", e),
        }
    }
}

impl std::error::Error for OperationError {}

impl From<EvalError> for OperationError {
    fn from(e: EvalError) -> Self {
        OperationError::EvalError(e)
    }
}

/// Initialize entity states from contract entity declarations.
///
/// Each entity starts in its declared initial state with the `_default` instance ID.
/// Multi-instance support (Plan 04-02) will extend this to accept instance IDs.
pub fn init_entity_states(contract: &crate::types::Contract) -> EntityStateMap {
    let mut map = EntityStateMap::new();
    for entity in &contract.entities {
        map.insert(
            (entity.id.clone(), DEFAULT_INSTANCE_ID.to_string()),
            entity.initial.clone(),
        );
    }
    map
}

/// Execute an operation against the current state.
///
/// Follows spec Section 9.2:
/// 1. Persona check
/// 2. Precondition evaluation
/// 3. Effect execution (entity state transitions)
/// 4. Outcome determination
///
/// The `instance_bindings` parameter maps entity_id → instance_id to identify
/// which specific instance each entity effect targets. An empty map falls back
/// to DEFAULT_INSTANCE_ID for each entity (backward compat with single-instance
/// contracts per §6.5 degenerate case).
pub fn execute_operation(
    op: &Operation,
    persona: &str,
    facts: &FactSet,
    verdicts: &VerdictSet,
    entity_states: &mut EntityStateMap,
    instance_bindings: &InstanceBindingMap,
) -> Result<OperationResult, OperationError> {
    // Step 1: Persona check
    if !op.allowed_personas.contains(&persona.to_string()) {
        return Err(OperationError::PersonaRejected {
            operation_id: op.id.clone(),
            persona: persona.to_string(),
        });
    }

    // Step 2: Precondition check
    let mut collector = ProvenanceCollector::new();
    let ctx = EvalContext::new();
    let cond_result = eval_pred(&op.precondition, facts, verdicts, &ctx, &mut collector)?;
    let precondition_met = cond_result.as_bool()?;
    if !precondition_met {
        return Err(OperationError::PreconditionFailed {
            operation_id: op.id.clone(),
            condition_desc: "precondition evaluated to false".to_string(),
        });
    }

    // Step 3: Effect execution
    let mut effects_applied = Vec::new();
    let mut outcome_from_effects: Option<String> = None;

    for effect in &op.effects {
        // Resolve the target instance for this entity from the binding map.
        // Falls back to DEFAULT_INSTANCE_ID if no binding provided (§6.5 degenerate case).
        let instance_id = resolve_instance_id(instance_bindings, &effect.entity_id).to_string();
        let key = (effect.entity_id.clone(), instance_id.clone());
        let current_state = entity_states
            .get(&key)
            .ok_or_else(|| OperationError::EntityNotFound {
                entity_id: effect.entity_id.clone(),
                instance_id: instance_id.clone(),
            })?
            .clone();

        if current_state != effect.from {
            return Err(OperationError::InvalidEntityState {
                entity_id: effect.entity_id.clone(),
                instance_id: instance_id.clone(),
                expected: effect.from.clone(),
                actual: current_state,
            });
        }

        // Apply state transition to the targeted (entity_id, instance_id) pair
        entity_states.insert(key, effect.to.clone());
        effects_applied.push(EffectRecord {
            entity_id: effect.entity_id.clone(),
            instance_id,
            from_state: effect.from.clone(),
            to_state: effect.to.clone(),
        });

        // Track outcome from effect (for multi-outcome routing)
        if let Some(ref effect_outcome) = effect.outcome {
            outcome_from_effects = Some(effect_outcome.clone());
        }
    }

    // Step 4: Outcome determination
    let outcome = if let Some(effect_outcome) = outcome_from_effects {
        // Multi-outcome: outcome determined by which effects triggered
        effect_outcome
    } else if op.outcomes.len() == 1 {
        // Single outcome -- implicit default is valid
        op.outcomes[0].clone()
    } else if op.outcomes.len() > 1 {
        // Multi-outcome operations REQUIRE effect-to-outcome mapping.
        // If no effect carried an outcome field, this is a contract error.
        return Err(OperationError::PreconditionFailed {
            operation_id: op.id.clone(),
            condition_desc: "multi-outcome operation has no effect-to-outcome mapping".to_string(),
        });
    } else {
        // No declared outcomes -- use "success" as default
        "success".to_string()
    };

    let provenance = OperationProvenance {
        operation_id: op.id.clone(),
        persona: persona.to_string(),
        effects: effects_applied.clone(),
    };

    Ok(OperationResult {
        outcome,
        effects_applied,
        provenance,
    })
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

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

    /// Helper: build a simple operation for testing.
    fn make_operation(
        id: &str,
        personas: Vec<&str>,
        effects: Vec<Effect>,
        outcomes: Vec<&str>,
    ) -> Operation {
        Operation {
            id: id.to_string(),
            allowed_personas: personas.into_iter().map(|p| p.to_string()).collect(),
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects,
            error_contract: vec![],
            outcomes: outcomes.into_iter().map(|o| o.to_string()).collect(),
        }
    }

    /// Helper: build an operation with a specific precondition.
    fn make_operation_with_precondition(
        id: &str,
        personas: Vec<&str>,
        precondition: Predicate,
        effects: Vec<Effect>,
        outcomes: Vec<&str>,
    ) -> Operation {
        Operation {
            id: id.to_string(),
            allowed_personas: personas.into_iter().map(|p| p.to_string()).collect(),
            precondition,
            effects,
            error_contract: vec![],
            outcomes: outcomes.into_iter().map(|o| o.to_string()).collect(),
        }
    }

    // ──────────────────────────────────────
    // Persona authorization tests
    // ──────────────────────────────────────

    #[test]
    fn authorized_persona_succeeds() {
        let op = make_operation(
            "submit_order",
            vec!["buyer"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None,
            }],
            vec!["submitted"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "draft".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "buyer",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.outcome, "submitted");
        assert_eq!(res.effects_applied.len(), 1);
        assert_eq!(res.provenance.persona, "buyer");
    }

    #[test]
    fn unauthorized_persona_rejected() {
        let op = make_operation("submit_order", vec!["buyer"], vec![], vec!["submitted"]);

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = EntityStateMap::new();

        let result = execute_operation(
            &op,
            "seller",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::PersonaRejected {
                operation_id,
                persona,
            } => {
                assert_eq!(operation_id, "submit_order");
                assert_eq!(persona, "seller");
            }
            other => panic!("expected PersonaRejected, got {:?}", other),
        }
    }

    #[test]
    fn multiple_allowed_personas() {
        let op = make_operation(
            "review_order",
            vec!["buyer", "admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "submitted".to_string(),
                to: "reviewed".to_string(),
                outcome: None,
            }],
            vec!["reviewed"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let entity_states = single_instance(
            [("order".to_string(), "submitted".to_string())]
                .into_iter()
                .collect(),
        );

        // Both buyer and admin should succeed
        let mut states_clone = entity_states.clone();
        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut states_clone,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
    }

    // ──────────────────────────────────────
    // Precondition evaluation tests
    // ──────────────────────────────────────

    #[test]
    fn met_precondition_proceeds() {
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(true));

        let op = make_operation_with_precondition(
            "activate",
            vec!["admin"],
            Predicate::Compare {
                left: Box::new(Predicate::FactRef("is_active".to_string())),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: bool_type(),
                }),
                comparison_type: None,
            },
            vec![Effect {
                entity_id: "account".to_string(),
                from: "pending".to_string(),
                to: "active".to_string(),
                outcome: None,
            }],
            vec!["activated"],
        );

        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("account".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        assert_eq!(
            get_instance_state(&entity_states, "account", DEFAULT_INSTANCE_ID).unwrap(),
            "active"
        );
    }

    #[test]
    fn unmet_precondition_fails() {
        let mut facts = FactSet::new();
        facts.insert("is_active".to_string(), Value::Bool(false));

        let op = make_operation_with_precondition(
            "activate",
            vec!["admin"],
            Predicate::Compare {
                left: Box::new(Predicate::FactRef("is_active".to_string())),
                op: "=".to_string(),
                right: Box::new(Predicate::Literal {
                    value: Value::Bool(true),
                    type_spec: bool_type(),
                }),
                comparison_type: None,
            },
            vec![],
            vec!["activated"],
        );

        let verdicts = VerdictSet::new();
        let mut entity_states = EntityStateMap::new();

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::PreconditionFailed { operation_id, .. } => {
                assert_eq!(operation_id, "activate");
            }
            other => panic!("expected PreconditionFailed, got {:?}", other),
        }
    }

    // ──────────────────────────────────────
    // Entity state transition tests
    // ──────────────────────────────────────

    #[test]
    fn valid_state_transition() {
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(
            res.effects_applied[0],
            EffectRecord {
                entity_id: "order".to_string(),
                instance_id: DEFAULT_INSTANCE_ID.to_string(),
                from_state: "pending".to_string(),
                to_state: "approved".to_string(),
            }
        );
        assert_eq!(
            get_instance_state(&entity_states, "order", DEFAULT_INSTANCE_ID).unwrap(),
            "approved"
        );
    }

    #[test]
    fn invalid_entity_state_fails() {
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "draft".to_string())]
                .into_iter()
                .collect(),
        ); // Wrong state

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::InvalidEntityState {
                entity_id,
                instance_id,
                expected,
                actual,
            } => {
                assert_eq!(entity_id, "order");
                assert_eq!(instance_id, DEFAULT_INSTANCE_ID);
                assert_eq!(expected, "pending");
                assert_eq!(actual, "draft");
            }
            other => panic!("expected InvalidEntityState, got {:?}", other),
        }
    }

    #[test]
    fn entity_not_found_fails() {
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = EntityStateMap::new(); // Empty -- no entities

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::EntityNotFound {
                entity_id,
                instance_id,
            } => {
                assert_eq!(entity_id, "order");
                assert_eq!(instance_id, DEFAULT_INSTANCE_ID);
            }
            other => panic!("expected EntityNotFound, got {:?}", other),
        }
    }

    #[test]
    fn multiple_effects_applied_in_order() {
        let op = make_operation(
            "complete_order",
            vec!["system"],
            vec![
                Effect {
                    entity_id: "order".to_string(),
                    from: "approved".to_string(),
                    to: "fulfilled".to_string(),
                    outcome: None,
                },
                Effect {
                    entity_id: "payment".to_string(),
                    from: "authorized".to_string(),
                    to: "captured".to_string(),
                    outcome: None,
                },
            ],
            vec!["completed"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [
                ("order".to_string(), "approved".to_string()),
                ("payment".to_string(), "authorized".to_string()),
            ]
            .into_iter()
            .collect(),
        );

        let result = execute_operation(
            &op,
            "system",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.effects_applied.len(), 2);
        assert_eq!(
            get_instance_state(&entity_states, "order", DEFAULT_INSTANCE_ID).unwrap(),
            "fulfilled"
        );
        assert_eq!(
            get_instance_state(&entity_states, "payment", DEFAULT_INSTANCE_ID).unwrap(),
            "captured"
        );
    }

    // ──────────────────────────────────────
    // Multi-outcome routing tests
    // ──────────────────────────────────────

    #[test]
    fn multi_outcome_routes_by_effect() {
        let op = Operation {
            id: "process_payment".to_string(),
            allowed_personas: vec!["system".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "payment".to_string(),
                from: "pending".to_string(),
                to: "captured".to_string(),
                outcome: Some("payment_success".to_string()),
            }],
            error_contract: vec![],
            outcomes: vec!["payment_success".to_string(), "payment_failed".to_string()],
        };

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("payment".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "system",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.outcome, "payment_success");
    }

    // ──────────────────────────────────────
    // Provenance tracking tests
    // ──────────────────────────────────────

    #[test]
    fn provenance_records_all_state_changes() {
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        )
        .unwrap();
        assert_eq!(result.provenance.operation_id, "approve");
        assert_eq!(result.provenance.persona, "admin");
        assert_eq!(result.provenance.effects.len(), 1);
        assert_eq!(
            result.provenance.effects[0],
            EffectRecord {
                entity_id: "order".to_string(),
                instance_id: DEFAULT_INSTANCE_ID.to_string(),
                from_state: "pending".to_string(),
                to_state: "approved".to_string(),
            }
        );
    }

    // ──────────────────────────────────────
    // Init entity states test
    // ──────────────────────────────────────

    #[test]
    fn init_entity_states_from_contract() {
        let contract = Contract::new(
            vec![],
            vec![
                Entity {
                    id: "order".to_string(),
                    states: vec!["draft".to_string(), "submitted".to_string()],
                    initial: "draft".to_string(),
                    transitions: vec![],
                },
                Entity {
                    id: "payment".to_string(),
                    states: vec!["pending".to_string(), "captured".to_string()],
                    initial: "pending".to_string(),
                    transitions: vec![],
                },
            ],
            vec![],
            vec![],
            vec![],
            vec![],
        );

        let states = init_entity_states(&contract);
        assert_eq!(
            get_instance_state(&states, "order", DEFAULT_INSTANCE_ID).unwrap(),
            "draft"
        );
        assert_eq!(
            get_instance_state(&states, "payment", DEFAULT_INSTANCE_ID).unwrap(),
            "pending"
        );
    }

    // ──────────────────────────────────────
    // Precondition with verdict check
    // ──────────────────────────────────────

    #[test]
    fn precondition_checks_verdict_presence() {
        let op = make_operation_with_precondition(
            "process",
            vec!["admin"],
            Predicate::VerdictPresent("order_valid".to_string()),
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "processing".to_string(),
                outcome: None,
            }],
            vec!["processing"],
        );

        // With verdict present -- should succeed
        let facts = FactSet::new();
        let mut verdicts = VerdictSet::new();
        verdicts.push(crate::types::VerdictInstance {
            verdict_type: "order_valid".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "check".to_string(),
                stratum: 0,
                facts_used: vec![],
                verdicts_used: vec![],
            },
        });
        let mut entity_states = single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());

        // Without verdict -- should fail precondition
        let empty_verdicts = VerdictSet::new();
        let mut entity_states2 = single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );
        let result2 = execute_operation(
            &op,
            "admin",
            &facts,
            &empty_verdicts,
            &mut entity_states2,
            &InstanceBindingMap::new(),
        );
        assert!(result2.is_err());
        match result2.unwrap_err() {
            OperationError::PreconditionFailed { .. } => {}
            other => panic!("expected PreconditionFailed, got {:?}", other),
        }
    }

    // ──────────────────────────────────────
    // C3: Multi-outcome fallback test (B3 fix)
    // ──────────────────────────────────────

    #[test]
    fn multi_outcome_no_mapping_returns_error() {
        // Operation with 2+ outcomes but effects carry no `outcome` field.
        // Should return PreconditionFailed, not silently fall back to outcomes[0].
        let op = make_operation(
            "process_payment",
            vec!["system"],
            vec![Effect {
                entity_id: "payment".to_string(),
                from: "pending".to_string(),
                to: "captured".to_string(),
                outcome: None, // No outcome mapping!
            }],
            vec!["payment_success", "payment_failed"], // 2 outcomes
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("payment".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "system",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::PreconditionFailed {
                operation_id,
                condition_desc,
            } => {
                assert_eq!(operation_id, "process_payment");
                assert!(
                    condition_desc.contains("multi-outcome"),
                    "error should mention multi-outcome, got: {}",
                    condition_desc
                );
            }
            other => panic!("expected PreconditionFailed, got {:?}", other),
        }
    }

    #[test]
    fn single_outcome_implicit_default_succeeds() {
        // Single-outcome Operation with no effect-to-outcome mapping
        // should succeed with implicit default.
        let op = make_operation(
            "simple_op",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None, // No mapping, but only 1 outcome
            }],
            vec!["submitted"], // Single outcome
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "draft".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().outcome, "submitted");
    }

    // ──────────────────────────────────────
    // Instance-targeted operation tests
    // ──────────────────────────────────────

    #[test]
    fn instance_targeted_operation_targets_specific_instance() {
        // Two order instances: "order-1" in draft, "order-2" in submitted.
        // Binding selects order-1 for the effect.
        let op = make_operation(
            "submit_order",
            vec!["buyer"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None,
            }],
            vec!["submitted"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();

        // Two instances of the same entity type
        let mut entity_states: EntityStateMap = BTreeMap::new();
        entity_states.insert(
            ("order".to_string(), "order-1".to_string()),
            "draft".to_string(),
        );
        entity_states.insert(
            ("order".to_string(), "order-2".to_string()),
            "submitted".to_string(),
        );

        let mut bindings = InstanceBindingMap::new();
        bindings.insert("order".to_string(), "order-1".to_string());

        let result = execute_operation(
            &op,
            "buyer",
            &facts,
            &verdicts,
            &mut entity_states,
            &bindings,
        );
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.outcome, "submitted");
        assert_eq!(res.effects_applied.len(), 1);
        assert_eq!(res.effects_applied[0].instance_id, "order-1");

        // order-1 should be in "submitted" now
        assert_eq!(
            get_instance_state(&entity_states, "order", "order-1").unwrap(),
            "submitted"
        );
        // order-2 should be UNCHANGED in "submitted"
        assert_eq!(
            get_instance_state(&entity_states, "order", "order-2").unwrap(),
            "submitted"
        );
    }

    #[test]
    fn instance_targeted_wrong_state_fails_for_targeted_instance() {
        // Effect targets order-2 (in "submitted"), but effect expects "draft".
        let op = make_operation(
            "submit_order",
            vec!["buyer"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None,
            }],
            vec!["submitted"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();

        let mut entity_states: EntityStateMap = BTreeMap::new();
        entity_states.insert(
            ("order".to_string(), "order-1".to_string()),
            "draft".to_string(),
        );
        entity_states.insert(
            ("order".to_string(), "order-2".to_string()),
            "submitted".to_string(),
        );

        // Target order-2 which is in "submitted", not "draft" -> InvalidEntityState
        let mut bindings = InstanceBindingMap::new();
        bindings.insert("order".to_string(), "order-2".to_string());

        let result = execute_operation(
            &op,
            "buyer",
            &facts,
            &verdicts,
            &mut entity_states,
            &bindings,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::InvalidEntityState {
                entity_id,
                instance_id,
                expected,
                actual,
            } => {
                assert_eq!(entity_id, "order");
                assert_eq!(instance_id, "order-2");
                assert_eq!(expected, "draft");
                assert_eq!(actual, "submitted");
            }
            other => panic!("expected InvalidEntityState, got {:?}", other),
        }
    }

    #[test]
    fn empty_bindings_fall_back_to_default_instance() {
        // Empty bindings should use DEFAULT_INSTANCE_ID (backward compat)
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states = single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        // Empty bindings -> falls back to DEFAULT_INSTANCE_ID
        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &InstanceBindingMap::new(),
        );
        assert!(result.is_ok());
        assert_eq!(
            get_instance_state(&entity_states, "order", DEFAULT_INSTANCE_ID).unwrap(),
            "approved"
        );
    }

    #[test]
    fn instance_not_in_state_map_returns_entity_not_found_with_instance_id() {
        // Binding references an instance that doesn't exist in entity_states
        let op = make_operation(
            "approve",
            vec!["admin"],
            vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states: EntityStateMap = BTreeMap::new();
        // No entry for "order/nonexistent"

        let mut bindings = InstanceBindingMap::new();
        bindings.insert("order".to_string(), "nonexistent".to_string());

        let result = execute_operation(
            &op,
            "admin",
            &facts,
            &verdicts,
            &mut entity_states,
            &bindings,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            OperationError::EntityNotFound {
                entity_id,
                instance_id,
            } => {
                assert_eq!(entity_id, "order");
                assert_eq!(instance_id, "nonexistent");
            }
            other => panic!("expected EntityNotFound, got {:?}", other),
        }
    }

    #[test]
    fn effect_record_carries_instance_id() {
        // Verify EffectRecord.instance_id is populated from the binding
        let op = make_operation(
            "process",
            vec!["system"],
            vec![Effect {
                entity_id: "payment".to_string(),
                from: "pending".to_string(),
                to: "captured".to_string(),
                outcome: None,
            }],
            vec!["done"],
        );

        let facts = FactSet::new();
        let verdicts = VerdictSet::new();
        let mut entity_states: EntityStateMap = BTreeMap::new();
        entity_states.insert(
            ("payment".to_string(), "pay-42".to_string()),
            "pending".to_string(),
        );

        let mut bindings = InstanceBindingMap::new();
        bindings.insert("payment".to_string(), "pay-42".to_string());

        let result = execute_operation(
            &op,
            "system",
            &facts,
            &verdicts,
            &mut entity_states,
            &bindings,
        )
        .unwrap();
        assert_eq!(result.effects_applied[0].entity_id, "payment");
        assert_eq!(result.effects_applied[0].instance_id, "pay-42");
        assert_eq!(result.effects_applied[0].from_state, "pending");
        assert_eq!(result.effects_applied[0].to_state, "captured");
    }
}
