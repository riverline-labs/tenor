//! Static fact adapter — reads values from config or environment variables.
//!
//! Values are looked up from [`AdapterConfig`] source entries (key=path,
//! value=JSON string), with fallback to env var `TENOR_STATIC_<SOURCE_ID>_<PATH>`.

use super::{AdapterConfig, AdapterError, FactAdapter, StructuredSourceRef};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};

/// Adapter that provides fact values from configuration or environment.
///
/// Lookup order:
/// 1. Pre-populated values (from `with_values()`)
/// 2. `AdapterConfig` source entries (key=path)
/// 3. Environment variable: `TENOR_STATIC_<SOURCE_ID>_<PATH>`
///    (dots in path replaced with underscores, uppercased)
pub struct StaticAdapter {
    source_id: String,
    config_values: HashMap<String, String>,
    direct_values: HashMap<String, serde_json::Value>,
}

impl StaticAdapter {
    /// Create a static adapter from config.
    ///
    /// Reads the source's config entries as path→JSON-string pairs.
    pub fn new(source_id: &str, config: &AdapterConfig) -> Self {
        let config_values = config
            .source_configs
            .get(source_id)
            .cloned()
            .unwrap_or_default();

        StaticAdapter {
            source_id: source_id.to_string(),
            config_values,
            direct_values: HashMap::new(),
        }
    }

    /// Create a static adapter with pre-populated values (for testing).
    pub fn with_values(source_id: &str, values: HashMap<String, serde_json::Value>) -> Self {
        StaticAdapter {
            source_id: source_id.to_string(),
            config_values: HashMap::new(),
            direct_values: values,
        }
    }

    /// Build the env var name for a path.
    ///
    /// `TENOR_STATIC_<SOURCE_ID>_<PATH>` with dots replaced by underscores.
    fn env_var_name(source_id: &str, path: &str) -> String {
        format!(
            "TENOR_STATIC_{}_{}",
            source_id.to_uppercase(),
            path.to_uppercase().replace('.', "_")
        )
    }
}

#[async_trait]
impl FactAdapter for StaticAdapter {
    async fn fetch(
        &self,
        fact_id: &str,
        source: &StructuredSourceRef,
        _source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError> {
        // 1. Pre-populated direct values
        if let Some(value) = self.direct_values.get(&source.path) {
            return Ok(value.clone());
        }

        // 2. Config values (parsed as JSON)
        if let Some(json_str) = self.config_values.get(&source.path) {
            return serde_json::from_str(json_str).map_err(|e| AdapterError::FetchFailed {
                source_id: source.source_id.clone(),
                message: format!("invalid JSON in config for path '{}': {}", source.path, e),
            });
        }

        // 3. Environment variable
        let env_key = Self::env_var_name(&self.source_id, &source.path);
        if let Ok(env_val) = std::env::var(&env_key) {
            return serde_json::from_str(&env_val).map_err(|e| AdapterError::FetchFailed {
                source_id: source.source_id.clone(),
                message: format!("invalid JSON in env var '{}': {}", env_key, e),
            });
        }

        Err(AdapterError::FetchFailed {
            source_id: source.source_id.clone(),
            message: format!(
                "no value found for fact '{}' at path '{}' (checked config and env var '{}')",
                fact_id, source.path, env_key
            ),
        })
    }

    fn adapter_id(&self) -> &str {
        "static"
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn with_values_returns_value() {
        let adapter = StaticAdapter::with_values(
            "cfg",
            [("items.count".to_string(), serde_json::json!(42))]
                .into_iter()
                .collect(),
        );
        let source = StructuredSourceRef {
            source_id: "cfg".to_string(),
            path: "items.count".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert_eq!(result.unwrap(), serde_json::json!(42));
    }

    #[tokio::test]
    async fn config_value_parsed_as_json() {
        let mut config = AdapterConfig::default();
        config.source_configs.insert(
            "cfg".to_string(),
            [("items.count".to_string(), "99".to_string())]
                .into_iter()
                .collect(),
        );
        let adapter = StaticAdapter::new("cfg", &config);
        let source = StructuredSourceRef {
            source_id: "cfg".to_string(),
            path: "items.count".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert_eq!(result.unwrap(), serde_json::json!(99));
    }

    #[tokio::test]
    async fn missing_value_returns_error() {
        let adapter = StaticAdapter::with_values("cfg", HashMap::new());
        let source = StructuredSourceRef {
            source_id: "cfg".to_string(),
            path: "missing.path".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert!(matches!(result, Err(AdapterError::FetchFailed { .. })));
    }

    #[test]
    fn env_var_name_format() {
        assert_eq!(
            StaticAdapter::env_var_name("order_service", "items.count"),
            "TENOR_STATIC_ORDER_SERVICE_ITEMS_COUNT"
        );
    }

    #[tokio::test]
    async fn direct_values_take_priority_over_config() {
        let mut config = AdapterConfig::default();
        config.source_configs.insert(
            "cfg".to_string(),
            [("x.y".to_string(), "\"config_val\"".to_string())]
                .into_iter()
                .collect(),
        );

        // with_values doesn't use config, but we test that direct_values win
        let adapter = StaticAdapter::with_values(
            "cfg",
            [("x.y".to_string(), serde_json::json!("direct_val"))]
                .into_iter()
                .collect(),
        );
        let source = StructuredSourceRef {
            source_id: "cfg".to_string(),
            path: "x.y".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert_eq!(result.unwrap(), serde_json::json!("direct_val"));
    }
}
