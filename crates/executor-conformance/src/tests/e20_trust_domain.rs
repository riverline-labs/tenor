//! E20: Trust domain obligation (conditional on trust).
//!
//! When trust is configured, provenance records MUST include a trust_domain
//! field matching the executor's configured domain. When trust is NOT
//! configured, this test passes unconditionally.

use crate::fixtures;
use crate::traits::TestableExecutor;

/// E20: Provenance records carry trust_domain when trust is configured.
///
/// This test is conditional — it only asserts trust behavior when
/// `is_trust_configured()` returns true.
pub async fn test_e20_trust_domain<E: TestableExecutor>(executor: &E) -> Result<(), String> {
    let contract = fixtures::trust_contract();
    executor
        .load_contract(&contract)
        .await
        .map_err(|e| format!("E20: load_contract failed: {}", e))?;

    // E20 is conditional on trust being configured.
    if !executor.is_trust_configured().await {
        // Trust not configured — test passes unconditionally (AL80: trust is optional).
        return Ok(());
    }

    let facts = fixtures::basic_facts();
    let entity_states = fixtures::initial_entity_states();

    let flow_result = executor
        .execute_flow("trust_flow", "clerk", &facts, &entity_states)
        .await
        .map_err(|e| format!("E20: execute_flow failed: {}", e))?;

    // When trust is configured, provenance records must carry trust_domain.
    if flow_result.provenance.is_empty() {
        return Err("E20: trust is configured but no provenance records were produced".to_string());
    }

    for (i, record) in flow_result.provenance.iter().enumerate() {
        let trust_domain = record.get("trust_domain");
        match trust_domain {
            None => {
                return Err(format!(
                    "E20: provenance record {} missing 'trust_domain' field (trust is configured)",
                    i
                ));
            }
            Some(v) if v.is_null() => {
                return Err(format!(
                    "E20: provenance record {} has null 'trust_domain' (trust is configured)",
                    i
                ));
            }
            Some(v) => {
                if v.as_str().map(|s| s.is_empty()).unwrap_or(false) {
                    return Err(format!(
                        "E20: provenance record {} has empty 'trust_domain' string",
                        i
                    ));
                }
            }
        }
    }

    Ok(())
}
