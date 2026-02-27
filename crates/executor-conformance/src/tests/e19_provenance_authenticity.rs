//! E19: Provenance authenticity obligation (conditional on trust).
//!
//! When trust is configured, provenance records MUST carry an attestation
//! field. When trust is NOT configured, this test passes unconditionally.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E19: Provenance records contain attestation when trust is configured.
///
/// This test is conditional — it only asserts trust behavior when
/// `is_trust_configured()` returns true.
pub async fn test_e19_provenance_authenticity<E: TestableExecutor>(
    executor: &E,
) -> Result<(), String> {
    let contract = fixtures::trust_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E19: load_contract failed: {}", e))?;

    // E19 is conditional on trust being configured.
    if !executor.is_trust_configured().await {
        // Trust not configured — test passes unconditionally (AL80: trust is optional).
        return Ok(());
    }

    let facts = fixtures::basic_facts();
    let entity_states = fixtures::initial_entity_states();

    let flow_result = executor
        .execute_flow("trust_flow", "clerk", &facts, &entity_states)
        .await
        .map_err(|e| format!("E19: execute_flow failed: {}", e))?;

    // When trust is configured, provenance records must carry an attestation.
    if flow_result.provenance.is_empty() {
        return Err("E19: trust is configured but no provenance records were produced".to_string());
    }

    for (i, record) in flow_result.provenance.iter().enumerate() {
        let attestation = record.get("attestation");
        match attestation {
            None => {
                return Err(format!(
                    "E19: provenance record {} missing 'attestation' field (trust is configured)",
                    i
                ));
            }
            Some(v) if v.is_null() => {
                return Err(format!(
                    "E19: provenance record {} has null 'attestation' (trust is configured)",
                    i
                ));
            }
            Some(v) => {
                if v.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                    return Err(format!(
                        "E19: provenance record {} has empty 'attestation' string",
                        i
                    ));
                }
            }
        }
    }

    Ok(())
}
