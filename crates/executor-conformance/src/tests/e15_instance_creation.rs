//! E15: Instance creation obligation.
//!
//! The executor MUST create entity instances in their initial state.
//! It must not create instances in non-initial states.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E15: Instance creation starts in initial state.
pub async fn test_e15_instance_creation<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::multi_instance_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E15: load_contract failed: {}", e))?;

    // Create a new instance.
    executor
        .create_instance("Order", "order-001")
        .await
        .map_err(|e| format!("E15: create_instance failed: {}", e))?;

    // Verify the instance was created in the initial state ("draft").
    let state = executor
        .get_entity_state("Order", "order-001")
        .await
        .map_err(|e| format!("E15: get_entity_state failed: {}", e))?;

    match state {
        None => {
            Err("E15: instance 'order-001' was created but not found in entity state".to_string())
        }
        Some(s) => {
            if s != "draft" {
                Err(format!(
                    "E15: instance 'order-001' created in state '{}', expected 'draft' (initial state)",
                    s
                ))
            } else {
                Ok(())
            }
        }
    }
}
