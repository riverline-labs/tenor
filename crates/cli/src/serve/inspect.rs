//! Contract inspection and manifest handlers.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::json_error;
use super::state::AppState;

/// GET /.well-known/tenor
///
/// Contract manifest endpoint per spec S19. Returns the TenorManifest JSON
/// with the full interchange bundle inlined. Sets ETag response header and
/// supports If-None-Match for conditional requests (304 Not Modified).
/// Satisfies executor obligations E10, E11, E12.
pub(crate) async fn handle_well_known_tenor(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let contracts = state.contracts.read().await;

    let bundle = match contracts.values().next() {
        Some(b) => b.clone(),
        None => return json_error(StatusCode::NOT_FOUND, "no contracts loaded").into_response(),
    };
    drop(contracts);

    let manifest_value = super::super::manifest::build_manifest(bundle);
    let etag = manifest_value
        .get("etag")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let etag_quoted = format!("\"{}\"", etag);

    // Check If-None-Match
    if let Some(inm) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(inm_str) = inm.to_str() {
            if inm_str == etag_quoted || inm_str == etag {
                return StatusCode::NOT_MODIFIED.into_response();
            }
        }
    }

    let mut response = Json(manifest_value).into_response();
    if let Ok(val) = etag_quoted.parse() {
        response.headers_mut().insert(header::ETAG, val);
    }
    response
}

/// GET /inspect
///
/// Structured summary of the loaded contract. Returns all declared constructs
/// with their key properties -- enough for an agent to cold-start or a UI to
/// render a contract explorer.
pub(crate) async fn handle_inspect(State(state): State<Arc<AppState>>) -> Response {
    let contracts = state.contracts.read().await;

    let mut all_facts = Vec::new();
    let mut all_entities = Vec::new();
    let mut all_rules = Vec::new();
    let mut all_personas = Vec::new();
    let mut all_operations = Vec::new();
    let mut all_flows = Vec::new();
    let mut etag_bundle = None;

    for bundle in contracts.values() {
        if etag_bundle.is_none() {
            etag_bundle = Some(bundle.clone());
        }

        let parsed = match tenor_interchange::from_interchange(bundle) {
            Ok(p) => p,
            Err(e) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to parse bundle: {}", e),
                )
                .into_response()
            }
        };

        for construct in &parsed.constructs {
            match construct {
                tenor_interchange::InterchangeConstruct::Fact(f) => {
                    let base = f
                        .fact_type
                        .get("base")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let mut fact_json = serde_json::json!({
                        "id": f.id,
                        "type": base,
                    });
                    if let Some(ref src) = f.source {
                        fact_json["source"] = src.clone();
                    }
                    if f.default.is_some() {
                        fact_json["has_default"] = serde_json::json!(true);
                    }
                    // Include full type spec for richer inspection
                    fact_json["type_spec"] = f.fact_type.clone();
                    all_facts.push(fact_json);
                }
                tenor_interchange::InterchangeConstruct::Entity(e) => {
                    let transitions: Vec<serde_json::Value> = e
                        .transitions
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "from": t.from,
                                "to": t.to,
                            })
                        })
                        .collect();
                    all_entities.push(serde_json::json!({
                        "id": e.id,
                        "states": e.states,
                        "initial": e.initial,
                        "transitions": transitions,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Rule(r) => {
                    let condition_summary = r.when().map(summarize_condition).unwrap_or_default();
                    let verdict_type = r
                        .produce()
                        .and_then(|p| p.get("verdict_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    all_rules.push(serde_json::json!({
                        "id": r.id,
                        "stratum": r.stratum,
                        "produces": verdict_type,
                        "condition_summary": condition_summary,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Persona(p) => {
                    all_personas.push(serde_json::json!({ "id": p.id }));
                }
                tenor_interchange::InterchangeConstruct::Operation(op) => {
                    let effects: Vec<serde_json::Value> = op
                        .effects
                        .iter()
                        .map(|e| {
                            serde_json::json!({
                                "entity_id": e.entity_id,
                                "from": e.from,
                                "to": e.to,
                            })
                        })
                        .collect();
                    let precondition_summary = op
                        .precondition
                        .as_ref()
                        .map(summarize_condition)
                        .unwrap_or_else(|| "none".to_string());
                    all_operations.push(serde_json::json!({
                        "id": op.id,
                        "allowed_personas": op.allowed_personas,
                        "effects": effects,
                        "precondition_summary": precondition_summary,
                        "outcomes": op.outcomes,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Flow(f) => {
                    let step_ids: Vec<&str> = f
                        .steps
                        .iter()
                        .filter_map(|s| s.get("id").and_then(|v| v.as_str()))
                        .collect();
                    all_flows.push(serde_json::json!({
                        "id": f.id,
                        "entry": f.entry,
                        "steps": step_ids,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Source(_)
                | tenor_interchange::InterchangeConstruct::System(_)
                | tenor_interchange::InterchangeConstruct::TypeDecl(_) => {}
            }
        }
    }
    drop(contracts);

    let etag = etag_bundle
        .map(|b| super::super::manifest::compute_etag(&b))
        .unwrap_or_default();

    let response = serde_json::json!({
        "facts": all_facts,
        "entities": all_entities,
        "rules": all_rules,
        "personas": all_personas,
        "operations": all_operations,
        "flows": all_flows,
        "etag": etag,
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Produce a human-readable summary of a condition expression for inspect output.
fn summarize_condition(cond: &serde_json::Value) -> String {
    if cond.is_null() {
        return "always".to_string();
    }
    if let Some(vp) = cond.get("verdict_present").and_then(|v| v.as_str()) {
        return format!("verdict '{}' present", vp);
    }
    if let Some(op) = cond.get("op").and_then(|v| v.as_str()) {
        let left = cond
            .get("left")
            .and_then(|l| l.get("fact_ref").and_then(|v| v.as_str()))
            .unwrap_or("...");
        let right = cond
            .get("right")
            .and_then(|r| {
                r.get("literal")
                    .map(|l| format!("{}", l))
                    .or_else(|| r.get("fact_ref").and_then(|v| v.as_str()).map(String::from))
            })
            .unwrap_or_else(|| "...".to_string());
        return format!("{} {} {}", left, op, right);
    }
    "complex".to_string()
}
