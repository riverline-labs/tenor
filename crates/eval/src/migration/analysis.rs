//! Migration analysis layer.
//!
//! Builds on the classified diff to produce entity-specific migration
//! actions and an overall severity assessment.

use serde::Serialize;
use serde_json::Value;

use super::classify::{classify_diff, ChangeSeverity, ClassifiedDiff};
use super::diff::diff_bundles;
use super::error::MigrationError;

/// Overall severity of a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MigrationSeverity {
    /// No breaking changes, no entity migration needed.
    Safe,
    /// Non-breaking changes exist, some entities may need attention.
    Cautious,
    /// Breaking changes detected, migration required.
    Breaking,
}

/// Result of analyzing two contract versions for migration impact.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationAnalysis {
    pub entity_changes: Vec<EntityMigrationAction>,
    pub breaking_changes: Vec<BreakingChange>,
    pub overall_severity: MigrationSeverity,
}

/// Migration action for a specific entity.
#[derive(Debug, Clone, Serialize)]
pub struct EntityMigrationAction {
    pub entity_id: String,
    pub action: EntityAction,
}

/// What needs to happen to an entity during migration.
#[derive(Debug, Clone, Serialize)]
pub enum EntityAction {
    NoChange,
    StatesAdded {
        new_states: Vec<String>,
    },
    StatesRemoved {
        removed_states: Vec<String>,
        suggested_target: Option<String>,
    },
    InitialChanged {
        from: String,
        to: String,
    },
    TransitionsChanged {
        added: Vec<(String, String)>,
        removed: Vec<(String, String)>,
    },
}

/// A breaking change found during analysis.
#[derive(Debug, Clone, Serialize)]
pub struct BreakingChange {
    pub construct_kind: String,
    pub construct_id: String,
    pub field: String,
    pub reason: String,
    pub severity: ChangeSeverity,
}

/// Analyze two contract versions and produce a migration analysis.
///
/// Diffs the bundles, classifies changes, then extracts entity-specific
/// migration actions and collects all breaking/requires-analysis changes.
pub fn analyze_migration(
    v1_bundle: &Value,
    v2_bundle: &Value,
) -> Result<MigrationAnalysis, MigrationError> {
    let diff = diff_bundles(v1_bundle, v2_bundle)?;
    let classified = classify_diff(&diff);

    let entity_changes = extract_entity_changes(&classified, v2_bundle);
    let breaking_changes = collect_breaking_changes(&classified);

    let overall_severity = if !breaking_changes.is_empty() {
        MigrationSeverity::Breaking
    } else if !entity_changes.is_empty() {
        MigrationSeverity::Cautious
    } else {
        MigrationSeverity::Safe
    };

    Ok(MigrationAnalysis {
        entity_changes,
        breaking_changes,
        overall_severity,
    })
}

/// Extract entity-specific migration actions from the classified diff.
fn extract_entity_changes(
    classified: &ClassifiedDiff,
    v2_bundle: &Value,
) -> Vec<EntityMigrationAction> {
    let mut actions = Vec::new();

    // Check changed entities
    for change in &classified.changed {
        if change.kind != "Entity" {
            continue;
        }

        for field_diff in &change.fields {
            match field_diff.field.as_str() {
                "states" => {
                    let before_states = extract_string_set(&field_diff.before);
                    let after_states = extract_string_set(&field_diff.after);

                    let added: Vec<String> =
                        after_states.difference(&before_states).cloned().collect();
                    let removed: Vec<String> =
                        before_states.difference(&after_states).cloned().collect();

                    if !removed.is_empty() {
                        // States removed -- suggest the v2 initial state as target
                        let suggested_target = get_entity_initial(v2_bundle, &change.id);
                        actions.push(EntityMigrationAction {
                            entity_id: change.id.clone(),
                            action: EntityAction::StatesRemoved {
                                removed_states: removed,
                                suggested_target,
                            },
                        });
                    } else if !added.is_empty() {
                        actions.push(EntityMigrationAction {
                            entity_id: change.id.clone(),
                            action: EntityAction::StatesAdded { new_states: added },
                        });
                    }
                }
                "initial" => {
                    let from = field_diff.before.as_str().unwrap_or_default().to_string();
                    let to = field_diff.after.as_str().unwrap_or_default().to_string();
                    actions.push(EntityMigrationAction {
                        entity_id: change.id.clone(),
                        action: EntityAction::InitialChanged { from, to },
                    });
                }
                "transitions" => {
                    let before_transitions = extract_transition_set(&field_diff.before);
                    let after_transitions = extract_transition_set(&field_diff.after);

                    let added: Vec<(String, String)> = after_transitions
                        .difference(&before_transitions)
                        .cloned()
                        .collect();
                    let removed: Vec<(String, String)> = before_transitions
                        .difference(&after_transitions)
                        .cloned()
                        .collect();

                    if !added.is_empty() || !removed.is_empty() {
                        actions.push(EntityMigrationAction {
                            entity_id: change.id.clone(),
                            action: EntityAction::TransitionsChanged { added, removed },
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Check removed entities
    for removed in &classified.removed {
        if removed.kind != "Entity" {
            continue;
        }
        actions.push(EntityMigrationAction {
            entity_id: removed.id.clone(),
            action: EntityAction::StatesRemoved {
                removed_states: vec!["(all states)".to_string()],
                suggested_target: None,
            },
        });
    }

    actions
}

/// Collect all breaking and requires-analysis changes.
///
/// Per research recommendation M6, REQUIRES_ANALYSIS is escalated to BREAKING.
fn collect_breaking_changes(classified: &ClassifiedDiff) -> Vec<BreakingChange> {
    let mut changes = Vec::new();

    // From added constructs
    for added in &classified.added {
        if added.classification.severity == ChangeSeverity::Breaking
            || added.classification.severity == ChangeSeverity::RequiresAnalysis
        {
            changes.push(BreakingChange {
                construct_kind: added.kind.clone(),
                construct_id: added.id.clone(),
                field: "(added)".to_string(),
                reason: added.classification.reason.clone(),
                severity: added.classification.severity.clone(),
            });
        }
    }

    // From removed constructs
    for removed in &classified.removed {
        if removed.classification.severity == ChangeSeverity::Breaking
            || removed.classification.severity == ChangeSeverity::RequiresAnalysis
        {
            changes.push(BreakingChange {
                construct_kind: removed.kind.clone(),
                construct_id: removed.id.clone(),
                field: "(removed)".to_string(),
                reason: removed.classification.reason.clone(),
                severity: removed.classification.severity.clone(),
            });
        }
    }

    // From changed constructs (per field)
    for change in &classified.changed {
        for field_diff in &change.fields {
            if field_diff.classification.severity == ChangeSeverity::Breaking
                || field_diff.classification.severity == ChangeSeverity::RequiresAnalysis
            {
                changes.push(BreakingChange {
                    construct_kind: change.kind.clone(),
                    construct_id: change.id.clone(),
                    field: field_diff.field.clone(),
                    reason: field_diff.classification.reason.clone(),
                    severity: field_diff.classification.severity.clone(),
                });
            }
        }
    }

    changes
}

/// Extract a set of strings from a JSON array value.
fn extract_string_set(value: &Value) -> std::collections::BTreeSet<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Extract a set of (from, to) transitions from a JSON array value.
fn extract_transition_set(value: &Value) -> std::collections::BTreeSet<(String, String)> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let from = t.get("from")?.as_str()?.to_string();
                    let to = t.get("to")?.as_str()?.to_string();
                    Some((from, to))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get the initial state for an entity in a bundle.
fn get_entity_initial(bundle: &Value, entity_id: &str) -> Option<String> {
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
    use serde_json::json;

    fn make_bundle(constructs: Vec<Value>) -> Value {
        json!({
            "constructs": constructs,
            "id": "test_bundle",
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

    fn make_transition(from: &str, to: &str) -> Value {
        json!({ "from": from, "to": to })
    }

    #[test]
    fn identical_bundles_safe() {
        let b = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let analysis = analyze_migration(&b, &b).unwrap();
        assert_eq!(analysis.overall_severity, MigrationSeverity::Safe);
        assert!(analysis.entity_changes.is_empty());
        assert!(analysis.breaking_changes.is_empty());
    }

    #[test]
    fn state_added_is_cautious() {
        let v1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let v2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let analysis = analyze_migration(&v1, &v2).unwrap();

        assert_eq!(analysis.entity_changes.len(), 1);
        assert_eq!(analysis.entity_changes[0].entity_id, "Order");
        match &analysis.entity_changes[0].action {
            EntityAction::StatesAdded { new_states } => {
                assert_eq!(new_states, &["approved".to_string()]);
            }
            other => panic!("expected StatesAdded, got {:?}", other),
        }
        // Adding states is non-breaking, but entity has changes -> Cautious
        assert_eq!(analysis.overall_severity, MigrationSeverity::Cautious);
    }

    #[test]
    fn state_removed_is_breaking() {
        let v1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let v2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let analysis = analyze_migration(&v1, &v2).unwrap();

        assert_eq!(analysis.entity_changes.len(), 1);
        match &analysis.entity_changes[0].action {
            EntityAction::StatesRemoved {
                removed_states,
                suggested_target,
            } => {
                assert_eq!(removed_states, &["approved".to_string()]);
                assert_eq!(suggested_target, &Some("draft".to_string()));
            }
            other => panic!("expected StatesRemoved, got {:?}", other),
        }
        assert_eq!(analysis.overall_severity, MigrationSeverity::Breaking);
    }

    #[test]
    fn initial_changed_is_breaking() {
        let v1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "draft",
            vec![make_transition("draft", "submitted")],
        )]);
        let v2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "submitted",
            vec![make_transition("draft", "submitted")],
        )]);
        let analysis = analyze_migration(&v1, &v2).unwrap();

        // Should have an entity change for initial
        let initial_change = analysis
            .entity_changes
            .iter()
            .find(|e| matches!(&e.action, EntityAction::InitialChanged { .. }));
        assert!(
            initial_change.is_some(),
            "should have InitialChanged action"
        );
        match &initial_change.unwrap().action {
            EntityAction::InitialChanged { from, to } => {
                assert_eq!(from, "draft");
                assert_eq!(to, "submitted");
            }
            _ => unreachable!(),
        }
        assert_eq!(analysis.overall_severity, MigrationSeverity::Breaking);
    }

    #[test]
    fn transitions_changed() {
        let v1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            "draft",
            vec![
                make_transition("draft", "submitted"),
                make_transition("submitted", "approved"),
            ],
        )]);
        let v2 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted", "approved"],
            "draft",
            vec![
                make_transition("draft", "submitted"),
                make_transition("draft", "approved"),
            ],
        )]);
        let analysis = analyze_migration(&v1, &v2).unwrap();

        let transition_change = analysis
            .entity_changes
            .iter()
            .find(|e| matches!(&e.action, EntityAction::TransitionsChanged { .. }));
        assert!(
            transition_change.is_some(),
            "should have TransitionsChanged action"
        );
        match &transition_change.unwrap().action {
            EntityAction::TransitionsChanged { added, removed } => {
                assert_eq!(
                    added,
                    &[("draft".to_string(), "approved".to_string())],
                    "should have added draft->approved"
                );
                assert_eq!(
                    removed,
                    &[("submitted".to_string(), "approved".to_string())],
                    "should have removed submitted->approved"
                );
            }
            _ => unreachable!(),
        }
        // Removed transitions are breaking
        assert_eq!(analysis.overall_severity, MigrationSeverity::Breaking);
    }

    #[test]
    fn removed_entity_is_breaking() {
        let v1 = make_bundle(vec![make_entity(
            "Order",
            vec!["draft", "submitted"],
            "draft",
            vec![],
        )]);
        let v2 = make_bundle(vec![]);
        let analysis = analyze_migration(&v1, &v2).unwrap();

        assert_eq!(analysis.entity_changes.len(), 1);
        assert_eq!(analysis.entity_changes[0].entity_id, "Order");
        match &analysis.entity_changes[0].action {
            EntityAction::StatesRemoved {
                removed_states,
                suggested_target,
            } => {
                assert_eq!(removed_states, &["(all states)".to_string()]);
                assert_eq!(suggested_target, &None);
            }
            other => panic!("expected StatesRemoved for removed entity, got {:?}", other),
        }
        assert_eq!(analysis.overall_severity, MigrationSeverity::Breaking);
    }
}
