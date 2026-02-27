use std::collections::BTreeMap;
use std::fs;

fn main() {
    let fixtures_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fixtures".to_string());

    println!("Generating conformance fixtures in: {}", fixtures_dir);

    // Read inputs
    let bundle_str =
        fs::read_to_string(format!("{}/escrow-bundle.json", fixtures_dir)).expect("escrow-bundle.json");
    let bundle: serde_json::Value =
        serde_json::from_str(&bundle_str).expect("parse escrow-bundle.json");

    let facts_str =
        fs::read_to_string(format!("{}/escrow-facts.json", fixtures_dir)).expect("escrow-facts.json");
    let facts: serde_json::Value =
        serde_json::from_str(&facts_str).expect("parse escrow-facts.json");

    let states_str =
        fs::read_to_string(format!("{}/escrow-entity-states.json", fixtures_dir)).expect("escrow-entity-states.json");
    let entity_states: serde_json::Value =
        serde_json::from_str(&states_str).expect("parse escrow-entity-states.json");

    let facts_inactive_str = fs::read_to_string(format!("{}/escrow-facts-inactive.json", fixtures_dir))
        .expect("escrow-facts-inactive.json");
    let facts_inactive: serde_json::Value =
        serde_json::from_str(&facts_inactive_str).expect("parse escrow-facts-inactive.json");

    // Parse entity states (flat format: {"Order": "pending"} -> {(Order, _default): pending})
    let entity_state_map = parse_entity_states(&entity_states).expect("parse entity states");

    // ──────────────────────────────────────────────────────────────────────────
    // 1. expected-verdicts.json — evaluate with is_active=true
    // ──────────────────────────────────────────────────────────────────────────
    let contract =
        tenor_eval::Contract::from_interchange(&bundle).expect("load contract");
    let fact_set =
        tenor_eval::assemble::assemble_facts(&contract, &facts).expect("assemble facts");
    let verdict_set =
        tenor_eval::rules::eval_strata(&contract, &fact_set).expect("eval strata");
    let verdicts_json = verdict_set.to_json();
    write_sorted(&format!("{}/expected-verdicts.json", fixtures_dir), &verdicts_json);

    // ──────────────────────────────────────────────────────────────────────────
    // 2. expected-action-space.json — compute_action_space (active, admin)
    // ──────────────────────────────────────────────────────────────────────────
    let action_space = tenor_eval::compute_action_space(
        &contract,
        &facts,
        &entity_state_map,
        "admin",
    )
    .expect("compute_action_space");
    let action_space_json = serde_json::to_value(&action_space).expect("serialize action_space");
    write_sorted(
        &format!("{}/expected-action-space.json", fixtures_dir),
        &action_space_json,
    );

    // ──────────────────────────────────────────────────────────────────────────
    // 3. expected-flow-result.json — simulate_flow (active, admin, approval_flow)
    //    Output format matches simulate_flow_with_bindings in the WASM module.
    // ──────────────────────────────────────────────────────────────────────────
    let flow_json = simulate_flow(&contract, &bundle, &facts, &entity_states, "approval_flow", "admin")
        .expect("simulate_flow");
    write_sorted(
        &format!("{}/expected-flow-result.json", fixtures_dir),
        &flow_json,
    );

    // ──────────────────────────────────────────────────────────────────────────
    // 4. expected-verdicts-inactive.json — evaluate with is_active=false
    // ──────────────────────────────────────────────────────────────────────────
    let fact_set_inactive =
        tenor_eval::assemble::assemble_facts(&contract, &facts_inactive).expect("assemble facts inactive");
    let verdict_set_inactive =
        tenor_eval::rules::eval_strata(&contract, &fact_set_inactive).expect("eval strata inactive");
    let verdicts_inactive_json = verdict_set_inactive.to_json();
    write_sorted(
        &format!("{}/expected-verdicts-inactive.json", fixtures_dir),
        &verdicts_inactive_json,
    );

    // ──────────────────────────────────────────────────────────────────────────
    // 5. expected-action-space-blocked.json — compute_action_space (inactive, admin)
    // ──────────────────────────────────────────────────────────────────────────
    let action_space_blocked = tenor_eval::compute_action_space(
        &contract,
        &facts_inactive,
        &entity_state_map,
        "admin",
    )
    .expect("compute_action_space blocked");
    let blocked_json = serde_json::to_value(&action_space_blocked).expect("serialize blocked");
    write_sorted(
        &format!("{}/expected-action-space-blocked.json", fixtures_dir),
        &blocked_json,
    );

    println!("Conformance fixtures generated successfully.");
}

/// Simulate a flow and return JSON matching the WASM simulate_flow_with_bindings output format.
///
/// Format:
/// ```json
/// {
///   "simulation": true,
///   "flow_id": "...",
///   "persona": "...",
///   "outcome": "...",
///   "path": [...],
///   "would_transition": [...],
///   "verdicts": [...],
///   "instance_bindings": {}
/// }
/// ```
fn simulate_flow(
    contract: &tenor_eval::Contract,
    _bundle: &serde_json::Value,
    facts: &serde_json::Value,
    entity_states_json: &serde_json::Value,
    flow_id: &str,
    persona: &str,
) -> Result<serde_json::Value, String> {
    // Parse entity states (supports flat and nested formats)
    let provided_states =
        parse_entity_states(entity_states_json)?;

    // Assemble facts and evaluate rules
    let fact_set = tenor_eval::assemble::assemble_facts(contract, facts)
        .map_err(|e| format!("fact assembly: {}", e))?;
    let verdict_set = tenor_eval::rules::eval_strata(contract, &fact_set)
        .map_err(|e| format!("eval strata: {}", e))?;

    // Create snapshot
    let snapshot = tenor_eval::Snapshot {
        facts: fact_set,
        verdicts: verdict_set.clone(),
    };

    // Merge contract defaults with provided states
    let mut merged_states = tenor_eval::operation::init_entity_states(contract);
    for (key, state) in provided_states {
        merged_states.insert(key, state);
    }

    // Find the flow
    let target_flow = contract
        .get_flow(flow_id)
        .ok_or_else(|| format!("flow '{}' not found", flow_id))?;

    // Use empty instance_bindings (backward compat => _default)
    let instance_bindings = BTreeMap::new();

    // Execute the flow
    let flow_result = tenor_eval::flow::execute_flow(
        target_flow,
        contract,
        &snapshot,
        &mut merged_states,
        &instance_bindings,
        None,
    )
    .map_err(|e| format!("flow execution: {}", e))?;

    // Build path — match WASM format (include instance_bindings if non-empty)
    let path: Vec<serde_json::Value> = flow_result
        .steps_executed
        .iter()
        .map(|s| {
            let mut step_json = serde_json::json!({
                "step_id": s.step_id,
                "step_type": s.step_type,
                "result": s.result,
            });
            if !s.instance_bindings.is_empty() {
                step_json["instance_bindings"] = serde_json::to_value(&s.instance_bindings)
                    .unwrap_or(serde_json::Value::Null);
            }
            step_json
        })
        .collect();

    // Build would_transition — match WASM format
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

    Ok(serde_json::json!({
        "simulation": true,
        "flow_id": flow_id,
        "persona": persona,
        "outcome": flow_result.outcome,
        "path": path,
        "would_transition": would_transition,
        "verdicts": verdict_set.to_json()["verdicts"],
        "instance_bindings": instance_bindings,
    }))
}

/// Parse entity_states JSON with auto-detection of flat and nested formats.
///
/// Flat: `{"Order": "pending"}` -> `{("Order", "_default"): "pending"}`
/// Nested: `{"Order": {"ord-001": "pending"}}` -> `{("Order", "ord-001"): "pending"}`
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
            entity_states.insert(
                (entity_id.clone(), tenor_eval::DEFAULT_INSTANCE_ID.to_string()),
                state_str.to_string(),
            );
        } else if let Some(instance_map) = value.as_object() {
            for (instance_id, state_val) in instance_map {
                let state_str = match state_val.as_str() {
                    Some(s) => s,
                    None => {
                        return Err(format!(
                            "entity_states[{}][{}] must be a string",
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
                "entity_states[{}] must be a string or object",
                entity_id
            ));
        }
    }

    Ok(entity_states)
}

/// Write a serde_json::Value to a file with sorted keys and pretty formatting.
fn write_sorted(path: &str, value: &serde_json::Value) {
    let sorted = sort_keys(value);
    let formatted = serde_json::to_string_pretty(&sorted).expect("serialize");
    fs::write(path, formatted + "\n").expect("write file");
    println!("  wrote {}", path);
}

/// Recursively sort all object keys lexicographically.
fn sort_keys(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted: serde_json::Map<String, serde_json::Value> =
                serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), sort_keys(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(sort_keys).collect())
        }
        other => other.clone(),
    }
}
