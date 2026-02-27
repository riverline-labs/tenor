use pyo3::prelude::*;
use std::collections::BTreeMap;

use crate::types::{json_to_py, py_to_json};

/// Tenor contract evaluator.
///
/// Wraps the Rust evaluator with a Python-friendly API.
/// All inputs are Python dicts/lists/strings; all outputs are Python dicts/lists.
#[pyclass]
pub struct TenorEvaluator {
    contract: tenor_eval::Contract,
}

#[pymethods]
impl TenorEvaluator {
    /// Load a contract from interchange JSON string.
    #[staticmethod]
    fn from_bundle_json(json: &str) -> PyResult<Self> {
        let bundle: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        let contract = tenor_eval::Contract::from_interchange(&bundle)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid contract: {}", e))
            })?;

        Ok(TenorEvaluator { contract })
    }

    /// Load a contract from a Python dict (interchange bundle).
    #[staticmethod]
    fn from_bundle(bundle: &Bound<'_, PyAny>) -> PyResult<Self> {
        let bundle_json = py_to_json(bundle)?;
        let contract = tenor_eval::Contract::from_interchange(&bundle_json)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid contract: {}", e))
            })?;
        Ok(TenorEvaluator { contract })
    }

    /// Evaluate rules against the provided facts.
    /// Returns a dict with "verdicts" list.
    fn evaluate(&self, py: Python<'_>, facts: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let facts_json = py_to_json(facts)?;
        let fact_set = tenor_eval::assemble::assemble_facts(&self.contract, &facts_json)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Fact assembly error: {}", e))
            })?;

        let verdict_set = tenor_eval::rules::eval_strata(&self.contract, &fact_set)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Evaluation error: {}", e))
            })?;

        let result = verdict_set.to_json();
        json_to_py(py, &result)
    }

    /// Compute the action space for a persona.
    ///
    /// `facts`: dict of {fact_id: value}
    /// `entity_states`: dict of {entity_id: state_string} (single-instance flat format)
    ///   OR {entity_id: {instance_id: state_string}} (multi-instance format)
    /// `persona`: persona ID string
    ///
    /// Returns a dict with "persona_id", "actions", "blocked_actions", "current_verdicts".
    fn compute_action_space(
        &self,
        py: Python<'_>,
        facts: &Bound<'_, PyAny>,
        entity_states: &Bound<'_, PyAny>,
        persona: &str,
    ) -> PyResult<PyObject> {
        let facts_json = py_to_json(facts)?;
        let states_json = py_to_json(entity_states)?;

        // Parse entity states — support both flat (entity_id -> state) and
        // nested (entity_id -> {instance_id -> state}) formats.
        let entity_map = parse_entity_states(&states_json).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid entity states: {}", e))
        })?;

        let action_space =
            tenor_eval::compute_action_space(&self.contract, &facts_json, &entity_map, persona)
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Action space error: {}",
                        e
                    ))
                })?;

        let result = serde_json::to_value(&action_space).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Serialization error: {}", e))
        })?;

        json_to_py(py, &result)
    }

    /// Execute (simulate) a flow.
    ///
    /// `flow_id`: ID of the flow to execute
    /// `facts`: dict of {fact_id: value}
    /// `entity_states`: dict of {entity_id: state_string} (flat) or
    ///   {entity_id: {instance_id: state}} (multi-instance).
    ///   Provided states are overlaid on contract defaults — an empty dict
    ///   uses contract initial states for all entities.
    /// `persona`: persona ID for provenance recording
    ///
    /// Returns a dict with "flow_id", "persona", "outcome", "path", "would_transition", "verdicts".
    fn execute_flow(
        &self,
        py: Python<'_>,
        flow_id: &str,
        facts: &Bound<'_, PyAny>,
        entity_states: &Bound<'_, PyAny>,
        persona: &str,
    ) -> PyResult<PyObject> {
        let facts_json = py_to_json(facts)?;
        let states_json = py_to_json(entity_states)?;

        // Parse the provided entity states (supports both flat and nested formats)
        let provided_states = parse_entity_states(&states_json).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid entity states: {}", e))
        })?;

        // Assemble facts and evaluate rules to get verdicts
        let fact_set = tenor_eval::assemble::assemble_facts(&self.contract, &facts_json)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Fact assembly error: {}", e))
            })?;

        let verdict_set = tenor_eval::rules::eval_strata(&self.contract, &fact_set)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Evaluation error: {}", e))
            })?;

        // Create frozen snapshot
        let snapshot = tenor_eval::Snapshot {
            facts: fact_set,
            verdicts: verdict_set.clone(),
        };

        // Merge contract defaults with provided states (provided states override defaults)
        let mut merged_states = tenor_eval::operation::init_entity_states(&self.contract);
        for (key, state) in provided_states {
            merged_states.insert(key, state);
        }

        // Find the target flow
        let target_flow = self
            .contract
            .get_flow(flow_id)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Flow execution error: flow '{}' not found in contract",
                    flow_id
                ))
            })?;

        // Use empty instance_bindings — falls back to DEFAULT_INSTANCE_ID (backward compat)
        let instance_bindings = BTreeMap::new();

        // Execute the flow
        let mut flow_result = tenor_eval::flow::execute_flow(
            target_flow,
            &self.contract,
            &snapshot,
            &mut merged_states,
            &instance_bindings,
            None,
        )
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Flow execution error: {}", e))
        })?;

        // Record persona for provenance
        flow_result.initiating_persona = Some(persona.to_string());

        let path: Vec<serde_json::Value> = flow_result
            .steps_executed
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step_id": s.step_id,
                    "step_type": s.step_type,
                    "result": s.result,
                })
            })
            .collect();

        let would_transition: Vec<serde_json::Value> = flow_result
            .entity_state_changes
            .iter()
            .map(|e| {
                serde_json::json!({
                    "entity_id": e.entity_id,
                    "instance_id": e.instance_id,
                    "from_state": e.from_state,
                    "to_state": e.to_state,
                })
            })
            .collect();

        let result = serde_json::json!({
            "flow_id": flow_id,
            "persona": persona,
            "outcome": flow_result.outcome,
            "path": path,
            "would_transition": would_transition,
            "verdicts": verdict_set.to_json()["verdicts"],
        });

        json_to_py(py, &result)
    }
}

/// Parse entity_states JSON with auto-detection of flat and nested formats.
///
/// Flat format (single-instance):
/// ```json
/// { "Order": "pending", "Invoice": "draft" }
/// ```
///
/// Nested format (multi-instance):
/// ```json
/// { "Order": {"ord-001": "pending"}, "Invoice": {"inv-001": "draft"} }
/// ```
fn parse_entity_states(
    json: &serde_json::Value,
) -> Result<tenor_eval::EntityStateMap, String> {
    let obj = match json.as_object() {
        Some(o) => o,
        None => return Err("entity_states must be a JSON object".to_string()),
    };

    let mut entity_states = tenor_eval::EntityStateMap::new();

    for (entity_id, value) in obj {
        if let Some(state_str) = value.as_str() {
            // Flat format: entity_id -> state string; convert to single instance
            entity_states.insert(
                (entity_id.clone(), tenor_eval::DEFAULT_INSTANCE_ID.to_string()),
                state_str.to_string(),
            );
        } else if let Some(instance_map) = value.as_object() {
            // Nested format: entity_id -> { instance_id -> state }
            for (instance_id, state_val) in instance_map {
                let state_str = match state_val.as_str() {
                    Some(s) => s,
                    None => {
                        return Err(format!(
                            "entity_states[{}][{}] must be a string state",
                            entity_id, instance_id
                        ))
                    }
                };
                entity_states.insert(
                    (entity_id.clone(), instance_id.clone()),
                    state_str.to_string(),
                );
            }
        } else {
            return Err(format!(
                "entity_states[{}] must be a string (flat format) or object (nested format)",
                entity_id
            ));
        }
    }

    Ok(entity_states)
}
