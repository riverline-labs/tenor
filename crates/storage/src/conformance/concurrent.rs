use std::future::Future;
use std::sync::Arc;

use super::TestResult;
use crate::{StorageError, TenorStorage};

/// Number of concurrent tasks to spawn in each test.
const N: usize = 10;

pub(super) async fn run_concurrent_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    results.push(TestResult::from_result(
        "concurrent",
        "concurrent_updates_exactly_one_wins",
        concurrent_updates_exactly_one_wins(factory).await,
    ));
    results.push(TestResult::from_result(
        "concurrent",
        "concurrent_initialize_exactly_one_wins",
        concurrent_initialize_exactly_one_wins(factory).await,
    ));
    results.push(TestResult::from_result(
        "concurrent",
        "concurrent_updates_different_entities_all_succeed",
        concurrent_updates_different_entities_all_succeed(factory).await,
    ));
    results.push(TestResult::from_result(
        "concurrent",
        "concurrent_updates_final_state_consistent",
        concurrent_updates_final_state_consistent(factory).await,
    ));

    results
}

// ── Concurrent update: exactly one wins ─────────────────────────────────────

/// N tasks each open a snapshot and attempt to update the same entity from
/// version 0. Exactly one commit succeeds; the rest must get ConcurrentConflict.
///
/// This exercises real concurrency — `tokio::spawn` creates parallel tasks
/// that race against the OCC version check, unlike the sequential simulation
/// in the `version` module.
async fn concurrent_updates_exactly_one_wins<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let storage = Arc::new(factory().await);

    // Initialize entity at version 0.
    {
        let mut snap = storage
            .begin_snapshot()
            .await
            .map_err(|e| format!("begin: {e}"))?;
        storage
            .initialize_entity(&mut snap, "Order", "order-1", "pending")
            .await
            .map_err(|e| format!("init: {e}"))?;
        storage
            .commit_snapshot(snap)
            .await
            .map_err(|e| format!("commit init: {e}"))?;
    }

    // Spawn N tasks that all try to update Order/order-1 at version 0.
    let mut handles = Vec::new();
    for i in 0..N {
        let s = storage.clone();
        handles.push(tokio::spawn(async move {
            let mut snap = s.begin_snapshot().await?;
            let result = s
                .update_entity_state(
                    &mut snap,
                    "Order",
                    "order-1",
                    0,
                    "confirmed",
                    &format!("flow-{i}"),
                    &format!("op-{i}"),
                )
                .await;
            match result {
                Ok(_new_version) => {
                    s.commit_snapshot(snap).await?;
                    Ok(true) // won the race
                }
                Err(StorageError::ConcurrentConflict { .. }) => {
                    s.abort_snapshot(snap).await?;
                    Ok(false) // lost the race
                }
                Err(e) => {
                    let _ = s.abort_snapshot(snap).await;
                    Err(e)
                }
            }
        }));
    }

    let mut winners = 0usize;
    let mut losers = 0usize;
    for handle in handles {
        let won = handle
            .await
            .map_err(|e| format!("task panic: {e}"))?
            .map_err(|e: StorageError| format!("storage error: {e}"))?;
        if won {
            winners += 1;
        } else {
            losers += 1;
        }
    }

    if winners != 1 {
        return Err(format!("expected exactly 1 winner, got {winners}"));
    }
    if losers != N - 1 {
        return Err(format!("expected {} losers, got {losers}", N - 1));
    }

    Ok(())
}

// ── Concurrent initialization: exactly one wins ─────────────────────────────

/// N tasks each attempt to initialize the same entity. Exactly one succeeds;
/// the rest must get AlreadyInitialized.
async fn concurrent_initialize_exactly_one_wins<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let storage = Arc::new(factory().await);

    let mut handles = Vec::new();
    for _i in 0..N {
        let s = storage.clone();
        handles.push(tokio::spawn(async move {
            let mut snap = s.begin_snapshot().await?;
            let result = s
                .initialize_entity(&mut snap, "Order", "order-1", "pending")
                .await;
            match result {
                Ok(()) => {
                    s.commit_snapshot(snap).await?;
                    Ok(true) // won
                }
                Err(StorageError::AlreadyInitialized { .. }) => {
                    s.abort_snapshot(snap).await?;
                    Ok(false) // lost
                }
                Err(e) => {
                    let _ = s.abort_snapshot(snap).await;
                    Err(e)
                }
            }
        }));
    }

    let mut winners = 0usize;
    let mut losers = 0usize;
    for handle in handles {
        let won = handle
            .await
            .map_err(|e| format!("task panic: {e}"))?
            .map_err(|e: StorageError| format!("storage error: {e}"))?;
        if won {
            winners += 1;
        } else {
            losers += 1;
        }
    }

    if winners != 1 {
        return Err(format!("expected exactly 1 winner, got {winners}"));
    }
    if losers != N - 1 {
        return Err(format!("expected {} losers, got {losers}", N - 1));
    }

    Ok(())
}

// ── Concurrent updates to different entities: all succeed ───────────────────

/// N tasks each update a different entity. All should succeed — no false
/// conflicts when there is no contention.
async fn concurrent_updates_different_entities_all_succeed<S, F, Fut>(
    factory: &F,
) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let storage = Arc::new(factory().await);

    // Initialize N different entity instances.
    {
        let mut snap = storage
            .begin_snapshot()
            .await
            .map_err(|e| format!("begin: {e}"))?;
        for i in 0..N {
            storage
                .initialize_entity(&mut snap, "Order", &format!("order-{i}"), "pending")
                .await
                .map_err(|e| format!("init order-{i}: {e}"))?;
        }
        storage
            .commit_snapshot(snap)
            .await
            .map_err(|e| format!("commit init: {e}"))?;
    }

    // Spawn N tasks, each updating a different instance.
    let mut handles = Vec::new();
    for i in 0..N {
        let s = storage.clone();
        handles.push(tokio::spawn(async move {
            let mut snap = s.begin_snapshot().await?;
            s.update_entity_state(
                &mut snap,
                "Order",
                &format!("order-{i}"),
                0,
                "confirmed",
                &format!("flow-{i}"),
                &format!("op-{i}"),
            )
            .await?;
            s.commit_snapshot(snap).await?;
            Ok::<(), StorageError>(())
        }));
    }

    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .await
            .map_err(|e| format!("task {i} panic: {e}"))?
            .map_err(|e| format!("task {i} failed: {e}"))?;
    }

    // Verify all entities were updated.
    for i in 0..N {
        let record = storage
            .get_entity_state("Order", &format!("order-{i}"))
            .await
            .map_err(|e| format!("get order-{i}: {e}"))?;
        if record.state != "confirmed" {
            return Err(format!(
                "order-{i}: expected state 'confirmed', got '{}'",
                record.state
            ));
        }
        if record.version != 1 {
            return Err(format!(
                "order-{i}: expected version 1, got {}",
                record.version
            ));
        }
    }

    Ok(())
}

// ── Concurrent updates: final state consistent ──────────────────────────────

/// After a concurrent update race on the same entity, the final state must
/// be consistent: exactly version 1, in the target state, and readable by
/// a non-locking read.
async fn concurrent_updates_final_state_consistent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let storage = Arc::new(factory().await);

    // Initialize entity at version 0.
    {
        let mut snap = storage
            .begin_snapshot()
            .await
            .map_err(|e| format!("begin: {e}"))?;
        storage
            .initialize_entity(&mut snap, "Account", "acct-1", "draft")
            .await
            .map_err(|e| format!("init: {e}"))?;
        storage
            .commit_snapshot(snap)
            .await
            .map_err(|e| format!("commit init: {e}"))?;
    }

    // Spawn N tasks that all try to update Account/acct-1 from version 0.
    let mut handles = Vec::new();
    for i in 0..N {
        let s = storage.clone();
        handles.push(tokio::spawn(async move {
            let mut snap = s.begin_snapshot().await?;
            let result = s
                .update_entity_state(
                    &mut snap,
                    "Account",
                    "acct-1",
                    0,
                    "active",
                    &format!("flow-{i}"),
                    &format!("op-{i}"),
                )
                .await;
            match result {
                Ok(_) => {
                    s.commit_snapshot(snap).await?;
                    Ok(())
                }
                Err(StorageError::ConcurrentConflict { .. }) => {
                    s.abort_snapshot(snap).await?;
                    Ok(())
                }
                Err(e) => {
                    let _ = s.abort_snapshot(snap).await;
                    Err(e)
                }
            }
        }));
    }

    // Wait for all tasks.
    for handle in handles {
        handle
            .await
            .map_err(|e| format!("task panic: {e}"))?
            .map_err(|e: StorageError| format!("storage error: {e}"))?;
    }

    // Verify final state is consistent.
    let record = storage
        .get_entity_state("Account", "acct-1")
        .await
        .map_err(|e| format!("get: {e}"))?;

    if record.version != 1 {
        return Err(format!(
            "expected version 1 after single winning update, got {}",
            record.version
        ));
    }
    if record.state != "active" {
        return Err(format!("expected state 'active', got '{}'", record.state));
    }

    Ok(())
}
