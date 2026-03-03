//! E10: Manifest endpoint obligation.
//!
//! The executor MUST serve a manifest with bundle, etag, tenor, and
//! capabilities fields. All four fields are required.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E10: Manifest has all required fields (bundle, etag, tenor, capabilities).
pub async fn test_e10_manifest_endpoint<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E10: load_contract failed: {}", e))?;

    let manifest = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E10: get_manifest failed: {}", e))?;

    // E10: manifest must have a bundle field.
    if manifest.bundle.is_null() {
        return Err("E10: manifest.bundle is null — must contain the contract bundle".to_string());
    }

    // E10: manifest must have a non-empty etag.
    if manifest.etag.is_empty() {
        return Err(
            "E10: manifest.etag is empty — must be a content-derived identifier".to_string(),
        );
    }

    // E10: manifest must have a tenor version.
    if manifest.tenor.is_empty() {
        return Err(
            "E10: manifest.tenor is empty — must identify the Tenor spec version".to_string(),
        );
    }

    // E10: manifest must have a capabilities object.
    if manifest.capabilities.is_null() {
        return Err(
            "E10: manifest.capabilities is null — must advertise supported features".to_string(),
        );
    }

    Ok(())
}
