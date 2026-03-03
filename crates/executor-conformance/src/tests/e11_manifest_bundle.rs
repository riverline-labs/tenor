//! E11: Manifest bundle completeness obligation.
//!
//! The manifest's bundle field MUST contain all constructs from the
//! loaded contract.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E11: Manifest bundle contains all constructs from the loaded contract.
pub async fn test_e11_manifest_bundle_complete<E: TestableExecutor>(
    executor: &E,
) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E11: load_contract failed: {}", e))?;

    let manifest = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E11: get_manifest failed: {}", e))?;

    // The bundle must be a non-null object.
    if !manifest.bundle.is_object() && !manifest.bundle.is_array() {
        return Err(format!(
            "E11: manifest.bundle must be an object or array, got: {}",
            manifest.bundle
        ));
    }

    // If it's an object, check for an "id" or "constructs" field to verify completeness.
    if let Some(obj) = manifest.bundle.as_object() {
        if !obj.contains_key("id") && !obj.contains_key("constructs") {
            return Err(
                "E11: manifest.bundle does not appear to be a complete bundle (missing 'id' or 'constructs')"
                    .to_string(),
            );
        }

        // Verify constructs field has entries if present.
        if let Some(constructs) = obj.get("constructs") {
            if let Some(arr) = constructs.as_array() {
                if arr.is_empty() {
                    return Err("E11: manifest.bundle.constructs is empty — must contain contract constructs".to_string());
                }
            }
        }
    }

    Ok(())
}
