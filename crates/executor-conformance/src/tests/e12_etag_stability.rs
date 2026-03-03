//! E12: ETag stability obligation.
//!
//! The manifest ETag MUST be stable for the same contract (identical on
//! repeated calls) and MUST change when a different contract is loaded.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E12: ETag is identical on repeated calls for same contract.
pub async fn test_e12_etag_stability<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::basic_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E12: load_contract (first) failed: {}", e))?;

    let manifest1 = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E12: get_manifest (first) failed: {}", e))?;

    let manifest2 = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E12: get_manifest (second) failed: {}", e))?;

    // E12: Same contract → same etag.
    if manifest1.etag != manifest2.etag {
        return Err(format!(
            "E12: etag changed between calls with same contract: '{}' vs '{}'",
            manifest1.etag, manifest2.etag
        ));
    }

    // Load a different contract and verify the etag changes.
    let other_contract = fixtures::multi_instance_contract();
    executor
        .load_contract(&other_contract)
        .await
        .map_err(|e| format!("E12: load_contract (different) failed: {}", e))?;

    let manifest3 = executor
        .get_manifest()
        .await
        .map_err(|e| format!("E12: get_manifest (after different contract) failed: {}", e))?;

    // E12: Different contract → different etag.
    if manifest1.etag == manifest3.etag {
        return Err(format!(
            "E12: etag did not change after loading different contract (etag: '{}')",
            manifest1.etag
        ));
    }

    Ok(())
}
