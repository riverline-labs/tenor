//! Atomic migration executor.
//!
//! Drives entity state transitions from a `MigrationPlan` through a
//! `TenorStorage` backend using snapshot (transaction) semantics.
//! All entities are migrated atomically -- either all succeed and the
//! snapshot is committed, or any failure aborts the entire snapshot (M4).
//!
//! Every migrated entity gets a provenance record inserted in the same
//! snapshot as the state change, satisfying C7.

use serde::Serialize;
use tenor_storage::{
    EntityTransitionRecord, OperationExecutionRecord, ProvenanceRecord, StorageError, TenorStorage,
};

use super::error::MigrationError;
use super::plan::MigrationPlan;

/// Result of executing a migration plan.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationResult {
    pub entities_migrated: Vec<EntityMigrationRecord>,
    pub provenance_records_created: usize,
    pub success: bool,
}

/// Record of a single entity instance migration.
#[derive(Debug, Clone, Serialize)]
pub struct EntityMigrationRecord {
    pub entity_id: String,
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
    pub from_version: i64,
    pub to_version: i64,
}

/// Execute a migration plan atomically via a TenorStorage backend.
///
/// For each `EntityStateMapping` in the plan:
/// 1. Read current entity state (with lock)
/// 2. Validate state matches expected `from_state`
/// 3. Update entity state to `to_state`
/// 4. Insert operation execution, entity transition, and provenance records
///
/// All mutations happen within a single storage snapshot (transaction).
/// If any step fails, the snapshot is aborted and no entities are migrated.
pub async fn execute_migration<S: TenorStorage>(
    storage: &S,
    plan: &MigrationPlan,
) -> Result<MigrationResult, MigrationError> {
    // Empty plan -> nothing to do
    if plan.entity_state_mappings.is_empty() {
        return Ok(MigrationResult {
            entities_migrated: Vec::new(),
            provenance_records_created: 0,
            success: true,
        });
    }

    let mut snapshot = storage.begin_snapshot().await.map_err(storage_err)?;

    let mut migrated = Vec::new();
    let mut provenance_count = 0;

    for mapping in &plan.entity_state_mappings {
        // 1. Read current state with lock
        let current = match storage
            .get_entity_state_for_update(&mut snapshot, &mapping.entity_id, &mapping.instance_id)
            .await
        {
            Ok(rec) => rec,
            Err(e) => {
                let _ = storage.abort_snapshot(snapshot).await;
                return Err(storage_err(e));
            }
        };

        // 2. Validate state matches expected from_state
        if current.state != mapping.from_state {
            let _ = storage.abort_snapshot(snapshot).await;
            return Err(MigrationError::StateMismatch {
                entity_id: mapping.entity_id.clone(),
                instance_id: mapping.instance_id.clone(),
                expected: mapping.from_state.clone(),
                found: current.state,
            });
        }

        // 3. Update entity state
        let new_version = match storage
            .update_entity_state(
                &mut snapshot,
                &mapping.entity_id,
                &mapping.instance_id,
                current.version,
                &mapping.to_state,
                "migration",
                "migration",
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                let _ = storage.abort_snapshot(snapshot).await;
                return Err(storage_err(e));
            }
        };

        // 4. Insert operation execution record
        let op_exec_id = format!("mig-op-{}-{}", mapping.entity_id, mapping.instance_id);
        let op_record = OperationExecutionRecord {
            id: op_exec_id.clone(),
            flow_execution_id: "migration".to_string(),
            operation_id: "migration".to_string(),
            persona_id: "system".to_string(),
            outcome: "success".to_string(),
            executed_at: now_iso8601(),
            step_id: "migration".to_string(),
        };
        if let Err(e) = storage
            .insert_operation_execution(&mut snapshot, op_record)
            .await
        {
            let _ = storage.abort_snapshot(snapshot).await;
            return Err(storage_err(e));
        }

        // 5. Insert entity transition record
        let transition_id = format!("mig-tr-{}-{}", mapping.entity_id, mapping.instance_id);
        let transition_record = EntityTransitionRecord {
            id: transition_id,
            operation_execution_id: op_exec_id.clone(),
            entity_id: mapping.entity_id.clone(),
            instance_id: mapping.instance_id.clone(),
            from_state: mapping.from_state.clone(),
            to_state: mapping.to_state.clone(),
            from_version: current.version,
            to_version: new_version,
        };
        if let Err(e) = storage
            .insert_entity_transition(&mut snapshot, transition_record)
            .await
        {
            let _ = storage.abort_snapshot(snapshot).await;
            return Err(storage_err(e));
        }

        // 6. Insert provenance record (C7: same snapshot as state change)
        let provenance_id = format!("mig-prov-{}-{}", mapping.entity_id, mapping.instance_id);
        let provenance_record = ProvenanceRecord {
            id: provenance_id,
            operation_execution_id: op_exec_id,
            facts_used: serde_json::json!({}),
            verdicts_used: serde_json::json!({}),
            verdict_set_snapshot: serde_json::json!({
                "migration": true,
                "from_contract": plan.v1_id,
                "to_contract": plan.v2_id
            }),
        };
        if let Err(e) = storage
            .insert_provenance_record(&mut snapshot, provenance_record)
            .await
        {
            let _ = storage.abort_snapshot(snapshot).await;
            return Err(storage_err(e));
        }

        provenance_count += 1;

        migrated.push(EntityMigrationRecord {
            entity_id: mapping.entity_id.clone(),
            instance_id: mapping.instance_id.clone(),
            from_state: mapping.from_state.clone(),
            to_state: mapping.to_state.clone(),
            from_version: current.version,
            to_version: new_version,
        });
    }

    // All entities processed -- commit atomically (M4)
    storage
        .commit_snapshot(snapshot)
        .await
        .map_err(storage_err)?;

    Ok(MigrationResult {
        entities_migrated: migrated,
        provenance_records_created: provenance_count,
        success: true,
    })
}

/// Generate a simple ISO 8601 timestamp.
fn now_iso8601() -> String {
    // Use the time crate which is already a dependency of tenor-eval.
    let now = time::OffsetDateTime::now_utc();
    // Format manually to avoid format_description macro overhead
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Convert a StorageError into a MigrationError.
fn storage_err(e: StorageError) -> MigrationError {
    MigrationError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::analysis::{MigrationAnalysis, MigrationSeverity};
    use super::super::plan::{EntityStateMapping, MigrationPlan, MigrationPolicy};
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use tenor_storage::{
        EntityStateRecord, EntityTransitionRecord, FlowExecutionRecord, OperationExecutionRecord,
        ProvenanceRecord, StorageError, TenorStorage,
    };

    // ── Mock storage ──────────────────────────────────────────────────

    /// In-memory mock storage for testing the executor.
    #[derive(Clone)]
    struct MockStorage {
        inner: Arc<Mutex<MockInner>>,
    }

    #[derive(Default)]
    struct MockInner {
        entities: Vec<EntityStateRecord>,
        op_executions: Vec<OperationExecutionRecord>,
        transitions: Vec<EntityTransitionRecord>,
        provenance: Vec<ProvenanceRecord>,
        committed: bool,
        aborted: bool,
    }

    /// The snapshot for MockStorage is just a marker -- all state is shared.
    struct MockSnapshot;

    impl MockStorage {
        fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(MockInner::default())),
            }
        }

        fn with_entities(entities: Vec<EntityStateRecord>) -> Self {
            let s = Self::new();
            s.inner.lock().unwrap().entities = entities;
            s
        }

        fn was_committed(&self) -> bool {
            self.inner.lock().unwrap().committed
        }

        fn was_aborted(&self) -> bool {
            self.inner.lock().unwrap().aborted
        }

        fn provenance_count(&self) -> usize {
            self.inner.lock().unwrap().provenance.len()
        }

        fn transition_count(&self) -> usize {
            self.inner.lock().unwrap().transitions.len()
        }

        fn op_execution_count(&self) -> usize {
            self.inner.lock().unwrap().op_executions.len()
        }

        fn entity_state(&self, entity_id: &str, instance_id: &str) -> Option<String> {
            let inner = self.inner.lock().unwrap();
            inner
                .entities
                .iter()
                .find(|e| e.entity_id == entity_id && e.instance_id == instance_id)
                .map(|e| e.state.clone())
        }
    }

    #[async_trait]
    impl TenorStorage for MockStorage {
        type Snapshot = MockSnapshot;

        async fn begin_snapshot(&self) -> Result<MockSnapshot, StorageError> {
            Ok(MockSnapshot)
        }

        async fn commit_snapshot(&self, _snapshot: MockSnapshot) -> Result<(), StorageError> {
            self.inner.lock().unwrap().committed = true;
            Ok(())
        }

        async fn abort_snapshot(&self, _snapshot: MockSnapshot) -> Result<(), StorageError> {
            // On abort, roll back all changes by clearing recorded data
            let mut inner = self.inner.lock().unwrap();
            inner.aborted = true;
            inner.op_executions.clear();
            inner.transitions.clear();
            inner.provenance.clear();
            Ok(())
        }

        async fn initialize_entity(
            &self,
            _snapshot: &mut MockSnapshot,
            entity_id: &str,
            instance_id: &str,
            initial_state: &str,
        ) -> Result<(), StorageError> {
            let mut inner = self.inner.lock().unwrap();
            if inner
                .entities
                .iter()
                .any(|e| e.entity_id == entity_id && e.instance_id == instance_id)
            {
                return Err(StorageError::AlreadyInitialized {
                    entity_id: entity_id.to_string(),
                    instance_id: instance_id.to_string(),
                });
            }
            inner.entities.push(EntityStateRecord {
                entity_id: entity_id.to_string(),
                instance_id: instance_id.to_string(),
                state: initial_state.to_string(),
                version: 0,
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                last_flow_id: None,
                last_operation_id: None,
            });
            Ok(())
        }

        async fn get_entity_state_for_update(
            &self,
            _snapshot: &mut MockSnapshot,
            entity_id: &str,
            instance_id: &str,
        ) -> Result<EntityStateRecord, StorageError> {
            let inner = self.inner.lock().unwrap();
            inner
                .entities
                .iter()
                .find(|e| e.entity_id == entity_id && e.instance_id == instance_id)
                .cloned()
                .ok_or(StorageError::EntityNotFound {
                    entity_id: entity_id.to_string(),
                    instance_id: instance_id.to_string(),
                })
        }

        async fn update_entity_state(
            &self,
            _snapshot: &mut MockSnapshot,
            entity_id: &str,
            instance_id: &str,
            expected_version: i64,
            new_state: &str,
            flow_id: &str,
            operation_id: &str,
        ) -> Result<i64, StorageError> {
            let mut inner = self.inner.lock().unwrap();
            let entity = inner
                .entities
                .iter_mut()
                .find(|e| e.entity_id == entity_id && e.instance_id == instance_id)
                .ok_or(StorageError::EntityNotFound {
                    entity_id: entity_id.to_string(),
                    instance_id: instance_id.to_string(),
                })?;
            if entity.version != expected_version {
                return Err(StorageError::ConcurrentConflict {
                    entity_id: entity_id.to_string(),
                    instance_id: instance_id.to_string(),
                    expected_version,
                });
            }
            entity.state = new_state.to_string();
            entity.version += 1;
            entity.last_flow_id = Some(flow_id.to_string());
            entity.last_operation_id = Some(operation_id.to_string());
            Ok(entity.version)
        }

        async fn insert_flow_execution(
            &self,
            _snapshot: &mut MockSnapshot,
            _record: FlowExecutionRecord,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn insert_operation_execution(
            &self,
            _snapshot: &mut MockSnapshot,
            record: OperationExecutionRecord,
        ) -> Result<(), StorageError> {
            self.inner.lock().unwrap().op_executions.push(record);
            Ok(())
        }

        async fn insert_entity_transition(
            &self,
            _snapshot: &mut MockSnapshot,
            record: EntityTransitionRecord,
        ) -> Result<(), StorageError> {
            self.inner.lock().unwrap().transitions.push(record);
            Ok(())
        }

        async fn insert_provenance_record(
            &self,
            _snapshot: &mut MockSnapshot,
            record: ProvenanceRecord,
        ) -> Result<(), StorageError> {
            self.inner.lock().unwrap().provenance.push(record);
            Ok(())
        }

        async fn get_entity_state(
            &self,
            entity_id: &str,
            instance_id: &str,
        ) -> Result<EntityStateRecord, StorageError> {
            let inner = self.inner.lock().unwrap();
            inner
                .entities
                .iter()
                .find(|e| e.entity_id == entity_id && e.instance_id == instance_id)
                .cloned()
                .ok_or(StorageError::EntityNotFound {
                    entity_id: entity_id.to_string(),
                    instance_id: instance_id.to_string(),
                })
        }

        async fn list_entity_states(
            &self,
            entity_id: &str,
            state_filter: Option<&str>,
        ) -> Result<Vec<EntityStateRecord>, StorageError> {
            let inner = self.inner.lock().unwrap();
            Ok(inner
                .entities
                .iter()
                .filter(|e| e.entity_id == entity_id && state_filter.map_or(true, |s| e.state == s))
                .cloned()
                .collect())
        }

        async fn get_flow_execution(
            &self,
            _execution_id: &str,
        ) -> Result<FlowExecutionRecord, StorageError> {
            Err(StorageError::ExecutionNotFound {
                execution_id: _execution_id.to_string(),
            })
        }

        async fn get_provenance(
            &self,
            operation_execution_id: &str,
        ) -> Result<Vec<ProvenanceRecord>, StorageError> {
            let inner = self.inner.lock().unwrap();
            Ok(inner
                .provenance
                .iter()
                .filter(|p| p.operation_execution_id == operation_execution_id)
                .cloned()
                .collect())
        }

        async fn list_flow_executions(
            &self,
            _flow_id: Option<&str>,
            _outcome: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<FlowExecutionRecord>, StorageError> {
            Ok(Vec::new())
        }
    }

    // ── Test helpers ──────────────────────────────────────────────────

    fn make_entity_record(entity_id: &str, instance_id: &str, state: &str) -> EntityStateRecord {
        EntityStateRecord {
            entity_id: entity_id.to_string(),
            instance_id: instance_id.to_string(),
            state: state.to_string(),
            version: 0,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            last_flow_id: None,
            last_operation_id: None,
        }
    }

    fn make_plan(v1_id: &str, v2_id: &str, mappings: Vec<EntityStateMapping>) -> MigrationPlan {
        MigrationPlan {
            v1_id: v1_id.to_string(),
            v2_id: v2_id.to_string(),
            analysis: MigrationAnalysis {
                entity_changes: Vec::new(),
                breaking_changes: Vec::new(),
                overall_severity: MigrationSeverity::Breaking,
            },
            flow_compatibility: Vec::new(),
            entity_state_mappings: mappings,
            severity: MigrationSeverity::Breaking,
            recommended_policy: MigrationPolicy::Abort,
        }
    }

    fn make_mapping(
        entity_id: &str,
        instance_id: &str,
        from: &str,
        to: &str,
    ) -> EntityStateMapping {
        EntityStateMapping {
            entity_id: entity_id.to_string(),
            instance_id: instance_id.to_string(),
            from_state: from.to_string(),
            to_state: to.to_string(),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn single_entity_migration_succeeds() {
        let storage =
            MockStorage::with_entities(vec![make_entity_record("Order", "order-1", "approved")]);

        let plan = make_plan(
            "contract_v1",
            "contract_v2",
            vec![make_mapping("Order", "order-1", "approved", "draft")],
        );

        let result = execute_migration(&storage, &plan).await.unwrap();

        assert!(result.success);
        assert_eq!(result.entities_migrated.len(), 1);
        assert_eq!(result.provenance_records_created, 1);

        let rec = &result.entities_migrated[0];
        assert_eq!(rec.entity_id, "Order");
        assert_eq!(rec.instance_id, "order-1");
        assert_eq!(rec.from_state, "approved");
        assert_eq!(rec.to_state, "draft");
        assert_eq!(rec.from_version, 0);
        assert_eq!(rec.to_version, 1);

        // Verify storage state
        assert!(storage.was_committed());
        assert!(!storage.was_aborted());
        assert_eq!(
            storage.entity_state("Order", "order-1"),
            Some("draft".to_string())
        );
        assert_eq!(storage.provenance_count(), 1);
        assert_eq!(storage.transition_count(), 1);
        assert_eq!(storage.op_execution_count(), 1);
    }

    #[tokio::test]
    async fn multi_entity_migration_is_atomic() {
        let storage = MockStorage::with_entities(vec![
            make_entity_record("Order", "order-1", "approved"),
            make_entity_record("Order", "order-2", "approved"),
            make_entity_record("Invoice", "inv-1", "pending"),
        ]);

        let plan = make_plan(
            "v1",
            "v2",
            vec![
                make_mapping("Order", "order-1", "approved", "draft"),
                make_mapping("Order", "order-2", "approved", "draft"),
                make_mapping("Invoice", "inv-1", "pending", "open"),
            ],
        );

        let result = execute_migration(&storage, &plan).await.unwrap();

        assert!(result.success);
        assert_eq!(result.entities_migrated.len(), 3);
        assert_eq!(result.provenance_records_created, 3);
        assert!(storage.was_committed());

        // All states transitioned
        assert_eq!(
            storage.entity_state("Order", "order-1"),
            Some("draft".to_string())
        );
        assert_eq!(
            storage.entity_state("Order", "order-2"),
            Some("draft".to_string())
        );
        assert_eq!(
            storage.entity_state("Invoice", "inv-1"),
            Some("open".to_string())
        );
    }

    #[tokio::test]
    async fn state_mismatch_aborts_all() {
        let storage = MockStorage::with_entities(vec![
            make_entity_record("Order", "order-1", "approved"),
            make_entity_record("Order", "order-2", "submitted"), // wrong state!
        ]);

        let plan = make_plan(
            "v1",
            "v2",
            vec![
                make_mapping("Order", "order-1", "approved", "draft"),
                make_mapping("Order", "order-2", "approved", "draft"), // expects "approved"
            ],
        );

        let result = execute_migration(&storage, &plan).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            MigrationError::StateMismatch {
                entity_id,
                instance_id,
                expected,
                found,
            } => {
                assert_eq!(entity_id, "Order");
                assert_eq!(instance_id, "order-2");
                assert_eq!(expected, "approved");
                assert_eq!(found, "submitted");
            }
            other => panic!("expected StateMismatch, got: {}", other),
        }

        // Snapshot was aborted -- provenance and transitions should be cleared
        assert!(storage.was_aborted());
        assert_eq!(storage.provenance_count(), 0);
        assert_eq!(storage.transition_count(), 0);
    }

    #[tokio::test]
    async fn empty_plan_succeeds() {
        let storage = MockStorage::new();
        let plan = make_plan("v1", "v2", vec![]);

        let result = execute_migration(&storage, &plan).await.unwrap();

        assert!(result.success);
        assert!(result.entities_migrated.is_empty());
        assert_eq!(result.provenance_records_created, 0);
    }
}
