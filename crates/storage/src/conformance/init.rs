use std::future::Future;

use super::TestResult;
use crate::{StorageError, TenorStorage};

pub(super) async fn run_init_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    results.push(TestResult::from_result(
        "init",
        "initialize_creates_entity_at_version_0",
        initialize_creates_entity_at_version_0(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_sets_correct_state",
        initialize_sets_correct_state(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_with_custom_state_name",
        initialize_with_custom_state_name(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialized_entity_readable_via_get_entity_state",
        initialized_entity_readable_via_get_entity_state(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialized_entity_readable_via_get_entity_state_for_update",
        initialized_entity_readable_via_get_entity_state_for_update(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "double_initialize_returns_already_initialized",
        double_initialize_returns_already_initialized(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "double_initialize_across_snapshots",
        double_initialize_across_snapshots(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "already_initialized_error_has_correct_fields",
        already_initialized_error_has_correct_fields(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "different_instances_are_independent",
        different_instances_are_independent(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "different_entities_are_independent",
        different_entities_are_independent(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_visible_after_commit",
        initialize_visible_after_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_not_visible_before_commit",
        initialize_not_visible_before_commit(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_not_visible_after_abort",
        initialize_not_visible_after_abort(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialized_entity_updatable",
        initialized_entity_updatable(factory).await,
    ));
    results.push(TestResult::from_result(
        "init",
        "initialize_sets_null_flow_and_operation_ids",
        initialize_sets_null_flow_and_operation_ids(factory).await,
    ));

    results
}

// ── Test implementations ──────────────────────────────────────────────────────

/// After initialize + commit, the entity version must be 0.
async fn initialize_creates_entity_at_version_0<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 0 {
        return Err(format!("expected version 0, got {}", rec.version));
    }
    Ok(())
}

/// After initialize with "initial", the state field must be "initial".
async fn initialize_sets_correct_state<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!("expected state \"initial\", got \"{}\"", rec.state));
    }
    Ok(())
}

/// Entities can be initialized with any state name (e.g. "draft").
async fn initialize_with_custom_state_name<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Invoice", "inv-1", "draft")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Invoice", "inv-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "draft" {
        return Err(format!("expected state \"draft\", got \"{}\"", rec.state));
    }
    Ok(())
}

/// After init + commit, get_entity_state must succeed.
async fn initialized_entity_readable_via_get_entity_state<S, F, Fut>(
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.entity_id != "Order" || rec.instance_id != "order-1" {
        return Err(format!(
            "expected Order/order-1, got {}/{}",
            rec.entity_id, rec.instance_id
        ));
    }
    Ok(())
}

/// After init + commit, get_entity_state_for_update in a new snapshot must succeed.
async fn initialized_entity_readable_via_get_entity_state_for_update<S, F, Fut>(
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let rec = s
        .get_entity_state_for_update(&mut snap2, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    if rec.entity_id != "Order" || rec.instance_id != "order-1" {
        return Err(format!(
            "expected Order/order-1, got {}/{}",
            rec.entity_id, rec.instance_id
        ));
    }
    Ok(())
}

/// Initializing the same entity twice in the same snapshot must return AlreadyInitialized.
async fn double_initialize_returns_already_initialized<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let result = s
        .initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    match result {
        Err(ref e) if matches!(e, StorageError::AlreadyInitialized { .. }) => Ok(()),
        Err(e) => Err(format!("expected AlreadyInitialized, got: {e}")),
        Ok(()) => Err("expected AlreadyInitialized error, but got Ok".to_string()),
    }
}

/// Initializing the same entity in a second snapshot after committing the first
/// must return AlreadyInitialized.
async fn double_initialize_across_snapshots<S, F, Fut>(factory: &F) -> Result<(), String>
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
    let result = s
        .initialize_entity(&mut snap2, "Order", "order-1", "initial")
        .await;
    s.abort_snapshot(snap2).await.map_err(|e| e.to_string())?;

    match result {
        Err(ref e) if matches!(e, StorageError::AlreadyInitialized { .. }) => Ok(()),
        Err(e) => Err(format!("expected AlreadyInitialized, got: {e}")),
        Ok(()) => Err("expected AlreadyInitialized error, but got Ok".to_string()),
    }
}

/// The AlreadyInitialized error must carry the correct entity_id and instance_id.
async fn already_initialized_error_has_correct_fields<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let result = s
        .initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    match result {
        Err(StorageError::AlreadyInitialized {
            entity_id,
            instance_id,
        }) => {
            if entity_id != "Order" {
                return Err(format!(
                    "expected entity_id \"Order\", got \"{}\"",
                    entity_id
                ));
            }
            if instance_id != "order-1" {
                return Err(format!(
                    "expected instance_id \"order-1\", got \"{}\"",
                    instance_id
                ));
            }
            Ok(())
        }
        Err(e) => Err(format!("expected AlreadyInitialized, got: {e}")),
        Ok(()) => Err("expected AlreadyInitialized error, but got Ok".to_string()),
    }
}

/// The same entity_id with different instance_ids must initialize independently.
async fn different_instances_are_independent<S, F, Fut>(factory: &F) -> Result<(), String>
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
    s.initialize_entity(&mut snap, "Order", "order-2", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec1 = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    let rec2 = s
        .get_entity_state("Order", "order-2")
        .await
        .map_err(|e| e.to_string())?;
    if rec1.instance_id != "order-1" || rec2.instance_id != "order-2" {
        return Err("instance ids do not match expected values".to_string());
    }
    Ok(())
}

/// Different entity_ids with the same instance_id must initialize independently.
async fn different_entities_are_independent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Invoice", "inst-1", "draft")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec1 = s
        .get_entity_state("Order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    let rec2 = s
        .get_entity_state("Invoice", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec1.entity_id != "Order" || rec2.entity_id != "Invoice" {
        return Err("entity ids do not match expected values".to_string());
    }
    if rec1.state != "initial" || rec2.state != "draft" {
        return Err("states do not match expected values".to_string());
    }
    Ok(())
}

/// After initialize + commit, get_entity_state must succeed (same as test 4 but
/// focuses on the commit being the visibility boundary).
async fn initialize_visible_after_commit<S, F, Fut>(factory: &F) -> Result<(), String>
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

    // Entity must be visible after commit.
    s.get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Before committing a snapshot, the initialized entity must NOT be visible
/// to read-path queries (get_entity_state operates outside the snapshot).
async fn initialize_not_visible_before_commit<S, F, Fut>(factory: &F) -> Result<(), String>
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

    // Snapshot is still open -- entity should not be visible outside it.
    let result = s.get_entity_state("Order", "order-1").await;
    // Clean up the snapshot regardless of the check outcome.
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    match result {
        Err(ref e) if matches!(e, StorageError::EntityNotFound { .. }) => Ok(()),
        Err(e) => Err(format!("expected EntityNotFound, got: {e}")),
        Ok(_) => Err("entity should not be visible before commit".to_string()),
    }
}

/// After initialize + abort, the entity must NOT exist.
async fn initialize_not_visible_after_abort<S, F, Fut>(factory: &F) -> Result<(), String>
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
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    let result = s.get_entity_state("Order", "order-1").await;
    match result {
        Err(ref e) if matches!(e, StorageError::EntityNotFound { .. }) => Ok(()),
        Err(e) => Err(format!("expected EntityNotFound, got: {e}")),
        Ok(_) => Err("entity should not be visible after abort".to_string()),
    }
}

/// An initialized entity can be updated via update_entity_state in a subsequent snapshot.
async fn initialized_entity_updatable<S, F, Fut>(factory: &F) -> Result<(), String>
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

    // Update in a new snapshot.
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
        return Err(format!("expected new version 1, got {new_version}"));
    }

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
    if rec.version != 1 {
        return Err(format!("expected version 1, got {}", rec.version));
    }
    Ok(())
}

/// After initialization, last_flow_id and last_operation_id must be None.
async fn initialize_sets_null_flow_and_operation_ids<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.last_flow_id.is_some() {
        return Err(format!(
            "expected last_flow_id to be None, got {:?}",
            rec.last_flow_id
        ));
    }
    if rec.last_operation_id.is_some() {
        return Err(format!(
            "expected last_operation_id to be None, got {:?}",
            rec.last_operation_id
        ));
    }
    Ok(())
}
