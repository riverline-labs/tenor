//! E18: Artifact integrity obligation (conditional on trust).
//!
//! When trust is configured, the manifest MUST include a bundle_attestation
//! in its trust section. When trust is NOT configured, this test passes
//! unconditionally (trust is optional per AL80).

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E18: Manifest includes bundle attestation when trust is configured.
///
/// This test is conditional — it only asserts trust behavior when
/// `is_trust_configured()` returns true.
pub async fn test_e18_artifact_integrity<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::trust_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E18: load_contract failed: {}", e))?;

    // E18 is conditional on trust being configured.
    if !executor.is_trust_configured().await {
        // Trust not configured — test passes unconditionally (AL80: trust is optional).
        return Ok(());
    }

    let manifest = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E18: get_manifest failed: {}", e))?;

    // When trust is configured, the manifest must have a trust section.
    let trust = manifest
        .trust
        .ok_or("E18: trust is configured but manifest has no trust section".to_string())?;

    // The trust section must have a bundle_attestation field.
    let attestation = trust
        .get("bundle_attestation")
        .ok_or("E18: trust section missing 'bundle_attestation' field".to_string())?;

    if attestation.is_null() || attestation.as_str().map(|s| s.is_empty()).unwrap_or(false) {
        return Err("E18: manifest.trust.bundle_attestation is null or empty".to_string());
    }

    Ok(())
}
