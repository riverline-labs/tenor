//! Tenor static analyzer -- S1-S8 analysis suite with structured output.
//!
//! The analyzer consumes interchange JSON (same pattern as tenor-eval),
//! not the raw DSL AST. Each analysis is a separate module producing
//! a serializable result struct. The `analyze()` function orchestrates
//! all analyses and aggregates results into an `AnalysisReport`.
//!
//! Implementation: Phase 4.

pub mod bundle;
pub mod report;
pub mod s1_state_space;
pub mod s2_reachability;
pub mod s3a_admissibility;
pub mod s4_authority;
pub mod s5_verdicts;
pub mod s6_flow_paths;
pub mod s7_complexity;
pub mod s8_verdict_uniqueness;

pub use bundle::{
    AnalysisBundle, AnalysisError, AnalysisSystem, FlowTrigger, SharedEntity, SharedPersona,
    SystemMember,
};
pub use report::{AnalysisReport, Finding, FindingSeverity};
pub use s1_state_space::{S1Result, StateSpaceResult};
pub use s2_reachability::{ReachabilityResult, S2Result};
pub use s3a_admissibility::{AdmissibilityKey, S3aResult};
pub use s4_authority::{AuthorityMap, CrossContractAuthority, S4Result, TransitionAuthority};
pub use s5_verdicts::{S5Result, VerdictTypeInfo};
pub use s6_flow_paths::{CrossContractFlowPath, FlowPathResult, S6Result};
pub use s7_complexity::{FlowDepthBound, PredicateComplexity, S7Result};
pub use s8_verdict_uniqueness::S8Result;

/// Run the full S1-S8 analysis suite on an interchange JSON bundle.
///
/// Deserializes the bundle, runs all analyses in dependency order,
/// extracts findings, and returns the aggregated report.
pub fn analyze(bundle: &serde_json::Value) -> Result<AnalysisReport, AnalysisError> {
    let analysis_bundle = AnalysisBundle::from_interchange(bundle)?;

    let s1 = s1_state_space::analyze_state_space(&analysis_bundle);
    let s2 = s2_reachability::analyze_reachability(&analysis_bundle);
    let s3a = s3a_admissibility::analyze_admissibility(&analysis_bundle);
    let s4 = s4_authority::analyze_authority(&analysis_bundle, &s3a);
    let s5 = s5_verdicts::analyze_verdict_space(&analysis_bundle);
    let s6 = s6_flow_paths::analyze_flow_paths(&analysis_bundle, &s5);
    let s7 = s7_complexity::analyze_complexity(&analysis_bundle, &s6);
    let s8 = s8_verdict_uniqueness::confirm_verdict_uniqueness();

    let mut report = AnalysisReport::new();
    report.s1_state_space = Some(s1);
    report.s2_reachability = Some(s2);
    report.s3a_admissibility = Some(s3a);
    report.s4_authority = Some(s4);
    report.s5_verdicts = Some(s5);
    report.s6_flow_paths = Some(s6);
    report.s7_complexity = Some(s7);
    report.s8_verdict_uniqueness = Some(s8);
    report.analyses_run = vec![
        "s1".to_string(),
        "s2".to_string(),
        "s3a".to_string(),
        "s4".to_string(),
        "s5".to_string(),
        "s6".to_string(),
        "s7".to_string(),
        "s8".to_string(),
    ];

    report.extract_findings();

    Ok(report)
}

/// Run selected analyses on an interchange JSON bundle.
///
/// Only runs the requested analyses (and their dependencies).
/// Valid analysis names: "s1", "s2", "s3a", "s4", "s5", "s6", "s7", "s8".
pub fn analyze_selected(
    bundle: &serde_json::Value,
    analyses: &[&str],
) -> Result<AnalysisReport, AnalysisError> {
    let analysis_bundle = AnalysisBundle::from_interchange(bundle)?;

    // Resolve dependencies: s4 needs s3a, s6 needs s5, s7 needs s6 (needs s5)
    let mut needed: std::collections::BTreeSet<&str> = analyses.iter().copied().collect();

    if needed.contains("s4") {
        needed.insert("s3a");
    }
    if needed.contains("s7") {
        needed.insert("s6");
    }
    if needed.contains("s6") {
        needed.insert("s5");
    }

    let mut report = AnalysisReport::new();

    // Run analyses in dependency order, only if needed
    let s1 = if needed.contains("s1") {
        let result = s1_state_space::analyze_state_space(&analysis_bundle);
        report.analyses_run.push("s1".to_string());
        Some(result)
    } else {
        None
    };
    report.s1_state_space = s1;

    let s2 = if needed.contains("s2") {
        let result = s2_reachability::analyze_reachability(&analysis_bundle);
        report.analyses_run.push("s2".to_string());
        Some(result)
    } else {
        None
    };
    report.s2_reachability = s2;

    let s3a = if needed.contains("s3a") {
        let result = s3a_admissibility::analyze_admissibility(&analysis_bundle);
        report.analyses_run.push("s3a".to_string());
        Some(result)
    } else {
        None
    };
    report.s3a_admissibility = s3a.clone();

    if needed.contains("s4") {
        if let Some(ref s3a_result) = s3a {
            let result = s4_authority::analyze_authority(&analysis_bundle, s3a_result);
            report.analyses_run.push("s4".to_string());
            report.s4_authority = Some(result);
        }
    }

    let s5 = if needed.contains("s5") {
        let result = s5_verdicts::analyze_verdict_space(&analysis_bundle);
        report.analyses_run.push("s5".to_string());
        Some(result)
    } else {
        None
    };
    report.s5_verdicts = s5.clone();

    let s6 = if needed.contains("s6") {
        if let Some(ref s5_result) = s5 {
            let result = s6_flow_paths::analyze_flow_paths(&analysis_bundle, s5_result);
            report.analyses_run.push("s6".to_string());
            Some(result)
        } else {
            None
        }
    } else {
        None
    };
    report.s6_flow_paths = s6.clone();

    if needed.contains("s7") {
        if let Some(ref s6_result) = s6 {
            let result = s7_complexity::analyze_complexity(&analysis_bundle, s6_result);
            report.analyses_run.push("s7".to_string());
            report.s7_complexity = Some(result);
        }
    }

    if needed.contains("s8") {
        let result = s8_verdict_uniqueness::confirm_verdict_uniqueness();
        report.analyses_run.push("s8".to_string());
        report.s8_verdict_uniqueness = Some(result);
    }

    report.extract_findings();

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_test_bundle() -> serde_json::Value {
        json!({
            "id": "test_bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [
                {
                    "id": "Order",
                    "kind": "Entity",
                    "initial": "draft",
                    "states": ["draft", "submitted", "approved"],
                    "transitions": [
                        {"from": "draft", "to": "submitted"},
                        {"from": "submitted", "to": "approved"}
                    ],
                    "provenance": {"file": "test.tenor", "line": 1},
                    "tenor": "1.0"
                },
                {
                    "id": "admin",
                    "kind": "Persona",
                    "provenance": {"file": "test.tenor", "line": 10},
                    "tenor": "1.0"
                },
                {
                    "id": "submit_order",
                    "kind": "Operation",
                    "allowed_personas": ["admin"],
                    "precondition": null,
                    "effects": [
                        {"entity_id": "Order", "from": "draft", "to": "submitted"}
                    ],
                    "outcomes": null,
                    "error_contract": [],
                    "provenance": {"file": "test.tenor", "line": 15},
                    "tenor": "1.0"
                },
                {
                    "id": "check_value",
                    "kind": "Rule",
                    "stratum": 0,
                    "body": {
                        "when": {
                            "left": {"fact_ref": "amount"},
                            "op": ">",
                            "right": {"literal": 100, "type": {"base": "Int"}}
                        },
                        "produce": {
                            "verdict_type": "high_value",
                            "payload": {"type": {"base": "Bool"}, "value": true}
                        }
                    },
                    "provenance": {"file": "test.tenor", "line": 20},
                    "tenor": "1.0"
                }
            ]
        })
    }

    #[test]
    fn test_full_analyze() {
        let bundle = make_test_bundle();
        let report = analyze(&bundle).unwrap();

        assert!(report.s1_state_space.is_some());
        assert!(report.s2_reachability.is_some());
        assert!(report.s3a_admissibility.is_some());
        assert!(report.s4_authority.is_some());
        assert!(report.s5_verdicts.is_some());
        assert!(report.s6_flow_paths.is_some());
        assert!(report.s7_complexity.is_some());
        assert!(report.s8_verdict_uniqueness.is_some());
        assert_eq!(report.analyses_run.len(), 8);
    }

    #[test]
    fn test_analyze_selected_s1_only() {
        let bundle = make_test_bundle();
        let report = analyze_selected(&bundle, &["s1"]).unwrap();

        assert!(report.s1_state_space.is_some());
        assert!(report.s2_reachability.is_none());
        assert!(report.s3a_admissibility.is_none());
        assert_eq!(report.analyses_run, vec!["s1"]);
    }

    #[test]
    fn test_analyze_selected_s4_pulls_s3a() {
        let bundle = make_test_bundle();
        let report = analyze_selected(&bundle, &["s4"]).unwrap();

        assert!(report.s3a_admissibility.is_some());
        assert!(report.s4_authority.is_some());
        assert!(report.analyses_run.contains(&"s3a".to_string()));
        assert!(report.analyses_run.contains(&"s4".to_string()));
    }

    #[test]
    fn test_analyze_selected_s7_pulls_s6_and_s5() {
        let bundle = make_test_bundle();
        let report = analyze_selected(&bundle, &["s7"]).unwrap();

        assert!(report.s5_verdicts.is_some());
        assert!(report.s6_flow_paths.is_some());
        assert!(report.s7_complexity.is_some());
        assert!(report.analyses_run.contains(&"s5".to_string()));
        assert!(report.analyses_run.contains(&"s6".to_string()));
        assert!(report.analyses_run.contains(&"s7".to_string()));
    }

    #[test]
    fn test_analyze_report_serializable() {
        let bundle = make_test_bundle();
        let report = analyze(&bundle).unwrap();
        let json = serde_json::to_value(&report).unwrap();
        assert!(json.is_object());
        assert!(json.get("analyses_run").unwrap().is_array());
    }
}
