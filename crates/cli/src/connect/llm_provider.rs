//! LLM-based matching provider — uses the Anthropic Messages API to propose
//! fact-to-source mappings with semantic understanding.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::matching::Confidence;
use super::provider::{
    EnvironmentInventory, FactDeclaration, MappingProposal, MatchingError, MatchingProvider,
};

/// Anthropic Messages API endpoint.
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Required API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default model to use for LLM matching.
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Configuration for the LLM matching provider.
#[derive(Debug, Clone)]
pub struct LlmMatchingConfig {
    /// Anthropic API key.
    pub api_key: String,
    /// Model identifier (defaults to claude-sonnet-4-20250514).
    pub model: String,
}

impl LlmMatchingConfig {
    /// Create a new config with the given API key and default model.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Create a new config with the given API key and model.
    pub fn with_model(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

/// An LLM-powered matching provider that calls the Anthropic Messages API
/// to propose fact-to-source mappings.
pub struct LlmMatchingProvider {
    config: LlmMatchingConfig,
}

impl LlmMatchingProvider {
    /// Create a new LLM matching provider with the given configuration.
    pub fn new(config: LlmMatchingConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MatchingProvider for LlmMatchingProvider {
    async fn propose_mappings(
        &self,
        facts: &[FactDeclaration],
        environment: &EnvironmentInventory,
    ) -> Result<Vec<MappingProposal>, MatchingError> {
        if facts.is_empty() {
            return Err(MatchingError::EmptyInput(
                "no fact declarations provided".to_string(),
            ));
        }

        let system_prompt = build_system_prompt();
        let user_prompt = build_user_prompt(facts, environment);

        // ureq is synchronous, so wrap in spawn_blocking
        let api_key = self.config.api_key.clone();
        let model = self.config.model.clone();

        let response_text = tokio::task::spawn_blocking(move || {
            call_anthropic_api(&api_key, &model, &system_prompt, &user_prompt)
        })
        .await
        .map_err(|e| MatchingError::Internal(format!("task join error: {}", e)))?
        .map_err(|e| MatchingError::ApiError(e.to_string()))?;

        parse_llm_response(&response_text)
    }
}

// ── Prompt construction ──────────────────────────────────────────────────────

/// Build the system prompt instructing the model to return JSON mappings.
fn build_system_prompt() -> String {
    r#"You are a Tenor contract language expert helping map declared facts to external API fields.

Given a list of facts (with types and source references) and an environment inventory
(external API schemas with endpoints and field types), propose the best mapping for each fact.

Return ONLY a JSON array. No explanation, no markdown, no code fences. Each element must be:
{
  "fact_id": "<fact identifier>",
  "source_id": "<source identifier>",
  "endpoint": "<API endpoint path, e.g. /orders/{id}>",
  "field_path": "<dot-delimited field path in the response, e.g. balance.amount>",
  "confidence": "HIGH" | "MEDIUM" | "LOW",
  "explanation": "<brief explanation of why this mapping is appropriate>"
}

Rules for confidence:
- HIGH: exact type match AND clear path correspondence
- MEDIUM: type is compatible (e.g. Int vs Number) OR path is similar but not exact
- LOW: only weak heuristic evidence, or the field doesn't exist in the schema

Rules for matching:
- Prefer type-compatible mappings (Int->integer, Money->number/object, Text->string, Bool->boolean)
- Path hints in the fact declaration (e.g. "orders.balance") suggest where to look
- Consider semantic meaning: "order_balance" likely maps to a "balance" field in an orders endpoint
- For structured sources (http, database), prefer exact field matches over inferred ones
- For freetext sources (manual, static), use LOW confidence

Return one mapping per fact. If no reasonable mapping exists, return the fact with confidence LOW
and an explanation of why no mapping was found."#
        .to_string()
}

/// Build the user prompt containing facts and environment inventory.
fn build_user_prompt(facts: &[FactDeclaration], environment: &EnvironmentInventory) -> String {
    let mut prompt = String::new();

    // Facts section
    prompt.push_str("## Facts to map\n\n");
    for fact in facts {
        prompt.push_str(&format!(
            "- **{}**: type={}, source={}, path={}\n",
            fact.fact_id, fact.base_type, fact.source_id, fact.path
        ));
        if fact.full_type != serde_json::json!({"base": fact.base_type}) {
            prompt.push_str(&format!(
                "  Full type: {}\n",
                serde_json::to_string(&fact.full_type).unwrap_or_default()
            ));
        }
    }
    prompt.push('\n');

    // Environment section
    prompt.push_str("## Environment inventory\n\n");
    if environment.schemas.is_empty() {
        prompt.push_str("No external schemas available. Use LOW confidence for all mappings.\n");
    } else {
        for (source_id, schema) in &environment.schemas {
            prompt.push_str(&format!("### Source: {}\n\n", source_id));
            for endpoint in &schema.endpoints {
                prompt.push_str(&format!("**{} {}**\n", endpoint.method, endpoint.path));
                if !endpoint.parameters.is_empty() {
                    prompt.push_str(&format!(
                        "  Parameters: {}\n",
                        endpoint.parameters.join(", ")
                    ));
                }
                if endpoint.response_fields.is_empty() {
                    prompt.push_str("  Response: (no fields extracted)\n");
                } else {
                    prompt.push_str("  Response fields:\n");
                    for field in &endpoint.response_fields {
                        prompt.push_str(&format!("    - {} : {}\n", field.path, field.field_type));
                    }
                }
                prompt.push('\n');
            }
        }
    }

    prompt
}

// ── API call ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessage>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[allow(dead_code)] // Required by serde for correct JSON deserialization
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

/// Make a synchronous call to the Anthropic Messages API.
fn call_anthropic_api(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, MatchingError> {
    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: 4096,
        system: system_prompt.to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: user_prompt.to_string(),
        }],
    };

    let agent = ureq::Agent::new_with_defaults();
    let response = agent
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .send_json(&request_body)
        .map_err(|e| MatchingError::ApiError(format!("API request failed: {}", e)))?;

    let resp: MessagesResponse = response
        .into_body()
        .read_json()
        .map_err(|e| MatchingError::ParseError(format!("failed to parse API response: {}", e)))?;

    resp.content
        .first()
        .and_then(|block| block.text.clone())
        .ok_or_else(|| {
            MatchingError::ParseError("API response contained no text content".to_string())
        })
}

// ── Response parsing ─────────────────────────────────────────────────────────

/// A single mapping entry from the LLM's JSON response.
#[derive(Deserialize, Debug)]
struct LlmMappingEntry {
    fact_id: String,
    source_id: String,
    endpoint: String,
    field_path: String,
    confidence: String,
    explanation: String,
}

/// Parse the LLM response text into `MappingProposal` entries.
fn parse_llm_response(response_text: &str) -> Result<Vec<MappingProposal>, MatchingError> {
    let trimmed = response_text.trim();

    // Strip markdown code fences if present
    let json_str = strip_code_fences(trimmed);

    let entries: Vec<LlmMappingEntry> = serde_json::from_str(json_str).map_err(|e| {
        MatchingError::ParseError(format!(
            "failed to parse LLM response as JSON array: {}. Response was: {}",
            e,
            truncate(trimmed, 200)
        ))
    })?;

    if entries.is_empty() {
        return Err(MatchingError::ParseError(
            "LLM returned empty mapping array".to_string(),
        ));
    }

    let proposals = entries
        .into_iter()
        .map(|entry| {
            let confidence = parse_confidence(&entry.confidence);
            MappingProposal {
                fact_id: entry.fact_id,
                source_id: entry.source_id,
                endpoint: entry.endpoint,
                field_path: entry.field_path,
                confidence,
                explanation: entry.explanation,
                alternatives: vec![],
            }
        })
        .collect();

    Ok(proposals)
}

/// Strip markdown code fences (```json ... ```) from the response.
fn strip_code_fences(text: &str) -> &str {
    let text = text.trim();
    if text.starts_with("```") {
        // Find the end of the first line (skip ```json or ```)
        let after_open = if let Some(nl) = text.find('\n') {
            &text[nl + 1..]
        } else {
            return text;
        };
        // Strip trailing ```
        if let Some(close) = after_open.rfind("```") {
            return after_open[..close].trim();
        }
        return after_open.trim();
    }
    text
}

/// Parse a confidence string from the LLM response.
fn parse_confidence(s: &str) -> Confidence {
    match s.trim().to_uppercase().as_str() {
        "HIGH" => Confidence::High,
        "MEDIUM" => Confidence::Medium,
        _ => Confidence::Low,
    }
}

/// Truncate a string for error messages.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connect::introspect::{
        Endpoint, ExternalSchema, ExternalType, SchemaField, SchemaFormat,
    };

    // ── Prompt construction tests ────────────────────────────────────────────

    #[test]
    fn test_system_prompt_is_nonempty() {
        let prompt = build_system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("fact_id"));
    }

    #[test]
    fn test_user_prompt_contains_facts() {
        let facts = vec![FactDeclaration {
            fact_id: "order_balance".to_string(),
            base_type: "Int".to_string(),
            source_id: "order_service".to_string(),
            path: "orders.balance".to_string(),
            full_type: serde_json::json!({"base": "Int"}),
        }];
        let env = EnvironmentInventory {
            schemas: BTreeMap::new(),
        };

        let prompt = build_user_prompt(&facts, &env);
        assert!(prompt.contains("order_balance"));
        assert!(prompt.contains("order_service"));
        assert!(prompt.contains("orders.balance"));
        assert!(prompt.contains("No external schemas available"));
    }

    #[test]
    fn test_user_prompt_contains_schema() {
        let facts = vec![FactDeclaration {
            fact_id: "order_balance".to_string(),
            base_type: "Int".to_string(),
            source_id: "order_service".to_string(),
            path: "balance".to_string(),
            full_type: serde_json::json!({"base": "Int"}),
        }];

        let mut schemas = BTreeMap::new();
        schemas.insert(
            "order_service".to_string(),
            ExternalSchema {
                format: SchemaFormat::OpenApi3,
                endpoints: vec![Endpoint {
                    method: "GET".to_string(),
                    path: "/orders/{id}".to_string(),
                    parameters: vec!["id".to_string()],
                    response_fields: vec![SchemaField {
                        path: "balance".to_string(),
                        field_type: ExternalType::Integer,
                    }],
                }],
            },
        );

        let env = EnvironmentInventory { schemas };
        let prompt = build_user_prompt(&facts, &env);

        assert!(prompt.contains("GET /orders/{id}"));
        assert!(prompt.contains("balance : integer"));
        assert!(prompt.contains("Source: order_service"));
    }

    #[test]
    fn test_user_prompt_full_type_shown_when_different() {
        let facts = vec![FactDeclaration {
            fact_id: "order_balance".to_string(),
            base_type: "Int".to_string(),
            source_id: "order_service".to_string(),
            path: "balance".to_string(),
            full_type: serde_json::json!({"base": "Int", "min": 0, "max": 100}),
        }];
        let env = EnvironmentInventory {
            schemas: BTreeMap::new(),
        };

        let prompt = build_user_prompt(&facts, &env);
        assert!(prompt.contains("Full type:"));
    }

    // ── Response parsing tests ───────────────────────────────────────────────

    #[test]
    fn test_parse_valid_response() {
        let response = r#"[
            {
                "fact_id": "order_balance",
                "source_id": "order_service",
                "endpoint": "/orders/{id}",
                "field_path": "balance",
                "confidence": "HIGH",
                "explanation": "Exact type and path match"
            }
        ]"#;

        let proposals = parse_llm_response(response).unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].fact_id, "order_balance");
        assert_eq!(proposals[0].endpoint, "/orders/{id}");
        assert_eq!(proposals[0].field_path, "balance");
        assert_eq!(proposals[0].confidence, Confidence::High);
    }

    #[test]
    fn test_parse_multiple_entries() {
        let response = r#"[
            {
                "fact_id": "order_balance",
                "source_id": "order_service",
                "endpoint": "/orders/{id}",
                "field_path": "balance",
                "confidence": "HIGH",
                "explanation": "Exact match"
            },
            {
                "fact_id": "order_status",
                "source_id": "order_service",
                "endpoint": "/orders/{id}",
                "field_path": "status",
                "confidence": "MEDIUM",
                "explanation": "Type compatible"
            }
        ]"#;

        let proposals = parse_llm_response(response).unwrap();
        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].confidence, Confidence::High);
        assert_eq!(proposals[1].confidence, Confidence::Medium);
    }

    #[test]
    fn test_parse_response_with_code_fences() {
        let response = r#"```json
[
    {
        "fact_id": "order_balance",
        "source_id": "order_service",
        "endpoint": "/orders/{id}",
        "field_path": "balance",
        "confidence": "HIGH",
        "explanation": "Match"
    }
]
```"#;

        let proposals = parse_llm_response(response).unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].fact_id, "order_balance");
    }

    #[test]
    fn test_parse_malformed_json() {
        let response = "This is not JSON at all";
        let result = parse_llm_response(response);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatchingError::ParseError(msg) => {
                assert!(msg.contains("failed to parse"));
            }
            other => panic!("Expected ParseError, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_array() {
        let response = "[]";
        let result = parse_llm_response(response);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatchingError::ParseError(msg) => {
                assert!(msg.contains("empty mapping array"));
            }
            other => panic!("Expected ParseError, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_confidence_values() {
        assert_eq!(parse_confidence("HIGH"), Confidence::High);
        assert_eq!(parse_confidence("high"), Confidence::High);
        assert_eq!(parse_confidence("MEDIUM"), Confidence::Medium);
        assert_eq!(parse_confidence("medium"), Confidence::Medium);
        assert_eq!(parse_confidence("LOW"), Confidence::Low);
        assert_eq!(parse_confidence("low"), Confidence::Low);
        assert_eq!(parse_confidence("unknown"), Confidence::Low);
    }

    #[test]
    fn test_strip_code_fences() {
        assert_eq!(strip_code_fences("[1, 2, 3]"), "[1, 2, 3]");
        assert_eq!(strip_code_fences("```json\n[1, 2]\n```"), "[1, 2]");
        assert_eq!(strip_code_fences("```\n[1, 2]\n```"), "[1, 2]");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    // ── Integration test (requires API key, skipped in CI) ───────────────────

    #[tokio::test]
    #[ignore]
    async fn test_llm_provider_integration() {
        let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");
        let config = LlmMatchingConfig::new(api_key);
        let provider = LlmMatchingProvider::new(config);

        let facts = vec![FactDeclaration {
            fact_id: "order_balance".to_string(),
            base_type: "Int".to_string(),
            source_id: "order_service".to_string(),
            path: "balance".to_string(),
            full_type: serde_json::json!({"base": "Int"}),
        }];

        let mut schemas = BTreeMap::new();
        schemas.insert(
            "order_service".to_string(),
            ExternalSchema {
                format: SchemaFormat::OpenApi3,
                endpoints: vec![Endpoint {
                    method: "GET".to_string(),
                    path: "/orders/{id}".to_string(),
                    parameters: vec!["id".to_string()],
                    response_fields: vec![
                        SchemaField {
                            path: "balance".to_string(),
                            field_type: ExternalType::Integer,
                        },
                        SchemaField {
                            path: "status".to_string(),
                            field_type: ExternalType::String,
                        },
                    ],
                }],
            },
        );
        let env = EnvironmentInventory { schemas };

        let proposals = provider.propose_mappings(&facts, &env).await.unwrap();
        assert!(!proposals.is_empty());
        assert_eq!(proposals[0].fact_id, "order_balance");
    }
}
