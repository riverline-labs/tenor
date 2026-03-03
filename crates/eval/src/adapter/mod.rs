//! Fact adapter abstraction for connecting source declarations to external data.
//!
//! Two levels of abstraction:
//! - [`FactProvider`](crate::FactProvider) — provides ALL facts at once for a contract
//! - [`FactAdapter`] — fetches ONE fact from ONE source via its protocol
//!
//! [`AdapterFactProvider`] bridges the two: given an [`AdapterRegistry`] and a
//! contract, it implements `FactProvider` by dispatching each structured-source
//! fact to the appropriate adapter, falling back to directly-provided facts.

pub mod database;
pub mod http;
pub mod manual;
pub mod static_adapter;

use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

// ──────────────────────────────────────────────
// StructuredSourceRef
// ──────────────────────────────────────────────

/// A typed reference to a source + path, parsed from interchange JSON.
///
/// Corresponds to `{"source_id": "x", "path": "y"}` in a fact's `source` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredSourceRef {
    pub source_id: String,
    pub path: String,
}

impl StructuredSourceRef {
    /// Parse from a JSON value. Returns `None` if the value is not a
    /// structured source reference (e.g. it's a legacy string source).
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;
        let source_id = obj.get("source_id")?.as_str()?;
        let path = obj.get("path")?.as_str()?;
        Some(StructuredSourceRef {
            source_id: source_id.to_string(),
            path: path.to_string(),
        })
    }
}

// ──────────────────────────────────────────────
// AdapterError
// ──────────────────────────────────────────────

/// Errors that can occur when a fact adapter fetches a fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The adapter for this protocol is not configured.
    NotConfigured { source_id: String, protocol: String },
    /// The referenced source was not found in the registry.
    SourceNotFound { source_id: String },
    /// The fetch operation failed.
    FetchFailed { source_id: String, message: String },
    /// The fetched value doesn't match the expected type.
    TypeMismatch { fact_id: String, message: String },
    /// Configuration error (missing credentials, bad URL, etc.).
    ConfigError { message: String },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::NotConfigured {
                source_id,
                protocol,
            } => {
                write!(
                    f,
                    "adapter not configured for source '{}' (protocol: {})",
                    source_id, protocol
                )
            }
            AdapterError::SourceNotFound { source_id } => {
                write!(f, "source '{}' not found in registry", source_id)
            }
            AdapterError::FetchFailed { source_id, message } => {
                write!(f, "fetch failed for source '{}': {}", source_id, message)
            }
            AdapterError::TypeMismatch { fact_id, message } => {
                write!(f, "type mismatch for fact '{}': {}", fact_id, message)
            }
            AdapterError::ConfigError { message } => {
                write!(f, "adapter config error: {}", message)
            }
        }
    }
}

impl std::error::Error for AdapterError {}

// ──────────────────────────────────────────────
// EnrichedFactProvenance
// ──────────────────────────────────────────────

/// Per §5A.10: provenance record for a fact fetched via an adapter.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnrichedFactProvenance {
    pub fact_id: String,
    pub source_id: String,
    pub path: String,
    pub value: serde_json::Value,
    pub assertion_source: String,
    pub adapter_id: String,
    pub fetch_timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_response: Option<serde_json::Value>,
}

// ──────────────────────────────────────────────
// AdapterConfig
// ──────────────────────────────────────────────

/// Configuration for adapters — provides credentials, URLs, and other
/// values that contracts deliberately omit (§5A.5).
///
/// Two levels:
/// - `source_configs`: per-source config keyed by source_id, then by key
/// - `global`: global config (fallback for any source)
#[derive(Debug, Clone, Default)]
pub struct AdapterConfig {
    pub source_configs: HashMap<String, HashMap<String, String>>,
    pub global: HashMap<String, String>,
}

impl AdapterConfig {
    /// Look up a config value for a specific source, falling back to global.
    pub fn get(&self, source_id: &str, key: &str) -> Option<&str> {
        self.source_configs
            .get(source_id)
            .and_then(|m| m.get(key))
            .or_else(|| self.global.get(key))
            .map(|s| s.as_str())
    }
}

// ──────────────────────────────────────────────
// FactAdapter trait
// ──────────────────────────────────────────────

/// Fetches ONE fact from ONE source via its protocol.
///
/// Implementations handle protocol-specific details (HTTP requests,
/// database queries, etc.). `source_fields` contains the Source
/// construct's declared fields (e.g. `base_url`, `dialect`) so
/// adapters don't need to carry a copy of the source declaration.
#[async_trait]
pub trait FactAdapter: Send + Sync {
    /// Fetch a single fact value from the source.
    async fn fetch(
        &self,
        fact_id: &str,
        source: &StructuredSourceRef,
        source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError>;

    /// Returns this adapter's identifier (e.g. "http", "database", "static").
    fn adapter_id(&self) -> &str;
}

// ──────────────────────────────────────────────
// AdapterRegistry
// ──────────────────────────────────────────────

/// Registry that maps source IDs to their adapters.
///
/// Built from the contract's Source constructs and an [`AdapterConfig`].
/// At fetch time, dispatches to the appropriate adapter and returns
/// both the value and enriched provenance.
pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn FactAdapter>>,
    source_fields: HashMap<String, BTreeMap<String, String>>,
}

impl AdapterRegistry {
    /// Build a registry from interchange Source constructs and adapter config.
    ///
    /// For each source, selects the appropriate adapter based on its protocol:
    /// - `"http"` → [`http::HttpAdapter`]
    /// - `"database"` → [`database::DatabaseAdapter`] (stub)
    /// - `"static"` → [`static_adapter::StaticAdapter`]
    /// - `"manual"` → [`manual::ManualAdapter`]
    /// - other → skipped (not configured)
    pub fn from_sources(
        sources: &[tenor_interchange::SourceConstruct],
        config: &AdapterConfig,
    ) -> Self {
        let mut adapters: HashMap<String, Box<dyn FactAdapter>> = HashMap::new();
        let mut source_fields_map: HashMap<String, BTreeMap<String, String>> = HashMap::new();

        for source in sources {
            source_fields_map.insert(source.id.clone(), source.fields.clone());

            let adapter: Box<dyn FactAdapter> = match source.protocol.as_str() {
                "http" => Box::new(http::HttpAdapter::new(&source.id, config)),
                "database" => Box::new(database::DatabaseAdapter::new_stub(&source.id)),
                "static" => Box::new(static_adapter::StaticAdapter::new(&source.id, config)),
                "manual" => Box::new(manual::ManualAdapter::new()),
                _ => continue,
            };
            adapters.insert(source.id.clone(), adapter);
        }

        AdapterRegistry {
            adapters,
            source_fields: source_fields_map,
        }
    }

    /// Create an empty registry (no adapters registered).
    pub fn empty() -> Self {
        AdapterRegistry {
            adapters: HashMap::new(),
            source_fields: HashMap::new(),
        }
    }

    /// Register a custom adapter for a source ID.
    pub fn register(
        &mut self,
        source_id: String,
        fields: BTreeMap<String, String>,
        adapter: Box<dyn FactAdapter>,
    ) {
        self.source_fields.insert(source_id.clone(), fields);
        self.adapters.insert(source_id, adapter);
    }

    /// Fetch a fact value and return it with enriched provenance.
    pub async fn fetch_fact(
        &self,
        fact_id: &str,
        source: &StructuredSourceRef,
    ) -> Result<(serde_json::Value, EnrichedFactProvenance), AdapterError> {
        let adapter =
            self.adapters
                .get(&source.source_id)
                .ok_or_else(|| AdapterError::SourceNotFound {
                    source_id: source.source_id.clone(),
                })?;

        let fields = self
            .source_fields
            .get(&source.source_id)
            .cloned()
            .unwrap_or_default();

        let value = adapter.fetch(fact_id, source, &fields).await?;

        let now = time::OffsetDateTime::now_utc();
        let timestamp = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let provenance = EnrichedFactProvenance {
            fact_id: fact_id.to_string(),
            source_id: source.source_id.clone(),
            path: source.path.clone(),
            value: value.clone(),
            assertion_source: "external".to_string(),
            adapter_id: adapter.adapter_id().to_string(),
            fetch_timestamp: timestamp,
            source_response: None,
        };

        Ok((value, provenance))
    }
}

// ──────────────────────────────────────────────
// AdapterFactProvider
// ──────────────────────────────────────────────

/// Bridges [`AdapterRegistry`] to [`FactProvider`](crate::FactProvider).
///
/// Direct-provided facts (in `direct_facts`) take priority.
/// Structured-source facts are fetched via adapters in the registry.
/// Facts with no structured source and no direct value will be missing
/// (the evaluator handles defaults).
pub struct AdapterFactProvider {
    pub registry: AdapterRegistry,
    pub direct_facts: HashMap<String, serde_json::Value>,
    pub provenance_log: std::sync::Arc<std::sync::Mutex<Vec<EnrichedFactProvenance>>>,
}

impl AdapterFactProvider {
    pub fn new(
        registry: AdapterRegistry,
        direct_facts: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            registry,
            direct_facts,
            provenance_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Returns collected provenance records from all adapter fetches.
    pub fn provenance(&self) -> Vec<EnrichedFactProvenance> {
        // Recover data even if mutex was poisoned by a panic in another thread
        self.provenance_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

#[async_trait]
impl crate::FactProvider for AdapterFactProvider {
    async fn provide(
        &self,
        contract: &serde_json::Value,
    ) -> Result<HashMap<String, serde_json::Value>, crate::FactProviderError> {
        let mut result = self.direct_facts.clone();

        // Parse constructs from bundle to find facts with structured sources
        let constructs = contract
            .get("constructs")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        for construct in &constructs {
            let kind = construct.get("kind").and_then(|k| k.as_str());
            if kind != Some("Fact") {
                continue;
            }

            let fact_id = match construct.get("id").and_then(|id| id.as_str()) {
                Some(id) => id,
                None => continue,
            };

            // Skip if direct-provided
            if result.contains_key(fact_id) {
                continue;
            }

            // Check for structured source reference
            let source_ref = construct
                .get("source")
                .and_then(StructuredSourceRef::from_json);

            if let Some(ref sref) = source_ref {
                match self.registry.fetch_fact(fact_id, sref).await {
                    Ok((value, provenance)) => {
                        result.insert(fact_id.to_string(), value);
                        self.provenance_log
                            .lock()
                            .map_err(|e| {
                                crate::FactProviderError::Provider(format!("lock poisoned: {e}"))
                            })?
                            .push(provenance);
                    }
                    Err(e) => {
                        return Err(crate::FactProviderError::Provider(e.to_string()));
                    }
                }
            }
        }

        Ok(result)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_source_ref_from_valid_json() {
        let json = serde_json::json!({"source_id": "order_service", "path": "orders.balance"});
        let sref = StructuredSourceRef::from_json(&json).unwrap();
        assert_eq!(sref.source_id, "order_service");
        assert_eq!(sref.path, "orders.balance");
    }

    #[test]
    fn structured_source_ref_from_legacy_string() {
        let json = serde_json::json!("buyer_portal.refund_requested");
        assert!(StructuredSourceRef::from_json(&json).is_none());
    }

    #[test]
    fn structured_source_ref_missing_fields() {
        let json = serde_json::json!({"source_id": "x"});
        assert!(StructuredSourceRef::from_json(&json).is_none());

        let json = serde_json::json!({"path": "y"});
        assert!(StructuredSourceRef::from_json(&json).is_none());
    }

    #[test]
    fn adapter_config_lookup() {
        let mut config = AdapterConfig::default();
        config.source_configs.insert(
            "src1".to_string(),
            [("auth_token".to_string(), "secret".to_string())]
                .into_iter()
                .collect(),
        );
        config
            .global
            .insert("timeout".to_string(), "30".to_string());

        // Source-specific lookup
        assert_eq!(config.get("src1", "auth_token"), Some("secret"));
        // Global fallback
        assert_eq!(config.get("src1", "timeout"), Some("30"));
        assert_eq!(config.get("other", "timeout"), Some("30"));
        // Missing
        assert_eq!(config.get("src1", "missing"), None);
    }

    #[test]
    fn adapter_error_display() {
        let err = AdapterError::NotConfigured {
            source_id: "s1".to_string(),
            protocol: "grpc".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "adapter not configured for source 's1' (protocol: grpc)"
        );

        let err = AdapterError::SourceNotFound {
            source_id: "s1".to_string(),
        };
        assert_eq!(err.to_string(), "source 's1' not found in registry");

        let err = AdapterError::FetchFailed {
            source_id: "s1".to_string(),
            message: "timeout".to_string(),
        };
        assert_eq!(err.to_string(), "fetch failed for source 's1': timeout");
    }

    #[tokio::test]
    async fn registry_source_not_found() {
        let registry = AdapterRegistry::empty();
        let sref = StructuredSourceRef {
            source_id: "missing".to_string(),
            path: "x.y".to_string(),
        };
        let result = registry.fetch_fact("f1", &sref).await;
        assert!(matches!(result, Err(AdapterError::SourceNotFound { .. })));
    }

    #[tokio::test]
    async fn registry_with_static_adapter() {
        let mut registry = AdapterRegistry::empty();
        let adapter = static_adapter::StaticAdapter::with_values(
            "test_src",
            [("items.count".to_string(), serde_json::json!(42))]
                .into_iter()
                .collect(),
        );
        registry.register("test_src".to_string(), BTreeMap::new(), Box::new(adapter));

        let sref = StructuredSourceRef {
            source_id: "test_src".to_string(),
            path: "items.count".to_string(),
        };
        let (value, provenance) = registry.fetch_fact("my_fact", &sref).await.unwrap();
        assert_eq!(value, serde_json::json!(42));
        assert_eq!(provenance.fact_id, "my_fact");
        assert_eq!(provenance.source_id, "test_src");
        assert_eq!(provenance.assertion_source, "external");
        assert_eq!(provenance.adapter_id, "static");
    }

    #[tokio::test]
    async fn adapter_fact_provider_direct_takes_priority() {
        let registry = AdapterRegistry::empty();
        let mut direct = HashMap::new();
        direct.insert("f1".to_string(), serde_json::json!(true));

        let provider = AdapterFactProvider::new(registry, direct);

        // Bundle with a fact that has a structured source — but direct value should win
        let contract = serde_json::json!({
            "id": "test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [{
                "id": "f1",
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": {"file": "test.tenor", "line": 1},
                "source": {"source_id": "src1", "path": "x.y"},
                "type": {"base": "Bool"}
            }]
        });

        let result = crate::FactProvider::provide(&provider, &contract)
            .await
            .unwrap();
        assert_eq!(result["f1"], serde_json::json!(true));
        assert!(provider.provenance().is_empty());
    }

    #[tokio::test]
    async fn adapter_fact_provider_fetches_via_adapter() {
        let mut registry = AdapterRegistry::empty();
        let adapter = static_adapter::StaticAdapter::with_values(
            "src1",
            [("x.y".to_string(), serde_json::json!(99))]
                .into_iter()
                .collect(),
        );
        registry.register("src1".to_string(), BTreeMap::new(), Box::new(adapter));

        let provider = AdapterFactProvider::new(registry, HashMap::new());

        let contract = serde_json::json!({
            "id": "test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [{
                "id": "my_int",
                "kind": "Fact",
                "tenor": "1.0",
                "provenance": {"file": "test.tenor", "line": 1},
                "source": {"source_id": "src1", "path": "x.y"},
                "type": {"base": "Int"}
            }]
        });

        let result = crate::FactProvider::provide(&provider, &contract)
            .await
            .unwrap();
        assert_eq!(result["my_int"], serde_json::json!(99));
        assert_eq!(provider.provenance().len(), 1);
        assert_eq!(provider.provenance()[0].adapter_id, "static");
    }
}
