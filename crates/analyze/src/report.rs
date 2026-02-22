//! AnalysisReport — aggregated output from all S1-S8 analyses.
//!
//! The report collects results from each analysis module and extracts
//! notable findings (warnings, info) for summary display.
//!
//! Spec reference: Section 15.

use crate::s1_state_space::S1Result;
use crate::s2_reachability::S2Result;
use crate::s3a_admissibility::S3aResult;
use crate::s4_authority::S4Result;
use crate::s5_verdicts::S5Result;
use crate::s6_flow_paths::S6Result;
use crate::s7_complexity::S7Result;
use crate::s8_verdict_uniqueness::S8Result;
use serde::Serialize;

/// Severity level for an analysis finding.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum FindingSeverity {
    Info,
    Warning,
}

/// A notable finding from analysis.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub analysis: String,
    pub severity: FindingSeverity,
    pub message: String,
    pub entity_id: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// Aggregated analysis report containing all S1-S8 results and findings.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisReport {
    pub s1_state_space: Option<S1Result>,
    pub s2_reachability: Option<S2Result>,
    pub s3a_admissibility: Option<S3aResult>,
    pub s4_authority: Option<S4Result>,
    pub s5_verdicts: Option<S5Result>,
    pub s6_flow_paths: Option<S6Result>,
    pub s7_complexity: Option<S7Result>,
    pub s8_verdict_uniqueness: Option<S8Result>,
    pub analyses_run: Vec<String>,
    pub findings: Vec<Finding>,
}

impl AnalysisReport {
    /// Create a new empty report.
    pub fn new() -> Self {
        AnalysisReport {
            s1_state_space: None,
            s2_reachability: None,
            s3a_admissibility: None,
            s4_authority: None,
            s5_verdicts: None,
            s6_flow_paths: None,
            s7_complexity: None,
            s8_verdict_uniqueness: None,
            analyses_run: Vec::new(),
            findings: Vec::new(),
        }
    }

    /// Extract findings from populated analysis results.
    pub fn extract_findings(&mut self) {
        self.findings.clear();

        // S2: Dead state warnings
        if let Some(ref s2) = self.s2_reachability {
            if s2.has_dead_states {
                for (entity_id, result) in &s2.entities {
                    if !result.unreachable_states.is_empty() {
                        let dead: Vec<String> = result.unreachable_states.iter().cloned().collect();
                        self.findings.push(Finding {
                            analysis: "s2".to_string(),
                            severity: FindingSeverity::Warning,
                            message: format!(
                                "Entity '{}' has {} unreachable state(s): {}",
                                entity_id,
                                dead.len(),
                                dead.join(", ")
                            ),
                            entity_id: Some(entity_id.clone()),
                            details: Some(serde_json::json!({
                                "unreachable_states": dead,
                            })),
                        });
                    }
                }
            }
        }

        // S6: Truncated flow warnings
        if let Some(ref s6) = self.s6_flow_paths {
            for (flow_id, flow_result) in &s6.flows {
                if flow_result.truncated {
                    self.findings.push(Finding {
                        analysis: "s6".to_string(),
                        severity: FindingSeverity::Warning,
                        message: format!(
                            "Flow '{}' path enumeration truncated at {} paths",
                            flow_id, flow_result.path_count
                        ),
                        entity_id: None,
                        details: Some(serde_json::json!({
                            "flow_id": flow_id,
                            "path_count": flow_result.path_count,
                        })),
                    });
                }

                // Unreachable steps
                if !flow_result.unreachable_steps.is_empty() {
                    let unreachable: Vec<String> =
                        flow_result.unreachable_steps.iter().cloned().collect();
                    self.findings.push(Finding {
                        analysis: "s6".to_string(),
                        severity: FindingSeverity::Info,
                        message: format!(
                            "Flow '{}' has {} unreachable step(s): {}",
                            flow_id,
                            unreachable.len(),
                            unreachable.join(", ")
                        ),
                        entity_id: None,
                        details: Some(serde_json::json!({
                            "flow_id": flow_id,
                            "unreachable_steps": unreachable,
                        })),
                    });
                }
            }
        }

        // S4 cross-contract: shared persona authority findings
        if let Some(ref s4) = self.s4_authority {
            if !s4.cross_contract_authorities.is_empty() {
                // Group by (system_id, persona_id) to report authority spread
                let mut persona_contracts: std::collections::BTreeMap<
                    (String, String),
                    Vec<String>,
                > = std::collections::BTreeMap::new();
                for cca in &s4.cross_contract_authorities {
                    persona_contracts
                        .entry((cca.system_id.clone(), cca.persona_id.clone()))
                        .or_default()
                        .push(cca.contract_id.clone());
                }
                for ((system_id, persona_id), contracts) in &persona_contracts {
                    self.findings.push(Finding {
                        analysis: "s4_cross".to_string(),
                        severity: FindingSeverity::Info,
                        message: format!(
                            "Persona '{}' has authority across {} contracts in system '{}': {}",
                            persona_id,
                            contracts.len(),
                            system_id,
                            contracts.join(", ")
                        ),
                        entity_id: None,
                        details: Some(serde_json::json!({
                            "system_id": system_id,
                            "persona_id": persona_id,
                            "contracts": contracts,
                        })),
                    });
                }
            }
        }

        // S6 cross-contract: flow trigger findings
        if let Some(ref s6) = self.s6_flow_paths {
            if !s6.cross_contract_paths.is_empty() {
                for path in &s6.cross_contract_paths {
                    self.findings.push(Finding {
                        analysis: "s6_cross".to_string(),
                        severity: FindingSeverity::Info,
                        message: format!(
                            "Cross-contract flow trigger: {}.{} --[{}]--> {}.{} (persona: {})",
                            path.source_contract,
                            path.source_flow,
                            path.on,
                            path.target_contract,
                            path.target_flow,
                            path.persona,
                        ),
                        entity_id: None,
                        details: Some(serde_json::json!({
                            "system_id": path.system_id,
                            "source_contract": path.source_contract,
                            "source_flow": path.source_flow,
                            "on": path.on,
                            "target_contract": path.target_contract,
                            "target_flow": path.target_flow,
                            "persona": path.persona,
                        })),
                    });
                }

                // Detect trigger cycles: build a graph and check for cycles
                let mut edges: Vec<(String, String)> = Vec::new();
                for path in &s6.cross_contract_paths {
                    let src = format!("{}.{}", path.source_contract, path.source_flow);
                    let tgt = format!("{}.{}", path.target_contract, path.target_flow);
                    edges.push((src, tgt));
                }
                if has_cycle(&edges) {
                    self.findings.push(Finding {
                        analysis: "s6_cross".to_string(),
                        severity: FindingSeverity::Warning,
                        message: "Cross-contract flow triggers form a cycle".to_string(),
                        entity_id: None,
                        details: None,
                    });
                }
            }
        }

        // S7: Deep flow warnings
        if let Some(ref s7) = self.s7_complexity {
            if s7.max_flow_depth > 100 {
                self.findings.push(Finding {
                    analysis: "s7".to_string(),
                    severity: FindingSeverity::Warning,
                    message: format!("Maximum flow depth is {} (exceeds 100)", s7.max_flow_depth),
                    entity_id: None,
                    details: Some(serde_json::json!({
                        "max_flow_depth": s7.max_flow_depth,
                    })),
                });
            }
        }

        // Sort findings for deterministic output
        self.findings.sort_by(|a, b| {
            a.analysis
                .cmp(&b.analysis)
                .then_with(|| format!("{:?}", a.severity).cmp(&format!("{:?}", b.severity)))
                .then_with(|| a.message.cmp(&b.message))
        });
    }
}

impl Default for AnalysisReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a directed graph (given as edge list) contains a cycle.
fn has_cycle(edges: &[(String, String)]) -> bool {
    use std::collections::{BTreeMap, BTreeSet};

    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for (src, tgt) in edges {
        graph.entry(src.as_str()).or_default().push(tgt.as_str());
        nodes.insert(src.as_str());
        nodes.insert(tgt.as_str());
    }

    let mut visited = BTreeSet::new();
    let mut in_path = BTreeSet::new();

    for node in &nodes {
        if !visited.contains(node) && cycle_dfs(node, &graph, &mut visited, &mut in_path) {
            return true;
        }
    }
    false
}

fn cycle_dfs<'a>(
    node: &'a str,
    graph: &std::collections::BTreeMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::BTreeSet<&'a str>,
    in_path: &mut std::collections::BTreeSet<&'a str>,
) -> bool {
    visited.insert(node);
    in_path.insert(node);

    if let Some(neighbors) = graph.get(node) {
        for &neighbor in neighbors {
            if in_path.contains(neighbor)
                || (!visited.contains(neighbor) && cycle_dfs(neighbor, graph, visited, in_path))
            {
                return true;
            }
        }
    }

    in_path.remove(node);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2_reachability::ReachabilityResult;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn test_new_report_all_none() {
        let report = AnalysisReport::new();
        assert!(report.s1_state_space.is_none());
        assert!(report.s2_reachability.is_none());
        assert!(report.s3a_admissibility.is_none());
        assert!(report.s4_authority.is_none());
        assert!(report.s5_verdicts.is_none());
        assert!(report.s6_flow_paths.is_none());
        assert!(report.s7_complexity.is_none());
        assert!(report.s8_verdict_uniqueness.is_none());
        assert!(report.analyses_run.is_empty());
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_extract_findings_dead_states() {
        let mut entities = BTreeMap::new();
        entities.insert(
            "Order".to_string(),
            ReachabilityResult {
                entity_id: "Order".to_string(),
                reachable_states: {
                    let mut s = BTreeSet::new();
                    s.insert("draft".to_string());
                    s
                },
                unreachable_states: {
                    let mut s = BTreeSet::new();
                    s.insert("archived".to_string());
                    s
                },
                initial_state: "draft".to_string(),
            },
        );

        let mut report = AnalysisReport::new();
        report.s2_reachability = Some(S2Result {
            entities,
            has_dead_states: true,
        });

        report.extract_findings();
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].analysis, "s2");
        assert_eq!(report.findings[0].severity, FindingSeverity::Warning);
        assert!(report.findings[0].message.contains("archived"));
        assert_eq!(report.findings[0].entity_id, Some("Order".to_string()));
    }

    #[test]
    fn test_extract_findings_no_issues() {
        let mut entities = BTreeMap::new();
        entities.insert(
            "Order".to_string(),
            ReachabilityResult {
                entity_id: "Order".to_string(),
                reachable_states: {
                    let mut s = BTreeSet::new();
                    s.insert("draft".to_string());
                    s.insert("done".to_string());
                    s
                },
                unreachable_states: BTreeSet::new(),
                initial_state: "draft".to_string(),
            },
        );

        let mut report = AnalysisReport::new();
        report.s2_reachability = Some(S2Result {
            entities,
            has_dead_states: false,
        });

        report.extract_findings();
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_default_trait() {
        let report = AnalysisReport::default();
        assert!(report.analyses_run.is_empty());
    }
}
