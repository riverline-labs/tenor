use std::future::Future;

use super::TestResult;
use crate::{StorageError, TenorStorage};

pub(super) async fn run_error_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    results.push(TestResult::from_result(
        "error",
        "get_entity_state_nonexistent",
        get_entity_state_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_entity_state_not_found_has_correct_fields",
        get_entity_state_not_found_has_correct_fields(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_entity_state_for_update_nonexistent",
        get_entity_state_for_update_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_entity_state_for_update_not_found_has_correct_fields",
        get_entity_state_for_update_not_found_has_correct_fields(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "update_entity_state_nonexistent",
        update_entity_state_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_flow_execution_nonexistent",
        get_flow_execution_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_flow_execution_not_found_has_correct_field",
        get_flow_execution_not_found_has_correct_field(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "get_provenance_empty_for_nonexistent",
        get_provenance_empty_for_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "list_entity_states_empty_for_nonexistent",
        list_entity_states_empty_for_nonexistent(factory).await,
    ));
    results.push(TestResult::from_result(
        "error",
        "list_flow_executions_empty",
        list_flow_executions_empty(factory).await,
    ));

    results
}

// ── 1. get_entity_state on empty store returns EntityNotFound ─────────────────

async fn get_entity_state_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let result = s.get_entity_state("order", "order-999").await;
    match result {
        Err(StorageError::EntityNotFound { .. }) => Ok(()),
        other => Err(format!("expected EntityNotFound, got {:?}", other)),
    }
}

// ── 2. EntityNotFound error carries correct entity_id and instance_id ────────

async fn get_entity_state_not_found_has_correct_fields<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let result = s.get_entity_state("loan", "loan-42").await;
    match result {
        Err(StorageError::EntityNotFound {
            entity_id,
            instance_id,
        }) => {
            if entity_id != "loan" {
                return Err(format!(
                    "expected entity_id \"loan\", got \"{}\"",
                    entity_id
                ));
            }
            if instance_id != "loan-42" {
                return Err(format!(
                    "expected instance_id \"loan-42\", got \"{}\"",
                    instance_id
                ));
            }
            Ok(())
        }
        other => Err(format!("expected EntityNotFound, got {:?}", other)),
    }
}

// ── 3. get_entity_state_for_update on nonexistent entity returns EntityNotFound

async fn get_entity_state_for_update_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .get_entity_state_for_update(&mut snap, "claim", "claim-77")
        .await;
    // Clean up the snapshot regardless of result.
    let _ = s.abort_snapshot(snap).await;
    match result {
        Err(StorageError::EntityNotFound { .. }) => Ok(()),
        other => Err(format!("expected EntityNotFound, got {:?}", other)),
    }
}

// ── 4. get_entity_state_for_update EntityNotFound has correct fields ─────────

async fn get_entity_state_for_update_not_found_has_correct_fields<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .get_entity_state_for_update(&mut snap, "invoice", "inv-123")
        .await;
    let _ = s.abort_snapshot(snap).await;
    match result {
        Err(StorageError::EntityNotFound {
            entity_id,
            instance_id,
        }) => {
            if entity_id != "invoice" {
                return Err(format!(
                    "expected entity_id \"invoice\", got \"{}\"",
                    entity_id
                ));
            }
            if instance_id != "inv-123" {
                return Err(format!(
                    "expected instance_id \"inv-123\", got \"{}\"",
                    instance_id
                ));
            }
            Ok(())
        }
        other => Err(format!("expected EntityNotFound, got {:?}", other)),
    }
}

// ── 5. update_entity_state on nonexistent entity returns EntityNotFound ──────

async fn update_entity_state_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .update_entity_state(
            &mut snap, "account", "acct-1", 0, "active", "flow-1", "op-1",
        )
        .await;
    let _ = s.abort_snapshot(snap).await;
    match result {
        Err(StorageError::EntityNotFound { .. }) => Ok(()),
        other => Err(format!("expected EntityNotFound, got {:?}", other)),
    }
}

// ── 6. get_flow_execution for nonexistent ID returns ExecutionNotFound ───────

async fn get_flow_execution_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let result = s.get_flow_execution("exec-nonexistent").await;
    match result {
        Err(StorageError::ExecutionNotFound { .. }) => Ok(()),
        other => Err(format!("expected ExecutionNotFound, got {:?}", other)),
    }
}

// ── 7. ExecutionNotFound error carries correct execution_id ──────────────────

async fn get_flow_execution_not_found_has_correct_field<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let result = s.get_flow_execution("exec-abc-789").await;
    match result {
        Err(StorageError::ExecutionNotFound { execution_id }) => {
            if execution_id != "exec-abc-789" {
                return Err(format!(
                    "expected execution_id \"exec-abc-789\", got \"{}\"",
                    execution_id
                ));
            }
            Ok(())
        }
        other => Err(format!("expected ExecutionNotFound, got {:?}", other)),
    }
}

// ── 8. get_provenance returns empty vec for nonexistent operation execution ──

async fn get_provenance_empty_for_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let records = s
        .get_provenance("op-exec-nonexistent")
        .await
        .map_err(|e| e.to_string())?;
    if records.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "expected empty provenance vec, got {} records",
            records.len()
        ))
    }
}

// ── 9. list_entity_states returns empty vec for nonexistent entity ───────────

async fn list_entity_states_empty_for_nonexistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let records = s
        .list_entity_states("nonexistent-entity", None)
        .await
        .map_err(|e| e.to_string())?;
    if records.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "expected empty entity states vec, got {} records",
            records.len()
        ))
    }
}

// ── 10. list_flow_executions returns empty vec on empty store ────────────────

async fn list_flow_executions_empty<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let records = s
        .list_flow_executions(None, None, 100)
        .await
        .map_err(|e| e.to_string())?;
    if records.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "expected empty flow executions vec, got {} records",
            records.len()
        ))
    }
}
