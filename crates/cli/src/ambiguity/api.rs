//! Anthropic Messages API client with retry and rate-limit handling.

use serde::{Deserialize, Serialize};

/// Anthropic Messages API endpoint.
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Required API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default model to use for ambiguity testing.
const DEFAULT_MODEL: &str = "claude-sonnet-4-5";

/// Default maximum retries for transient errors.
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Initial backoff delay in milliseconds (doubles each retry).
const INITIAL_BACKOFF_MS: u64 = 1000;

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)] // Deserialized from API response; required by serde for correct parsing
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[allow(dead_code)] // Deserialized from API response; required by serde for correct parsing
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Read the Anthropic API key from the `ANTHROPIC_API_KEY` environment variable.
pub fn get_api_key() -> Result<String, String> {
    std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        "ANTHROPIC_API_KEY environment variable is not set. \
         Set it to your Anthropic API key to run ambiguity tests."
            .to_string()
    })
}

/// Return the default model identifier.
pub fn default_model() -> &'static str {
    DEFAULT_MODEL
}

/// Call the Anthropic Messages API with the given system prompt, user prompt,
/// and model. Returns the text content from the first content block.
///
/// Retries on 429 (rate limit), 500, and 503 errors with exponential backoff.
pub fn call_anthropic(
    api_key: &str,
    system: &str,
    user_prompt: &str,
    model: &str,
) -> Result<String, String> {
    with_retry(
        || call_anthropic_once(api_key, system, user_prompt, model),
        DEFAULT_MAX_RETRIES,
    )
}

// ── Internal ─────────────────────────────────────────────────────────────────

/// Make a single API call (no retry).
fn call_anthropic_once(
    api_key: &str,
    system: &str,
    user_prompt: &str,
    model: &str,
) -> Result<String, String> {
    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: 4096,
        system: system.to_string(),
        messages: vec![Message {
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
        .map_err(|e| format!("API request failed: {}", e))?;

    let resp: MessagesResponse = response
        .into_body()
        .read_json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    resp.content
        .first()
        .and_then(|block| block.text.clone())
        .ok_or_else(|| "API response contained no text content".to_string())
}

/// Retry a fallible operation with exponential backoff.
///
/// Retries only on errors deemed retryable (429, 500, 503).
/// Backoff starts at `INITIAL_BACKOFF_MS` and doubles each retry.
fn with_retry<T, F: Fn() -> Result<T, String>>(f: F, max_retries: u32) -> Result<T, String> {
    let mut last_error = String::new();
    let mut backoff_ms = INITIAL_BACKOFF_MS;

    for attempt in 0..=max_retries {
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt < max_retries && is_retryable(&e) {
                    eprintln!(
                        "Retryable error (attempt {}/{}): {}. Backing off {}ms...",
                        attempt + 1,
                        max_retries + 1,
                        e,
                        backoff_ms
                    );
                    std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    backoff_ms *= 2;
                    last_error = e;
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(format!(
        "All {} retries exhausted. Last error: {}",
        max_retries + 1,
        last_error
    ))
}

/// Extract an HTTP status code from an error string.
///
/// Looks for a 3-digit HTTP status code pattern in the error message.
/// ureq v3 formats errors as "http status: NNN ..." which this captures.
fn extract_http_status(error: &str) -> Option<u16> {
    // Look for patterns like "status: 429", "status 429", or standalone 3-digit codes
    // preceded by whitespace or colon (to avoid matching port numbers etc.)
    for word in error.split_whitespace() {
        // Strip trailing punctuation (commas, periods, colons)
        let clean = word.trim_matches(|c: char| !c.is_ascii_digit());
        if clean.len() == 3 {
            if let Ok(code) = clean.parse::<u16>() {
                if (400..=599).contains(&code) {
                    return Some(code);
                }
            }
        }
    }
    None
}

/// Determine if an error message indicates a retryable condition.
///
/// Retryable HTTP status codes: 429 (rate limit), 500 (internal server error),
/// 502 (bad gateway), 503 (service unavailable).
/// Also retries on network-level errors (connection failures, timeouts).
fn is_retryable(error: &str) -> bool {
    // Check for retryable HTTP status codes via structured extraction
    if let Some(status) = extract_http_status(error) {
        if matches!(status, 429 | 500 | 502 | 503) {
            return true;
        }
    }

    // Network-level errors (no HTTP status code available)
    let lower = error.to_lowercase();
    lower.contains("connection") || lower.contains("timeout")
}
