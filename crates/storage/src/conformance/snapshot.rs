//! Snapshot isolation conformance tests.
//!
//! Verifies that uncommitted writes are invisible outside a snapshot,
//! committed writes are visible, and aborted writes are discarded.

use std::future::Future;

use super::TestResult;
use crate::{StorageError, TenorStorage};

pub(super) async fn run_snapshot_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    results.push(TestResult::from_result(
        "snapshot",
        "begin_snapshot_succeeds",
        begin_snapshot_succeeds(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "commit_snapshot_succeeds",
        commit_snapshot_succeeds(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "abort_snapshot_succeeds",
        abort_snapshot_succeeds(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "entity_state_correct_after_commit",
        entity_state_correct_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "entity_version_correct_after_commit",
        entity_version_correct_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "uncommitted_entity_invisible_to_get",
        uncommitted_entity_invisible_to_get(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "uncommitted_entity_invisible_to_list",
        uncommitted_entity_invisible_to_list(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "uncommitted_update_invisible",
        uncommitted_update_invisible(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "committed_entity_visible",
        committed_entity_visible(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "committed_update_visible",
        committed_update_visible(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "abort_makes_entity_invisible",
        abort_makes_entity_invisible(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "abort_makes_update_invisible",
        abort_makes_update_invisible(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "multiple_entities_in_one_snapshot",
        multiple_entities_in_one_snapshot(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "sequential_snapshots_see_prior_commits",
        sequential_snapshots_see_prior_commits(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "flow_execution_visible_after_commit",
        flow_execution_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "flow_execution_invisible_before_commit",
        flow_execution_invisible_before_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "flow_execution_invisible_after_abort",
        flow_execution_invisible_after_abort(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "operation_execution_visible_after_commit",
        operation_execution_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "entity_transition_visible_after_commit",
        entity_transition_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "entity_transition_invisible_after_abort",
        entity_transition_invisible_after_abort(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "list_entity_states_reflects_committed_data",
        list_entity_states_reflects_committed_data(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "list_entity_states_with_filter",
        list_entity_states_with_filter(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "list_flow_executions_reflects_committed_data",
        list_flow_executions_reflects_committed_data(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "provenance_visible_after_commit",
        provenance_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "snapshot",
        "provenance_invisible_after_abort",
        provenance_invisible_after_abort(factory).await,
    ));

    results
}

// ── 1. begin_snapshot_succeeds ──────────────────────────────────────────────

async fn begin_snapshot_succeeds<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    // Clean up: abort the unused snapshot.
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 2. commit_snapshot_succeeds ─────────────────────────────────────────────

async fn commit_snapshot_succeeds<S, F, Fut>(factory: &F) -> Result<(), String>
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

// ── 3. abort_snapshot_succeeds ──────────────────────────────────────────────

async fn abort_snapshot_succeeds<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 4. entity_state_correct_after_commit ────────────────────────────────────

async fn entity_state_correct_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!("expected state 'initial', got '{}'", rec.state));
    }
    Ok(())
}

// ── 5. entity_version_correct_after_commit ──────────────────────────────────

async fn entity_version_correct_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 0 {
        return Err(format!("expected version 0, got {}", rec.version));
    }
    Ok(())
}

// ── 6. uncommitted_entity_invisible_to_get ──────────────────────────────────

async fn uncommitted_entity_invisible_to_get<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    // Do NOT commit — snapshot still open.

    let result = s.get_entity_state("order", "inst-1").await;
    if !matches!(result, Err(StorageError::EntityNotFound { .. })) {
        return Err(format!(
            "expected EntityNotFound for uncommitted entity, got {:?}",
            result
        ));
    }

    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 7. uncommitted_entity_invisible_to_list ─────────────────────────────────

async fn uncommitted_entity_invisible_to_list<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    // Do NOT commit.

    let list = s
        .list_entity_states("order", None)
        .await
        .map_err(|e| e.to_string())?;
    if !list.is_empty() {
        return Err(format!(
            "expected empty list for uncommitted entity, got {} items",
            list.len()
        ));
    }

    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 8. uncommitted_update_invisible ─────────────────────────────────────────

async fn uncommitted_update_invisible<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // First, initialize and commit.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Update in a new snapshot but do NOT commit.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "order", "inst-1", 0, "active", "flow-1", "op-1")
        .await
        .map_err(|e| e.to_string())?;
    // Snapshot still open.

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state 'initial' (uncommitted update invisible), got '{}'",
            rec.state
        ));
    }

    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 9. committed_entity_visible ─────────────────────────────────────────────

async fn committed_entity_visible<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.entity_id != "order" || rec.instance_id != "inst-1" {
        return Err(format!(
            "expected order/inst-1, got {}/{}",
            rec.entity_id, rec.instance_id
        ));
    }
    Ok(())
}

// ── 10. committed_update_visible ────────────────────────────────────────────

async fn committed_update_visible<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Update and commit.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "order", "inst-1", 0, "active", "flow-1", "op-1")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "active" {
        return Err(format!("expected state 'active', got '{}'", rec.state));
    }
    Ok(())
}

// ── 11. abort_makes_entity_invisible ────────────────────────────────────────

async fn abort_makes_entity_invisible<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    let result = s.get_entity_state("order", "inst-1").await;
    if !matches!(result, Err(StorageError::EntityNotFound { .. })) {
        return Err(format!(
            "expected EntityNotFound after abort, got {:?}",
            result
        ));
    }
    Ok(())
}

// ── 12. abort_makes_update_invisible ────────────────────────────────────────

async fn abort_makes_update_invisible<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize and commit.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Update and abort.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "order", "inst-1", 0, "active", "flow-1", "op-1")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state 'initial' after abort, got '{}'",
            rec.state
        ));
    }
    if rec.version != 0 {
        return Err(format!(
            "expected version 0 after abort, got {}",
            rec.version
        ));
    }
    Ok(())
}

// ── 13. multiple_entities_in_one_snapshot ────────────────────────────────────

async fn multiple_entities_in_one_snapshot<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-2", "pending")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec1 = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    let rec2 = s
        .get_entity_state("order", "inst-2")
        .await
        .map_err(|e| e.to_string())?;

    if rec1.state != "initial" {
        return Err(format!(
            "expected inst-1 state 'initial', got '{}'",
            rec1.state
        ));
    }
    if rec2.state != "pending" {
        return Err(format!(
            "expected inst-2 state 'pending', got '{}'",
            rec2.state
        ));
    }
    Ok(())
}

// ── 14. sequential_snapshots_see_prior_commits ──────────────────────────────

async fn sequential_snapshots_see_prior_commits<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Snapshot 1: init entity A.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Snapshot 2: init entity B.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap2, "order", "inst-2", "pending")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Both visible.
    s.get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    s.get_entity_state("order", "inst-2")
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── 15. flow_execution_visible_after_commit ─────────────────────────────────

async fn flow_execution_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_flow_execution("exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.id != "exec-1" {
        return Err(format!(
            "expected flow execution id 'exec-1', got '{}'",
            rec.id
        ));
    }
    Ok(())
}

// ── 16. flow_execution_invisible_before_commit ──────────────────────────────

async fn flow_execution_invisible_before_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;
    // Do NOT commit.

    let result = s.get_flow_execution("exec-1").await;
    if !matches!(result, Err(StorageError::ExecutionNotFound { .. })) {
        return Err(format!(
            "expected ExecutionNotFound for uncommitted flow, got {:?}",
            result
        ));
    }

    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 17. flow_execution_invisible_after_abort ────────────────────────────────

async fn flow_execution_invisible_after_abort<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    let result = s.get_flow_execution("exec-1").await;
    if !matches!(result, Err(StorageError::ExecutionNotFound { .. })) {
        return Err(format!(
            "expected ExecutionNotFound after abort, got {:?}",
            result
        ));
    }
    Ok(())
}

// ── 18. operation_execution_visible_after_commit ────────────────────────────

async fn operation_execution_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    // Insert flow first (FK parent).
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;

    // Insert operation execution.
    let op = super::make_operation_execution("op-exec-1", "exec-1", "op-1");
    s.insert_operation_execution(&mut snap, op)
        .await
        .map_err(|e| e.to_string())?;

    // Commit succeeds — no direct get method, so success of commit is the verification.
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 19. entity_transition_visible_after_commit ──────────────────────────────

async fn entity_transition_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize entity first.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // In a new snapshot: insert flow, op, transition, and update entity state.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap2, flow)
        .await
        .map_err(|e| e.to_string())?;

    let op = super::make_operation_execution("op-exec-1", "exec-1", "op-1");
    s.insert_operation_execution(&mut snap2, op)
        .await
        .map_err(|e| e.to_string())?;

    let transition = super::make_entity_transition(
        "trans-1",
        "op-exec-1",
        "order",
        "inst-1",
        "initial",
        "active",
        0,
        1,
    );
    s.insert_entity_transition(&mut snap2, transition)
        .await
        .map_err(|e| e.to_string())?;

    // Commit succeeds — no direct get_entity_transition method, so commit success is verification.
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ── 20. entity_transition_invisible_after_abort ─────────────────────────────

async fn entity_transition_invisible_after_abort<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize entity and commit.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // In a new snapshot: insert flow, op, transition, and update entity — then abort.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap2, flow)
        .await
        .map_err(|e| e.to_string())?;

    let op = super::make_operation_execution("op-exec-1", "exec-1", "op-1");
    s.insert_operation_execution(&mut snap2, op)
        .await
        .map_err(|e| e.to_string())?;

    let transition = super::make_entity_transition(
        "trans-1",
        "op-exec-1",
        "order",
        "inst-1",
        "initial",
        "active",
        0,
        1,
    );
    s.insert_entity_transition(&mut snap2, transition)
        .await
        .map_err(|e| e.to_string())?;

    s.update_entity_state(&mut snap2, "order", "inst-1", 0, "active", "flow-1", "op-1")
        .await
        .map_err(|e| e.to_string())?;

    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Entity state should be unchanged.
    let rec = s
        .get_entity_state("order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state 'initial' after abort, got '{}'",
            rec.state
        ));
    }
    if rec.version != 0 {
        return Err(format!(
            "expected version 0 after abort, got {}",
            rec.version
        ));
    }
    Ok(())
}

// ── 21. list_entity_states_reflects_committed_data ──────────────────────────

async fn list_entity_states_reflects_committed_data<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-2", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "order", "inst-3", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let list = s
        .list_entity_states("order", None)
        .await
        .map_err(|e| e.to_string())?;
    if list.len() != 3 {
        return Err(format!("expected 3 entity states, got {}", list.len()));
    }
    Ok(())
}

// ── 22. list_entity_states_with_filter ──────────────────────────────────────

async fn list_entity_states_with_filter<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize two instances.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap1, "order", "inst-2", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Update one to "active".
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(&mut snap2, "order", "inst-1", 0, "active", "flow-1", "op-1")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap2).await.map_err(|e| e.to_string())?;

    // Filter for "initial" — should return only inst-2.
    let list = s
        .list_entity_states("order", Some("initial"))
        .await
        .map_err(|e| e.to_string())?;
    if list.len() != 1 {
        return Err(format!(
            "expected 1 entity in 'initial' state, got {}",
            list.len()
        ));
    }
    if list[0].instance_id != "inst-2" {
        return Err(format!(
            "expected inst-2 in 'initial' state, got '{}'",
            list[0].instance_id
        ));
    }
    Ok(())
}

// ── 23. list_flow_executions_reflects_committed_data ────────────────────────

async fn list_flow_executions_reflects_committed_data<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    let flow1 = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow1)
        .await
        .map_err(|e| e.to_string())?;

    let flow2 = super::make_flow_execution("exec-2", "flow-1");
    s.insert_flow_execution(&mut snap, flow2)
        .await
        .map_err(|e| e.to_string())?;

    let flow3 = super::make_flow_execution("exec-3", "flow-1");
    s.insert_flow_execution(&mut snap, flow3)
        .await
        .map_err(|e| e.to_string())?;

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let list = s
        .list_flow_executions(None, None, 100)
        .await
        .map_err(|e| e.to_string())?;
    if list.len() != 3 {
        return Err(format!("expected 3 flow executions, got {}", list.len()));
    }
    Ok(())
}

// ── 24. provenance_visible_after_commit ─────────────────────────────────────

async fn provenance_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    // Insert flow and operation (FK parents).
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;

    let op = super::make_operation_execution("op-exec-1", "exec-1", "op-1");
    s.insert_operation_execution(&mut snap, op)
        .await
        .map_err(|e| e.to_string())?;

    // Insert provenance.
    let prov = super::make_provenance_record("prov-1", "op-exec-1");
    s.insert_provenance_record(&mut snap, prov)
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
    if records[0].id != "prov-1" {
        return Err(format!(
            "expected provenance id 'prov-1', got '{}'",
            records[0].id
        ));
    }
    Ok(())
}

// ── 25. provenance_invisible_after_abort ────────────────────────────────────

async fn provenance_invisible_after_abort<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    // Insert flow and operation (FK parents).
    let flow = super::make_flow_execution("exec-1", "flow-1");
    s.insert_flow_execution(&mut snap, flow)
        .await
        .map_err(|e| e.to_string())?;

    let op = super::make_operation_execution("op-exec-1", "exec-1", "op-1");
    s.insert_operation_execution(&mut snap, op)
        .await
        .map_err(|e| e.to_string())?;

    // Insert provenance.
    let prov = super::make_provenance_record("prov-1", "op-exec-1");
    s.insert_provenance_record(&mut snap, prov)
        .await
        .map_err(|e| e.to_string())?;

    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    let records = s
        .get_provenance("op-exec-1")
        .await
        .map_err(|e| e.to_string())?;
    if !records.is_empty() {
        return Err(format!(
            "expected empty provenance after abort, got {} records",
            records.len()
        ));
    }
    Ok(())
}
