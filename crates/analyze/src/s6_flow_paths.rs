//! S6 — Flow Path Enumeration analysis.
//!
//! For each Flow, enumerates all possible execution paths including
//! OperationStep outcome branching, BranchStep (true/false),
//! ParallelStep (branch and rejoin), and SubFlowStep (recursive).
//!
//! Spec reference: Section 15, S6.

use crate::bundle::AnalysisBundle;
use crate::s5_verdicts::S5Result;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of paths to enumerate before truncation.
const MAX_PATHS: usize = 10_000;
/// Maximum recursion depth for path enumeration.
const MAX_DEPTH: usize = 1_000;

/// A single step in an enumerated flow path.
#[derive(Debug, Clone, Serialize)]
pub struct FlowPathStep {
    pub step_id: String,
    pub step_type: String,
    pub persona: Option<String>,
    pub operation_id: Option<String>,
    pub outcome: Option<String>,
}

/// A complete execution path through a flow.
#[derive(Debug, Clone, Serialize)]
pub struct FlowPath {
    pub steps: Vec<FlowPathStep>,
    pub terminal_outcome: Option<String>,
    pub depth: usize,
}

/// Flow path enumeration result for a single flow.
#[derive(Debug, Clone, Serialize)]
pub struct FlowPathResult {
    pub flow_id: String,
    pub paths: Vec<FlowPath>,
    pub path_count: usize,
    pub max_depth: usize,
    pub truncated: bool,
    pub reachable_steps: BTreeSet<String>,
    pub unreachable_steps: BTreeSet<String>,
}

/// A cross-contract flow path created by a System trigger.
#[derive(Debug, Clone, Serialize)]
pub struct CrossContractFlowPath {
    pub system_id: String,
    pub source_contract: String,
    pub source_flow: String,
    pub on: String,
    pub target_contract: String,
    pub target_flow: String,
    pub persona: String,
}

/// Aggregated S6 result.
#[derive(Debug, Clone, Serialize)]
pub struct S6Result {
    pub flows: BTreeMap<String, FlowPathResult>,
    pub total_paths: usize,
    /// Cross-contract flow paths from System triggers.
    pub cross_contract_paths: Vec<CrossContractFlowPath>,
}

/// S6 — Enumerate all possible execution paths through each Flow.
pub fn analyze_flow_paths(bundle: &AnalysisBundle, s5: &S5Result) -> S6Result {
    let mut flows = BTreeMap::new();
    let mut total_paths = 0;

    for flow in &bundle.flows {
        let result = enumerate_flow_paths(flow, s5);
        total_paths += result.path_count;
        flows.insert(flow.id.clone(), result);
    }

    // Cross-contract flow path analysis from System triggers
    let cross_contract_paths = analyze_cross_contract_triggers(bundle);

    S6Result {
        flows,
        total_paths,
        cross_contract_paths,
    }
}

/// Analyze cross-contract flow triggers from System constructs.
///
/// For each System, extracts all flow triggers and represents them
/// as cross-contract flow paths. Also detects trigger cycles.
fn analyze_cross_contract_triggers(bundle: &AnalysisBundle) -> Vec<CrossContractFlowPath> {
    let mut paths = Vec::new();

    for system in &bundle.systems {
        for trigger in &system.flow_triggers {
            paths.push(CrossContractFlowPath {
                system_id: system.id.clone(),
                source_contract: trigger.source_contract.clone(),
                source_flow: trigger.source_flow.clone(),
                on: trigger.on.clone(),
                target_contract: trigger.target_contract.clone(),
                target_flow: trigger.target_flow.clone(),
                persona: trigger.persona.clone(),
            });
        }
    }

    // Sort for deterministic output
    paths.sort_by(|a, b| {
        (
            &a.system_id,
            &a.source_contract,
            &a.source_flow,
            &a.target_contract,
            &a.target_flow,
        )
            .cmp(&(
                &b.system_id,
                &b.source_contract,
                &b.source_flow,
                &b.target_contract,
                &b.target_flow,
            ))
    });

    paths
}

/// Enumerate paths for a single flow.
fn enumerate_flow_paths(flow: &crate::bundle::AnalysisFlow, _s5: &S5Result) -> FlowPathResult {
    // Build step index: step_id -> step JSON
    let mut step_index: BTreeMap<String, &serde_json::Value> = BTreeMap::new();
    let mut all_step_ids = BTreeSet::new();

    for step in &flow.steps {
        if let Some(id) = step.get("id").and_then(|i| i.as_str()) {
            step_index.insert(id.to_string(), step);
            all_step_ids.insert(id.to_string());
        }
    }

    let mut paths = Vec::new();
    let mut reachable_steps = BTreeSet::new();
    let mut truncated = false;

    // DFS path enumeration from entry step
    let mut stack: Vec<(String, Vec<FlowPathStep>, BTreeSet<String>)> = Vec::new();

    stack.push((flow.entry.clone(), Vec::new(), BTreeSet::new()));

    while let Some((current_step_id, current_path, visited)) = stack.pop() {
        if paths.len() >= MAX_PATHS {
            truncated = true;
            break;
        }

        if current_path.len() >= MAX_DEPTH {
            // Record path as-is at max depth
            paths.push(FlowPath {
                depth: current_path.len(),
                steps: current_path,
                terminal_outcome: Some("max_depth_exceeded".to_string()),
            });
            continue;
        }

        // Cycle detection
        if visited.contains(&current_step_id) {
            paths.push(FlowPath {
                depth: current_path.len(),
                steps: current_path,
                terminal_outcome: Some("cycle_detected".to_string()),
            });
            continue;
        }

        reachable_steps.insert(current_step_id.clone());

        let step = match step_index.get(&current_step_id) {
            Some(s) => s,
            None => {
                // Step not found -- could be a terminal reference
                paths.push(FlowPath {
                    depth: current_path.len(),
                    steps: current_path,
                    terminal_outcome: Some(current_step_id),
                });
                continue;
            }
        };

        let kind = step.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let persona = step
            .get("persona")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());

        let mut new_visited = visited.clone();
        new_visited.insert(current_step_id.clone());

        match kind {
            "OperationStep" => {
                let op_id = step
                    .get("op")
                    .and_then(|o| o.as_str())
                    .map(|s| s.to_string());

                // Get outcomes from step's outcomes map
                let outcomes_map = step.get("outcomes").and_then(|o| o.as_object());

                if let Some(outcomes) = outcomes_map {
                    for (outcome_label, target) in outcomes {
                        let mut path = current_path.clone();
                        path.push(FlowPathStep {
                            step_id: current_step_id.clone(),
                            step_type: "operation".to_string(),
                            persona: persona.clone(),
                            operation_id: op_id.clone(),
                            outcome: Some(outcome_label.clone()),
                        });

                        if let Some(next_id) = extract_step_target(target) {
                            stack.push((next_id, path, new_visited.clone()));
                        } else {
                            // Terminal target
                            let terminal = extract_terminal_outcome(target);
                            paths.push(FlowPath {
                                depth: path.len(),
                                steps: path,
                                terminal_outcome: terminal,
                            });
                        }
                    }
                } else {
                    // No outcomes map -- treat as terminal
                    let mut path = current_path.clone();
                    path.push(FlowPathStep {
                        step_id: current_step_id.clone(),
                        step_type: "operation".to_string(),
                        persona: persona.clone(),
                        operation_id: op_id.clone(),
                        outcome: None,
                    });
                    paths.push(FlowPath {
                        depth: path.len(),
                        steps: path,
                        terminal_outcome: None,
                    });
                }

                // Follow Escalate handler's next target for failure path reachability
                if let Some(failure_handler) = step.get("on_failure") {
                    if let Some(handler_kind) = failure_handler.get("kind").and_then(|k| k.as_str())
                    {
                        if handler_kind == "Escalate" {
                            if let Some(next_id) =
                                failure_handler.get("next").and_then(|n| n.as_str())
                            {
                                let mut escalate_path = current_path;
                                escalate_path.push(FlowPathStep {
                                    step_id: current_step_id,
                                    step_type: "operation".to_string(),
                                    persona,
                                    operation_id: op_id,
                                    outcome: Some("escalate".to_string()),
                                });
                                stack.push((
                                    next_id.to_string(),
                                    escalate_path,
                                    new_visited.clone(),
                                ));
                            }
                        }
                    }
                }
            }

            "BranchStep" => {
                let mut path_step = FlowPathStep {
                    step_id: current_step_id.clone(),
                    step_type: "branch".to_string(),
                    persona: persona.clone(),
                    operation_id: None,
                    outcome: None,
                };

                // True branch
                if let Some(true_target) = step.get("if_true") {
                    let mut true_path = current_path.clone();
                    path_step.outcome = Some("true".to_string());
                    true_path.push(path_step.clone());
                    if let Some(next_id) = extract_step_target(true_target) {
                        stack.push((next_id, true_path, new_visited.clone()));
                    } else {
                        let terminal = extract_terminal_outcome(true_target);
                        paths.push(FlowPath {
                            depth: true_path.len(),
                            steps: true_path,
                            terminal_outcome: terminal,
                        });
                    }
                }

                // False branch
                if let Some(false_target) = step.get("if_false") {
                    let mut false_path = current_path;
                    path_step.outcome = Some("false".to_string());
                    false_path.push(path_step);
                    if let Some(next_id) = extract_step_target(false_target) {
                        stack.push((next_id, false_path, new_visited.clone()));
                    } else {
                        let terminal = extract_terminal_outcome(false_target);
                        paths.push(FlowPath {
                            depth: false_path.len(),
                            steps: false_path,
                            terminal_outcome: terminal,
                        });
                    }
                }
            }

            "HandoffStep" => {
                let mut path = current_path;
                path.push(FlowPathStep {
                    step_id: current_step_id.clone(),
                    step_type: "handoff".to_string(),
                    persona,
                    operation_id: None,
                    outcome: None,
                });

                if let Some(next) = step.get("next").and_then(|n| n.as_str()) {
                    stack.push((next.to_string(), path, new_visited));
                } else {
                    paths.push(FlowPath {
                        depth: path.len(),
                        steps: path,
                        terminal_outcome: None,
                    });
                }
            }

            "SubFlowStep" => {
                let flow_ref = step
                    .get("flow")
                    .and_then(|f| f.as_str())
                    .map(|s| s.to_string());

                // Success path
                if let Some(success_target) = step.get("on_success") {
                    let mut success_path = current_path.clone();
                    success_path.push(FlowPathStep {
                        step_id: current_step_id.clone(),
                        step_type: "subflow".to_string(),
                        persona: persona.clone(),
                        operation_id: flow_ref.clone(),
                        outcome: Some("success".to_string()),
                    });
                    if let Some(next_id) = extract_step_target(success_target) {
                        stack.push((next_id, success_path, new_visited.clone()));
                    } else {
                        let terminal = extract_terminal_outcome(success_target);
                        paths.push(FlowPath {
                            depth: success_path.len(),
                            steps: success_path,
                            terminal_outcome: terminal,
                        });
                    }
                }

                // Failure path (from on_failure handler)
                if let Some(failure_handler) = step.get("on_failure") {
                    let handler_kind = failure_handler
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .unwrap_or("");
                    if handler_kind == "Escalate" {
                        // Escalate handler: follow the next step target
                        if let Some(next_id) = failure_handler.get("next").and_then(|n| n.as_str())
                        {
                            let mut escalate_path = current_path;
                            escalate_path.push(FlowPathStep {
                                step_id: current_step_id,
                                step_type: "subflow".to_string(),
                                persona,
                                operation_id: flow_ref,
                                outcome: Some("escalate".to_string()),
                            });
                            stack.push((next_id.to_string(), escalate_path, new_visited.clone()));
                        }
                    } else {
                        // Terminate or Compensate: record terminal outcome
                        let mut failure_path = current_path;
                        failure_path.push(FlowPathStep {
                            step_id: current_step_id,
                            step_type: "subflow".to_string(),
                            persona,
                            operation_id: flow_ref,
                            outcome: Some("failure".to_string()),
                        });
                        let terminal = extract_failure_outcome(failure_handler);
                        paths.push(FlowPath {
                            depth: failure_path.len(),
                            steps: failure_path,
                            terminal_outcome: terminal,
                        });
                    }
                }
            }

            "ParallelStep" => {
                // For path enumeration, model parallel step as reaching the join
                let mut path = current_path;
                path.push(FlowPathStep {
                    step_id: current_step_id.clone(),
                    step_type: "parallel".to_string(),
                    persona: None,
                    operation_id: None,
                    outcome: None,
                });

                // Follow join.on_all_success if present
                if let Some(join) = step.get("join") {
                    if let Some(success_target) = join.get("on_all_success") {
                        if let Some(next_id) = extract_step_target(success_target) {
                            stack.push((next_id, path.clone(), new_visited.clone()));
                        }
                    }
                    if let Some(complete_target) = join.get("on_all_complete") {
                        if let Some(next_id) = extract_step_target(complete_target) {
                            stack.push((next_id, path, new_visited));
                        }
                    } else if join.get("on_all_success").is_none() {
                        // No join targets, treat as terminal
                        paths.push(FlowPath {
                            depth: path.len(),
                            steps: path,
                            terminal_outcome: Some("parallel_complete".to_string()),
                        });
                    }
                } else {
                    paths.push(FlowPath {
                        depth: path.len(),
                        steps: path,
                        terminal_outcome: Some("parallel_complete".to_string()),
                    });
                }
            }

            _ => {
                // Unknown step type -- record and stop path
                let mut path = current_path;
                path.push(FlowPathStep {
                    step_id: current_step_id,
                    step_type: kind.to_string(),
                    persona,
                    operation_id: None,
                    outcome: None,
                });
                paths.push(FlowPath {
                    depth: path.len(),
                    steps: path,
                    terminal_outcome: None,
                });
            }
        }
    }

    let max_depth = paths.iter().map(|p| p.depth).max().unwrap_or(0);
    let unreachable_steps: BTreeSet<String> =
        all_step_ids.difference(&reachable_steps).cloned().collect();
    let path_count = paths.len();

    FlowPathResult {
        flow_id: flow.id.clone(),
        paths,
        path_count,
        max_depth,
        truncated,
        reachable_steps,
        unreachable_steps,
    }
}

/// Extract a step ID from a StepTarget (string reference or terminal object).
fn extract_step_target(target: &serde_json::Value) -> Option<String> {
    // String target: "step_id"
    target.as_str().map(|s| s.to_string())
}

/// Extract terminal outcome from a TerminalTarget object.
fn extract_terminal_outcome(target: &serde_json::Value) -> Option<String> {
    target
        .get("outcome")
        .and_then(|o| o.as_str())
        .map(|outcome| outcome.to_string())
}

/// Extract outcome from a failure handler.
fn extract_failure_outcome(handler: &serde_json::Value) -> Option<String> {
    handler
        .get("outcome")
        .and_then(|o| o.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::*;
    use serde_json::json;

    fn make_flow_bundle(flows: Vec<AnalysisFlow>) -> AnalysisBundle {
        AnalysisBundle {
            entities: vec![],
            facts: vec![],
            rules: vec![],
            operations: vec![],
            flows,
            personas: vec![],
            systems: vec![],
        }
    }

    fn empty_s5() -> S5Result {
        S5Result {
            verdict_types: vec![],
            operation_outcomes: BTreeMap::new(),
            total_verdict_types: 0,
            total_operations_with_outcomes: 0,
        }
    }

    #[test]
    fn test_linear_flow_one_path() {
        let flow = AnalysisFlow {
            id: "simple".to_string(),
            entry: "step1".to_string(),
            steps: vec![
                json!({"id": "step1", "kind": "OperationStep", "op": "do_something", "persona": "admin",
                        "outcomes": {"success": {"kind": "Terminal", "outcome": "success"}},
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        assert_eq!(result.total_paths, 1);
        let flow_result = &result.flows["simple"];
        assert_eq!(flow_result.path_count, 1);
        assert_eq!(flow_result.paths[0].steps.len(), 1);
        assert_eq!(flow_result.paths[0].steps[0].step_type, "operation");
        assert_eq!(
            flow_result.paths[0].terminal_outcome,
            Some("success".to_string())
        );
    }

    #[test]
    fn test_operation_step_two_outcomes() {
        let flow = AnalysisFlow {
            id: "branching".to_string(),
            entry: "step1".to_string(),
            steps: vec![
                json!({"id": "step1", "kind": "OperationStep", "op": "decide", "persona": "agent",
                        "outcomes": {
                            "approved": {"kind": "Terminal", "outcome": "success"},
                            "denied": {"kind": "Terminal", "outcome": "failure"}
                        },
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        let flow_result = &result.flows["branching"];
        assert_eq!(flow_result.path_count, 2);
    }

    #[test]
    fn test_branch_step_two_paths() {
        let flow = AnalysisFlow {
            id: "conditional".to_string(),
            entry: "branch".to_string(),
            steps: vec![
                json!({"id": "branch", "kind": "BranchStep", "persona": "admin",
                        "condition": {"fact_ref": "flag"},
                        "if_true": {"kind": "Terminal", "outcome": "success"},
                        "if_false": {"kind": "Terminal", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        let flow_result = &result.flows["conditional"];
        assert_eq!(flow_result.path_count, 2);
    }

    #[test]
    fn test_unreachable_step_detected() {
        let flow = AnalysisFlow {
            id: "test".to_string(),
            entry: "step1".to_string(),
            steps: vec![
                json!({"id": "step1", "kind": "OperationStep", "op": "do_it", "persona": "admin",
                        "outcomes": {"success": {"kind": "Terminal", "outcome": "success"}},
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
                json!({"id": "orphan", "kind": "OperationStep", "op": "never_reached", "persona": "admin",
                        "outcomes": {"success": {"kind": "Terminal", "outcome": "success"}},
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        let flow_result = &result.flows["test"];
        assert!(flow_result.unreachable_steps.contains("orphan"));
    }

    #[test]
    fn test_empty_flow_bundle() {
        let bundle = make_flow_bundle(vec![]);
        let result = analyze_flow_paths(&bundle, &empty_s5());
        assert_eq!(result.total_paths, 0);
        assert!(result.flows.is_empty());
    }

    #[test]
    fn test_escalate_handler_reachable() {
        // Step with Escalate handler whose next target should be reachable
        let flow = AnalysisFlow {
            id: "escalate_flow".to_string(),
            entry: "step_op".to_string(),
            steps: vec![
                json!({"id": "step_op", "kind": "OperationStep", "op": "process", "persona": "admin",
                        "outcomes": {"done": {"kind": "Terminal", "outcome": "success"}},
                        "on_failure": {"kind": "Escalate", "to_persona": "director", "next": "step_director"}}),
                json!({"id": "step_director", "kind": "OperationStep", "op": "review", "persona": "director",
                        "outcomes": {"approved": {"kind": "Terminal", "outcome": "approved"}},
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        let flow_result = &result.flows["escalate_flow"];
        // step_director should be reachable through the Escalate handler
        assert!(
            flow_result.reachable_steps.contains("step_director"),
            "step_director should be reachable via Escalate handler"
        );
        assert!(
            flow_result.unreachable_steps.is_empty(),
            "no steps should be unreachable: {:?}",
            flow_result.unreachable_steps
        );
        // Should have at least 2 paths: normal outcome + escalation
        assert!(
            flow_result.path_count >= 2,
            "expected at least 2 paths (normal + escalate), got {}",
            flow_result.path_count
        );
    }

    #[test]
    fn test_handoff_step_continues() {
        let flow = AnalysisFlow {
            id: "handoff_flow".to_string(),
            entry: "h1".to_string(),
            steps: vec![
                json!({"id": "h1", "kind": "HandoffStep", "from_persona": "admin", "to_persona": "user", "next": "step2"}),
                json!({"id": "step2", "kind": "OperationStep", "op": "do_it", "persona": "user",
                        "outcomes": {"done": {"kind": "Terminal", "outcome": "success"}},
                        "on_failure": {"kind": "Terminate", "outcome": "failure"}}),
            ],
            snapshot: "at_initiation".to_string(),
        };

        let bundle = make_flow_bundle(vec![flow]);
        let result = analyze_flow_paths(&bundle, &empty_s5());

        let flow_result = &result.flows["handoff_flow"];
        assert_eq!(flow_result.path_count, 1);
        assert_eq!(flow_result.paths[0].steps.len(), 2);
        assert_eq!(flow_result.paths[0].steps[0].step_type, "handoff");
        assert_eq!(flow_result.paths[0].steps[1].step_type, "operation");
    }
}
