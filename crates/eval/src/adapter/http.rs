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
}
