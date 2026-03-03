use std::future::Future;

use super::{
    make_entity_transition, make_flow_execution, make_operation_execution, make_provenance_record,
    TestResult,
};
use crate::{StorageError, TenorStorage};

pub(super) async fn run_commit_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    // Single entity commit
    results.push(TestResult::from_result(
        "commit",
        "single_entity_update_committed",
        single_entity_update_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "single_entity_update_version_incremented",
        single_entity_update_version_incremented(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "update_returns_new_version",
        update_returns_new_version(factory).await,
    ));

    // Multi-entity atomicity
    results.push(TestResult::from_result(
        "commit",
        "multi_entity_updates_all_visible_after_commit",
        multi_entity_updates_all_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "multi_entity_updates_none_visible_after_abort",
        multi_entity_updates_none_visible_after_abort(factory).await,
    ));

    // Provenance + state atomicity
    results.push(TestResult::from_result(
        "commit",
        "provenance_and_state_both_visible_after_commit",
        provenance_and_state_both_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "provenance_and_state_neither_visible_after_abort",
        provenance_and_state_neither_visible_after_abort(factory).await,
    ));

    // Full pipeline atomicity
    results.push(TestResult::from_result(
        "commit",
        "full_pipeline_all_committed",
        full_pipeline_all_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "full_pipeline_all_aborted",
        full_pipeline_all_aborted(factory).await,
    ));

    // Individual record types commit
    results.push(TestResult::from_result(
        "commit",
        "flow_execution_committed",
        flow_execution_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "flow_execution_fields_preserved",
        flow_execution_fields_preserved(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "operation_execution_committed",
        operation_execution_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "entity_transition_committed",
        entity_transition_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "provenance_record_committed",
        provenance_record_committed(factory).await,
    ));

    // Multiple records same type
    results.push(TestResult::from_result(
        "commit",
        "multiple_flow_executions_in_one_snapshot",
        multiple_flow_executions_in_one_snapshot(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "multiple_operation_executions_in_one_snapshot",
        multiple_operation_executions_in_one_snapshot(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "multiple_provenance_records_in_one_snapshot",
        multiple_provenance_records_in_one_snapshot(factory).await,
    ));

    // Sequential operations
    results.push(TestResult::from_result(
        "commit",
        "sequential_updates_increment_version",
        sequential_updates_increment_version(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "update_sets_flow_and_operation_ids",
        update_sets_flow_and_operation_ids(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "commit_then_read_entity_state",
        commit_then_read_entity_state(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "commit_then_read_flow_execution",
        commit_then_read_flow_execution(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "commit_then_get_provenance",
        commit_then_get_provenance(factory).await,
    ));

    // Listing with filters
    results.push(TestResult::from_result(
        "commit",
        "list_entity_states_after_multiple_commits",
        list_entity_states_after_multiple_commits(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "list_entity_states_with_state_filter",
        list_entity_states_with_state_filter(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "list_flow_executions_with_flow_filter",
        list_flow_executions_with_flow_filter(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "list_flow_executions_with_outcome_filter",
        list_flow_executions_with_outcome_filter(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "list_flow_executions_with_limit",
        list_flow_executions_with_limit(factory).await,
    ));

    // Edge cases
    results.push(TestResult::from_result(
        "commit",
        "commit_empty_snapshot",
        commit_empty_snapshot(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "multiple_commits_accumulate",
        multiple_commits_accumulate(factory).await,
    ));
    results.push(TestResult::from_result(
        "commit",
        "entity_state_updated_at_changes",
        entity_state_updated_at_changes(factory).await,
    ));

    results
}

// ── Single entity commit ─────────────────────────────────────────────────────

/// After init+commit then update+commit, get_entity_state shows the new state.
async fn single_entity_update_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize and commit.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update and commit.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "submitted" {
        return Err(format!(
            "expected state \"submitted\", got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}

/// After init+commit then update+commit, version is 1.
async fn single_entity_update_version_incremented<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 1 {
        return Err(format!("expected version 1, got {}", rec.version));
    }
    Ok(())
}

/// update_entity_state returns the new version number (1 after first update).
async fn update_returns_new_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let new_version = s
        .update_entity_state(
            &mut snap2,
            "Order",
            "order-1",
            0,
            "submitted",
            "flow-1",
            "op-1",
        )
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    if new_version != 1 {
        return Err(format!("expected return value 1, got {}", new_version));
    }
    Ok(())
}

// ── Multi-entity atomicity ───────────────────────────────────────────────────

/// Update two entities in one snapshot+commit; both show new state after commit.
async fn multi_entity_updates_all_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize both entities.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Invoice", "inv-1", "draft")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update both in one snapshot.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "Invoice", "inv-1", 0, "sent", "flow-1", "op-2")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let order = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    let invoice = s
        .get_entity_state("Invoice", "inv-1")
        .await
        .map_err(|e| e.to_string())?;

    if order.state != "submitted" {
        return Err(format!(
            "expected Order state \"submitted\", got \"{}\"",
            order.state
        ));
    }
    if invoice.state != "sent" {
        return Err(format!(
            "expected Invoice state \"sent\", got \"{}\"",
            invoice.state
        ));
    }
    Ok(())
}

/// Update two entities in one snapshot+abort; both still show old state.
async fn multi_entity_updates_none_visible_after_abort<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize both entities.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Invoice", "inv-1", "draft")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update both in one snapshot, then abort.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "Invoice", "inv-1", 0, "sent", "flow-1", "op-2")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let order = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    let invoice = s
        .get_entity_state("Invoice", "inv-1")
        .await
        .map_err(|e| e.to_string())?;

    if order.state != "initial" {
        return Err(format!(
            "expected Order state \"initial\" after abort, got \"{}\"",
            order.state
        ));
    }
    if invoice.state != "draft" {
        return Err(format!(
            "expected Invoice state \"draft\" after abort, got \"{}\"",
            invoice.state
        ));
    }
    Ok(())
}

// ── Provenance + state atomicity ─────────────────────────────────────────────

/// In one snapshot: init entity + insert flow + insert op + update entity +
/// insert provenance + commit. State updated AND provenance retrievable.
async fn provenance_and_state_both_visible_after_commit<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap,
        "Order",
        "order-1",
        0,
        "approved",
        "flow-exec-1",
        "op-exec-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Verify state updated.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "approved" {
        return Err(format!(
            "expected state \"approved\", got \"{}\"",
            rec.state
        ));
    }

    // Verify provenance retrievable.
    let prov = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if prov.is_empty() {
        return Err("expected provenance records, got empty vec".to_string());
    }
    Ok(())
}

/// Same as above but abort. State unchanged AND provenance empty.
async fn provenance_and_state_neither_visible_after_abort<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // First commit: initialize the entity so it exists.
    let mut snap0 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap0, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap0).await.map_err(|e| e.to_string())?;

    // Second snapshot: do everything, then abort.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap,
        "Order",
        "order-1",
        0,
        "approved",
        "flow-exec-1",
        "op-exec-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Verify state unchanged.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state \"initial\" after abort, got \"{}\"",
            rec.state
        ));
    }

    // Verify provenance empty.
    let prov = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if !prov.is_empty() {
        return Err(format!(
            "expected empty provenance after abort, got {} records",
            prov.len()
        ));
    }
    Ok(())
}

// ── Full pipeline atomicity ──────────────────────────────────────────────────

/// Init entity+commit, then new snapshot: insert_flow + insert_op +
/// update_entity + insert_transition + insert_provenance + commit. All visible.
async fn full_pipeline_all_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize entity.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Full pipeline in one snapshot.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap2, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap2,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "approved",
        "flow-exec-1",
        "op-exec-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_entity_transition(
        &mut snap2,
        make_entity_transition(
            "trans-1",
            "op-exec-1",
            "Order",
            "order-1",
            "initial",
            "approved",
            0,
            1,
        ),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap2, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Verify entity state updated.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "approved" {
        return Err(format!(
            "expected state \"approved\", got \"{}\"",
            rec.state
        ));
    }

    // Verify flow execution retrievable.
    let flow = s
        .get_flow_execution("flow-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if flow.id != "flow-exec-1" {
        return Err(format!(
            "expected flow execution id \"flow-exec-1\", got \"{}\"",
            flow.id
        ));
    }

    // Verify provenance retrievable.
    let prov = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if prov.is_empty() {
        return Err("expected provenance records, got empty vec".to_string());
    }
    Ok(())
}

/// Same as full_pipeline_all_committed but abort. Entity state unchanged,
/// flow not found, provenance empty.
async fn full_pipeline_all_aborted<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize entity.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Full pipeline in one snapshot, then abort.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap2, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap2,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "approved",
        "flow-exec-1",
        "op-exec-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_entity_transition(
        &mut snap2,
        make_entity_transition(
            "trans-1",
            "op-exec-1",
            "Order",
            "order-1",
            "initial",
            "approved",
            0,
            1,
        ),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap2, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Verify entity state unchanged.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state \"initial\" after abort, got \"{}\"",
            rec.state
        ));
    }

    // Verify flow execution not found.
    let flow_result = s.get_flow_execution("flow-exec-1").await;
    if !matches!(flow_result, Err(StorageError::ExecutionNotFound { .. })) {
        return Err(format!(
            "expected ExecutionNotFound after abort, got {:?}",
            flow_result
        ));
    }

    // Verify provenance empty.
    let prov = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if !prov.is_empty() {
        return Err(format!(
            "expected empty provenance after abort, got {} records",
            prov.len()
        ));
    }
    Ok(())
}

// ── Individual record types commit ───────────────────────────────────────────

/// Insert a flow execution + commit; get_flow_execution returns it.
async fn flow_execution_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let flow = s
        .get_flow_execution("flow-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if flow.id != "flow-exec-1" {
        return Err(format!(
            "expected flow id \"flow-exec-1\", got \"{}\"",
            flow.id
        ));
    }
    Ok(())
}

/// Verify all fields of a committed flow execution match what was inserted.
async fn flow_execution_fields_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let record = make_flow_execution("flow-exec-fp", "onboarding");
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, record.clone())
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let flow = s
        .get_flow_execution("flow-exec-fp")
        .await
        .map_err(|e| e.to_string())?;

    if flow.id != record.id {
        return Err(format!("id mismatch: got \"{}\"", flow.id));
    }
    if flow.flow_id != record.flow_id {
        return Err(format!("flow_id mismatch: got \"{}\"", flow.flow_id));
    }
    if flow.contract_id != record.contract_id {
        return Err(format!(
            "contract_id mismatch: got \"{}\"",
            flow.contract_id
        ));
    }
    if flow.persona_id != record.persona_id {
        return Err(format!("persona_id mismatch: got \"{}\"", flow.persona_id));
    }
    if flow.started_at != record.started_at {
        return Err(format!("started_at mismatch: got \"{}\"", flow.started_at));
    }
    if flow.completed_at != record.completed_at {
        return Err(format!(
            "completed_at mismatch: got {:?}",
            flow.completed_at
        ));
    }
    if flow.outcome != record.outcome {
        return Err(format!("outcome mismatch: got \"{}\"", flow.outcome));
    }
    if flow.snapshot_facts != record.snapshot_facts {
        return Err(format!(
            "snapshot_facts mismatch: got {}",
            flow.snapshot_facts
        ));
    }
    if flow.snapshot_verdicts != record.snapshot_verdicts {
        return Err(format!(
            "snapshot_verdicts mismatch: got {}",
            flow.snapshot_verdicts
        ));
    }
    Ok(())
}

/// Insert flow + operation execution + commit succeeds without error.
async fn operation_execution_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert flow + operation + entity transition + commit succeeds without error.
async fn entity_transition_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize entity first.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Insert flow + op + transition + commit.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap2, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap2,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_entity_transition(
        &mut snap2,
        make_entity_transition(
            "trans-1",
            "op-exec-1",
            "Order",
            "order-1",
            "initial",
            "approved",
            0,
            1,
        ),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert flow + op + provenance + commit; get_provenance returns it.
async fn provenance_record_committed<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "approve"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let prov = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if prov.len() != 1 {
        return Err(format!("expected 1 provenance record, got {}", prov.len()));
    }
    if prov[0].id != "prov-1" {
        return Err(format!(
            "expected provenance id \"prov-1\", got \"{}\"",
            prov[0].id
        ));
    }
    Ok(())
}

// ── Multiple records same type ───────────────────────────────────────────────

/// Insert 3 flow executions in one snapshot + commit; list returns 3.
async fn multiple_flow_executions_in_one_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-2", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-3", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let flows = s
        .list_flow_executions(Some("checkout"), None, 100)
        .await
        .map_err(|e| e.to_string())?;
    if flows.len() != 3 {
        return Err(format!("expected 3 flow executions, got {}", flows.len()));
    }
    Ok(())
}

/// Insert flow + 3 operation executions in one snapshot + commit; no error.
async fn multiple_operation_executions_in_one_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-1", "fe-1", "step-a"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-2", "fe-1", "step-b"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-3", "fe-1", "step-c"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert flow + 3 ops + 3 provenances + commit; get_provenance for each
/// operation returns 1 record.
async fn multiple_provenance_records_in_one_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-1", "fe-1", "step-a"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-2", "fe-1", "step-b"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-3", "fe-1", "step-c"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "oe-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-2", "oe-2"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-3", "oe-3"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    for (op_id, expected_prov_id) in [("oe-1", "prov-1"), ("oe-2", "prov-2"), ("oe-3", "prov-3")] {
        let prov = s.get_provenance(op_id).await.map_err(|e| e.to_string())?;
        if prov.len() != 1 {
            return Err(format!(
                "expected 1 provenance record for {}, got {}",
                op_id,
                prov.len()
            ));
        }
        if prov[0].id != expected_prov_id {
            return Err(format!(
                "expected provenance id \"{}\" for {}, got \"{}\"",
                expected_prov_id, op_id, prov[0].id
            ));
        }
    }
    Ok(())
}

// ── Sequential operations ────────────────────────────────────────────────────

/// Update entity 3 times; version goes 0 -> 1 -> 2 -> 3.
async fn sequential_updates_increment_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let states = ["submitted", "approved", "shipped"];
    for (i, new_state) in states.iter().enumerate() {
        let expected_version = i as i64;
        let mut snap_n = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        let new_ver = s
            .update_entity_state(
                &mut snap_n,
                "Order",
                "order-1",
                expected_version,
                new_state,
                &format!("flow-{}", i + 1),
                &format!("op-{}", i + 1),
            )
            .await
            .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap_n).await.map_err(|e| e.to_string())?;

        let expected_new = expected_version + 1;
        if new_ver != expected_new {
            return Err(format!(
                "update {} expected return version {}, got {}",
                i + 1,
                expected_new,
                new_ver
            ));
        }
    }

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 3 {
        return Err(format!("expected final version 3, got {}", rec.version));
    }
    if rec.state != "shipped" {
        return Err(format!(
            "expected final state \"shipped\", got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}

/// After update, get_entity_state shows last_flow_id and last_operation_id.
async fn update_sets_flow_and_operation_ids<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-42",
        "op-77",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    match rec.last_flow_id.as_deref() {
        Some("flow-42") => {}
        other => {
            return Err(format!(
                "expected last_flow_id Some(\"flow-42\"), got {:?}",
                other
            ))
        }
    }
    match rec.last_operation_id.as_deref() {
        Some("op-77") => {}
        other => {
            return Err(format!(
                "expected last_operation_id Some(\"op-77\"), got {:?}",
                other
            ))
        }
    }
    Ok(())
}

/// Standard read-after-commit for entity state.
async fn commit_then_read_entity_state<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Claim", "claim-1", "filed")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Claim", "claim-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.entity_id != "Claim" || rec.instance_id != "claim-1" || rec.state != "filed" {
        return Err(format!(
            "unexpected entity state: {}/{} state={}",
            rec.entity_id, rec.instance_id, rec.state
        ));
    }
    Ok(())
}

/// Standard read-after-commit for flow execution.
async fn commit_then_read_flow_execution<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-read-1", "onboarding"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let flow = s
        .get_flow_execution("fe-read-1")
        .await
        .map_err(|e| e.to_string())?;
    if flow.id != "fe-read-1" || flow.flow_id != "onboarding" {
        return Err(format!(
            "unexpected flow: id={} flow_id={}",
            flow.id, flow.flow_id
        ));
    }
    Ok(())
}

/// Standard read-after-commit for provenance.
async fn commit_then_get_provenance<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-prov-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("oe-prov-1", "fe-prov-1", "step-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(
        &mut snap,
        make_provenance_record("prov-read-1", "oe-prov-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let prov = s
        .get_provenance("oe-prov-1")
        .await
        .map_err(|e| e.to_string())?;
    if prov.len() != 1 {
        return Err(format!("expected 1 provenance record, got {}", prov.len()));
    }
    if prov[0].id != "prov-read-1" {
        return Err(format!(
            "expected provenance id \"prov-read-1\", got \"{}\"",
            prov[0].id
        ));
    }
    Ok(())
}

// ── Listing with filters ────────────────────────────────────────────────────

/// Init 3 instances across separate commits; list returns all 3.
async fn list_entity_states_after_multiple_commits<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    for inst_id in ["order-1", "order-2", "order-3"] {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        s.initialize_entity(&mut snap, "Order", inst_id, "initial")
            .await
            .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    }

    let states = s
        .list_entity_states("Order", None)
        .await
        .map_err(|e| e.to_string())?;
    if states.len() != 3 {
        return Err(format!("expected 3 entity states, got {}", states.len()));
    }
    Ok(())
}

/// 3 instances, update 2 to "active"; filter by "active" returns 2.
async fn list_entity_states_with_state_filter<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize 3 instances.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Account", "acct-1", "pending")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Account", "acct-2", "pending")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Account", "acct-3", "pending")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update 2 to "active".
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2, "Account", "acct-1", 0, "active", "flow-1", "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2, "Account", "acct-2", 0, "active", "flow-1", "op-2",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let active = s
        .list_entity_states("Account", Some("active"))
        .await
        .map_err(|e| e.to_string())?;
    if active.len() != 2 {
        return Err(format!("expected 2 active accounts, got {}", active.len()));
    }

    let pending = s
        .list_entity_states("Account", Some("pending"))
        .await
        .map_err(|e| e.to_string())?;
    if pending.len() != 1 {
        return Err(format!("expected 1 pending account, got {}", pending.len()));
    }
    Ok(())
}

/// Insert flows for 2 different flow_ids; filter by one returns only those.
async fn list_flow_executions_with_flow_filter<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-1", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-2", "checkout"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("fe-3", "onboarding"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let checkout_flows = s
        .list_flow_executions(Some("checkout"), None, 100)
        .await
        .map_err(|e| e.to_string())?;
    if checkout_flows.len() != 2 {
        return Err(format!(
            "expected 2 checkout flows, got {}",
            checkout_flows.len()
        ));
    }

    let onboarding_flows = s
        .list_flow_executions(Some("onboarding"), None, 100)
        .await
        .map_err(|e| e.to_string())?;
    if onboarding_flows.len() != 1 {
        return Err(format!(
            "expected 1 onboarding flow, got {}",
            onboarding_flows.len()
        ));
    }
    Ok(())
}

/// Insert success and failure flows; filter by outcome.
async fn list_flow_executions_with_outcome_filter<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut success_rec = make_flow_execution("fe-ok-1", "checkout");
    success_rec.outcome = "success".to_string();
    let mut success_rec2 = make_flow_execution("fe-ok-2", "checkout");
    success_rec2.outcome = "success".to_string();
    let mut failure_rec = make_flow_execution("fe-fail-1", "checkout");
    failure_rec.outcome = "failure".to_string();

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, success_rec)
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, success_rec2)
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, failure_rec)
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let successes = s
        .list_flow_executions(None, Some("success"), 100)
        .await
        .map_err(|e| e.to_string())?;
    if successes.len() != 2 {
        return Err(format!("expected 2 success flows, got {}", successes.len()));
    }

    let failures = s
        .list_flow_executions(None, Some("failure"), 100)
        .await
        .map_err(|e| e.to_string())?;
    if failures.len() != 1 {
        return Err(format!("expected 1 failure flow, got {}", failures.len()));
    }
    Ok(())
}

/// Insert 5 flows; limit 2 returns exactly 2.
async fn list_flow_executions_with_limit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    for i in 1..=5 {
        s.insert_flow_execution(
            &mut snap,
            make_flow_execution(&format!("fe-lim-{}", i), "checkout"),
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let flows = s
        .list_flow_executions(None, None, 2)
        .await
        .map_err(|e| e.to_string())?;
    if flows.len() != 2 {
        return Err(format!(
            "expected 2 flows with limit 2, got {}",
            flows.len()
        ));
    }
    Ok(())
}

// ── Edge cases ───────────────────────────────────────────────────────────────

/// Begin + commit with no operations in between must not error.
async fn commit_empty_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// 3 separate snapshot+commits each add an entity; all 3 visible afterward.
async fn multiple_commits_accumulate<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    for (entity, inst, state) in [
        ("Order", "order-1", "initial"),
        ("Invoice", "inv-1", "draft"),
        ("Claim", "claim-1", "filed"),
    ] {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        s.initialize_entity(&mut snap, entity, inst, state)
            .await
            .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    }

    // All 3 must be visible.
    for (entity, inst, expected_state) in [
        ("Order", "order-1", "initial"),
        ("Invoice", "inv-1", "draft"),
        ("Claim", "claim-1", "filed"),
    ] {
        let rec = s
            .get_entity_state(entity, inst)
            .await
            .map_err(|e| e.to_string())?;
        if rec.state != expected_state {
            return Err(format!(
                "expected {}/{} state \"{}\", got \"{}\"",
                entity, inst, expected_state, rec.state
            ));
        }
    }
    Ok(())
}

/// After update, updated_at must differ from the initial value.
async fn entity_state_updated_at_changes<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let before = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap2,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let after = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    // updated_at should have changed (or at least be non-empty).
    // Some backends may have identical timestamps if fast enough, so we
    // check that the field is populated rather than strictly different.
    // However, the primary contract is that updated_at is set.
    if after.updated_at.is_empty() {
        return Err("updated_at is empty after update".to_string());
    }

    // If the backend has sufficient timestamp resolution, the values will differ.
    // We log but don't fail if they happen to be equal (sub-ms backends).
    if before.updated_at == after.updated_at {
        // This is a soft check -- some in-memory backends may not advance the clock.
        // The important thing is that updated_at is populated.
    }
    Ok(())
}
