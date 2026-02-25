use std::future::Future;

use super::TestResult;
use crate::{StorageError, TenorStorage};

pub(super) async fn run_version_tests<S, F, Fut>(factory: &F) -> Vec<TestResult>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let mut results = Vec::new();

    // Basic version tracking
    results.push(TestResult::from_result(
        "version",
        "version_starts_at_zero",
        version_starts_at_zero(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "version_increments_to_one",
        version_increments_to_one(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "version_increments_sequentially",
        version_increments_sequentially(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "update_returns_correct_new_version",
        update_returns_correct_new_version(factory).await,
    ));

    // Correct version succeeds
    results.push(TestResult::from_result(
        "version",
        "update_with_correct_version_succeeds",
        update_with_correct_version_succeeds(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "update_from_version_1",
        update_from_version_1(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "multiple_updates_correct_versions",
        multiple_updates_correct_versions(factory).await,
    ));

    // Wrong version fails
    results.push(TestResult::from_result(
        "version",
        "update_with_wrong_version_returns_conflict",
        update_with_wrong_version_returns_conflict(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "update_with_version_minus_one",
        update_with_version_minus_one(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "update_with_version_plus_one",
        update_with_version_plus_one(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "stale_version_after_intervening_commit",
        stale_version_after_intervening_commit(factory).await,
    ));

    // Conflict error fields
    results.push(TestResult::from_result(
        "version",
        "conflict_has_correct_entity_id",
        conflict_has_correct_entity_id(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "conflict_has_correct_instance_id",
        conflict_has_correct_instance_id(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "conflict_has_correct_expected_version",
        conflict_has_correct_expected_version(factory).await,
    ));

    // Conflict does not mutate state
    results.push(TestResult::from_result(
        "version",
        "conflict_does_not_change_state",
        conflict_does_not_change_state(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "conflict_does_not_increment_version",
        conflict_does_not_increment_version(factory).await,
    ));

    // Race conditions (sequential simulation)
    results.push(TestResult::from_result(
        "version",
        "two_snapshots_race_one_wins",
        two_snapshots_race_one_wins(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "after_race_state_reflects_winner",
        after_race_state_reflects_winner(factory).await,
    ));

    // Per-instance independence
    results.push(TestResult::from_result(
        "version",
        "version_per_instance_independent",
        version_per_instance_independent(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "version_per_entity_independent",
        version_per_entity_independent(factory).await,
    ));

    // get_entity_state_for_update
    results.push(TestResult::from_result(
        "version",
        "for_update_returns_current_version",
        for_update_returns_current_version(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "for_update_version_matches_get_entity_state",
        for_update_version_matches_get_entity_state(factory).await,
    ));

    // Sequential within same snapshot
    results.push(TestResult::from_result(
        "version",
        "second_update_same_snapshot_uses_new_version",
        second_update_same_snapshot_uses_new_version(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "rapid_sequential_updates",
        rapid_sequential_updates(factory).await,
    ));
    results.push(TestResult::from_result(
        "version",
        "version_survives_abort",
        version_survives_abort(factory).await,
    ));

    results
}

// ── Basic version tracking ───────────────────────────────────────────────────

/// After initialize + commit, the entity version must be 0.
async fn version_starts_at_zero<S, F, Fut>(factory: &F) -> Result<(), String>
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

/// After initialize + commit + update(v0) + commit, version must be 1.
async fn version_increments_to_one<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update from version 0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 1 {
        return Err(format!("expected version 1, got {}", rec.version));
    }
    Ok(())
}

/// Three successive updates must yield versions 0 -> 1 -> 2 -> 3.
async fn version_increments_sequentially<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let states = ["submitted", "approved", "completed"];

    // Initialize at version 0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Three updates in separate snapshots.
    for (i, state) in states.iter().enumerate() {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        s.update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            i as i64,
            state,
            &format!("flow-{}", i + 1),
            &format!("op-{}", i + 1),
        )
        .await
        .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

        let rec = s
            .get_entity_state("Order", "order-1")
            .await
            .map_err(|e| e.to_string())?;
        let expected_version = (i + 1) as i64;
        if rec.version != expected_version {
            return Err(format!(
                "after update {}, expected version {}, got {}",
                i + 1,
                expected_version,
                rec.version
            ));
        }
    }
    Ok(())
}

/// The return value of update_entity_state must match the new version.
async fn update_returns_correct_new_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update and check return value.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let returned = s
        .update_entity_state(
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    if returned != 1 {
        return Err(format!(
            "update_entity_state returned {}, expected 1",
            returned
        ));
    }

    // Confirm stored version matches.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != returned {
        return Err(format!(
            "stored version {} does not match returned version {}",
            rec.version, returned
        ));
    }
    Ok(())
}

// ── Correct version succeeds ─────────────────────────────────────────────────

/// Update with expected_version=0 after init must succeed.
async fn update_with_correct_version_succeeds<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

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

/// Update from version 1 (after init + one update) must succeed.
async fn update_from_version_1<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update to version 1.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update from version 1 to version 2.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let new_version = s
        .update_entity_state(
            &mut snap, "Order", "order-1", 1, "approved", "flow-2", "op-2",
        )
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    if new_version != 2 {
        return Err(format!("expected new version 2, got {}", new_version));
    }

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
    Ok(())
}

/// A chain of 5 updates, each using the correct version, must all succeed.
async fn multiple_updates_correct_versions<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;
    let states = ["submitted", "reviewed", "approved", "shipped", "delivered"];

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    for (i, state) in states.iter().enumerate() {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        let new_version = s
            .update_entity_state(
                &mut snap,
                "Order",
                "order-1",
                i as i64,
                state,
                &format!("flow-{}", i + 1),
                &format!("op-{}", i + 1),
            )
            .await
            .map_err(|e| format!("update {} failed: {}", i + 1, e))?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

        let expected = (i + 1) as i64;
        if new_version != expected {
            return Err(format!(
                "update {} returned version {}, expected {}",
                i + 1,
                new_version,
                expected
            ));
        }
    }

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 5 {
        return Err(format!("expected final version 5, got {}", rec.version));
    }
    if rec.state != "delivered" {
        return Err(format!(
            "expected final state \"delivered\", got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}

// ── Wrong version fails ──────────────────────────────────────────────────────

/// Update with a wildly wrong version (999) must return ConcurrentConflict.
async fn update_with_wrong_version_returns_conflict<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            999,
            "submitted",
            "flow-1",
            "op-1",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { .. }) => {
            let _ = s.abort_snapshot(snap).await;
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// Update with expected_version=-1 must return ConcurrentConflict.
async fn update_with_version_minus_one<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            -1,
            "submitted",
            "flow-1",
            "op-1",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { .. }) => {
            let _ = s.abort_snapshot(snap).await;
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// Update with expected_version=1 when actual is 0 must return ConcurrentConflict.
async fn update_with_version_plus_one<S, F, Fut>(factory: &F) -> Result<(), String>
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

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            1,
            "submitted",
            "flow-1",
            "op-1",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { .. }) => {
            let _ = s.abort_snapshot(snap).await;
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// After a successful update to v1, a stale update with v0 must conflict.
async fn stale_version_after_intervening_commit<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize at v0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update from v0 to v1.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Stale update with v0 should conflict.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(
            &mut snap, "Order", "order-1", 0, "rejected", "flow-2", "op-2",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { .. }) => {
            let _ = s.abort_snapshot(snap).await;
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

// ── Conflict error fields ────────────────────────────────────────────────────

/// ConcurrentConflict error must carry the correct entity_id.
async fn conflict_has_correct_entity_id<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Loan", "loan-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(&mut snap, "Loan", "loan-1", 999, "active", "flow-1", "op-1")
        .await
    {
        Err(StorageError::ConcurrentConflict { entity_id, .. }) => {
            let _ = s.abort_snapshot(snap).await;
            if entity_id != "Loan" {
                return Err(format!(
                    "expected entity_id \"Loan\", got \"{}\"",
                    entity_id
                ));
            }
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// ConcurrentConflict error must carry the correct instance_id.
async fn conflict_has_correct_instance_id<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Loan", "loan-42", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(
            &mut snap, "Loan", "loan-42", 999, "active", "flow-1", "op-1",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { instance_id, .. }) => {
            let _ = s.abort_snapshot(snap).await;
            if instance_id != "loan-42" {
                return Err(format!(
                    "expected instance_id \"loan-42\", got \"{}\"",
                    instance_id
                ));
            }
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// ConcurrentConflict error must carry the expected_version that was passed.
async fn conflict_has_correct_expected_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Loan", "loan-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    match s
        .update_entity_state(&mut snap, "Loan", "loan-1", 42, "active", "flow-1", "op-1")
        .await
    {
        Err(StorageError::ConcurrentConflict {
            expected_version, ..
        }) => {
            let _ = s.abort_snapshot(snap).await;
            if expected_version != 42 {
                return Err(format!(
                    "expected expected_version 42, got {}",
                    expected_version
                ));
            }
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

// ── Conflict does not mutate state ───────────────────────────────────────────

/// After a conflicting update, get_entity_state must show the original state.
async fn conflict_does_not_change_state<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize with "initial".
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Attempt a conflicting update (wrong version).
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            999,
            "should_not_appear",
            "flow-1",
            "op-1",
        )
        .await;
    let _ = s.abort_snapshot(snap).await;

    if !matches!(result, Err(StorageError::ConcurrentConflict { .. })) {
        return Err(format!("expected ConcurrentConflict, got {:?}", result));
    }

    // Verify original state is unchanged.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "initial" {
        return Err(format!(
            "expected state \"initial\" after conflict, got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}

/// After a conflicting update, the version must remain unchanged.
async fn conflict_does_not_increment_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize at v0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Attempt a conflicting update.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            999,
            "should_not_appear",
            "flow-1",
            "op-1",
        )
        .await;
    let _ = s.abort_snapshot(snap).await;

    if !matches!(result, Err(StorageError::ConcurrentConflict { .. })) {
        return Err(format!("expected ConcurrentConflict, got {:?}", result));
    }

    // Verify version is still 0.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 0 {
        return Err(format!(
            "expected version 0 after conflict, got {}",
            rec.version
        ));
    }
    Ok(())
}

// ── Race conditions (sequential simulation) ──────────────────────────────────

/// Two snapshots both read v0, snap1 commits (v1), snap2 update(v0) must conflict.
async fn two_snapshots_race_one_wins<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize at v0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Snap1 reads v0.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let rec1 = s
        .get_entity_state_for_update(&mut snap1, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec1.version != 0 {
        let _ = s.abort_snapshot(snap1).await;
        return Err(format!("snap1 expected version 0, got {}", rec1.version));
    }

    // Snap2 reads v0.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let rec2 = s
        .get_entity_state_for_update(&mut snap2, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec2.version != 0 {
        let _ = s.abort_snapshot(snap1).await;
        let _ = s.abort_snapshot(snap2).await;
        return Err(format!("snap2 expected version 0, got {}", rec2.version));
    }

    // Snap1 updates and commits (v0 -> v1).
    s.update_entity_state(
        &mut snap1,
        "Order",
        "order-1",
        0,
        "winner_state",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| {
        // Cannot abort snap2 in map_err easily, but the test will fail anyway.
        format!("snap1 update failed: {}", e)
    })?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Snap2 tries to update with stale v0 -- must conflict.
    match s
        .update_entity_state(
            &mut snap2,
            "Order",
            "order-1",
            0,
            "loser_state",
            "flow-2",
            "op-2",
        )
        .await
    {
        Err(StorageError::ConcurrentConflict { .. }) => {
            let _ = s.abort_snapshot(snap2).await;
            Ok(())
        }
        other => {
            let _ = s.abort_snapshot(snap2).await;
            Err(format!("expected ConcurrentConflict, got {:?}", other))
        }
    }
}

/// After a race, get_entity_state must reflect the winner's state.
async fn after_race_state_reflects_winner<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize at v0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Snap1 reads v0.
    let mut snap1 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.get_entity_state_for_update(&mut snap1, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    // Snap2 reads v0.
    let mut snap2 = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.get_entity_state_for_update(&mut snap2, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    // Snap1 wins.
    s.update_entity_state(
        &mut snap1,
        "Order",
        "order-1",
        0,
        "winner_state",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap1).await.map_err(|e| e.to_string())?;

    // Snap2 loses.
    let _ = s
        .update_entity_state(
            &mut snap2,
            "Order",
            "order-1",
            0,
            "loser_state",
            "flow-2",
            "op-2",
        )
        .await;
    let _ = s.abort_snapshot(snap2).await;

    // Verify the winner's state is persisted.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.state != "winner_state" {
        return Err(format!(
            "expected state \"winner_state\", got \"{}\"",
            rec.state
        ));
    }
    if rec.version != 1 {
        return Err(format!("expected version 1, got {}", rec.version));
    }
    Ok(())
}

// ── Per-instance independence ─────────────────────────────────────────────────

/// Updating one instance must not affect another instance's version.
async fn version_per_instance_independent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize two instances of the same entity.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-2", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update only order-1.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // order-1 should be at version 1.
    let rec1 = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec1.version != 1 {
        return Err(format!("expected order-1 version 1, got {}", rec1.version));
    }

    // order-2 should still be at version 0.
    let rec2 = s
        .get_entity_state("Order", "order-2")
        .await
        .map_err(|e| e.to_string())?;
    if rec2.version != 0 {
        return Err(format!("expected order-2 version 0, got {}", rec2.version));
    }
    Ok(())
}

/// Updating one entity must not affect another entity's version.
async fn version_per_entity_independent<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize two different entities.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "inst-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Invoice", "inst-1", "draft")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update only Order.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.update_entity_state(
        &mut snap,
        "Order",
        "inst-1",
        0,
        "submitted",
        "flow-1",
        "op-1",
    )
    .await
    .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Order should be at version 1.
    let order_rec = s
        .get_entity_state("Order", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if order_rec.version != 1 {
        return Err(format!(
            "expected Order version 1, got {}",
            order_rec.version
        ));
    }

    // Invoice should still be at version 0.
    let invoice_rec = s
        .get_entity_state("Invoice", "inst-1")
        .await
        .map_err(|e| e.to_string())?;
    if invoice_rec.version != 0 {
        return Err(format!(
            "expected Invoice version 0, got {}",
            invoice_rec.version
        ));
    }
    Ok(())
}

// ── get_entity_state_for_update ──────────────────────────────────────────────

/// After 3 updates, get_entity_state_for_update must return version 3.
async fn for_update_returns_current_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // 3 updates: v0->v1, v1->v2, v2->v3.
    for i in 0..3 {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        s.update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            i,
            &format!("state-{}", i + 1),
            &format!("flow-{}", i + 1),
            &format!("op-{}", i + 1),
        )
        .await
        .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    }

    // Read with for_update.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let rec = s
        .get_entity_state_for_update(&mut snap, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    if rec.version != 3 {
        return Err(format!("expected version 3, got {}", rec.version));
    }
    Ok(())
}

/// get_entity_state_for_update and get_entity_state must return the same version.
async fn for_update_version_matches_get_entity_state<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize and do 2 updates.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    for i in 0..2 {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        s.update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            i,
            &format!("state-{}", i + 1),
            &format!("flow-{}", i + 1),
            &format!("op-{}", i + 1),
        )
        .await
        .map_err(|e| e.to_string())?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;
    }

    // Read via get_entity_state.
    let rec_read = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;

    // Read via get_entity_state_for_update.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let rec_for_update = s
        .get_entity_state_for_update(&mut snap, "Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    if rec_read.version != rec_for_update.version {
        return Err(format!(
            "get_entity_state returned version {}, for_update returned {}",
            rec_read.version, rec_for_update.version
        ));
    }
    Ok(())
}

// ── Sequential within same snapshot ──────────────────────────────────────────

/// In one snapshot: init, update(v0)->v1, update(v1)->v2, commit. get shows v2.
async fn second_update_same_snapshot_uses_new_version<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;

    // Initialize.
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;

    // First update: v0 -> v1.
    let v1 = s
        .update_entity_state(
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
    if v1 != 1 {
        let _ = s.abort_snapshot(snap).await;
        return Err(format!("first update returned {}, expected 1", v1));
    }

    // Second update: v1 -> v2.
    let v2 = s
        .update_entity_state(
            &mut snap, "Order", "order-1", 1, "approved", "flow-2", "op-2",
        )
        .await
        .map_err(|e| e.to_string())?;
    if v2 != 2 {
        let _ = s.abort_snapshot(snap).await;
        return Err(format!("second update returned {}, expected 2", v2));
    }

    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 2 {
        return Err(format!("expected version 2, got {}", rec.version));
    }
    if rec.state != "approved" {
        return Err(format!(
            "expected state \"approved\", got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}

/// 10 updates in separate snapshots, final version must be 10.
async fn rapid_sequential_updates<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Counter", "counter-1", "s0")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // 10 updates, each in its own snapshot.
    for i in 0..10 {
        let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
        let new_version = s
            .update_entity_state(
                &mut snap,
                "Counter",
                "counter-1",
                i,
                &format!("s{}", i + 1),
                &format!("flow-{}", i + 1),
                &format!("op-{}", i + 1),
            )
            .await
            .map_err(|e| format!("update {} failed: {}", i + 1, e))?;
        s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

        let expected = i + 1;
        if new_version != expected {
            return Err(format!(
                "update {} returned version {}, expected {}",
                i + 1,
                new_version,
                expected
            ));
        }
    }

    let rec = s
        .get_entity_state("Counter", "counter-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 10 {
        return Err(format!("expected final version 10, got {}", rec.version));
    }
    Ok(())
}

/// Committed state survives a subsequent aborted snapshot.
/// init+commit(v0), update(v0)+commit(v1), start new snap+update(wrong v0)+abort,
/// get shows v1 still.
async fn version_survives_abort<S, F, Fut>(factory: &F) -> Result<(), String>
where
    S: TenorStorage,
    F: Fn() -> Fut,
    Fut: Future<Output = S>,
{
    let s = factory().await;

    // Initialize at v0.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    s.initialize_entity(&mut snap, "Order", "order-1", "initial")
        .await
        .map_err(|e| e.to_string())?;
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Update v0 -> v1.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
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
    s.commit_snapshot(snap).await.map_err(|e| e.to_string())?;

    // Start a new snapshot, attempt stale update (v0), expect conflict, then abort.
    let mut snap = s.begin_snapshot().await.map_err(|e| e.to_string())?;
    let result = s
        .update_entity_state(
            &mut snap,
            "Order",
            "order-1",
            0,
            "should_not_appear",
            "flow-2",
            "op-2",
        )
        .await;
    s.abort_snapshot(snap).await.map_err(|e| e.to_string())?;

    if !matches!(result, Err(StorageError::ConcurrentConflict { .. })) {
        return Err(format!("expected ConcurrentConflict, got {:?}", result));
    }

    // Verify the committed state is unchanged.
    let rec = s
        .get_entity_state("Order", "order-1")
        .await
        .map_err(|e| e.to_string())?;
    if rec.version != 1 {
        return Err(format!(
            "expected version 1 after abort, got {}",
            rec.version
        ));
    }
    if rec.state != "submitted" {
        return Err(format!(
            "expected state \"submitted\" after abort, got \"{}\"",
            rec.state
        ));
    }
    Ok(())
}
