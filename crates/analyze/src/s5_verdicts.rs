//! S5 — Verdict and Outcome Space analysis.
//!
//! Enumerates all possible verdict types from rules and all possible
//! outcomes for each Operation.
//!
//! Spec reference: Section 15, S5.

use crate::bundle::AnalysisBundle;
use serde::Serialize;
use std::collections::BTreeMap;

/// Information about a verdict type produced by a rule.
#[derive(Debug, Clone, Serialize)]
pub struct VerdictTypeInfo {
    pub verdict_type: String,
    pub producing_rule: String,
    pub stratum: u64,
}

/// Information about an Operation's declared outcomes.
#[derive(Debug, Clone, Serialize)]
pub struct OperationOutcomeInfo {
    pub operation_id: String,
    pub outcomes: Vec<String>,
    pub outcome_count: usize,
}

/// Aggregated S5 result.
#[derive(Debug, Clone, Serialize)]
pub struct S5Result {
    /// All verdict types produced by rules, sorted by (stratum, verdict_type).
    pub verdict_types: Vec<VerdictTypeInfo>,
    /// operation_id -> outcome info (only operations with declared outcomes).
    pub operation_outcomes: BTreeMap<String, OperationOutcomeInfo>,
    pub total_verdict_types: usize,
    pub total_operations_with_outcomes: usize,
}

/// S5 — Enumerate the verdict and outcome space for the bundle.
pub fn analyze_verdict_space(bundle: &AnalysisBundle) -> S5Result {
    // Collect verdict types from rules
    let mut verdict_types: Vec<VerdictTypeInfo> = bundle
        .rules
        .iter()
        .map(|rule| VerdictTypeInfo {
            verdict_type: rule.produce_verdict_type.clone(),
            producing_rule: rule.id.clone(),
            stratum: rule.stratum,
        })
        .collect();

    // Sort by (stratum, verdict_type) for deterministic output
    verdict_types.sort_by(|a, b| {
        a.stratum
            .cmp(&b.stratum)
            .then_with(|| a.verdict_type.cmp(&b.verdict_type))
    });

    let total_verdict_types = verdict_types.len();

    // Collect operation outcomes
    let mut operation_outcomes = BTreeMap::new();
    for operation in &bundle.operations {
        if !operation.outcomes.is_empty() {
            let info = OperationOutcomeInfo {
                operation_id: operation.id.clone(),
                outcomes: operation.outcomes.clone(),
                outcome_count: operation.outcomes.len(),
            };
            operation_outcomes.insert(operation.id.clone(), info);
        }
    }

    let total_operations_with_outcomes = operation_outcomes.len();

    S5Result {
        verdict_types,
        operation_outcomes,
        total_verdict_types,
        total_operations_with_outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::*;
    use serde_json::json;

    fn make_bundle_with(
        rules: Vec<AnalysisRule>,
        operations: Vec<AnalysisOperation>,
    ) -> AnalysisBundle {
        AnalysisBundle {
            entities: vec![],
            facts: vec![],
            rules,
            operations,
            flows: vec![],
            personas: vec![],
            systems: vec![],
        }
    }

    #[test]
    fn test_verdict_types_enumerated() {
        let bundle = make_bundle_with(
            vec![
                AnalysisRule {
                    id: "rule_a".to_string(),
                    stratum: 0,
                    when: json!({}),
                    produce_verdict_type: "approved".to_string(),
                    produce_payload: json!({}),
                },
                AnalysisRule {
                    id: "rule_b".to_string(),
                    stratum: 1,
                    when: json!({}),
                    produce_verdict_type: "high_value".to_string(),
                    produce_payload: json!({}),
                },
            ],
            vec![],
        );

        let result = analyze_verdict_space(&bundle);
        assert_eq!(result.total_verdict_types, 2);
        assert_eq!(result.verdict_types[0].verdict_type, "approved");
        assert_eq!(result.verdict_types[0].stratum, 0);
        assert_eq!(result.verdict_types[1].verdict_type, "high_value");
        assert_eq!(result.verdict_types[1].stratum, 1);
    }

    #[test]
    fn test_operation_outcomes_enumerated() {
        let bundle = make_bundle_with(
            vec![],
            vec![AnalysisOperation {
                id: "decide".to_string(),
                allowed_personas: vec!["agent".to_string()],
                precondition: None,
                effects: vec![],
                outcomes: vec!["approved".to_string(), "denied".to_string()],
                error_contract: None,
            }],
        );

        let result = analyze_verdict_space(&bundle);
        assert_eq!(result.total_operations_with_outcomes, 1);
        let decide = &result.operation_outcomes["decide"];
        assert_eq!(decide.outcomes, vec!["approved", "denied"]);
        assert_eq!(decide.outcome_count, 2);
    }

    #[test]
    fn test_operation_no_outcomes() {
        let bundle = make_bundle_with(
            vec![],
            vec![AnalysisOperation {
                id: "simple".to_string(),
                allowed_personas: vec!["user".to_string()],
                precondition: None,
                effects: vec![],
                outcomes: vec![], // No outcomes
                error_contract: None,
            }],
        );

        let result = analyze_verdict_space(&bundle);
        assert_eq!(result.total_operations_with_outcomes, 0);
        assert!(result.operation_outcomes.is_empty());
    }

    #[test]
    fn test_multiple_rules_same_stratum() {
        let bundle = make_bundle_with(
            vec![
                AnalysisRule {
                    id: "rule_b".to_string(),
                    stratum: 0,
                    when: json!({}),
                    produce_verdict_type: "zebra".to_string(),
                    produce_payload: json!({}),
                },
                AnalysisRule {
                    id: "rule_a".to_string(),
                    stratum: 0,
                    when: json!({}),
                    produce_verdict_type: "alpha".to_string(),
                    produce_payload: json!({}),
                },
            ],
            vec![],
        );

        let result = analyze_verdict_space(&bundle);
        assert_eq!(result.total_verdict_types, 2);
        // Sorted by (stratum, verdict_type) -- alpha before zebra
        assert_eq!(result.verdict_types[0].verdict_type, "alpha");
        assert_eq!(result.verdict_types[1].verdict_type, "zebra");
    }

    #[test]
    fn test_empty_bundle() {
        let bundle = make_bundle_with(vec![], vec![]);
        let result = analyze_verdict_space(&bundle);
        assert_eq!(result.total_verdict_types, 0);
        assert_eq!(result.total_operations_with_outcomes, 0);
    }
}
