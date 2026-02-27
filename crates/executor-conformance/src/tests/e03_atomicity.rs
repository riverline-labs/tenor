//! E3: Atomicity obligation.
//!
//! When an operation affects multiple entities and one effect cannot be applied,
//! the executor MUST roll back all effects — none should be applied.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E3: Executor rolls back all effects when any single effect fails.
pub async fn test_e03_atomicity<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::multi_entity_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E3: load_contract failed: {}", e))?;

    let facts = fixtures::basic_facts();
    // Order is in draft (valid), Payment is in processed (invalid — needs pending).
    // The operation submit_with_payment touches both entities.
    // Payment cannot transition from processed → processed.
    let entity_states = serde_json::json!({
        "Order": "draft",
        "Payment": "processed"   // wrong state — Payment effect must fail
    });

    let result = executor
        .execute_flow("submit_flow", "clerk", &facts, &entity_states)
        .await;

    // Execution must fail (atomicity: at least one effect is invalid).
    match result {
        Err(_) => {
            // Error return — atomic rollback occurred.
            Ok(())
        }
        Ok(flow_result) => {
            let outcome = flow_result.outcome.to_lowercase();
            if outcome.contains("fail") || outcome.contains("error") || outcome.contains("invalid")
            {
                // Check that Order was NOT transitioned (rolled back).
                let order_state = executor
                    .get_entity_state("Order", "_default")
                    .await
                    .map_err(|e| format!("E3: get_entity_state failed: {}", e))?;

                if let Some(state) = order_state {
                    if state != "draft" {
                        return Err(format!(
                            "E3: atomicity violation — Order transitioned to '{}' despite rollback",
                            state
                        ));
                    }
                }
                Ok(())
            } else {
                Err(format!(
                    "E3: executor accepted invalid atomic operation (outcome: '{}')",
                    flow_result.outcome
                ))
            }
        }
    }
}
