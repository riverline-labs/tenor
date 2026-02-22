//! S7 — Evaluation Complexity Bounds analysis.
//!
//! Computes complexity bounds for predicate expressions (rules and operation
//! preconditions) and flow execution depth bounds.
//!
//! Spec reference: Section 15, S7.

use crate::bundle::AnalysisBundle;
use crate::s6_flow_paths::S6Result;
use serde::Serialize;
use std::collections::BTreeMap;

/// Complexity analysis for a single predicate expression.
#[derive(Debug, Clone, Serialize)]
pub struct PredicateComplexity {
    pub source_id: String,
    pub node_count: usize,
    pub max_depth: usize,
    pub has_quantifiers: bool,
    pub complexity_class: String,
}

/// Flow execution depth bound.
#[derive(Debug, Clone, Serialize)]
pub struct FlowDepthBound {
    pub flow_id: String,
    pub max_path_depth: usize,
    pub max_step_count: usize,
    pub has_cycles: bool,
}

/// Aggregated S7 result.
#[derive(Debug, Clone, Serialize)]
pub struct S7Result {
    pub predicate_complexities: Vec<PredicateComplexity>,
    pub flow_depth_bounds: BTreeMap<String, FlowDepthBound>,
    pub max_predicate_depth: usize,
    pub max_flow_depth: usize,
}

/// S7 — Compute evaluation complexity bounds.
pub fn analyze_complexity(bundle: &AnalysisBundle, s6: &S6Result) -> S7Result {
    let mut predicate_complexities = Vec::new();
    let mut max_predicate_depth: usize = 0;

    // Analyze rule predicates
    for rule in &bundle.rules {
        let (node_count, depth) = walk_expression_tree(&rule.when);
        let has_quantifiers = has_quantifier_nodes(&rule.when);
        let complexity_class = classify_complexity(node_count, has_quantifiers);

        max_predicate_depth = max_predicate_depth.max(depth);

        predicate_complexities.push(PredicateComplexity {
            source_id: rule.id.clone(),
            node_count,
            max_depth: depth,
            has_quantifiers,
            complexity_class,
        });
    }

    // Analyze operation preconditions
    for operation in &bundle.operations {
        if let Some(ref precondition) = operation.precondition {
            let (node_count, depth) = walk_expression_tree(precondition);
            let has_quantifiers = has_quantifier_nodes(precondition);
            let complexity_class = classify_complexity(node_count, has_quantifiers);

            max_predicate_depth = max_predicate_depth.max(depth);

            predicate_complexities.push(PredicateComplexity {
                source_id: operation.id.clone(),
                node_count,
                max_depth: depth,
                has_quantifiers,
                complexity_class,
            });
        }
    }

    // Sort for deterministic output
    predicate_complexities.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    // Compute flow depth bounds from S6
    let mut flow_depth_bounds = BTreeMap::new();
    let mut max_flow_depth: usize = 0;

    for (flow_id, flow_result) in &s6.flows {
        let has_cycles = flow_result.paths.iter().any(|p| {
            p.terminal_outcome
                .as_deref()
                .map_or(false, |o| o == "cycle_detected")
        });

        let bound = FlowDepthBound {
            flow_id: flow_id.clone(),
            max_path_depth: flow_result.max_depth,
            max_step_count: flow_result.reachable_steps.len(),
            has_cycles,
        };

        max_flow_depth = max_flow_depth.max(flow_result.max_depth);
        flow_depth_bounds.insert(flow_id.clone(), bound);
    }

    S7Result {
        predicate_complexities,
        flow_depth_bounds,
        max_predicate_depth,
        max_flow_depth,
    }
}

/// Walk an expression tree and return (node_count, max_depth).
fn walk_expression_tree(expr: &serde_json::Value) -> (usize, usize) {
    if expr.is_null() || expr.is_boolean() || expr.is_number() || expr.is_string() {
        return (1, 1);
    }

    if let Some(obj) = expr.as_object() {
        // Leaf nodes
        if obj.contains_key("fact_ref") || obj.contains_key("literal") {
            return (1, 1);
        }

        if obj.contains_key("verdict_present") {
            return (1, 1);
        }

        // Comparison node
        if obj.contains_key("op") && (obj.contains_key("left") || obj.contains_key("right")) {
            let (left_count, left_depth) = obj
                .get("left")
                .map(|l| walk_expression_tree(l))
                .unwrap_or((0, 0));
            let (right_count, right_depth) = obj
                .get("right")
                .map(|r| walk_expression_tree(r))
                .unwrap_or((0, 0));
            return (
                1 + left_count + right_count,
                1 + left_depth.max(right_depth),
            );
        }

        // And node
        if let Some(children) = obj.get("and").and_then(|a| a.as_array()) {
            let mut total_nodes = 1;
            let mut max_child_depth = 0;
            for child in children {
                let (count, depth) = walk_expression_tree(child);
                total_nodes += count;
                max_child_depth = max_child_depth.max(depth);
            }
            return (total_nodes, 1 + max_child_depth);
        }

        // Or node
        if let Some(children) = obj.get("or").and_then(|a| a.as_array()) {
            let mut total_nodes = 1;
            let mut max_child_depth = 0;
            for child in children {
                let (count, depth) = walk_expression_tree(child);
                total_nodes += count;
                max_child_depth = max_child_depth.max(depth);
            }
            return (total_nodes, 1 + max_child_depth);
        }

        // Not node
        if let Some(inner) = obj.get("not") {
            let (count, depth) = walk_expression_tree(inner);
            return (1 + count, 1 + depth);
        }

        // Forall/exists node
        if obj.contains_key("forall") || obj.contains_key("exists") {
            let body_key = if obj.contains_key("forall") {
                "forall"
            } else {
                "exists"
            };
            let (count, depth) = obj
                .get(body_key)
                .map(|b| walk_expression_tree(b))
                .unwrap_or((0, 0));
            return (1 + count, 1 + depth);
        }
    }

    // Default: unknown structure counts as 1 node
    (1, 1)
}

/// Check if an expression tree contains quantifier nodes (forall/exists).
fn has_quantifier_nodes(expr: &serde_json::Value) -> bool {
    if let Some(obj) = expr.as_object() {
        if obj.contains_key("forall") || obj.contains_key("exists") {
            return true;
        }

        // Check children recursively
        if let Some(children) = obj.get("and").and_then(|a| a.as_array()) {
            return children.iter().any(has_quantifier_nodes);
        }
        if let Some(children) = obj.get("or").and_then(|a| a.as_array()) {
            return children.iter().any(has_quantifier_nodes);
        }
        if let Some(inner) = obj.get("not") {
            return has_quantifier_nodes(inner);
        }
        if let Some(left) = obj.get("left") {
            if has_quantifier_nodes(left) {
                return true;
            }
        }
        if let Some(right) = obj.get("right") {
            if has_quantifier_nodes(right) {
                return true;
            }
        }
    }
    false
}

/// Classify complexity based on expression structure.
fn classify_complexity(node_count: usize, has_quantifiers: bool) -> String {
    if has_quantifiers {
        "O(n)".to_string()
    } else if node_count <= 1 {
        "O(1)".to_string()
    } else {
        format!("O({})", node_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::*;
    use crate::s6_flow_paths::FlowPathResult;
    use serde_json::json;
    use std::collections::BTreeSet;

    fn empty_s6() -> S6Result {
        S6Result {
            flows: BTreeMap::new(),
            total_paths: 0,
        }
    }

    fn make_bundle_with(rules: Vec<AnalysisRule>, operations: Vec<AnalysisOperation>) -> AnalysisBundle {
        AnalysisBundle {
            entities: vec![],
            facts: vec![],
            rules,
            operations,
            flows: vec![],
            personas: vec![],
        }
    }

    #[test]
    fn test_simple_comparison_predicate() {
        let bundle = make_bundle_with(
            vec![AnalysisRule {
                id: "rule_a".to_string(),
                stratum: 0,
                when: json!({"left": {"fact_ref": "amount"}, "op": ">", "right": {"literal": 100}}),
                produce_verdict_type: "high_value".to_string(),
                produce_payload: json!({}),
            }],
            vec![],
        );

        let result = analyze_complexity(&bundle, &empty_s6());
        assert_eq!(result.predicate_complexities.len(), 1);
        let pc = &result.predicate_complexities[0];
        assert_eq!(pc.source_id, "rule_a");
        assert_eq!(pc.node_count, 3); // compare + left + right
        assert_eq!(pc.max_depth, 2); // compare -> leaf
        assert!(!pc.has_quantifiers);
    }

    #[test]
    fn test_nested_and_predicate() {
        let bundle = make_bundle_with(
            vec![AnalysisRule {
                id: "rule_b".to_string(),
                stratum: 0,
                when: json!({
                    "and": [
                        {"left": {"fact_ref": "a"}, "op": ">", "right": {"literal": 1}},
                        {"left": {"fact_ref": "b"}, "op": "<", "right": {"literal": 2}},
                        {"verdict_present": "approved"}
                    ]
                }),
                produce_verdict_type: "combined".to_string(),
                produce_payload: json!({}),
            }],
            vec![],
        );

        let result = analyze_complexity(&bundle, &empty_s6());
        let pc = &result.predicate_complexities[0];
        // and(1) + compare(3) + compare(3) + verdict_present(1) = 8
        assert_eq!(pc.node_count, 8);
        // and -> compare -> leaf = depth 3
        assert_eq!(pc.max_depth, 3);
        assert!(!pc.has_quantifiers);
    }

    #[test]
    fn test_flow_depth_from_s6() {
        let mut flows = BTreeMap::new();
        flows.insert(
            "main".to_string(),
            FlowPathResult {
                flow_id: "main".to_string(),
                paths: vec![],
                path_count: 0,
                max_depth: 5,
                truncated: false,
                reachable_steps: {
                    let mut s = BTreeSet::new();
                    s.insert("s1".to_string());
                    s.insert("s2".to_string());
                    s.insert("s3".to_string());
                    s
                },
                unreachable_steps: BTreeSet::new(),
            },
        );

        let s6 = S6Result {
            flows,
            total_paths: 0,
        };

        let bundle = make_bundle_with(vec![], vec![]);
        let result = analyze_complexity(&bundle, &s6);

        assert_eq!(result.max_flow_depth, 5);
        let bound = &result.flow_depth_bounds["main"];
        assert_eq!(bound.max_path_depth, 5);
        assert_eq!(bound.max_step_count, 3);
        assert!(!bound.has_cycles);
    }

    #[test]
    fn test_empty_predicates() {
        let bundle = make_bundle_with(vec![], vec![]);
        let result = analyze_complexity(&bundle, &empty_s6());
        assert!(result.predicate_complexities.is_empty());
        assert_eq!(result.max_predicate_depth, 0);
        assert_eq!(result.max_flow_depth, 0);
    }

    #[test]
    fn test_operation_precondition_analyzed() {
        let bundle = make_bundle_with(
            vec![],
            vec![AnalysisOperation {
                id: "op_a".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: Some(json!({"verdict_present": "approved"})),
                effects: vec![],
                outcomes: vec![],
                error_contract: None,
            }],
        );

        let result = analyze_complexity(&bundle, &empty_s6());
        assert_eq!(result.predicate_complexities.len(), 1);
        let pc = &result.predicate_complexities[0];
        assert_eq!(pc.source_id, "op_a");
        assert_eq!(pc.node_count, 1);
        assert_eq!(pc.max_depth, 1);
    }

    #[test]
    fn test_quantifier_detection() {
        let bundle = make_bundle_with(
            vec![AnalysisRule {
                id: "rule_q".to_string(),
                stratum: 0,
                when: json!({"forall": {"left": {"fact_ref": "x"}, "op": "==", "right": {"literal": 1}}}),
                produce_verdict_type: "all_match".to_string(),
                produce_payload: json!({}),
            }],
            vec![],
        );

        let result = analyze_complexity(&bundle, &empty_s6());
        let pc = &result.predicate_complexities[0];
        assert!(pc.has_quantifiers);
        assert_eq!(pc.complexity_class, "O(n)");
    }
}
