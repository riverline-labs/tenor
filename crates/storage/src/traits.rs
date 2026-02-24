use async_trait::async_trait;

use crate::error::StorageError;
use crate::record::{
    EntityStateRecord, EntityTransitionRecord, FlowExecutionRecord, OperationExecutionRecord,
    ProvenanceRecord,
};

/// The storage trait for Tenor execution backends.
///
/// A `TenorStorage` implementation provides durable, transactional storage
/// for entity state, flow executions, operation executions, entity transitions,
/// and provenance records.
///
/// ## Snapshot Semantics
///
/// All mutating operations take `&mut Self::Snapshot`, a type representing an
/// in-progress transaction. The lifecycle is:
///
/// 1. `begin_snapshot()` — start a transaction, returns a `Snapshot`
/// 2. Call mutating methods with `&mut snapshot`
/// 3. `commit_snapshot(snapshot)` — commit and consume the transaction
///    OR `abort_snapshot(snapshot)` — roll back and consume the transaction
///
/// If a `Snapshot` is dropped without committing, the underlying transaction
/// MUST be rolled back (drop semantics on the underlying DB transaction).
///
/// ## OCC Conflict Detection
///
/// `update_entity_state` performs an optimistic concurrency check:
/// `UPDATE WHERE version = expected_version`. If zero rows are affected,
/// the method returns `Err(StorageError::ConcurrentConflict { ... })`.
///
/// ## Thread Safety
///
/// Implementations must be `Send + Sync + 'static` to be used in axum
/// application state and across async task boundaries.
#[async_trait]
pub trait TenorStorage: Send + Sync + 'static {
    /// The snapshot (transaction) type used by this storage backend.
    ///
    /// Must be `Send` to allow passing across async task boundaries.
    type Snapshot: Send;

    // ── Snapshot lifecycle ────────────────────────────────────────────────────

    /// Begin a new snapshot (transaction).
    async fn begin_snapshot(&self) -> Result<Self::Snapshot, StorageError>;

    /// Commit a snapshot, making all mutations durable.
    async fn commit_snapshot(&self, snapshot: Self::Snapshot) -> Result<(), StorageError>;

    /// Abort (roll back) a snapshot, discarding all mutations.
    async fn abort_snapshot(&self, snapshot: Self::Snapshot) -> Result<(), StorageError>;

    // ── Entity operations (within snapshot) ──────────────────────────────────

    /// Initialize a new entity instance at state `"initial"` with version 0.
    ///
    /// Returns `Err(StorageError::AlreadyInitialized)` if the entity already exists.
    async fn initialize_entity(
        &self,
        snapshot: &mut Self::Snapshot,
        entity_id: &str,
        instance_id: &str,
    ) -> Result<(), StorageError>;

    /// Read an entity's current state, locking the row for update.
    ///
    /// Uses `SELECT ... FOR UPDATE` semantics to prevent concurrent
    /// modification until the snapshot is committed or aborted.
    ///
    /// Returns `Err(StorageError::EntityNotFound)` if the entity does not exist.
    async fn get_entity_state_for_update(
        &self,
        snapshot: &mut Self::Snapshot,
        entity_id: &str,
        instance_id: &str,
    ) -> Result<EntityStateRecord, StorageError>;

    /// Apply a version-validated UPDATE to an entity's state (OCC).
    ///
    /// The UPDATE is conditional on `version = expected_version`.
    /// If zero rows are affected, returns `Err(StorageError::ConcurrentConflict)`.
    ///
    /// Returns the new version number on success.
    #[allow(clippy::too_many_arguments)]
    async fn update_entity_state(
        &self,
        snapshot: &mut Self::Snapshot,
        entity_id: &str,
        instance_id: &str,
        expected_version: i64,
        new_state: &str,
        flow_id: &str,
        operation_id: &str,
    ) -> Result<i64, StorageError>;

    // ── Recording operations (within snapshot) ────────────────────────────────

    /// Insert a flow execution record.
    async fn insert_flow_execution(
        &self,
        snapshot: &mut Self::Snapshot,
        record: FlowExecutionRecord,
    ) -> Result<(), StorageError>;

    /// Insert an operation execution record.
    ///
    /// Must be inserted BEFORE any `entity_transitions` that reference it,
    /// due to the FK constraint `entity_transitions.operation_execution_id`.
    async fn insert_operation_execution(
        &self,
        snapshot: &mut Self::Snapshot,
        record: OperationExecutionRecord,
    ) -> Result<(), StorageError>;

    /// Insert an entity transition record.
    ///
    /// FK: `operation_execution_id` must reference an existing `operation_executions` row.
    async fn insert_entity_transition(
        &self,
        snapshot: &mut Self::Snapshot,
        record: EntityTransitionRecord,
    ) -> Result<(), StorageError>;

    /// Insert a provenance record (C7 atomicity).
    ///
    /// CRITICAL: Must be inserted in the SAME snapshot (transaction) as the
    /// `update_entity_state` call. This is what enforces C7: no state change
    /// without provenance.
    ///
    /// FK: `operation_execution_id` must reference an existing `operation_executions` row.
    async fn insert_provenance_record(
        &self,
        snapshot: &mut Self::Snapshot,
        record: ProvenanceRecord,
    ) -> Result<(), StorageError>;

    // ── Query operations (outside snapshot, against pool/connection) ──────────

    /// Read an entity's current state without locking.
    ///
    /// Returns `Err(StorageError::EntityNotFound)` if the entity does not exist.
    async fn get_entity_state(
        &self,
        entity_id: &str,
        instance_id: &str,
    ) -> Result<EntityStateRecord, StorageError>;

    /// List all instances of an entity, optionally filtered by state.
    async fn list_entity_states(
        &self,
        entity_id: &str,
        state_filter: Option<&str>,
    ) -> Result<Vec<EntityStateRecord>, StorageError>;

    /// Read a flow execution record by execution ID.
    ///
    /// Returns `Err(StorageError::ExecutionNotFound)` if not found.
    async fn get_flow_execution(
        &self,
        execution_id: &str,
    ) -> Result<FlowExecutionRecord, StorageError>;

    /// List flow executions with optional filters.
    ///
    /// - `flow_id`: filter to a specific flow
    /// - `outcome`: filter to a specific outcome string (e.g. `"success"`)
    /// - `limit`: maximum number of results (0 = no limit)
    async fn list_flow_executions(
        &self,
        flow_id: Option<&str>,
        outcome: Option<&str>,
        limit: usize,
    ) -> Result<Vec<FlowExecutionRecord>, StorageError>;
}
