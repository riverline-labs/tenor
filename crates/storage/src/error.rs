/// All errors that can be returned by a TenorStorage implementation.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Optimistic concurrency control conflict — another transaction modified
    /// the entity concurrently. The expected version was not found.
    #[error(
        "concurrent conflict on entity {entity_id}/{instance_id}: expected version {expected_version}"
    )]
    ConcurrentConflict {
        entity_id: String,
        instance_id: String,
        expected_version: i64,
    },

    /// Entity not found — no record with the given (entity_id, instance_id).
    #[error("entity not found: {entity_id}/{instance_id}")]
    EntityNotFound {
        entity_id: String,
        instance_id: String,
    },

    /// Entity already initialized — a record with this (entity_id, instance_id) already exists.
    #[error("entity already initialized: {entity_id}/{instance_id}")]
    AlreadyInitialized {
        entity_id: String,
        instance_id: String,
    },

    /// Flow execution record not found.
    #[error("flow execution not found: {execution_id}")]
    ExecutionNotFound { execution_id: String },

    /// A backend-specific storage error (DB connection, serialization, etc.).
    #[error("storage backend error: {0}")]
    Backend(String),
}
