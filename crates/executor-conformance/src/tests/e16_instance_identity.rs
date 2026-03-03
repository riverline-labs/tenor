//! E16: Instance identity obligation.
//!
//! Each instance MUST be addressable by its ID. The executor must
//! maintain separate state per instance.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E16: Instance identity is preserved — each instance has independent state.
pub async fn test_e16_instance_identity<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::multi_instance_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E16: load_contract failed: {}", e))?;

    // Create two instances.
    executor
        .create_instance("Order", "order-alpha")
        .await
        .map_err(|e| format!("E16: create_instance (alpha) failed: {}", e))?;

    executor
        .create_instance("Order", "order-beta")
        .await
        .map_err(|e| format!("E16: create_instance (beta) failed: {}", e))?;

    // Verify each instance has its own state.
    let state_alpha = executor
        .get_entity_state("Order", "order-alpha")
        .await
        .map_err(|e| format!("E16: get_entity_state (alpha) failed: {}", e))?;

    let state_beta = executor
        .get_entity_state("Order", "order-beta")
        .await
        .map_err(|e| format!("E16: get_entity_state (beta) failed: {}", e))?;

    // Both instances must exist.
    if state_alpha.is_none() {
        return Err("E16: instance 'order-alpha' not found after creation".to_string());
    }
    if state_beta.is_none() {
        return Err("E16: instance 'order-beta' not found after creation".to_string());
    }

    // Both instances should be in their initial state independently.
    let alpha = state_alpha.unwrap();
    let beta = state_beta.unwrap();

    if alpha != "draft" {
        return Err(format!(
            "E16: instance 'order-alpha' in state '{}', expected 'draft'",
            alpha
        ));
    }
    if beta != "draft" {
        return Err(format!(
            "E16: instance 'order-beta' in state '{}', expected 'draft'",
            beta
        ));
    }

    Ok(())
}
