//! Fact provider trait and implementations.
//!
//! A `FactProvider` asynchronously supplies fact values for contract evaluation.
//! The contract bundle is passed so implementations can inspect declared fact IDs,
//! types, and sources to determine what to fetch.

use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

/// Errors that can occur when a fact provider fetches facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactProviderError {
    /// A provider-specific error occurred.
    Provider(String),
}

impl fmt::Display for FactProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FactProviderError::Provider(msg) => write!(f, "fact provider error: {}", msg),
        }
    }
}

impl std::error::Error for FactProviderError {}

// ──────────────────────────────────────────────
// Trait
// ──────────────────────────────────────────────

/// Asynchronous provider of fact values for contract evaluation.
///
/// Implementations fetch fact values from external systems (databases,
/// APIs, etc.) based on the contract's declared facts. The contract
/// bundle is provided so implementations can inspect declared fact IDs,
/// types, and sources to determine what to fetch.
#[async_trait]
pub trait FactProvider: Send + Sync {
    /// Provide fact values for evaluation.
    ///
    /// The contract bundle is provided so implementations can inspect
    /// declared fact IDs, types, and sources to determine what to fetch.
    /// Returns a map from fact_id to fact value (as JSON).
    async fn provide(
        &self,
        contract: &serde_json::Value,
    ) -> Result<HashMap<String, serde_json::Value>, FactProviderError>;
}

// ──────────────────────────────────────────────
// StaticFactProvider
// ──────────────────────────────────────────────

/// A fact provider that returns a fixed set of facts.
///
/// Wraps a `HashMap<String, serde_json::Value>` and returns it unchanged
/// on every call to `provide`. Useful for testing and scenarios where
/// all fact values are known ahead of time.
pub struct StaticFactProvider {
    facts: HashMap<String, serde_json::Value>,
}

impl StaticFactProvider {
    /// Create a new `StaticFactProvider` with the given facts.
    pub fn new(facts: HashMap<String, serde_json::Value>) -> Self {
        Self { facts }
    }

    /// Create a new `StaticFactProvider` with no facts.
    pub fn empty() -> Self {
        Self {
            facts: HashMap::new(),
        }
    }
}

#[async_trait]
impl FactProvider for StaticFactProvider {
    async fn provide(
        &self,
        _contract: &serde_json::Value,
    ) -> Result<HashMap<String, serde_json::Value>, FactProviderError> {
        Ok(self.facts.clone())
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn static_provider_returns_facts() {
        let mut facts = HashMap::new();
        facts.insert("is_active".to_string(), serde_json::Value::Bool(true));
        facts.insert(
            "balance".to_string(),
            serde_json::json!({"amount": "5000.00", "currency": "USD"}),
        );

        let provider = StaticFactProvider::new(facts.clone());
        let contract = serde_json::json!({"id": "test", "kind": "Bundle"});

        let result = provider.provide(&contract).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result["is_active"], serde_json::Value::Bool(true));
        assert_eq!(
            result["balance"],
            serde_json::json!({"amount": "5000.00", "currency": "USD"})
        );
    }

    #[tokio::test]
    async fn empty_provider_returns_no_facts() {
        let provider = StaticFactProvider::empty();
        let contract = serde_json::json!({"id": "test", "kind": "Bundle"});

        let result = provider.provide(&contract).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn error_display() {
        let err = FactProviderError::Provider("connection refused".to_string());
        assert_eq!(err.to_string(), "fact provider error: connection refused");
    }
}
