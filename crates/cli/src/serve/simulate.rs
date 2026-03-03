//! Flow simulation and action space handlers.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::json_error;
use super::state::AppState;

/// Internal error type for simulate_flow to distinguish persona errors from eval errors.
enum SimulateError {
    PersonaNotFound(String),
    Eval(tenor_eval::EvalError),
}

/// Core simulation logic, runs synchronously in a blocking task.
fn simulate_flow_inner(
    bundle: &serde_json::Value,
    facts: &serde_json::Value,
    flow_id: &str,
    persona_id: &str,
    entity_states_input: Option<&serde_json::Value>,
) -> Result<serde_json::Value, SimulateError> {
    let contract = tenor_eval::Contract::from_interchange(bundle).map_err(SimulateError::Eval)?;

    // Check persona exists in contract
    if !contract.personas.contains(&persona_id.to_string()) {
        return Err(SimulateError::PersonaNotFound(persona_id.to_string()));
    }

    // Assemble facts
    let fact_set =
        tenor_eval::assemble::assemble_facts(&contract, facts).map_err(SimulateError::Eval)?;

    // Evaluate rules to produce verdicts
    let verdict_set =
        tenor_eval::rules::eval_strata(&contract, &fact_set).map_err(SimulateError::Eval)?;

    // Create frozen snapshot
    let snapshot = tenor_eval::Snapshot {
        facts: fact_set,
        verdicts: verdict_set.clone(),
    };

    // Build entity states: start from contract defaults, override with request.
    // Overrides use DEFAULT_INSTANCE_ID since the API takes plain entity_id -> state.
    let mut entity_states = tenor_eval::operation::init_entity_states(&contract);
    if let Some(es_input) = entity_states_input {
        if let Some(obj) = es_input.as_object() {
            for (entity_id, state_info) in obj {
                if let Some(state) = state_info.get("state").and_then(|v| v.as_str()) {
                    entity_states.insert(
                        (
                            entity_id.clone(),
                            tenor_eval::DEFAULT_INSTANCE_ID.to_string(),
                        ),
                        state.to_string(),
                    );
                }
            }
        }
    }

    // Find the flow
    let target_flow = contract.get_flow(flow_id).ok_or_else(|| {
        SimulateError::Eval(tenor_eval::EvalError::DeserializeError {
            message: format!("flow '{}' not found in contract", flow_id),
        })
    })?;

    // Build step index for enriching the response path with operation/persona info
    let step_info: HashMap<String, (String, String)> = target_flow
        .steps
        .iter()
        .filter_map(|s| match s {
            tenor_eval::types::FlowStep::OperationStep {
                id, op, persona, ..
            } => Some((id.clone(), (op.clone(), persona.clone()))),
            _ => None,
        })
        .collect();

    // Execute the flow
    let flow_result = tenor_eval::flow::execute_flow(
        target_flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &tenor_eval::InstanceBindingMap::new(),
        None,
    )
    .map_err(SimulateError::Eval)?;

    // Build path
    let path: Vec<serde_json::Value> = flow_result
        .steps_executed
        .iter()
        .map(|step| {
            let mut step_json = serde_json::json!({
                "step": step.step_id,
                "type": step.step_type,
                "outcome": step.result,
            });
            if let Some((op_name, step_persona)) = step_info.get(&step.step_id) {
                step_json["operation"] = serde_json::json!(op_name);
                step_json["persona"] = serde_json::json!(step_persona);
            }
            step_json
        })
        .collect();

    // Build would_transition
    let would_transition: Vec<serde_json::Value> = flow_result
        .entity_state_changes
        .iter()
        .map(|e| {
            serde_json::json!({
                "entity_id": e.entity_id,
                "from": e.from_state,
                "to": e.to_state,
            })
        })
        .collect();

    // Build verdicts
    let verdicts: Vec<serde_json::Value> = verdict_set
        .0
        .iter()
        .map(|v| {
            serde_json::json!({
                "type": v.verdict_type,
                "payload": v.payload.to_json(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "simulation": true,
        "flow_id": flow_id,
        "outcome": flow_result.outcome,
        "path": path,
        "would_transition": would_transition,
        "verdicts": verdicts,
    }))
}

/// POST /flows/{flow_id}/simulate
///
/// Dedicated flow simulation endpoint. Stateless -- entity states come from
/// the request body, nothing is persisted. Returns simulation: true always.
pub(crate) async fn handle_simulate_flow(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
    Json(parsed): Json<serde_json::Value>,
) -> Response {
    let persona_id = match parsed.get("persona_id").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'persona_id' field")
                .into_response()
        }
    };

    let facts = parsed
        .get("facts")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let entity_states_input = parsed.get("entity_states").cloned();

    // Find the contract containing this flow
    let contracts = state.contracts.read().await;
    let mut found_bundle: Option<serde_json::Value> = None;

    for bundle in contracts.values() {
        if let Some(constructs) = bundle.get("constructs").and_then(|c| c.as_array()) {
            let has_flow = constructs.iter().any(|c| {
                c.get("kind").and_then(|k| k.as_str()) == Some("Flow")
                    && c.get("id").and_then(|i| i.as_str()) == Some(&flow_id)
            });
            if has_flow {
                found_bundle = Some(bundle.clone());
                break;
            }
        }
    }
    drop(contracts);

    let bundle = match found_bundle {
        Some(b) => b,
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("flow '{}' not found", flow_id),
            )
            .into_response()
        }
    };

    let fid = flow_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        simulate_flow_inner(
            &bundle,
            &facts,
            &fid,
            &persona_id,
            entity_states_input.as_ref(),
        )
    })
    .await;

    match result {
        Ok(Ok(response_json)) => (StatusCode::OK, Json(response_json)).into_response(),
        Ok(Err(SimulateError::PersonaNotFound(p))) => json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            &format!("persona '{}' not found in contract", p),
        )
        .into_response(),
        Ok(Err(SimulateError::Eval(e))) => {
            json_error(StatusCode::UNPROCESSABLE_ENTITY, &format!("{}", e)).into_response()
        }
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

/// POST /actions
///
/// Compute the action space for a persona. Stateless -- facts and entity
/// states come from the request body, nothing is persisted. Returns all
/// available and blocked actions with reasons.
///
/// Input: { "persona_id": "...", "facts": {...}, "entity_states": {...} }
/// Output: ActionSpace JSON
pub(crate) async fn handle_actions(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> Response {
    let persona_id = match parsed.get("persona_id").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'persona_id' field")
                .into_response()
        }
    };

    let facts = parsed
        .get("facts")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let entity_states_input: std::collections::BTreeMap<String, String> =
        match parsed.get("entity_states") {
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(m) => m,
                Err(e) => {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        &format!("invalid entity_states: {}", e),
                    )
                    .into_response()
                }
            },
            None => std::collections::BTreeMap::new(),
        };

    // Find first loaded contract (same pattern as /evaluate)
    let contracts = state.contracts.read().await;
    let bundle = match contracts.values().next() {
        Some(b) => b.clone(),
        None => return json_error(StatusCode::NOT_FOUND, "no contracts loaded").into_response(),
    };
    drop(contracts);

    let result = tokio::task::spawn_blocking(move || {
        let contract = match tenor_eval::Contract::from_interchange(&bundle) {
            Ok(c) => c,
            Err(e) => return Err(format!("invalid contract: {}", e)),
        };

        // Convert flat entity_id -> state map to composite (entity_id, instance_id) key format.
        let entity_states = tenor_eval::single_instance(entity_states_input);
        tenor_eval::action_space::compute_action_space(
            &contract,
            &facts,
            &entity_states,
            &persona_id,
        )
        .map_err(|e| format!("{}", e))
    })
    .await;

    match result {
        Ok(Ok(action_space)) => match serde_json::to_value(&action_space) {
            Ok(json) => (StatusCode::OK, Json(json)).into_response(),
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("serialization error: {}", e),
            )
            .into_response(),
        },
        Ok(Err(e)) => json_error(StatusCode::UNPROCESSABLE_ENTITY, &e).into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}
