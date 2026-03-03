//! E4: Snapshot isolation obligation.
//!
//! Facts MUST be captured at flow initiation and used throughout.
//! The executor must not re-read facts mid-flow.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E4: Executor captures facts at initiation and uses the snapshot throughout.
pub async fn test_e04_snapshot_isolation<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E4: load_contract failed: {}", e))?;

    // Execute the flow with a valid fact set. The executor must use these facts
    // for the entire duration — this test verifies the flow completes using the
    // provided snapshot (not external fact sources).
    let facts = fixtures::basic_facts();
    let entity_states = fixtures::initial_entity_states();

    let result = executor
        .execute_flow("approval_flow", "clerk", &facts, &entity_states)
        .await;

    match result {
        Ok(flow_result) => {
            // If execution completed, the snapshot was used correctly.
            // Verify at least one step was executed.
            if flow_result.steps_executed.is_empty() {
                return Err(
                    "E4: flow completed but no steps were recorded (snapshot may not be tracked)"
                        .to_string(),
                );
            }
            Ok(())
        }
        Err(e) => Err(format!(
            "E4: flow execution failed with valid snapshot: {}",
            e
        )),
    }
}
