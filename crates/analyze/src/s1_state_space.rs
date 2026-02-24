//! S1 — Complete State Space analysis.
//!
//! For each Entity, enumerates the complete set of declared states S(e).
//! Spec reference: Section 15, S1.
//!
//! This is a straightforward extraction: each entity's `states` array
//! is the complete state space. The analysis also captures the initial
//! state and transition relation for downstream analyses (S2, S3a).

use crate::bundle::AnalysisBundle;
use serde::Serialize;
use std::collections::BTreeMap;

/// State space result for a single Entity.
#[derive(Debug, Clone, Serialize)]
pub struct StateSpaceResult {
    pub entity_id: String,
    pub declared_states: Vec<String>,
    pub initial_state: String,
    pub transitions: Vec<(String, String)>,
    pub state_count: usize,
}

/// Aggregated S1 result across all entities.
#[derive(Debug, Clone, Serialize)]
pub struct S1Result {
    /// Keyed by entity ID for deterministic output ordering.
    pub entities: BTreeMap<String, StateSpaceResult>,
}

/// S1 — Enumerate the complete state space for every Entity in the bundle.
///
/// For each Entity, builds a `StateSpaceResult` containing all declared
/// states, the initial state, and the transition relation.
pub fn analyze_state_space(bundle: &AnalysisBundle) -> S1Result {
    let mut entities = BTreeMap::new();

    for entity in &bundle.entities {
        let transitions: Vec<(String, String)> = entity
            .transitions
            .iter()
            .map(|t| (t.from.clone(), t.to.clone()))
            .collect();

        let result = StateSpaceResult {
            entity_id: entity.id.clone(),
            declared_states: entity.states.clone(),
            initial_state: entity.initial.clone(),
            state_count: entity.states.len(),
            transitions,
        };

        entities.insert(entity.id.clone(), result);
    }

    S1Result { entities }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{AnalysisEntity, Transition};

    fn make_bundle(entities: Vec<AnalysisEntity>) -> AnalysisBundle {
        AnalysisBundle {
            entities,
            facts: vec![],
            rules: vec![],
            operations: vec![],
            flows: vec![],
            personas: vec![],
            systems: vec![],
        }
    }

    #[test]
    fn test_single_entity_three_states() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Order".to_string(),
            states: vec![
                "draft".to_string(),
                "submitted".to_string(),
                "approved".to_string(),
            ],
            initial: "draft".to_string(),
            transitions: vec![
                Transition {
                    from: "draft".to_string(),
                    to: "submitted".to_string(),
                },
                Transition {
                    from: "submitted".to_string(),
                    to: "approved".to_string(),
                },
            ],
            parent: None,
        }]);

        let result = analyze_state_space(&bundle);
        assert_eq!(result.entities.len(), 1);

        let order = &result.entities["Order"];
        assert_eq!(order.entity_id, "Order");
        assert_eq!(order.declared_states.len(), 3);
        assert_eq!(order.initial_state, "draft");
        assert_eq!(order.state_count, 3);
        assert_eq!(order.transitions.len(), 2);
        assert_eq!(
            order.transitions[0],
            ("draft".to_string(), "submitted".to_string())
        );
    }

    #[test]
    fn test_multiple_entities() {
        let bundle = make_bundle(vec![
            AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string(), "done".to_string()],
                initial: "draft".to_string(),
                transitions: vec![Transition {
                    from: "draft".to_string(),
                    to: "done".to_string(),
                }],
                parent: None,
            },
            AnalysisEntity {
                id: "Payment".to_string(),
                states: vec![
                    "pending".to_string(),
                    "paid".to_string(),
                    "refunded".to_string(),
                ],
                initial: "pending".to_string(),
                transitions: vec![
                    Transition {
                        from: "pending".to_string(),
                        to: "paid".to_string(),
                    },
                    Transition {
                        from: "paid".to_string(),
                        to: "refunded".to_string(),
                    },
                ],
                parent: None,
            },
        ]);

        let result = analyze_state_space(&bundle);
        assert_eq!(result.entities.len(), 2);
        assert!(result.entities.contains_key("Order"));
        assert!(result.entities.contains_key("Payment"));
        assert_eq!(result.entities["Order"].state_count, 2);
        assert_eq!(result.entities["Payment"].state_count, 3);
    }

    #[test]
    fn test_single_state_no_transitions() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Singleton".to_string(),
            states: vec!["active".to_string()],
            initial: "active".to_string(),
            transitions: vec![],
            parent: None,
        }]);

        let result = analyze_state_space(&bundle);
        assert_eq!(result.entities.len(), 1);
        let singleton = &result.entities["Singleton"];
        assert_eq!(singleton.state_count, 1);
        assert_eq!(singleton.declared_states, vec!["active"]);
        assert!(singleton.transitions.is_empty());
    }

    #[test]
    fn test_empty_bundle() {
        let bundle = make_bundle(vec![]);
        let result = analyze_state_space(&bundle);
        assert!(result.entities.is_empty());
    }

    #[test]
    fn test_deterministic_btreemap_ordering() {
        let bundle = make_bundle(vec![
            AnalysisEntity {
                id: "Zebra".to_string(),
                states: vec!["a".to_string()],
                initial: "a".to_string(),
                transitions: vec![],
                parent: None,
            },
            AnalysisEntity {
                id: "Alpha".to_string(),
                states: vec!["b".to_string()],
                initial: "b".to_string(),
                transitions: vec![],
                parent: None,
            },
        ]);

        let result = analyze_state_space(&bundle);
        let keys: Vec<&String> = result.entities.keys().collect();
        // BTreeMap orders alphabetically
        assert_eq!(keys, vec!["Alpha", "Zebra"]);
    }

    #[test]
    fn test_entity_with_parent() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "SubOrder".to_string(),
            states: vec!["open".to_string(), "closed".to_string()],
            initial: "open".to_string(),
            transitions: vec![Transition {
                from: "open".to_string(),
                to: "closed".to_string(),
            }],
            parent: Some("Order".to_string()),
        }]);

        let result = analyze_state_space(&bundle);
        assert_eq!(result.entities.len(), 1);
        // S1 enumerates states regardless of parent relationship
        assert_eq!(result.entities["SubOrder"].state_count, 2);
    }
}
