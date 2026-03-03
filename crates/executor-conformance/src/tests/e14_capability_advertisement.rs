//! E14: Capability advertisement obligation.
//!
//! The manifest MUST advertise executor capabilities via the capabilities
//! object. The object must be present and be a JSON object (not null/array).

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E14: Manifest capabilities object is present and well-formed.
pub async fn test_e14_capability_advertisement<E: TestableExecutor>(
    executor: &E,
) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E14: load_contract failed: {}", e))?;

    let manifest = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E14: get_manifest failed: {}", e))?;

    // E14: capabilities must be present (not null).
    if manifest.capabilities.is_null() {
        return Err(
            "E14: manifest.capabilities is null — must be a JSON object advertising features"
                .to_string(),
        );
    }

    // E14: capabilities must be a JSON object.
    if !manifest.capabilities.is_object() {
        return Err(format!(
            "E14: manifest.capabilities must be a JSON object, got: {}",
            manifest.capabilities
        ));
    }

    // E14: capabilities object may be empty — but it must exist.
    // Specific keys (like "multi_instance", "trust", "simulation") are
    // dynamically advertised based on executor configuration.
    Ok(())
}
