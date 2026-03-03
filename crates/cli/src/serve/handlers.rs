//! Core HTTP route handlers: health, contracts, elaborate, evaluate, explain.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use super::state::AppState;
use super::{json_error, MAX_SOURCE_SIZE};

/// Fallback handler for unmatched routes.
pub(crate) async fn handle_not_found() -> impl IntoResponse {
    json_error(StatusCode::NOT_FOUND, "not found")
}

/// GET /health
pub(crate) async fn handle_health() -> impl IntoResponse {
    let response = serde_json::json!({
        "status": "ok",
        "tenor_version": tenor_core::TENOR_BUNDLE_VERSION,
    });
    (StatusCode::OK, Json(response))
}

/// GET /contracts
pub(crate) async fn handle_list_contracts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let contract_list: Vec<serde_json::Value> = contracts
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

    let response = serde_json::json!({ "contracts": contract_list });
    (StatusCode::OK, Json(response))
}

/// GET /contracts/{id}/operations
pub(crate) async fn handle_get_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let bundle = match contracts.get(&id) {
        Some(b) => b,
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", id),
            )
            .into_response()
        }
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
    (StatusCode::OK, Json(response)).into_response()
}

/// Sanitize a filename for use in a temp directory.
///
/// Rejects filenames with path separators or `..` components.
/// Returns `source.tenor` for invalid or missing filenames.
fn sanitize_filename(raw: Option<&str>) -> String {
    let name = match raw {
        Some(n) if !n.is_empty() => n,
        _ => return "source.tenor".to_string(),
    };

    // Reject path separators and parent directory references
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return "source.tenor".to_string();
    }

    name.to_string()
}

/// Check if an import path would escape the sandbox directory.
///
/// Scans source text for `import` directives whose paths contain
/// parent-directory traversals (`..`). This is a fast pre-check before
/// writing to disk; pass1_bundle.rs performs full canonicalization later.
fn has_escaping_imports(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("import") {
            // Check for .. in the import path
            if rest.contains("..") {
                return true;
            }
        }
    }
    false
}

/// POST /elaborate
pub(crate) async fn handle_elaborate(Json(parsed): Json<serde_json::Value>) -> impl IntoResponse {
    let source = match parsed.get("source").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'source' field").into_response()
        }
    };

    // --- Input validation (before any disk I/O) ---

    // 1. Content size validation
    if source.len() > MAX_SOURCE_SIZE {
        return json_error(
            StatusCode::BAD_REQUEST,
            "source content exceeds maximum size",
        )
        .into_response();
    }

    // 2. Null byte rejection
    if source.contains('\0') {
        return json_error(
            StatusCode::BAD_REQUEST,
            "source content must not contain null bytes",
        )
        .into_response();
    }

    // 3. Import injection prevention
    if has_escaping_imports(&source) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "import paths must not escape sandbox",
        )
        .into_response();
    }

    // 4. Filename sanitization
    let filename = sanitize_filename(parsed.get("filename").and_then(|v| v.as_str()));

    let result = tokio::task::spawn_blocking(move || {
        let tmp_dir = tempfile::tempdir()?;
        let tmp_path = tmp_dir.path().join(&filename);
        std::fs::write(&tmp_path, &source)?;
        let elab_result = tenor_core::elaborate::elaborate(&tmp_path);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(elab_result)
    })
    .await;

    match result {
        Ok(Ok(Ok(bundle))) => (StatusCode::OK, Json(bundle)).into_response(),
        Ok(Ok(Err(e))) => {
            let err_response = serde_json::json!({
                "error": format!("{:?}", e),
                "details": e.to_json_value(),
            });
            (StatusCode::BAD_REQUEST, Json(err_response)).into_response()
        }
        Ok(Err(e)) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to elaborate: {}", e),
        )
        .into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

/// POST /evaluate
pub(crate) async fn handle_evaluate(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'bundle_id' field").into_response()
        }
    };

    let facts = match parsed.get("facts") {
        Some(f) => f.clone(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'facts' field").into_response()
        }
    };

    let flow_id = parsed
        .get("flow_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let persona = parsed
        .get("persona")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let contracts = state.contracts.read().await;
    let bundle = match contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", bundle_id),
            )
            .into_response()
        }
    };
    drop(contracts);

    if let Some(fid) = flow_id {
        let p = match persona {
            Some(p) => p,
            None => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "'persona' is required when 'flow_id' is specified",
                )
                .into_response()
            }
        };

        let fid_for_response = fid.clone();

        let result = tokio::task::spawn_blocking(move || {
            tenor_eval::evaluate_flow(
                &bundle,
                &facts,
                &fid,
                &p,
                None,
                &tenor_eval::InstanceBindingMap::new(),
            )
        })
        .await;

        match result {
            Ok(Ok(result)) => {
                let mut json_output = serde_json::Map::new();
                json_output.insert("flow_id".to_string(), serde_json::json!(fid_for_response));
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
                (StatusCode::OK, Json(serde_json::Value::Object(json_output))).into_response()
            }
            Ok(Err(e)) => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)).into_response()
            }
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("task join error: {}", e),
            )
            .into_response(),
        }
    } else {
        let result =
            tokio::task::spawn_blocking(move || tenor_eval::evaluate(&bundle, &facts)).await;

        match result {
            Ok(Ok(result)) => (StatusCode::OK, Json(result.verdicts.to_json())).into_response(),
            Ok(Err(e)) => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)).into_response()
            }
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("task join error: {}", e),
            )
            .into_response(),
        }
    }
}

/// POST /explain
pub(crate) async fn handle_explain(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'bundle_id' field").into_response()
        }
    };

    let contracts = state.contracts.read().await;
    let bundle = match contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", bundle_id),
            )
            .into_response()
        }
    };
    drop(contracts);

    let result =
        tokio::task::spawn_blocking(move || super::super::explain::explain_bundle(&bundle)).await;

    match result {
        Ok(Ok(explanation)) => (StatusCode::OK, Json(explanation)).into_response(),
        Ok(Err(e)) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("explain error: {}", e),
        )
        .into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}
