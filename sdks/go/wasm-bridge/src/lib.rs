//! Tenor WASM bridge for the Go SDK.
//!
//! Exports C-ABI functions callable by wazero without wasm-bindgen or Node.js.
//!
//! # Memory protocol
//!
//! Go passes strings to WASM using this pattern:
//! 1. Allocate memory in WASM via `alloc(len)` → returns pointer
//! 2. Write the UTF-8 string bytes into WASM memory at the pointer
//! 3. Call the target function with `(ptr, len)` args
//! 4. Read the result via `get_result_ptr()` + `get_result_len()`
//! 5. Optionally free the input buffer via `dealloc(ptr, len)`
//!
//! Functions that take a contract handle receive it as the first `u32` argument,
//! followed by string arguments as `(ptr, len)` pairs.

use slab::Slab;
use std::cell::RefCell;
use std::collections::BTreeMap;
use tenor_eval::Contract;

struct StoredContract {
    contract: Contract,
    // Keep the bundle around for future inspect_contract support
    #[allow(dead_code)]
    bundle: serde_json::Value,
}

thread_local! {
    static CONTRACTS: RefCell<Slab<StoredContract>> = RefCell::new(Slab::new());
    static RESULT_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn set_result(s: &str) {
    RESULT_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend_from_slice(s.as_bytes());
    });
}

fn error_result(msg: &str) {
    set_result(&serde_json::json!({ "error": msg }).to_string());
}

fn with_contract<F>(handle: u32, f: F)
where
    F: FnOnce(&StoredContract) -> String,
{
    let result = CONTRACTS.with(|contracts| {
        let contracts = contracts.borrow();
        match contracts.get(handle as usize) {
            Some(stored) => f(stored),
            None => serde_json::json!({ "error": format!("invalid contract handle: {}", handle) })
                .to_string(),
        }
    });
    set_result(&result);
}

/// Parse entity_states JSON with auto-detection of old and new formats.
///
/// Old format (flat):  `{ "Order": "pending" }`
/// New format (nested): `{ "Order": { "ord-001": "pending" } }`
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
                (
                    entity_id.clone(),
                    tenor_eval::DEFAULT_INSTANCE_ID.to_string(),
                ),
                state_str.to_string(),
            );
        } else if let Some(instance_map) = value.as_object() {
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
                "entity_states[{}] must be a string (old format) or object (new format)",
                entity_id
            ));
        }
    }

    Ok(entity_states)
}

/// Parse instance_bindings JSON: entity_id -> instance_id.
fn parse_instance_bindings(json_str: &str) -> Result<tenor_eval::InstanceBindingMap, String> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() || trimmed == "null" || trimmed == "{}" {
        return Ok(BTreeMap::new());
    }

    let val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("invalid instance_bindings JSON: {}", e))?;

    match val {
        serde_json::Value::Null => Ok(BTreeMap::new()),
        serde_json::Value::Object(map) => {
            let mut result = BTreeMap::new();
            for (entity_id, instance_val) in map {
                let instance_id = match instance_val.as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        return Err(format!(
                            "instance_bindings[{}] must be a string instance_id",
                            entity_id
                        ))
                    }
                };
                result.insert(entity_id, instance_id);
            }
            Ok(result)
        }
        _ => Err("instance_bindings must be a JSON object or null".to_string()),
    }
}

// ── Memory management exports ──

/// Allocate `len` bytes in WASM memory. Returns a pointer to the buffer.
/// The caller must write `len` bytes at the returned pointer, then pass
/// the pointer and length to the target function.
#[no_mangle]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously allocated with `alloc`.
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, len: u32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, len as usize, len as usize);
    }
}

/// Return a pointer to the result buffer. Valid until the next API call.
#[no_mangle]
pub extern "C" fn get_result_ptr() -> *const u8 {
    RESULT_BUF.with(|buf| buf.borrow().as_ptr())
}

/// Return the length of the result buffer.
#[no_mangle]
pub extern "C" fn get_result_len() -> u32 {
    RESULT_BUF.with(|buf| buf.borrow().len() as u32)
}

// ── Contract management exports ──

/// Load a contract from interchange bundle JSON.
///
/// Input:  UTF-8 JSON bytes at `ptr[0..len]`
/// Result: `{"handle": N}` or `{"error": "..."}`
#[no_mangle]
pub unsafe extern "C" fn load_contract(ptr: *const u8, len: u32) {
    let json_str = match std::str::from_utf8(std::slice::from_raw_parts(ptr, len as usize)) {
        Ok(s) => s,
        Err(e) => {
            error_result(&format!("invalid UTF-8: {}", e));
            return;
        }
    };

    let bundle: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid JSON: {}", e));
            return;
        }
    };

    let contract = match Contract::from_interchange(&bundle) {
        Ok(c) => c,
        Err(e) => {
            error_result(&format!("invalid contract: {}", e));
            return;
        }
    };

    let handle = CONTRACTS.with(|contracts| {
        contracts
            .borrow_mut()
            .insert(StoredContract { contract, bundle })
    });

    set_result(&serde_json::json!({ "handle": handle }).to_string());
}

/// Free a loaded contract by handle.
///
/// No-op if the handle is invalid.
#[no_mangle]
pub extern "C" fn free_contract(handle: u32) {
    CONTRACTS.with(|contracts| {
        let mut contracts = contracts.borrow_mut();
        if contracts.contains(handle as usize) {
            contracts.remove(handle as usize);
        }
    });
    set_result("{}");
}

// ── Evaluation exports ──

/// Evaluate rules against facts.
///
/// Args:   handle, facts_ptr, facts_len
/// Result: VerdictSet JSON or `{"error": "..."}`
#[no_mangle]
pub unsafe extern "C" fn evaluate(handle: u32, ptr: *const u8, len: u32) {
    let facts_str = match std::str::from_utf8(std::slice::from_raw_parts(ptr, len as usize)) {
        Ok(s) => s,
        Err(e) => {
            error_result(&format!("invalid UTF-8 in facts: {}", e));
            return;
        }
    };

    let facts: serde_json::Value = match serde_json::from_str(facts_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid facts JSON: {}", e));
            return;
        }
    };

    with_contract(handle, |stored| {
        let fact_set =
            match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
                Ok(fs) => fs,
                Err(e) => {
                    return serde_json::json!({ "error": format!("fact assembly error: {}", e) })
                        .to_string()
                }
            };

        let verdict_set = match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
            Ok(vs) => vs,
            Err(e) => {
                return serde_json::json!({ "error": format!("evaluation error: {}", e) })
                    .to_string()
            }
        };

        verdict_set.to_json().to_string()
    });
}

/// Compute the action space for a persona.
///
/// Args:   handle, facts_ptr, facts_len, entity_states_ptr, entity_states_len, persona_ptr, persona_len
/// Result: ActionSpace JSON or `{"error": "..."}`
#[no_mangle]
pub unsafe extern "C" fn compute_action_space(
    handle: u32,
    facts_ptr: *const u8,
    facts_len: u32,
    states_ptr: *const u8,
    states_len: u32,
    persona_ptr: *const u8,
    persona_len: u32,
) {
    let facts_str =
        match std::str::from_utf8(std::slice::from_raw_parts(facts_ptr, facts_len as usize)) {
            Ok(s) => s,
            Err(e) => {
                error_result(&format!("invalid UTF-8 in facts: {}", e));
                return;
            }
        };

    let states_str =
        match std::str::from_utf8(std::slice::from_raw_parts(states_ptr, states_len as usize)) {
            Ok(s) => s,
            Err(e) => {
                error_result(&format!("invalid UTF-8 in entity_states: {}", e));
                return;
            }
        };

    let persona_str =
        match std::str::from_utf8(std::slice::from_raw_parts(persona_ptr, persona_len as usize)) {
            Ok(s) => s,
            Err(e) => {
                error_result(&format!("invalid UTF-8 in persona: {}", e));
                return;
            }
        };

    let facts: serde_json::Value = match serde_json::from_str(facts_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid facts JSON: {}", e));
            return;
        }
    };

    let entity_states_val: serde_json::Value = match serde_json::from_str(states_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid entity_states JSON: {}", e));
            return;
        }
    };

    with_contract(handle, |stored| {
        let entity_states = match parse_entity_states(&entity_states_val) {
            Ok(s) => s,
            Err(e) => {
                return serde_json::json!({ "error": format!("invalid entity states: {}", e) })
                    .to_string()
            }
        };

        let result = tenor_eval::action_space::compute_action_space(
            &stored.contract,
            &facts,
            &entity_states,
            persona_str,
        );

        match result {
            Ok(action_space) => match serde_json::to_string(&action_space) {
                Ok(json) => json,
                Err(e) => {
                    serde_json::json!({ "error": format!("serialization error: {}", e) })
                        .to_string()
                }
            },
            Err(e) => {
                serde_json::json!({ "error": format!("action space error: {}", e) }).to_string()
            }
        }
    });
}

/// Simulate a flow execution.
///
/// Args:   handle, flow_id_ptr, flow_id_len, persona_ptr, persona_len,
///         facts_ptr, facts_len, entity_states_ptr, entity_states_len
/// Result: FlowResult JSON or `{"error": "..."}`
///
/// Uses _default instance for backward compatibility (no instance_bindings).
#[no_mangle]
pub unsafe extern "C" fn simulate_flow(
    handle: u32,
    flow_id_ptr: *const u8,
    flow_id_len: u32,
    persona_ptr: *const u8,
    persona_len: u32,
    facts_ptr: *const u8,
    facts_len: u32,
    states_ptr: *const u8,
    states_len: u32,
) {
    simulate_flow_with_bindings_impl(
        handle,
        flow_id_ptr,
        flow_id_len,
        persona_ptr,
        persona_len,
        facts_ptr,
        facts_len,
        states_ptr,
        states_len,
        std::ptr::null(),
        0,
    );
}

/// Simulate a flow with explicit instance bindings.
///
/// Args:   handle, flow_id_ptr, flow_id_len, persona_ptr, persona_len,
///         facts_ptr, facts_len, entity_states_ptr, entity_states_len,
///         instance_bindings_ptr, instance_bindings_len
/// Result: FlowResult JSON or `{"error": "..."}`
#[no_mangle]
pub unsafe extern "C" fn simulate_flow_with_bindings(
    handle: u32,
    flow_id_ptr: *const u8,
    flow_id_len: u32,
    persona_ptr: *const u8,
    persona_len: u32,
    facts_ptr: *const u8,
    facts_len: u32,
    states_ptr: *const u8,
    states_len: u32,
    bindings_ptr: *const u8,
    bindings_len: u32,
) {
    simulate_flow_with_bindings_impl(
        handle,
        flow_id_ptr,
        flow_id_len,
        persona_ptr,
        persona_len,
        facts_ptr,
        facts_len,
        states_ptr,
        states_len,
        bindings_ptr,
        bindings_len,
    );
}

#[allow(clippy::too_many_arguments)]
unsafe fn simulate_flow_with_bindings_impl(
    handle: u32,
    flow_id_ptr: *const u8,
    flow_id_len: u32,
    persona_ptr: *const u8,
    persona_len: u32,
    facts_ptr: *const u8,
    facts_len: u32,
    states_ptr: *const u8,
    states_len: u32,
    bindings_ptr: *const u8,
    bindings_len: u32,
) {
    macro_rules! parse_str {
        ($ptr:expr, $len:expr, $name:expr) => {
            if $len == 0 || $ptr.is_null() {
                ""
            } else {
                match std::str::from_utf8(std::slice::from_raw_parts($ptr, $len as usize)) {
                    Ok(s) => s,
                    Err(e) => {
                        error_result(&format!("invalid UTF-8 in {}: {}", $name, e));
                        return;
                    }
                }
            }
        };
    }

    let flow_id_str = parse_str!(flow_id_ptr, flow_id_len, "flow_id");
    let persona_str = parse_str!(persona_ptr, persona_len, "persona");
    let facts_str = parse_str!(facts_ptr, facts_len, "facts");
    let states_str = parse_str!(states_ptr, states_len, "entity_states");
    let bindings_str = parse_str!(bindings_ptr, bindings_len, "instance_bindings");

    let facts: serde_json::Value = match serde_json::from_str(facts_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid facts JSON: {}", e));
            return;
        }
    };

    let entity_states_val: serde_json::Value = match serde_json::from_str(states_str) {
        Ok(v) => v,
        Err(e) => {
            error_result(&format!("invalid entity_states JSON: {}", e));
            return;
        }
    };

    let instance_bindings = match parse_instance_bindings(bindings_str) {
        Ok(b) => b,
        Err(e) => {
            error_result(&e);
            return;
        }
    };

    with_contract(handle, |stored| {
        let entity_states = match parse_entity_states(&entity_states_val) {
            Ok(s) => s,
            Err(e) => {
                return serde_json::json!({ "error": format!("invalid entity states: {}", e) })
                    .to_string()
            }
        };

        let fact_set = match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
            Ok(fs) => fs,
            Err(e) => {
                return serde_json::json!({ "error": format!("fact assembly error: {}", e) })
                    .to_string()
            }
        };

        let verdict_set = match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
            Ok(vs) => vs,
            Err(e) => {
                return serde_json::json!({ "error": format!("evaluation error: {}", e) })
                    .to_string()
            }
        };

        let snapshot = tenor_eval::Snapshot {
            facts: fact_set,
            verdicts: verdict_set.clone(),
        };

        let mut merged_entity_states =
            tenor_eval::operation::init_entity_states(&stored.contract);
        for (key, state) in entity_states {
            merged_entity_states.insert(key, state);
        }

        let target_flow = match stored.contract.get_flow(flow_id_str) {
            Some(f) => f,
            None => {
                return serde_json::json!({ "error": format!("flow '{}' not found", flow_id_str) })
                    .to_string()
            }
        };

        let flow_result = match tenor_eval::flow::execute_flow(
            target_flow,
            &stored.contract,
            &snapshot,
            &mut merged_entity_states,
            &instance_bindings,
            None,
        ) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::json!({ "error": format!("flow execution error: {}", e) })
                    .to_string()
            }
        };

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
                    step_json["instance_bindings"] =
                        serde_json::to_value(&s.instance_bindings)
                            .unwrap_or(serde_json::Value::Null);
                }
                step_json
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

        serde_json::json!({
            "simulation": true,
            "flow_id": flow_id_str,
            "persona": persona_str,
            "outcome": flow_result.outcome,
            "path": path,
            "would_transition": would_transition,
            "verdicts": verdict_set.to_json()["verdicts"],
            "instance_bindings": instance_bindings,
        })
        .to_string()
    });
}
