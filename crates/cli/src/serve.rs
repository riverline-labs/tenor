//! `tenor serve` -- HTTP JSON API server for the Tenor evaluator.
//!
//! Exposes the Rust elaborator and evaluator as a synchronous HTTP service
//! using `tiny_http`. No async runtime required.
//!
//! Endpoints:
//! - GET  /health                  - Server status
//! - GET  /contracts               - List loaded contract bundles
//! - GET  /contracts/{id}/operations - Operations for a specific contract
//! - POST /elaborate               - Elaborate .tenor source text
//! - POST /evaluate                - Evaluate a contract against facts
//! - POST /explain                 - Explain a contract bundle
//!
//! All responses use Content-Type: application/json.

use std::collections::HashMap;
use std::io::Read as IoRead;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::explain;

/// Maximum request body size: 10 MB.
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Server state shared across request handlers.
struct ServeState {
    /// Loaded contract bundles keyed by bundle ID.
    contracts: HashMap<String, serde_json::Value>,
}

/// Start the HTTP server on the given port, optionally pre-loading contracts.
pub fn start_server(port: u16, contract_paths: Vec<PathBuf>) {
    let mut state = ServeState {
        contracts: HashMap::new(),
    };

    // Pre-load contracts
    for path in &contract_paths {
        match tenor_core::elaborate::elaborate(path) {
            Ok(bundle) => {
                let bundle_id = bundle
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                eprintln!("Loaded contract: {} (from {})", bundle_id, path.display());
                state.contracts.insert(bundle_id, bundle);
            }
            Err(e) => {
                eprintln!("Warning: failed to load {}: {:?}", path.display(), e);
            }
        }
    }

    let addr = format!("0.0.0.0:{}", port);
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    eprintln!("Tenor evaluator listening on http://0.0.0.0:{}", port);

    let state = Arc::new(Mutex::new(state));

    // Install signal handlers for graceful shutdown.
    SHUTDOWN_FLAG.store(false, std::sync::atomic::Ordering::SeqCst);
    install_signal_handlers();

    // Request loop: use recv_timeout so we can check the shutdown flag periodically.
    loop {
        if SHUTDOWN_FLAG.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let request = match server.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Some(r)) => r,
            Ok(None) => continue, // Timeout, check shutdown flag
            Err(_) => break,      // Server error
        };

        let state = Arc::clone(&state);
        handle_request(request, &state);
    }

    eprintln!("\nServer shut down.");
}

static SHUTDOWN_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Install signal handlers for SIGINT and SIGTERM that set SHUTDOWN_FLAG.
fn install_signal_handlers() {
    unsafe {
        libc::signal(
            libc::SIGINT,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            signal_handler as *const () as libc::sighandler_t,
        );
    }
}

extern "C" fn signal_handler(_sig: libc::c_int) {
    SHUTDOWN_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Route a request to the appropriate handler.
fn handle_request(mut request: tiny_http::Request, state: &Arc<Mutex<ServeState>>) {
    let method = request.method().to_string();
    let url = request.url().to_string();

    // Parse the path (strip query string)
    let path = url.split('?').next().unwrap_or(&url);

    let (status, body) = match (method.as_str(), path) {
        ("GET", "/health") => handle_health(),
        ("GET", "/contracts") => handle_list_contracts(state),
        ("GET", p) if p.starts_with("/contracts/") && p.ends_with("/operations") => {
            let id = &p["/contracts/".len()..p.len() - "/operations".len()];
            handle_get_operations(state, id)
        }
        ("POST", "/elaborate") => {
            let body = read_body(&mut request);
            match body {
                Ok(b) => handle_elaborate(&b),
                Err(e) => (400, json_error(&e)),
            }
        }
        ("POST", "/evaluate") => {
            let body = read_body(&mut request);
            match body {
                Ok(b) => handle_evaluate(state, &b),
                Err(e) => (400, json_error(&e)),
            }
        }
        ("POST", "/explain") => {
            let body = read_body(&mut request);
            match body {
                Ok(b) => handle_explain(state, &b),
                Err(e) => (400, json_error(&e)),
            }
        }
        _ => (404, json_error("not found")),
    };

    let response = tiny_http::Response::from_string(body)
        .with_status_code(status)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                .expect("valid header"),
        );

    let _ = request.respond(response);
}

/// Read the request body up to MAX_BODY_SIZE.
fn read_body(request: &mut tiny_http::Request) -> Result<String, String> {
    let content_length = request.body_length().unwrap_or(0);

    if content_length > MAX_BODY_SIZE {
        return Err(format!(
            "request body too large: {} bytes (max {})",
            content_length, MAX_BODY_SIZE
        ));
    }

    let mut body = Vec::with_capacity(content_length.min(65536));
    request
        .as_reader()
        .take(MAX_BODY_SIZE as u64)
        .read_to_end(&mut body)
        .map_err(|e| format!("failed to read request body: {}", e))?;

    String::from_utf8(body).map_err(|e| format!("invalid UTF-8 in request body: {}", e))
}

/// Construct a JSON error response.
fn json_error(message: &str) -> String {
    serde_json::json!({"error": message}).to_string()
}

// ─── Route handlers ─────────────────────────────────────────────────────────

/// GET /health
fn handle_health() -> (u16, String) {
    let response = serde_json::json!({
        "status": "ok",
        "tenor_version": tenor_core::TENOR_BUNDLE_VERSION,
    });
    (
        200,
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )
}

/// GET /contracts
fn handle_list_contracts(state: &Arc<Mutex<ServeState>>) -> (u16, String) {
    let state = state.lock().expect("lock poisoned");

    let contracts: Vec<serde_json::Value> = state
        .contracts
        .iter()
        .map(|(id, bundle)| {
            let constructs = bundle
                .get("constructs")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            let mut facts = Vec::new();
            let mut operations = Vec::new();
            let mut flows = Vec::new();
            let mut construct_count = 0;

            for c in &constructs {
                construct_count += 1;
                let kind = c.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let cid = c.get("id").and_then(|i| i.as_str()).unwrap_or("");
                match kind {
                    "Fact" => facts.push(cid.to_string()),
                    "Operation" => operations.push(cid.to_string()),
                    "Flow" => flows.push(cid.to_string()),
                    _ => {}
                }
            }

            serde_json::json!({
                "id": id,
                "construct_count": construct_count,
                "facts": facts,
                "operations": operations,
                "flows": flows,
            })
        })
        .collect();

    let response = serde_json::json!({ "contracts": contracts });
    (
        200,
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )
}

/// GET /contracts/{id}/operations
fn handle_get_operations(state: &Arc<Mutex<ServeState>>, id: &str) -> (u16, String) {
    let state = state.lock().expect("lock poisoned");

    let bundle = match state.contracts.get(id) {
        Some(b) => b,
        None => return (404, json_error(&format!("contract '{}' not found", id))),
    };

    let constructs = bundle
        .get("constructs")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let operations: Vec<serde_json::Value> = constructs
        .iter()
        .filter(|c| c.get("kind").and_then(|k| k.as_str()) == Some("Operation"))
        .map(|op| {
            let op_id = op.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let allowed_personas: Vec<String> = op
                .get("allowed_personas")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let effects: Vec<serde_json::Value> = op
                .get("effects")
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|eff| {
                            serde_json::json!({
                                "entity_id": eff.get("entity_id").and_then(|v| v.as_str()).unwrap_or(""),
                                "from": eff.get("from").and_then(|v| v.as_str()).unwrap_or(""),
                                "to": eff.get("to").and_then(|v| v.as_str()).unwrap_or(""),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Build a preconditions summary from effects
            let summary: Vec<String> = effects
                .iter()
                .map(|e| {
                    format!(
                        "{} -> {}",
                        e.get("from").and_then(|v| v.as_str()).unwrap_or("?"),
                        e.get("to").and_then(|v| v.as_str()).unwrap_or("?"),
                    )
                })
                .collect();

            serde_json::json!({
                "id": op_id,
                "allowed_personas": allowed_personas,
                "effects": effects,
                "preconditions_summary": summary.join(", "),
            })
        })
        .collect();

    let response = serde_json::json!({ "operations": operations });
    (
        200,
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    )
}

/// POST /elaborate
fn handle_elaborate(body: &str) -> (u16, String) {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return (400, json_error(&format!("invalid JSON: {}", e))),
    };

    let source = match parsed.get("source").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (400, json_error("missing 'source' field")),
    };

    let filename = parsed
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("input.tenor");

    // Write to a unique temp file and elaborate
    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return (
                500,
                json_error(&format!("failed to create temp dir: {}", e)),
            )
        }
    };
    let tmp_path = tmp_dir.path().join(filename);

    if let Err(e) = std::fs::write(&tmp_path, source) {
        return (
            500,
            json_error(&format!("failed to write temp file: {}", e)),
        );
    }

    let result = tenor_core::elaborate::elaborate(&tmp_path);
    // tmp_dir is dropped here, cleaning up automatically

    match result {
        Ok(bundle) => (
            200,
            serde_json::to_string_pretty(&bundle).unwrap_or_default(),
        ),
        Err(e) => {
            let err_response = serde_json::json!({
                "error": format!("{:?}", e),
                "details": e.to_json_value(),
            });
            (
                400,
                serde_json::to_string_pretty(&err_response).unwrap_or_default(),
            )
        }
    }
}

/// POST /evaluate
fn handle_evaluate(state: &Arc<Mutex<ServeState>>, body: &str) -> (u16, String) {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return (400, json_error(&format!("invalid JSON: {}", e))),
    };

    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return (400, json_error("missing 'bundle_id' field")),
    };

    let facts = match parsed.get("facts") {
        Some(f) => f.clone(),
        None => return (400, json_error("missing 'facts' field")),
    };

    let flow_id = parsed.get("flow_id").and_then(|v| v.as_str());
    let persona = parsed.get("persona").and_then(|v| v.as_str());

    let state = state.lock().expect("lock poisoned");
    let bundle = match state.contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return (
                404,
                json_error(&format!("contract '{}' not found", bundle_id)),
            )
        }
    };
    // Release the lock before evaluation
    drop(state);

    // Flow evaluation
    if let Some(fid) = flow_id {
        let p = match persona {
            Some(p) => p,
            None => {
                return (
                    400,
                    json_error("'persona' is required when 'flow_id' is specified"),
                )
            }
        };

        match tenor_eval::evaluate_flow(&bundle, &facts, fid, p) {
            Ok(result) => {
                let mut json_output = serde_json::Map::new();
                json_output.insert("flow_id".to_string(), serde_json::json!(fid));
                json_output.insert(
                    "outcome".to_string(),
                    serde_json::json!(result.flow_result.outcome),
                );
                json_output.insert(
                    "initiating_persona".to_string(),
                    serde_json::json!(result.flow_result.initiating_persona),
                );
                let entity_changes: serde_json::Value = result
                    .flow_result
                    .entity_state_changes
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "entity_id": e.entity_id,
                            "from": e.from_state,
                            "to": e.to_state
                        })
                    })
                    .collect();
                json_output.insert("entity_state_changes".to_string(), entity_changes);
                let steps: serde_json::Value = result
                    .flow_result
                    .steps_executed
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "step_id": s.step_id,
                            "result": s.result
                        })
                    })
                    .collect();
                json_output.insert("steps_executed".to_string(), steps);
                json_output.insert("verdicts".to_string(), result.verdicts.to_json());
                (
                    200,
                    serde_json::to_string_pretty(&serde_json::Value::Object(json_output))
                        .unwrap_or_default(),
                )
            }
            Err(e) => (
                500,
                serde_json::json!({"error": format!("{}", e)}).to_string(),
            ),
        }
    } else {
        // Rule-only evaluation
        match tenor_eval::evaluate(&bundle, &facts) {
            Ok(result) => (
                200,
                serde_json::to_string_pretty(&result.verdicts.to_json()).unwrap_or_default(),
            ),
            Err(e) => (
                500,
                serde_json::json!({"error": format!("{}", e)}).to_string(),
            ),
        }
    }
}

/// POST /explain
fn handle_explain(state: &Arc<Mutex<ServeState>>, body: &str) -> (u16, String) {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return (400, json_error(&format!("invalid JSON: {}", e))),
    };

    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return (400, json_error("missing 'bundle_id' field")),
    };

    let state = state.lock().expect("lock poisoned");
    let bundle = match state.contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return (
                404,
                json_error(&format!("contract '{}' not found", bundle_id)),
            )
        }
    };
    drop(state);

    // Use the explain module to generate a markdown explanation
    match explain::explain_bundle(&bundle) {
        Ok(explanation) => (
            200,
            serde_json::to_string_pretty(&explanation).unwrap_or_default(),
        ),
        Err(e) => (500, json_error(&format!("explain error: {}", e))),
    }
}
