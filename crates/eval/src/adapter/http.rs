//! HTTP fact adapter — fetches facts from REST APIs.
//!
//! Uses `ureq` (sync) wrapped in `tokio::task::spawn_blocking` to avoid
//! blocking the async runtime. Reads `base_url` from source fields and
//! converts dot-paths to URL path segments.

use super::{AdapterConfig, AdapterError, FactAdapter, StructuredSourceRef};
use async_trait::async_trait;
use std::collections::BTreeMap;

/// Adapter that fetches facts via HTTP GET requests.
///
/// - `base_url` from source fields (required)
/// - `auth_token` from config or `TENOR_SOURCE_<ID>_AUTH_TOKEN` env var
/// - Dot-path converted to URL segments: `orders.balance` → `{base_url}/orders/balance`
pub struct HttpAdapter {
    source_id: String,
    auth_token: Option<String>,
}

impl HttpAdapter {
    /// Create a new HTTP adapter for the given source.
    ///
    /// Looks up `auth_token` from config (source-specific, then global),
    /// then falls back to `TENOR_SOURCE_<ID>_AUTH_TOKEN` env var.
    pub fn new(source_id: &str, config: &AdapterConfig) -> Self {
        let auth_token = config
            .get(source_id, "auth_token")
            .map(|s| s.to_string())
            .or_else(|| {
                let env_key = format!("TENOR_SOURCE_{}_AUTH_TOKEN", source_id.to_uppercase());
                std::env::var(&env_key).ok()
            });

        HttpAdapter {
            source_id: source_id.to_string(),
            auth_token,
        }
    }

    /// Convert a dot-path to URL path segments.
    ///
    /// `orders.balance` → `/orders/balance`
    pub fn path_to_url_segments(path: &str) -> String {
        path.split('.').collect::<Vec<_>>().join("/")
    }
}

#[async_trait]
impl FactAdapter for HttpAdapter {
    async fn fetch(
        &self,
        _fact_id: &str,
        source: &StructuredSourceRef,
        source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError> {
        let base_url = source_fields
            .get("base_url")
            .ok_or_else(|| AdapterError::ConfigError {
                message: format!(
                    "source '{}' missing required field 'base_url'",
                    source.source_id
                ),
            })?;

        let segments = Self::path_to_url_segments(&source.path);
        let url = format!("{}/{}", base_url.trim_end_matches('/'), segments);

        let auth_token = self.auth_token.clone();
        let source_id = self.source_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            let agent = ureq::Agent::new_with_defaults();
            let mut request = agent.get(&url);

            if let Some(ref token) = auth_token {
                request = request.header("Authorization", &format!("Bearer {}", token));
            }

            let response = request.call().map_err(|e| AdapterError::FetchFailed {
                source_id: source_id.clone(),
                message: e.to_string(),
            })?;

            let value: serde_json::Value =
                response
                    .into_body()
                    .read_json()
                    .map_err(|e| AdapterError::FetchFailed {
                        source_id,
                        message: format!("failed to parse response as JSON: {}", e),
                    })?;

            Ok(value)
        })
        .await
        .map_err(|e| AdapterError::FetchFailed {
            source_id: self.source_id.clone(),
            message: format!("task join error: {}", e),
        })?;

        result
    }

    fn adapter_id(&self) -> &str {
        "http"
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_to_url_segments_simple() {
        assert_eq!(
            HttpAdapter::path_to_url_segments("orders.balance"),
            "orders/balance"
        );
    }

    #[test]
    fn path_to_url_segments_single() {
        assert_eq!(HttpAdapter::path_to_url_segments("status"), "status");
    }

    #[test]
    fn path_to_url_segments_deep() {
        assert_eq!(HttpAdapter::path_to_url_segments("a.b.c.d"), "a/b/c/d");
    }

    #[tokio::test]
    async fn missing_base_url_returns_config_error() {
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "x.y".to_string(),
        };

        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert!(matches!(result, Err(AdapterError::ConfigError { .. })));
    }

    #[test]
    fn auth_token_from_config() {
        let mut config = AdapterConfig::default();
        config.source_configs.insert(
            "api".to_string(),
            [("auth_token".to_string(), "my-token".to_string())]
                .into_iter()
                .collect(),
        );
        let adapter = HttpAdapter::new("api", &config);
        assert_eq!(adapter.auth_token, Some("my-token".to_string()));
    }

    #[test]
    fn no_auth_token_is_none() {
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("api", &config);
        assert_eq!(adapter.auth_token, None);
    }

    // -----------------------------------------------------------------------
    // HTTP adapter protocol tests (Wave 2E)
    // -----------------------------------------------------------------------

    /// Spin up a tiny TCP server that returns a fixed HTTP response, then
    /// returns the base URL to connect to.
    fn spawn_mock_http(status: u16, body: &str) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let response = format!(
            "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        );

        std::thread::spawn(move || {
            // Accept up to 2 connections (some tests may retry)
            for _ in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buf = [0u8; 4096];
                    let _ = stream.read(&mut buf);
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            }
        });

        format!("http://127.0.0.1:{}", port)
    }

    #[tokio::test]
    async fn http_successful_fetch_returns_json_value() {
        let base_url = spawn_mock_http(200, r#"{"amount": 42.5}"#);
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let mut fields = BTreeMap::new();
        fields.insert("base_url".to_string(), base_url);

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "orders.balance".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        let value = result.unwrap();
        assert_eq!(value, serde_json::json!({"amount": 42.5}));
    }

    #[tokio::test]
    async fn http_404_returns_fetch_failed() {
        let base_url = spawn_mock_http(404, r#"{"error": "not found"}"#);
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let mut fields = BTreeMap::new();
        fields.insert("base_url".to_string(), base_url);

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "missing.resource".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "HTTP 404 should produce FetchFailed, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn http_500_returns_fetch_failed() {
        let base_url = spawn_mock_http(500, r#"{"error": "internal"}"#);
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let mut fields = BTreeMap::new();
        fields.insert("base_url".to_string(), base_url);

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "broken.endpoint".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "HTTP 500 should produce FetchFailed, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn http_connection_refused_returns_fetch_failed() {
        // Use a port that nothing listens on (timeout / connection refused)
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let mut fields = BTreeMap::new();
        // Port 1 is unlikely to have a server listening
        fields.insert("base_url".to_string(), "http://127.0.0.1:1".to_string());

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "x.y".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "connection refused should produce FetchFailed, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn http_non_json_response_returns_fetch_failed() {
        // Server returns valid HTTP but body is not valid JSON
        let base_url = spawn_mock_http(200, "this is not json");
        let config = AdapterConfig::default();
        let adapter = HttpAdapter::new("test_src", &config);

        let mut fields = BTreeMap::new();
        fields.insert("base_url".to_string(), base_url);

        let source = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "text.endpoint".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "non-JSON response should produce FetchFailed, got: {:?}",
            result
        );
        if let Err(AdapterError::FetchFailed { message, .. }) = &result {
            assert!(
                message.contains("JSON"),
                "error should mention JSON parsing, got: {}",
                message
            );
        }
    }

    #[tokio::test]
    async fn http_bearer_token_auth_header_included() {
        // Verify auth token is passed by checking the request on the server side
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{Arc, Mutex};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let captured_request = Arc::new(Mutex::new(String::new()));
        let captured_clone = Arc::clone(&captured_request);

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                *captured_clone.lock().unwrap() = request;

                let body = r#"{"ok": true}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        let mut config = AdapterConfig::default();
        config.source_configs.insert(
            "auth_src".to_string(),
            [("auth_token".to_string(), "my-secret-token".to_string())]
                .into_iter()
                .collect(),
        );
        let adapter = HttpAdapter::new("auth_src", &config);

        let mut fields = BTreeMap::new();
        fields.insert("base_url".to_string(), format!("http://127.0.0.1:{}", port));

        let source = StructuredSourceRef {
            source_id: "auth_src".to_string(),
            path: "protected.resource".to_string(),
        };

        let result = adapter.fetch("f1", &source, &fields).await;
        assert!(result.is_ok(), "authenticated request should succeed");

        // Give the thread a moment to capture
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let request_text = captured_request.lock().unwrap().clone();
        assert!(
            request_text
                .to_lowercase()
                .contains("authorization: bearer my-secret-token"),
            "request should contain bearer auth header, got: {}",
            request_text
        );
    }
}
