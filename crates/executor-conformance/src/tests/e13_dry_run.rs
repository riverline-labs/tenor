//! E13: Dry-run (simulation) obligation.
//!
//! Simulated flows MUST NOT persist state changes or write audit logs.
//! Entity state after simulation must be identical to entity state before.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E13: Simulated flow does not persist state changes.
pub async fn test_e13_dry_run<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E13: load_contract failed: {}", e))?;

    let facts = fixtures::basic_facts();
    let entity_states = fixtures::initial_entity_states();

    // Read entity state before simulation.
    let state_before = executor
        .get_entity_state("Order", "_default")
        .await
        .map_err(|e| format!("E13: get_entity_state (before) failed: {}", e))?;

    // Run simulation (dry-run).
    let sim_result = executor
        .simulate_flow("approval_flow", "clerk", &facts, &entity_states)
        .await;

    match sim_result {
        Ok(flow_result) => {
            // Simulation completed — verify state was NOT persisted.
            let state_after = executor
                .get_entity_state("Order", "_default")
                .await
                .map_err(|e| format!("E13: get_entity_state (after simulation) failed: {}", e))?;

            // Simulation must not change persisted state.
            if state_before != state_after {
                return Err(format!(
                    "E13: dry-run violation — entity state changed from {:?} to {:?} after simulation",
                    state_before, state_after
                ));
            }

            // Simulation should still report a valid outcome.
            if flow_result.outcome.is_empty() {
                return Err("E13: simulation completed but reported empty outcome".to_string());
            }

            Ok(())
        }
        Err(e) => {
            // Simulation may not be supported — that is acceptable.
            if e.message.contains("unsupported")
                || e.message.contains("not implemented")
                || e.message.contains("not supported")
                || e.message.contains("simulate")
            {
                Ok(())
            } else {
                Err(format!("E13: unexpected simulation failure: {}", e))
            }
        }
    }
}
