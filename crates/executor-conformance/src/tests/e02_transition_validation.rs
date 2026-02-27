//! E2: Transition validation obligation.
//!
//! The executor MUST reject operations when the entity is not in the
//! required source state. It must NOT silently succeed on invalid transitions.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E2: Executor rejects transition from wrong state.
pub async fn test_e02_transition_validation<E: TestableExecutor>(
    executor: &E,
) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E2: load_contract failed: {}", e))?;

    let facts = fixtures::basic_facts();
    // Order is already in "submitted" state — cannot submit again (submit requires "draft").
    let entity_states = fixtures::order_submitted_states();

    let result = executor
        .execute_flow("approval_flow", "clerk", &facts, &entity_states)
        .await;

    // The executor must either:
    //   (a) return an error (transition validation failed), or
    //   (b) return a flow result with a failure/error outcome.
    match result {
        Err(_) => {
            // Error return is acceptable — transition rejected.
            Ok(())
        }
        Ok(flow_result) => {
            // If the flow "succeeds" despite invalid state, it must report
            // a non-success outcome (failure, error, etc.).
            let outcome = flow_result.outcome.to_lowercase();
            if outcome.contains("fail")
                || outcome.contains("error")
                || outcome.contains("invalid")
                || outcome.contains("reject")
            {
                Ok(())
            } else {
                Err(format!(
                    "E2: executor silently accepted invalid transition (outcome: '{}')",
                    flow_result.outcome
                ))
            }
        }
    }
}
