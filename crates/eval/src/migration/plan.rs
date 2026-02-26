//! MigrationPlan type and builder.
//!
//! Aggregates migration analysis, flow compatibility results, and
//! entity state mappings into a complete migration plan.

use serde::Serialize;
use serde_json::Value;

use super::analysis::{EntityAction, MigrationAnalysis, MigrationSeverity};
use super::error::MigrationError;

/// A complete migration plan from v1 to v2.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationPlan {
    pub v1_id: String,
    pub v2_id: String,
    pub analysis: MigrationAnalysis,
    /// Flow compatibility results (populated by the compatibility checker in Plan 02).
    pub flow_compatibility: Vec<FlowCompatibilityResult>,
    pub entity_state_mappings: Vec<EntityStateMapping>,
    pub severity: MigrationSeverity,
    pub recommended_policy: MigrationPolicy,
}

/// Mapping of an entity instance from one state to another during migration.
#[derive(Debug, Clone, Serialize)]
pub struct EntityStateMapping {
    pub entity_id: String,
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
}

/// Recommended migration policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MigrationPolicy {
    /// Abort in-flight flows, migrate entities.
    Abort,
    /// Run v1 and v2 simultaneously, drain v1.
    BluGreen,
    /// Migrate in-flight flows to v2 positions.
    ForceMigrate,
}

/// Result of checking a flow's compatibility with v2.
///
/// Forward-declared here; the full compatibility checker is in Plan 02.
#[derive(Debug, Clone, Serialize)]
pub struct FlowCompatibilityResult {
    pub flow_id: String,
    pub position: Option<String>,
    pub compatible: bool,
    pub layer_results: LayerResults,
    pub reasons: Vec<IncompatibilityReason>,
}

/// Per-layer compatibility verdicts.
#[derive(Debug, Clone, Serialize)]
pub struct LayerResults {
    pub layer1_verdict_isolation: bool,
    pub layer2_entity_state: bool,
    pub layer3_structure: bool,
}

/// Reason a flow is incompatible with v2.
#[derive(Debug, Clone, Serialize)]
pub enum IncompatibilityReason {
    EntityStateNotInV2 {
        entity_id: String,
        state: String,
    },
    TransitionNotInV2 {
        entity_id: String,
        from: String,
        to: String,
    },
    StepNotInV2 {
        step_id: String,
    },
    OperationChangedInV2 {
        operation_id: String,
    },
    PersonaNotAuthorized {
        step_id: String,
        persona: String,
    },
    FactDependencyUnsatisfied {
        step_id: String,
        fact_id: String,
    },
}

/// Build a migration plan from analysis results.
///
/// Creates entity state mappings for entities with removed states,
/// sets severity from analysis, and defaults to Abort policy.
pub fn build_migration_plan(
    v1_bundle: &Value,
    v2_bundle: &Value,
    analysis: MigrationAnalysis,
) -> Result<MigrationPlan, MigrationError> {
    let v1_id = v1_bundle
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let v2_id = v2_bundle
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Build entity state mappings for entities with removed states
    let mut entity_state_mappings = Vec::new();
    for entity_change in &analysis.entity_changes {
        if let EntityAction::StatesRemoved {
            removed_states,
            suggested_target,
        } = &entity_change.action
        {
            // Get the v2 initial state as the fallback target
            let target = suggested_target.clone().unwrap_or_else(|| {
                get_v2_initial(v2_bundle, &entity_change.entity_id)
                    .unwrap_or_else(|| "(unknown)".to_string())
            });

            for state in removed_states {
                entity_state_mappings.push(EntityStateMapping {
                    entity_id: entity_change.entity_id.clone(),
                    instance_id: String::new(), // placeholder -- populated at execution time
                    from_state: state.clone(),
                    to_state: target.clone(),
                });
            }
        }
    }

    let severity = analysis.overall_severity;

    Ok(MigrationPlan {
        v1_id,
        v2_id,
        analysis,
        flow_compatibility: Vec::new(), // populated by Plan 02's compatibility checker
        entity_state_mappings,
        severity,
        recommended_policy: MigrationPolicy::Abort, // default for Phase 1 per research
    })
}

/// Get the initial state for an entity in the v2 bundle.
fn get_v2_initial(bundle: &Value, entity_id: &str) -> Option<String> {
    bundle
        .get("constructs")?
        .as_array()?
        .iter()
        .find(|c| {
            c.get("kind").and_then(|k| k.as_str()) == Some("Entity")
                && c.get("id").and_then(|i| i.as_str()) == Some(entity_id)
        })
        .and_then(|c| c.get("initial")?.as_str().map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::analysis::analyze_migration;
    use serde_json::json;

    fn make_bundle_with_id(id: &str, constructs: Vec<Value>) -> Value {
        json!({
            "constructs": constructs,
            "id": id,
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0"
        })
    }

    fn make_entity(id: &str, states: Vec<&str>, initial: &str, transitions: Vec<Value>) -> Value {
        json!({
            "id": id,
            "kind": "Entity",
            "initial": initial,
            "provenance": { "file": "test.tenor", "line": 1 },
            "states": states,
            "tenor": "1.0",
            "transitions": transitions
        })
    }

    #[test]
    fn build_plan_identical_bundles() {
        let v1 = make_bundle_with_id(
            "contract_v1",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted"],
                "draft",
                vec![],
            )],
        );
        let v2 = make_bundle_with_id(
            "contract_v2",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted"],
                "draft",
                vec![],
            )],
        );

        let analysis = analyze_migration(&v1, &v2).unwrap();
        let plan = build_migration_plan(&v1, &v2, analysis).unwrap();

        assert_eq!(plan.v1_id, "contract_v1");
        assert_eq!(plan.v2_id, "contract_v2");
        assert_eq!(plan.severity, MigrationSeverity::Safe);
        assert!(plan.entity_state_mappings.is_empty());
        assert!(plan.flow_compatibility.is_empty());
        assert_eq!(plan.recommended_policy, MigrationPolicy::Abort);
    }

    #[test]
    fn build_plan_with_removed_states() {
        let v1 = make_bundle_with_id(
            "contract_v1",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted", "approved"],
                "draft",
                vec![],
            )],
        );
        let v2 = make_bundle_with_id(
            "contract_v2",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted"],
                "draft",
                vec![],
            )],
        );

        let analysis = analyze_migration(&v1, &v2).unwrap();
        let plan = build_migration_plan(&v1, &v2, analysis).unwrap();

        assert_eq!(plan.severity, MigrationSeverity::Breaking);
        assert_eq!(plan.entity_state_mappings.len(), 1);
        assert_eq!(plan.entity_state_mappings[0].entity_id, "Order");
        assert_eq!(plan.entity_state_mappings[0].from_state, "approved");
        assert_eq!(plan.entity_state_mappings[0].to_state, "draft");
    }

    #[test]
    fn build_plan_with_added_states() {
        let v1 = make_bundle_with_id(
            "contract_v1",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted"],
                "draft",
                vec![],
            )],
        );
        let v2 = make_bundle_with_id(
            "contract_v2",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted", "approved"],
                "draft",
                vec![],
            )],
        );

        let analysis = analyze_migration(&v1, &v2).unwrap();
        let plan = build_migration_plan(&v1, &v2, analysis).unwrap();

        assert_eq!(plan.severity, MigrationSeverity::Cautious);
        // No state mappings needed for added states
        assert!(plan.entity_state_mappings.is_empty());
    }

    #[test]
    fn plan_serializes_to_json() {
        let v1 = make_bundle_with_id(
            "contract_v1",
            vec![make_entity(
                "Order",
                vec!["draft", "submitted"],
                "draft",
                vec![],
            )],
        );

        let analysis = analyze_migration(&v1, &v1).unwrap();
        let plan = build_migration_plan(&v1, &v1, analysis).unwrap();

        let json = serde_json::to_value(&plan).unwrap();
        assert_eq!(json["v1_id"], "contract_v1");
        assert_eq!(json["v2_id"], "contract_v1");
        assert_eq!(json["severity"], "Safe");
        assert_eq!(json["recommended_policy"], "Abort");
    }
}
