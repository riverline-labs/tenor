//! S2 — Reachable States analysis.
//!
//! For each Entity, derives the set of states reachable from the initial
//! state via the declared transition relation using BFS. Also reports
//! dead states (declared but unreachable from initial).
//!
//! Spec reference: Section 15, S2.

use crate::bundle::AnalysisBundle;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Reachability result for a single Entity.
#[derive(Debug, Clone, Serialize)]
pub struct ReachabilityResult {
    pub entity_id: String,
    pub reachable_states: BTreeSet<String>,
    pub unreachable_states: BTreeSet<String>,
    pub initial_state: String,
}

/// Aggregated S2 result across all entities.
#[derive(Debug, Clone, Serialize)]
pub struct S2Result {
    /// Keyed by entity ID for deterministic output ordering.
    pub entities: BTreeMap<String, ReachabilityResult>,
    /// Convenience flag: true if ANY entity has unreachable states.
    pub has_dead_states: bool,
}

/// S2 — Derive reachable states for every Entity via BFS from initial.
///
/// For each Entity, performs BFS from the initial state following the
/// declared transition relation. States declared but not reachable from
/// the initial state are reported as dead states.
pub fn analyze_reachability(bundle: &AnalysisBundle) -> S2Result {
    let mut entities = BTreeMap::new();
    let mut has_dead_states = false;

    for entity in &bundle.entities {
        // Build adjacency list: from_state -> [to_states]
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        for t in &entity.transitions {
            adjacency.entry(t.from.as_str()).or_default().push(t.to.as_str());
        }

        // BFS from initial state
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();

        visited.insert(entity.initial.clone());
        queue.push_back(entity.initial.as_str());

        while let Some(state) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(state) {
                for &next in neighbors {
                    if visited.insert(next.to_string()) {
                        queue.push_back(next);
                    }
                }
            }
        }

        // Compute unreachable states: declared - reachable
        let declared: BTreeSet<String> = entity.states.iter().cloned().collect();
        let unreachable: BTreeSet<String> = declared
            .difference(&visited)
            .cloned()
            .collect();

        if !unreachable.is_empty() {
            has_dead_states = true;
        }

        let result = ReachabilityResult {
            entity_id: entity.id.clone(),
            reachable_states: visited,
            unreachable_states: unreachable,
            initial_state: entity.initial.clone(),
        };

        entities.insert(entity.id.clone(), result);
    }

    S2Result {
        entities,
        has_dead_states,
    }
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
        }
    }

    #[test]
    fn test_all_states_reachable() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Order".to_string(),
            states: vec![
                "draft".to_string(),
                "submitted".to_string(),
                "approved".to_string(),
            ],
            initial: "draft".to_string(),
            transitions: vec![
                Transition { from: "draft".to_string(), to: "submitted".to_string() },
                Transition { from: "submitted".to_string(), to: "approved".to_string() },
            ],
            parent: None,
        }]);

        let result = analyze_reachability(&bundle);
        let order = &result.entities["Order"];
        assert_eq!(order.reachable_states.len(), 3);
        assert!(order.unreachable_states.is_empty());
        assert!(!result.has_dead_states);
    }

    #[test]
    fn test_dead_state_detected() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Order".to_string(),
            states: vec![
                "draft".to_string(),
                "submitted".to_string(),
                "archived".to_string(),
            ],
            initial: "draft".to_string(),
            transitions: vec![
                Transition { from: "draft".to_string(), to: "submitted".to_string() },
                // No path to "archived" from "draft"
            ],
            parent: None,
        }]);

        let result = analyze_reachability(&bundle);
        let order = &result.entities["Order"];
        assert_eq!(order.reachable_states.len(), 2);
        assert!(order.reachable_states.contains("draft"));
        assert!(order.reachable_states.contains("submitted"));
        assert_eq!(order.unreachable_states.len(), 1);
        assert!(order.unreachable_states.contains("archived"));
        assert!(result.has_dead_states);
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

        let result = analyze_reachability(&bundle);
        let singleton = &result.entities["Singleton"];
        assert_eq!(singleton.reachable_states.len(), 1);
        assert!(singleton.reachable_states.contains("active"));
        assert!(singleton.unreachable_states.is_empty());
        assert!(!result.has_dead_states);
    }

    #[test]
    fn test_disconnected_subgraph_multiple_dead() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Order".to_string(),
            states: vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
            ],
            initial: "a".to_string(),
            transitions: vec![
                Transition { from: "a".to_string(), to: "b".to_string() },
                // d -> e is a disconnected subgraph, unreachable from a
                Transition { from: "d".to_string(), to: "e".to_string() },
            ],
            parent: None,
        }]);

        let result = analyze_reachability(&bundle);
        let order = &result.entities["Order"];
        assert_eq!(order.reachable_states.len(), 2); // a, b
        assert_eq!(order.unreachable_states.len(), 3); // c, d, e
        assert!(order.unreachable_states.contains("c"));
        assert!(order.unreachable_states.contains("d"));
        assert!(order.unreachable_states.contains("e"));
        assert!(result.has_dead_states);
    }

    #[test]
    fn test_cycle_in_transitions() {
        let bundle = make_bundle(vec![AnalysisEntity {
            id: "Order".to_string(),
            states: vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
            ],
            initial: "a".to_string(),
            transitions: vec![
                Transition { from: "a".to_string(), to: "b".to_string() },
                Transition { from: "b".to_string(), to: "c".to_string() },
                Transition { from: "c".to_string(), to: "a".to_string() }, // cycle
            ],
            parent: None,
        }]);

        let result = analyze_reachability(&bundle);
        let order = &result.entities["Order"];
        // All reachable despite cycle
        assert_eq!(order.reachable_states.len(), 3);
        assert!(order.unreachable_states.is_empty());
        assert!(!result.has_dead_states);
    }

    #[test]
    fn test_multiple_entities_mixed() {
        let bundle = make_bundle(vec![
            AnalysisEntity {
                id: "Clean".to_string(),
                states: vec!["a".to_string(), "b".to_string()],
                initial: "a".to_string(),
                transitions: vec![Transition { from: "a".to_string(), to: "b".to_string() }],
                parent: None,
            },
            AnalysisEntity {
                id: "Dirty".to_string(),
                states: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                initial: "x".to_string(),
                transitions: vec![Transition { from: "x".to_string(), to: "y".to_string() }],
                parent: None,
            },
        ]);

        let result = analyze_reachability(&bundle);
        assert!(!result.entities["Clean"].unreachable_states.contains("a"));
        assert!(result.entities["Dirty"].unreachable_states.contains("z"));
        assert!(result.has_dead_states);
    }
}
