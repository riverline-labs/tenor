//! Database fact adapter — fetches facts via database queries.
//!
//! Defines a [`DatabaseQuery`] sub-trait as the extension point for real
//! database drivers. The default [`StubDatabaseQuery`] returns
//! `AdapterError::NotConfigured` — real implementations live in the
//! private repo's storage crate.

use super::{AdapterError, FactAdapter, StructuredSourceRef};
use async_trait::async_trait;
use std::collections::BTreeMap;

// ──────────────────────────────────────────────
// DatabaseQuery sub-trait
// ──────────────────────────────────────────────

/// Extension point for database-backed fact fetching.
///
/// Implement this trait to connect a real database driver (e.g. sqlx/postgres)
/// to the adapter system. The `query` method receives a SQL-safe query string
/// derived from the fact's source path.
#[async_trait]
pub trait DatabaseQuery: Send + Sync {
    /// Execute a query and return the result as a JSON value.
    async fn query(
        &self,
        source_id: &str,
        query_str: &str,
        source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError>;
}

/// Stub implementation that always returns `NotConfigured`.
///
/// Used as the default when no real database driver is available.
pub struct StubDatabaseQuery {
    source_id: String,
}

#[async_trait]
impl DatabaseQuery for StubDatabaseQuery {
    async fn query(
        &self,
        _source_id: &str,
        _query_str: &str,
        _source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError> {
        Err(AdapterError::NotConfigured {
            source_id: self.source_id.clone(),
            protocol: "database".to_string(),
        })
    }
}

// ──────────────────────────────────────────────
// DatabaseAdapter
// ──────────────────────────────────────────────

/// Adapter that fetches facts via database queries.
///
/// Delegates to a [`DatabaseQuery`] implementation. The dot-path from
/// the source reference is converted to a query string:
/// `orders.balance` → `SELECT value FROM orders WHERE key = 'balance'`
pub struct DatabaseAdapter {
    source_id: String,
    query_impl: Box<dyn DatabaseQuery>,
}

impl DatabaseAdapter {
    /// Create a database adapter with a stub query implementation.
    ///
    /// The stub always returns `NotConfigured`. Use `new` with a real
    /// `DatabaseQuery` implementation for production use.
    pub fn new_stub(source_id: &str) -> Self {
        DatabaseAdapter {
            source_id: source_id.to_string(),
            query_impl: Box::new(StubDatabaseQuery {
                source_id: source_id.to_string(),
            }),
        }
    }

    /// Create a database adapter with a custom query implementation.
    pub fn new(source_id: &str, query_impl: Box<dyn DatabaseQuery>) -> Self {
        DatabaseAdapter {
            source_id: source_id.to_string(),
            query_impl,
        }
    }

    /// Convert a dot-path to a simple SQL-style query string.
    ///
    /// `orders.balance` → `SELECT value FROM orders WHERE key = 'balance'`
    ///
    /// This is a reference implementation. Real database adapters may
    /// use a different query format based on their schema.
    pub fn path_to_query(path: &str) -> String {
        let parts: Vec<&str> = path.splitn(2, '.').collect();
        match parts.len() {
            1 => format!("SELECT value FROM {}", parts[0]),
            _ => format!("SELECT value FROM {} WHERE key = '{}'", parts[0], parts[1]),
        }
    }
}

#[async_trait]
impl FactAdapter for DatabaseAdapter {
    async fn fetch(
        &self,
        _fact_id: &str,
        source: &StructuredSourceRef,
        source_fields: &BTreeMap<String, String>,
    ) -> Result<serde_json::Value, AdapterError> {
        let query_str = Self::path_to_query(&source.path);
        self.query_impl
            .query(&self.source_id, &query_str, source_fields)
            .await
    }

    fn adapter_id(&self) -> &str {
        "database"
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_returns_not_configured() {
        let adapter = DatabaseAdapter::new_stub("my_db");
        let source = StructuredSourceRef {
            source_id: "my_db".to_string(),
            path: "users.email".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert!(matches!(result, Err(AdapterError::NotConfigured { .. })));
    }

    #[test]
    fn path_to_query_with_key() {
        assert_eq!(
            DatabaseAdapter::path_to_query("orders.balance"),
            "SELECT value FROM orders WHERE key = 'balance'"
        );
    }

    #[test]
    fn path_to_query_table_only() {
        assert_eq!(
            DatabaseAdapter::path_to_query("orders"),
            "SELECT value FROM orders"
        );
    }

    #[test]
    fn path_to_query_deep_path() {
        // Only splits on first dot — the rest is the key
        assert_eq!(
            DatabaseAdapter::path_to_query("orders.items.count"),
            "SELECT value FROM orders WHERE key = 'items.count'"
        );
    }

    /// Test with a custom DatabaseQuery implementation.
    struct MockQuery;

    #[async_trait]
    impl DatabaseQuery for MockQuery {
        async fn query(
            &self,
            _source_id: &str,
            query_str: &str,
            _source_fields: &BTreeMap<String, String>,
        ) -> Result<serde_json::Value, AdapterError> {
            // Return a fixed value based on the query
            if query_str.contains("balance") {
                Ok(serde_json::json!(42))
            } else {
                Err(AdapterError::FetchFailed {
                    source_id: "test".to_string(),
                    message: "not found".to_string(),
                })
            }
        }
    }

    #[tokio::test]
    async fn custom_query_impl() {
        let adapter = DatabaseAdapter::new("test_db", Box::new(MockQuery));
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "orders.balance".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert_eq!(result.unwrap(), serde_json::json!(42));
    }

    #[tokio::test]
    async fn custom_query_impl_not_found() {
        let adapter = DatabaseAdapter::new("test_db", Box::new(MockQuery));
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "users.name".to_string(),
        };
        let result = adapter.fetch("f1", &source, &BTreeMap::new()).await;
        assert!(matches!(result, Err(AdapterError::FetchFailed { .. })));
    }

    // -----------------------------------------------------------------------
    // Database adapter protocol tests (Wave 2E)
    // -----------------------------------------------------------------------

    /// Mock that returns a successful JSON value for any query.
    struct SuccessfulQuery {
        value: serde_json::Value,
    }

    #[async_trait]
    impl DatabaseQuery for SuccessfulQuery {
        async fn query(
            &self,
            _source_id: &str,
            _query_str: &str,
            _source_fields: &BTreeMap<String, String>,
        ) -> Result<serde_json::Value, AdapterError> {
            Ok(self.value.clone())
        }
    }

    #[tokio::test]
    async fn database_successful_query_returns_correct_value() {
        let adapter = DatabaseAdapter::new(
            "test_db",
            Box::new(SuccessfulQuery {
                value: serde_json::json!({"balance": 1500}),
            }),
        );
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "accounts.balance".to_string(),
        };
        let result = adapter
            .fetch("account_balance", &source, &BTreeMap::new())
            .await;
        assert_eq!(result.unwrap(), serde_json::json!({"balance": 1500}));
    }

    /// Mock that simulates "no rows returned" (empty result).
    struct NoRowsQuery;

    #[async_trait]
    impl DatabaseQuery for NoRowsQuery {
        async fn query(
            &self,
            source_id: &str,
            query_str: &str,
            _source_fields: &BTreeMap<String, String>,
        ) -> Result<serde_json::Value, AdapterError> {
            Err(AdapterError::FetchFailed {
                source_id: source_id.to_string(),
                message: format!("query returned no rows: {}", query_str),
            })
        }
    }

    #[tokio::test]
    async fn database_no_rows_returns_fetch_failed() {
        let adapter = DatabaseAdapter::new("test_db", Box::new(NoRowsQuery));
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "users.email".to_string(),
        };
        let result = adapter.fetch("user_email", &source, &BTreeMap::new()).await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "no rows should produce FetchFailed, got: {:?}",
            result
        );
        if let Err(AdapterError::FetchFailed { message, .. }) = &result {
            assert!(
                message.contains("no rows"),
                "error should mention no rows, got: {}",
                message
            );
        }
    }

    /// Mock that simulates a type mismatch (wrong column type).
    struct WrongTypeQuery;

    #[async_trait]
    impl DatabaseQuery for WrongTypeQuery {
        async fn query(
            &self,
            _source_id: &str,
            _query_str: &str,
            _source_fields: &BTreeMap<String, String>,
        ) -> Result<serde_json::Value, AdapterError> {
            Err(AdapterError::TypeMismatch {
                fact_id: "price".to_string(),
                message: "expected numeric type, got TEXT".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn database_wrong_type_returns_type_mismatch() {
        let adapter = DatabaseAdapter::new("test_db", Box::new(WrongTypeQuery));
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "products.price".to_string(),
        };
        let result = adapter.fetch("price", &source, &BTreeMap::new()).await;
        assert!(
            matches!(result, Err(AdapterError::TypeMismatch { .. })),
            "wrong type should produce TypeMismatch, got: {:?}",
            result
        );
        if let Err(AdapterError::TypeMismatch { message, .. }) = &result {
            assert!(
                message.contains("expected numeric type"),
                "error should describe the type mismatch, got: {}",
                message
            );
        }
    }

    /// Mock that simulates a connection failure.
    struct ConnectionFailureQuery;

    #[async_trait]
    impl DatabaseQuery for ConnectionFailureQuery {
        async fn query(
            &self,
            source_id: &str,
            _query_str: &str,
            _source_fields: &BTreeMap<String, String>,
        ) -> Result<serde_json::Value, AdapterError> {
            Err(AdapterError::FetchFailed {
                source_id: source_id.to_string(),
                message: "connection refused: could not connect to database at localhost:5432"
                    .to_string(),
            })
        }
    }

    #[tokio::test]
    async fn database_connection_failure_returns_fetch_failed() {
        let adapter = DatabaseAdapter::new("test_db", Box::new(ConnectionFailureQuery));
        let source = StructuredSourceRef {
            source_id: "test_db".to_string(),
            path: "orders.total".to_string(),
        };
        let result = adapter
            .fetch("order_total", &source, &BTreeMap::new())
            .await;
        assert!(
            matches!(result, Err(AdapterError::FetchFailed { .. })),
            "connection failure should produce FetchFailed, got: {:?}",
            result
        );
        if let Err(AdapterError::FetchFailed { message, .. }) = &result {
            assert!(
                message.contains("connection refused"),
                "error should mention connection refused, got: {}",
                message
            );
        }
    }
}
