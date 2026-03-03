use std::future::Future;

use super::{make_flow_execution, make_operation_execution, make_provenance_record, TestResult};
use crate::{ProvenanceRecord, StorageError, TenorStorage};

pub(super) async fn run_provenance_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    // Basic retrieval
    results.push(TestResult::from_result(
        "provenance",
        "get_provenance_after_commit",
        get_provenance_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "get_provenance_after_abort_returns_empty",
        get_provenance_after_abort_returns_empty(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "get_provenance_empty_for_unknown_id",
        get_provenance_empty_for_unknown_id(factory).await,
    ));

    // Field preservation
    results.push(TestResult::from_result(
        "provenance",
        "provenance_id_preserved",
        provenance_id_preserved(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_operation_execution_id_preserved",
        provenance_operation_execution_id_preserved(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_facts_used_preserved",
        provenance_facts_used_preserved(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_verdicts_used_preserved",
        provenance_verdicts_used_preserved(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_verdict_set_snapshot_preserved",
        provenance_verdict_set_snapshot_preserved(factory).await,
    ));

    // Multiple records
    results.push(TestResult::from_result(
        "provenance",
        "multiple_provenance_for_different_operations",
        multiple_provenance_for_different_operations(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "multiple_provenance_same_operation",
        multiple_provenance_same_operation(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "multiple_provenance_same_snapshot",
        multiple_provenance_same_snapshot(factory).await,
    ));

    // Atomicity with state
    results.push(TestResult::from_result(
        "provenance",
        "provenance_coupled_with_state_update_committed",
        provenance_coupled_with_state_update_committed(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_coupled_with_state_update_aborted",
        provenance_coupled_with_state_update_aborted(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_not_visible_before_commit",
        provenance_not_visible_before_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_visible_in_new_snapshot_after_commit",
        provenance_visible_in_new_snapshot_after_commit(factory).await,
    ));

    // Cross-snapshot
    results.push(TestResult::from_result(
        "provenance",
        "provenance_from_different_snapshots",
        provenance_from_different_snapshots(factory).await,
    ));

    // JSON value handling
    results.push(TestResult::from_result(
        "provenance",
        "provenance_with_complex_json",
        provenance_with_complex_json(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_with_empty_arrays",
        provenance_with_empty_arrays(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_with_null_values",
        provenance_with_null_values(factory).await,
    ));
    results.push(TestResult::from_result(
        "provenance",
        "provenance_with_large_json",
        provenance_with_large_json(factory).await,
    ));

    results
}

// ── Basic retrieval ─────────────────────────────────────────────────────────

/// Insert flow + operation + provenance, commit, then get_provenance returns 1 record.
async fn get_provenance_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if records.len() != 1 {
        return Err(format!(
            "expected 1 provenance record, got {}",
            records.len()
        ));
    }
    Ok(())
}

/// Insert flow + operation + provenance, abort, then get_provenance returns empty vec.
async fn get_provenance_after_abort_returns_empty<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if !records.is_empty() {
        return Err(format!(
            "expected empty provenance vec after abort, got {} records",
            records.len()
        ));
    }
    Ok(())
}

/// get_provenance for a nonexistent operation_execution_id returns empty vec.
async fn get_provenance_empty_for_unknown_id<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let records = s
        .get_provenance("nonexistent")
        .await
        .map_err(|e| e.to_string())?;
    if !records.is_empty() {
        return Err(format!(
            "expected empty provenance vec for unknown id, got {} records",
            records.len()
        ));
    }
    Ok(())
}

// ── Field preservation ──────────────────────────────────────────────────────

/// Helper: insert a standard provenance record and return it after commit.
async fn insert_and_retrieve_provenance<S, F, Fut>(factory: &F) -> Result<ProvenanceRecord, String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if records.len() != 1 {
        return Err(format!(
            "expected 1 provenance record, got {}",
            records.len()
        ));
    }
    Ok(records.into_iter().next().unwrap())
}

/// The provenance record id must match what was inserted.
async fn provenance_id_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let rec = insert_and_retrieve_provenance::<S, F, Fut>(factory).await?;
    if rec.id != "prov-1" {
        return Err(format!("expected id \"prov-1\", got \"{}\"", rec.id));
    }
    Ok(())
}

/// The provenance record operation_execution_id must match what was inserted.
async fn provenance_operation_execution_id_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let rec = insert_and_retrieve_provenance::<S, F, Fut>(factory).await?;
    if rec.operation_execution_id != "op-exec-1" {
        return Err(format!(
            "expected operation_execution_id \"op-exec-1\", got \"{}\"",
            rec.operation_execution_id
        ));
    }
    Ok(())
}

/// The provenance record facts_used JSON must match what was inserted.
async fn provenance_facts_used_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let rec = insert_and_retrieve_provenance::<S, F, Fut>(factory).await?;
    let expected = serde_json::json!(["fact_a", "fact_b"]);
    if rec.facts_used != expected {
        return Err(format!(
            "expected facts_used {:?}, got {:?}",
            expected, rec.facts_used
        ));
    }
    Ok(())
}

/// The provenance record verdicts_used JSON must match what was inserted.
async fn provenance_verdicts_used_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let rec = insert_and_retrieve_provenance::<S, F, Fut>(factory).await?;
    let expected = serde_json::json!(["verdict_x"]);
    if rec.verdicts_used != expected {
        return Err(format!(
            "expected verdicts_used {:?}, got {:?}",
            expected, rec.verdicts_used
        ));
    }
    Ok(())
}

/// The provenance record verdict_set_snapshot JSON must match what was inserted.
async fn provenance_verdict_set_snapshot_preserved<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let rec = insert_and_retrieve_provenance::<S, F, Fut>(factory).await?;
    let expected = serde_json::json!({"verdict_x": true});
    if rec.verdict_set_snapshot != expected {
        return Err(format!(
            "expected verdict_set_snapshot {:?}, got {:?}",
            expected, rec.verdict_set_snapshot
        ));
    }
    Ok(())
}

// ── Multiple records ────────────────────────────────────────────────────────

/// Two operations with separate provenance in the same snapshot; each get_provenance
/// returns only its own record.
async fn multiple_provenance_for_different_operations<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-2", "flow-exec-1", "op-2"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-2", "op-exec-2"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records_1 = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    let records_2 = s
        .get_provenance("op-exec-2")
        .await
        .map_err(|e| e.to_string())?;

    if records_1.len() != 1 {
        return Err(format!(
            "expected 1 provenance record for op-exec-1, got {}",
            records_1.len()
        ));
    }
    if records_2.len() != 1 {
        return Err(format!(
            "expected 1 provenance record for op-exec-2, got {}",
            records_2.len()
        ));
    }
    if records_1[0].id != "prov-1" {
        return Err(format!(
            "expected prov-1 for op-exec-1, got \"{}\"",
            records_1[0].id
        ));
    }
    if records_2[0].id != "prov-2" {
        return Err(format!(
            "expected prov-2 for op-exec-2, got \"{}\"",
            records_2[0].id
        ));
    }
    Ok(())
}

/// Two provenance records for the same operation execution (different ids);
/// get_provenance returns both.
async fn multiple_provenance_same_operation<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-2", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if records.len() != 2 {
        return Err(format!(
            "expected 2 provenance records for op-exec-1, got {}",
            records.len()
        ));
    }
    // Verify both ids are present (order may vary).
    let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
    if !ids.contains(&"prov-1") || !ids.contains(&"prov-2") {
        return Err(format!(
            "expected provenance ids [\"prov-1\", \"prov-2\"], got {:?}",
            ids
        ));
    }
    Ok(())
}

/// Three different operations, three provenances, all in one snapshot + commit;
/// all retrievable independently.
async fn multiple_provenance_same_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-2", "flow-exec-1", "op-2"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-3", "flow-exec-1", "op-3"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-2", "op-exec-2"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-3", "op-exec-3"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    for (op_exec_id, expected_prov_id) in [
        ("op-exec-1", "prov-1"),
        ("op-exec-2", "prov-2"),
        ("op-exec-3", "prov-3"),
    ] {
        let records = s
            .get_provenance(op_exec_id)
            .await
            .map_err(|e| e.to_string())?;
        if records.len() != 1 {
            return Err(format!(
                "expected 1 provenance record for {}, got {}",
                op_exec_id,
                records.len()
            ));
        }
        if records[0].id != expected_prov_id {
            return Err(format!(
                "expected provenance id \"{}\" for {}, got \"{}\"",
                expected_prov_id, op_exec_id, records[0].id
            ));
        }
    }
    Ok(())
}

// ── Atomicity with state ────────────────────────────────────────────────────

/// In one snapshot: init entity, insert flow + op, update entity state, insert
/// provenance, commit. Provenance must be visible AND entity state must be updated.
async fn provenance_coupled_with_state_update_committed<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // First snapshot: initialize the entity.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Second snapshot: flow + op + state update + provenance, all atomically.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap2, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap2,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
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
    s.insert_provenance_record(&mut snap2, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Verify provenance is visible.
    let prov_records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if prov_records.len() != 1 {
        return Err(format!(
            "expected 1 provenance record, got {}",
            prov_records.len()
        ));
    }

    // Verify entity state was updated.
    let entity = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if entity.state != "submitted" {
        return Err(format!(
            "expected entity state \"submitted\", got \"{}\"",
            entity.state
        ));
    }
    if entity.version != 1 {
        return Err(format!("expected entity version 1, got {}", entity.version));
    }
    Ok(())
}

/// In one snapshot: init entity, insert flow + op, update entity state, insert
/// provenance, abort. Provenance must be empty AND entity must not be found
/// (since init was in the same aborted snapshot).
async fn provenance_coupled_with_state_update_aborted<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Single snapshot: init + flow + op + update + provenance, then abort.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap,
        "Order",
        "order-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Verify provenance is empty.
    let prov_records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if !prov_records.is_empty() {
        return Err(format!(
            "expected empty provenance after abort, got {} records",
            prov_records.len()
        ));
    }

    // Verify entity was not persisted (init was in the aborted snapshot).
    let entity_result = s.get_entity_state("Order", "order-1").await;
    match entity_result {
        Err(StorageError::EntityNotFound { .. }) => Ok(()),
        Err(e) => Err(format!("expected EntityNotFound, got: {e}")),
        Ok(_) => Err("entity should not exist after abort".to_string()),
    }
}

/// Insert provenance without committing; get_provenance must return empty.
async fn provenance_not_visible_before_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    // Snapshot is still open -- provenance should not be visible outside it.
    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;

    // Clean up the snapshot regardless.
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    if !records.is_empty() {
        return Err(format!(
            "expected empty provenance before commit, got {} records",
            records.len()
        ));
    }
    Ok(())
}

/// After committing provenance, verify it is durable by reading it back.
async fn provenance_visible_in_new_snapshot_after_commit<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Verify committed data is durable via a fresh read.
    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if records.len() != 1 {
        return Err(format!(
            "expected 1 provenance record after commit, got {}",
            records.len()
        ));
    }
    if records[0].id != "prov-1" {
        return Err(format!(
            "expected provenance id \"prov-1\", got \"{}\"",
            records[0].id
        ));
    }
    Ok(())
}

// ── Cross-snapshot ──────────────────────────────────────────────────────────

/// Snapshot 1: flow1 + op1 + prov1 + commit. Snapshot 2: flow2 + op2 + prov2 + commit.
/// Both provenance records must be retrievable independently.
async fn provenance_from_different_snapshots<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Snapshot 1
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap1, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap1,
        make_operation_execution("op-exec-1", "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap1, make_provenance_record("prov-1", "op-exec-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Snapshot 2
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.insert_flow_execution(&mut snap2, make_flow_execution("flow-exec-2", "flow-2"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap2,
        make_operation_execution("op-exec-2", "flow-exec-2", "op-2"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap2, make_provenance_record("prov-2", "op-exec-2"))
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Verify both are independently retrievable.
    let records_1 = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    let records_2 = s
        .get_provenance("op-exec-2")
        .await
        .map_err(|e| e.to_string())?;

    if records_1.len() != 1 {
        return Err(format!(
            "expected 1 provenance record for op-exec-1, got {}",
            records_1.len()
        ));
    }
    if records_2.len() != 1 {
        return Err(format!(
            "expected 1 provenance record for op-exec-2, got {}",
            records_2.len()
        ));
    }
    if records_1[0].id != "prov-1" {
        return Err(format!("expected prov-1, got \"{}\"", records_1[0].id));
    }
    if records_2[0].id != "prov-2" {
        return Err(format!("expected prov-2, got \"{}\"", records_2[0].id));
    }
    Ok(())
}

// ── JSON value handling ─────────────────────────────────────────────────────

/// Helper: insert a custom provenance record and return it after commit.
async fn insert_custom_provenance_and_retrieve<S, F, Fut>(
    factory: &F,
    record: ProvenanceRecord,
) -> Result<ProvenanceRecord, String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let op_exec_id = record.operation_execution_id.clone();
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    s.insert_flow_execution(&mut snap, make_flow_execution("flow-exec-1", "flow-1"))
        .await
        .map_err(|e| e.to_string())?;
    s.insert_operation_execution(
        &mut snap,
        make_operation_execution(&op_exec_id, "flow-exec-1", "op-1"),
    )
    .await
    .map_err(|e| e.to_string())?;
    s.insert_provenance_record(&mut snap, record)
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance(&op_exec_id)
        .await
        .map_err(|e| e.to_string())?;
    if records.len() != 1 {
        return Err(format!(
            "expected 1 provenance record, got {}",
            records.len()
        ));
    }
    Ok(records.into_iter().next().unwrap())
}

/// Provenance with deeply nested JSON objects must be preserved exactly.
async fn provenance_with_complex_json<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let complex_facts = serde_json::json!({
        "applicant": {
            "name": "Alice",
            "scores": [100, 200, 300],
            "metadata": {
                "source": "api",
                "nested": {"deep": true}
            }
        }
    });
    let complex_verdicts = serde_json::json!({
        "risk_assessment": {
            "level": "low",
            "factors": ["credit_score", "income"],
            "confidence": 0.95
        }
    });
    let complex_snapshot = serde_json::json!({
        "verdict_x": true,
        "verdict_y": false,
        "details": [1, 2, 3]
    });

    let record = ProvenanceRecord {
        id: "prov-complex".to_string(),
        operation_execution_id: "op-exec-1".to_string(),
        facts_used: complex_facts.clone(),
        verdicts_used: complex_verdicts.clone(),
        verdict_set_snapshot: complex_snapshot.clone(),
    };

    let retrieved = insert_custom_provenance_and_retrieve::<S, F, Fut>(factory, record).await?;

    if retrieved.facts_used != complex_facts {
        return Err(format!(
            "facts_used mismatch:\n  expected: {}\n  got: {}",
            complex_facts, retrieved.facts_used
        ));
    }
    if retrieved.verdicts_used != complex_verdicts {
        return Err(format!(
            "verdicts_used mismatch:\n  expected: {}\n  got: {}",
            complex_verdicts, retrieved.verdicts_used
        ));
    }
    if retrieved.verdict_set_snapshot != complex_snapshot {
        return Err(format!(
            "verdict_set_snapshot mismatch:\n  expected: {}\n  got: {}",
            complex_snapshot, retrieved.verdict_set_snapshot
        ));
    }
    Ok(())
}

/// Provenance with empty arrays for facts_used and verdicts_used must be preserved.
async fn provenance_with_empty_arrays<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let record = ProvenanceRecord {
        id: "prov-empty".to_string(),
        operation_execution_id: "op-exec-1".to_string(),
        facts_used: serde_json::json!([]),
        verdicts_used: serde_json::json!([]),
        verdict_set_snapshot: serde_json::json!({}),
    };

    let retrieved = insert_custom_provenance_and_retrieve::<S, F, Fut>(factory, record).await?;

    if retrieved.facts_used != serde_json::json!([]) {
        return Err(format!(
            "expected empty array for facts_used, got {}",
            retrieved.facts_used
        ));
    }
    if retrieved.verdicts_used != serde_json::json!([]) {
        return Err(format!(
            "expected empty array for verdicts_used, got {}",
            retrieved.verdicts_used
        ));
    }
    if retrieved.verdict_set_snapshot != serde_json::json!({}) {
        return Err(format!(
            "expected empty object for verdict_set_snapshot, got {}",
            retrieved.verdict_set_snapshot
        ));
    }
    Ok(())
}

/// Provenance with null verdict_set_snapshot must be preserved.
async fn provenance_with_null_values<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let record = ProvenanceRecord {
        id: "prov-null".to_string(),
        operation_execution_id: "op-exec-1".to_string(),
        facts_used: serde_json::json!(null),
        verdicts_used: serde_json::json!(null),
        verdict_set_snapshot: serde_json::json!(null),
    };

    let retrieved = insert_custom_provenance_and_retrieve::<S, F, Fut>(factory, record).await?;

    if !retrieved.facts_used.is_null() {
        return Err(format!(
            "expected null for facts_used, got {}",
            retrieved.facts_used
        ));
    }
    if !retrieved.verdicts_used.is_null() {
        return Err(format!(
            "expected null for verdicts_used, got {}",
            retrieved.verdicts_used
        ));
    }
    if !retrieved.verdict_set_snapshot.is_null() {
        return Err(format!(
            "expected null for verdict_set_snapshot, got {}",
            retrieved.verdict_set_snapshot
        ));
    }
    Ok(())
}

/// Provenance with a large JSON array (100 items) must be preserved exactly.
async fn provenance_with_large_json<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let large_facts: serde_json::Value =
        serde_json::to_value((0..100).map(|i| format!("fact_{}", i)).collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
    let large_verdicts: serde_json::Value = serde_json::to_value(
        (0..100)
            .map(|i| format!("verdict_{}", i))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| e.to_string())?;
    let large_snapshot: serde_json::Value = {
        let mut map = serde_json::Map::new();
        for i in 0..100 {
            map.insert(format!("verdict_{}", i), serde_json::json!(i % 2 == 0));
        }
        serde_json::Value::Object(map)
    };

    let record = ProvenanceRecord {
        id: "prov-large".to_string(),
        operation_execution_id: "op-exec-1".to_string(),
        facts_used: large_facts.clone(),
        verdicts_used: large_verdicts.clone(),
        verdict_set_snapshot: large_snapshot.clone(),
    };

    let retrieved = insert_custom_provenance_and_retrieve::<S, F, Fut>(factory, record).await?;

    if retrieved.facts_used != large_facts {
        return Err("large facts_used JSON was not preserved exactly".to_string());
    }
    if retrieved.verdicts_used != large_verdicts {
        return Err("large verdicts_used JSON was not preserved exactly".to_string());
    }
    if retrieved.verdict_set_snapshot != large_snapshot {
        return Err("large verdict_set_snapshot JSON was not preserved exactly".to_string());
    }
    Ok(())
}
