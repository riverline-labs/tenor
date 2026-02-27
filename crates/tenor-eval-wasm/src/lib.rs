use slab::Slab;
use std::cell::RefCell;
use tenor_eval::Contract;
use wasm_bindgen::prelude::*;

mod inspect;

struct StoredContract {
    contract: Contract,
    bundle: serde_json::Value,
}

thread_local! {
    static CONTRACTS: RefCell<Slab<StoredContract>> = RefCell::new(Slab::new());
}

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

fn with_contract<F>(handle: u32, f: F) -> String
where
    F: FnOnce(&StoredContract) -> String,
{
    CONTRACTS.with(|contracts| {
        let contracts = contracts.borrow();
        match contracts.get(handle as usize) {
            Some(stored) => f(stored),
            None => error_json(&format!("invalid contract handle: {}", handle)),
        }
    })
}

#[wasm_bindgen]
pub fn load_contract(interchange_json: &str) -> String {
    let bundle: serde_json::Value = match serde_json::from_str(interchange_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid JSON: {}", e)),
    };

    let contract = match Contract::from_interchange(&bundle) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("invalid contract: {}", e)),
    };

    let handle = CONTRACTS.with(|contracts| {
        contracts
            .borrow_mut()
            .insert(StoredContract { contract, bundle })
    });

    serde_json::json!({ "handle": handle }).to_string()
}

#[wasm_bindgen]
pub fn free_contract(handle: u32) {
    CONTRACTS.with(|contracts| {
        let mut contracts = contracts.borrow_mut();
        if contracts.contains(handle as usize) {
            contracts.remove(handle as usize);
        }
    });
}

#[wasm_bindgen]
pub fn evaluate(handle: u32, facts_json: &str) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    with_contract(handle, |stored| {
        let fact_set = match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
            Ok(fs) => fs,
            Err(e) => return error_json(&format!("fact assembly error: {}", e)),
        };

        let verdict_set = match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
            Ok(vs) => vs,
            Err(e) => return error_json(&format!("evaluation error: {}", e)),
        };

        verdict_set.to_json().to_string()
    })
}

#[wasm_bindgen]
pub fn simulate_flow(
    handle: u32,
    flow_id: &str,
    persona_id: &str,
    facts_json: &str,
    entity_states_json: &str,
) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    let entity_overrides: std::collections::BTreeMap<String, String> =
        match serde_json::from_str(entity_states_json) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid entity states JSON: {}", e)),
        };

    with_contract(handle, |stored| {
        let fact_set =
            match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
                Ok(fs) => fs,
                Err(e) => return error_json(&format!("fact assembly error: {}", e)),
            };

        let verdict_set =
            match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
                Ok(vs) => vs,
                Err(e) => return error_json(&format!("evaluation error: {}", e)),
            };

        let snapshot = tenor_eval::Snapshot {
            facts: fact_set,
            verdicts: verdict_set.clone(),
        };

        let mut entity_states = tenor_eval::operation::init_entity_states(&stored.contract);
        // entity_overrides is a flat entity_id -> state map; convert to composite key.
        for (key, state) in &entity_overrides {
            entity_states.insert(
                (key.clone(), tenor_eval::DEFAULT_INSTANCE_ID.to_string()),
                state.clone(),
            );
        }

        let target_flow = match stored.contract.get_flow(flow_id) {
            Some(f) => f,
            None => return error_json(&format!("flow '{}' not found", flow_id)),
        };

        let flow_result = match tenor_eval::flow::execute_flow(
            target_flow,
            &stored.contract,
            &snapshot,
            &mut entity_states,
            None,
        ) {
            Ok(r) => r,
            Err(e) => return error_json(&format!("flow execution error: {}", e)),
        };

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
                    "from_state": e.from_state,
                    "to_state": e.to_state,
                })
            })
            .collect();

        serde_json::json!({
            "simulation": true,
            "flow_id": flow_id,
            "persona": persona_id,
            "outcome": flow_result.outcome,
            "path": path,
            "would_transition": would_transition,
            "verdicts": verdict_set.to_json()["verdicts"],
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn inspect_contract(handle: u32) -> String {
    with_contract(handle, |stored| {
        match inspect::build_inspect(&stored.bundle) {
            Ok(json) => json.to_string(),
            Err(e) => error_json(&format!("inspect error: {}", e)),
        }
    })
}

#[wasm_bindgen]
pub fn compute_action_space(
    handle: u32,
    facts_json: &str,
    entity_states_json: &str,
    persona_id: &str,
) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    let entity_overrides: std::collections::BTreeMap<String, String> =
        match serde_json::from_str(entity_states_json) {
            Ok(v) => v,
            Err(e) => return error_json(&format!("invalid entity states JSON: {}", e)),
        };

    with_contract(handle, |stored| {
        // Convert flat entity_id -> state map to composite (entity_id, instance_id) key format.
        let entity_states = tenor_eval::single_instance(entity_overrides.clone());
        let result = tenor_eval::action_space::compute_action_space(
            &stored.contract,
            &facts,
            &entity_states,
            persona_id,
        );

        match result {
            Ok(action_space) => match serde_json::to_string(&action_space) {
                Ok(json) => json,
                Err(e) => error_json(&format!("serialization error: {}", e)),
            },
            Err(e) => error_json(&format!("action space error: {}", e)),
        }
    })
}
