//! Manual fact adapter — for facts that require human input.
//!
//! By default, returns an error indicating manual input is required.
//! Can be pre-populated with values for testing or batch processing.

use super::{AdapterError, FactAdapter, StructuredSourceRef};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};

/// Adapter for manually-provided facts.
///
/// Default behavior: returns an error "manual input required for fact '...'".
/// Use `with_values()` to pre-populate values for testing.
pub struct ManualAdapter {
    values: HashMap<String, serde_json::Value>,
}

impl ManualAdapter {
    /// Create a manual adapter with no pre-populated values.
    ///
    /// All fetch calls will return an error.
    pub fn new() -> Self {
        ManualAdapter {
            values: HashMap::new(),
        }
    }

    /// Create a manual adapter with pre-populated values.
    ///
    /// Keys are fact IDs. If a requested fact_id is in the map,
    /// its value is returned; otherwise an error is returned.
    pub fn with_values(values: HashMap<String, serde_json::Value>) -> Self {
        ManualAdapter { values }
    }
}

impl Default for ManualAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FactAdapter for ManualAdapter {
    async fn fetch(
        &self,
        fact_id: &str,
        source: &StructuredSourceRef,
        _source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError> {
        if let Some(value) = self.values.get(fact_id) {
            return Ok(value.clone());
        }

        Err(AdapterError::FetchFailed {
            source_id: source.source_id.clone(),
            message: format!("manual input required for fact '{}'", fact_id),
        })
    }

    fn adapter_id(&self) -> &str {
        "manual"
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_values_returns_error() {
        let adapter = ManualAdapter::new();
        let source = StructuredSourceRef {
            source_id: "manual_src".to_string(),
            path: "x.y".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        match result {
            Err(AdapterError::FetchFailed { message, .. }) => {
                assert!(message.contains("manual input required"));
                assert!(message.contains("f1"));
            }
            other => panic!("expected FetchFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn with_values_returns_value() {
        let adapter = ManualAdapter::with_values(
            [("f1".to_string(), serde_json::json!("entered_value"))]
                .into_iter()
                .collect(),
        );
        let source = StructuredSourceRef {
            source_id: "manual_src".to_string(),
            path: "x.y".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert_eq!(result.unwrap(), serde_json::json!("entered_value"));
    }

    #[tokio::test]
    async fn with_values_missing_fact_returns_error() {
        let adapter = ManualAdapter::with_values(
            [("f1".to_string(), serde_json::json!(true))]
                .into_iter()
                .collect(),
        );
        let source = StructuredSourceRef {
            source_id: "manual_src".to_string(),
            path: "x.y".to_string(),
        };
        let result = adapter.fetch("f2", &source, &BTreeMap::new()).await;
        assert!(matches!(result, Err(AdapterError::FetchFailed { .. })));
    }

    #[test]
    fn default_impl() {
        let adapter = ManualAdapter::default();
        assert!(adapter.values.is_empty());
    }
}
