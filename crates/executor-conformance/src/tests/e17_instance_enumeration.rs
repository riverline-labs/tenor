//! E17: Instance enumeration obligation.
//!
//! The executor MUST return all created instances when listing instances
//! for an entity.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E17: All created instances are returned by list_instances.
pub async fn test_e17_instance_enumeration<E: TestableExecutor>(
    executor: &E,
) -> Result<(), String> {
    let contract = fixtures::multi_instance_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E17: load_contract failed: {}", e))?;

    // Create three distinct instances.
    let instance_ids = ["order-x1", "order-x2", "order-x3"];
    for id in &instance_ids {
        executor
            .create_instance("Order", id)
            .await
            .map_err(|e| format!("E17: create_instance ({}) failed: {}", id, e))?;
    }

    // List all instances.
    let listed = executor
        .list_instances("Order")
        .await
        .map_err(|e| format!("E17: list_instances failed: {}", e))?;

    // All three created instances must be present.
    for id in &instance_ids {
        if !listed.iter().any(|s| s == id) {
            return Err(format!(
                "E17: instance '{}' not found in list_instances result (found: {:?})",
                id, listed
            ));
        }
    }

    Ok(())
}
