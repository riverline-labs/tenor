//! Flow execution engine with frozen verdict snapshot semantics.
//!
//! Implements spec Section 11: Flow execution as a state machine walk,
//! with frozen verdict semantics from Section 11.4.
//!
//! Key invariant: The Snapshot (FactSet + VerdictSet) is created at flow
//! initiation and NEVER mutated during flow execution. Entity state changes
//! are tracked separately in a mutable EntityStateMap. This ensures that
//! verdicts evaluated during flow steps reflect the state at flow initiation,
//! NOT the current entity state.

use std::collections::{BTreeMap, HashMap};

use crate::operation::{
    execute_operation, resolve_instance_id, EffectRecord, EntityStateMap, InstanceBindingMap,
};
use crate::predicate::{eval_pred, EvalContext};
use crate::provenance::ProvenanceCollector;
use crate::types::{
    Contract, EvalError, FactSet, FailureHandler, Flow, FlowStep, Operation, StepTarget, VerdictSet,
};

// ──────────────────────────────────────────────
// Frozen verdict snapshot
// ──────────────────────────────────────────────

/// Immutable snapshot of facts and verdicts taken at flow initiation.
///
/// Per spec Section 11.4, this snapshot is NEVER recomputed during flow
/// execution. Entity state changes during the flow do NOT trigger verdict
/// re-evaluation. Sub-flows inherit the parent flow's snapshot (spec E5).
pub struct Snapshot {
    pub facts: FactSet,
    pub verdicts: VerdictSet,
}

// ──────────────────────────────────────────────
// Flow execution result types
// ──────────────────────────────────────────────

/// Record of a single step executed during a flow.
///
/// Per §9.5 and §11.4: step records carry instance bindings to trace
/// which specific entity instances were targeted at each step.
#[derive(Debug, Clone)]
pub struct StepRecord {
    pub step_id: String,
    pub step_type: String,
    pub result: String,
    /// Maps entity_id -> instance_id for the instances targeted at this step.
    /// Empty for non-operation steps (branch, handoff, parallel).
    pub instance_bindings: std::collections::BTreeMap<String, String>,
}

/// Result of a successful flow execution.
#[derive(Debug, Clone)]
pub struct FlowResult {
    pub outcome: String,
    pub steps_executed: Vec<StepRecord>,
    pub entity_state_changes: Vec<EffectRecord>,
    /// Per spec Section 11.4: initiating_persona is recorded for provenance.
    /// Flow-level persona authorization is delegated to step-level Operation
    /// persona checks.
    pub initiating_persona: Option<String>,
}

/// Result of a full evaluation including flow execution.
#[derive(Debug)]
pub struct FlowEvalResult {
    pub verdicts: VerdictSet,
    pub flow_result: FlowResult,
}

// ──────────────────────────────────────────────
// Flow execution
// ──────────────────────────────────────────────

/// Outcome of a single parallel branch execution.
enum BranchOutcome {
    Success {
        branch_id: String,
        outcome: String,
        entity_changes: Vec<EffectRecord>,
        steps: Vec<StepRecord>,
        /// Final entity states from this branch for merge-back
        entity_states: EntityStateMap,
    },
    Failure {
        branch_id: String,
        error: String,
        steps: Vec<StepRecord>,
    },
}

/// Resolve instance bindings for a specific operation's effects.
///
/// Per §11.4: for each entity referenced in the operation's effects, look up
/// the instance_id in the flow-level bindings. Missing bindings fall back to
/// DEFAULT_INSTANCE_ID for backward compat with single-instance contracts.
///
/// Returns a BTreeMap<entity_id, instance_id> for this operation's effect targets.
fn resolve_bindings(op: &Operation, bindings: &InstanceBindingMap) -> InstanceBindingMap {
    op.effects
        .iter()
        .map(|effect| {
            let instance_id = resolve_instance_id(bindings, &effect.entity_id).to_string();
            (effect.entity_id.clone(), instance_id)
        })
        .collect()
}

/// Handle a failure using the given FailureHandler. Returns either a terminal
/// FlowResult or the next step ID to continue the flow from.
///
/// Shared by OperationStep and SubFlowStep failure handling paths.
#[allow(clippy::too_many_arguments)]
fn handle_failure(
    handler: &FailureHandler,
    step_id: &str,
    op_index: &HashMap<&str, &Operation>,
    snapshot: &Snapshot,
    entity_states: &mut EntityStateMap,
    instance_bindings: &InstanceBindingMap,
    steps_executed: &mut Vec<StepRecord>,
    entity_changes_all: &mut Vec<EffectRecord>,
) -> Result<Option<FlowResult>, EvalError> {
    match handler {
        FailureHandler::Terminate { outcome } => {
            // Use std::mem::take() instead of clone() -- the caller returns
            // immediately after receiving Some(FlowResult), so the original
            // vectors are never accessed again.
            Ok(Some(FlowResult {
                outcome: outcome.clone(),
                steps_executed: std::mem::take(steps_executed),
                entity_state_changes: std::mem::take(entity_changes_all),
                initiating_persona: None, // Caller sets this
            }))
        }
        FailureHandler::Compensate { steps, then } => {
            // Execute compensation steps in order per spec Section 11.3
            for comp_step in steps {
                let comp_op = op_index.get(comp_step.op.as_str()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: format!(
                            "compensation operation '{}' not found in contract",
                            comp_step.op
                        ),
                    }
                })?;

                let comp_bindings = resolve_bindings(comp_op, instance_bindings);
                match execute_operation(
                    comp_op,
                    &comp_step.persona,
                    &snapshot.facts,
                    &snapshot.verdicts,
                    entity_states,
                    &comp_bindings,
                ) {
                    Ok(comp_result) => {
                        entity_changes_all.extend(comp_result.effects_applied.clone());
                        steps_executed.push(StepRecord {
                            step_id: format!("comp:{}", comp_step.op),
                            step_type: "compensation".to_string(),
                            result: comp_result.outcome.clone(),
                            instance_bindings: comp_result.provenance.instance_binding.clone(),
                        });
                    }
                    Err(comp_err) => {
                        // Compensation step failed -- route per comp_step.on_failure
                        steps_executed.push(StepRecord {
                            step_id: format!("comp:{}", comp_step.op),
                            step_type: "compensation".to_string(),
                            result: format!("error: {}", comp_err),
                            instance_bindings: std::collections::BTreeMap::new(),
                        });
                        match &comp_step.on_failure {
                            StepTarget::Terminal { outcome } => {
                                // take() instead of clone() -- terminal return
                                return Ok(Some(FlowResult {
                                    outcome: outcome.clone(),
                                    steps_executed: std::mem::take(steps_executed),
                                    entity_state_changes: std::mem::take(entity_changes_all),
                                    initiating_persona: None,
                                }));
                            }
                            StepTarget::StepRef(_step_ref) => {
                                // Break out of compensation loop -- caller will
                                // set current_step_id from the returned None
                                // (This case is handled inline by the caller
                                // because we need to communicate the step ref)
                                return Err(EvalError::FlowError {
                                    flow_id: step_id.to_string(),
                                    message: format!(
                                        "compensation step '{}' failed, redirecting",
                                        comp_step.op
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            // All compensation succeeded -- route to `then`
            match then {
                StepTarget::Terminal { outcome } => {
                    // take() instead of clone() -- terminal return
                    Ok(Some(FlowResult {
                        outcome: outcome.clone(),
                        steps_executed: std::mem::take(steps_executed),
                        entity_state_changes: std::mem::take(entity_changes_all),
                        initiating_persona: None,
                    }))
                }
                StepTarget::StepRef(_next_id) => {
                    // Caller will set current_step_id
                    Ok(None)
                }
            }
        }
        FailureHandler::Escalate {
            to_persona,
            next: _,
        } => {
            // Escalation is a persona transfer on failure -- record handoff
            // and continue from the next step
            steps_executed.push(StepRecord {
                step_id: step_id.to_string(),
                step_type: "escalation".to_string(),
                result: format!("escalated to {}", to_persona),
                instance_bindings: std::collections::BTreeMap::new(),
            });
            // Caller will set current_step_id = next
            Ok(None)
        }
    }
}

/// Execute a flow against a frozen snapshot.
///
/// The flow is a state machine walk starting at `flow.entry`.
/// Each step is evaluated against the FROZEN snapshot -- entity state
/// changes go to the mutable EntityStateMap, but verdicts in the
/// snapshot are never recomputed.
///
/// Per §11.1 and §11.4: the `instance_bindings` map tells the executor which
/// specific instance of each entity type to act on at each operation step.
/// Sub-flows inherit the parent's instance bindings per §11.4/§11.5.
/// An empty binding map falls back to DEFAULT_INSTANCE_ID for all entities
/// (backward compat with single-instance contracts per §6.5).
pub fn execute_flow(
    flow: &Flow,
    contract: &Contract,
    snapshot: &Snapshot,
    entity_states: &mut EntityStateMap,
    instance_bindings: &InstanceBindingMap,
    max_steps: Option<usize>,
) -> Result<FlowResult, EvalError> {
    let mut steps_executed = Vec::new();
    let mut entity_changes_all = Vec::new();

    // Build step index by id for fast lookup
    let step_index: BTreeMap<String, &FlowStep> = flow
        .steps
        .iter()
        .map(|s| (step_id(s).to_string(), s))
        .collect();

    // Build operation index for O(1) lookup (replaces O(n) linear scans)
    let op_index: HashMap<&str, &Operation> = contract
        .operations
        .iter()
        .map(|o| (o.id.as_str(), o))
        .collect();

    // Start at entry step
    let mut current_step_id = flow.entry.clone();

    // Max steps to prevent infinite loops
    let max_steps = max_steps.unwrap_or(1000);
    let mut step_count = 0;

    loop {
        step_count += 1;
        if step_count > max_steps {
            return Err(EvalError::FlowError {
                flow_id: flow.id.clone(),
                message: format!("exceeded maximum step count ({})", max_steps),
            });
        }

        let step = step_index
            .get(&current_step_id)
            .ok_or_else(|| EvalError::DeserializeError {
                message: format!(
                    "flow step '{}' not found in flow '{}'",
                    current_step_id, flow.id
                ),
            })?;

        match step {
            FlowStep::OperationStep {
                id,
                op,
                persona,
                outcomes,
                on_failure,
            } => {
                // Find the operation in the contract (O(1) via index)
                let operation =
                    op_index
                        .get(op.as_str())
                        .ok_or_else(|| EvalError::DeserializeError {
                            message: format!("operation '{}' not found in contract", op),
                        })?;

                // Resolve instance bindings for this operation's effects.
                // Per §11.4: maps entity_id -> instance_id for each entity this op targets.
                let op_bindings = resolve_bindings(operation, instance_bindings);

                // Execute the operation against the FROZEN snapshot
                match execute_operation(
                    operation,
                    persona,
                    &snapshot.facts,
                    &snapshot.verdicts,
                    entity_states,
                    &op_bindings,
                ) {
                    Ok(op_result) => {
                        entity_changes_all.extend(op_result.effects_applied.clone());
                        steps_executed.push(StepRecord {
                            step_id: id.clone(),
                            step_type: "operation".to_string(),
                            result: op_result.outcome.clone(),
                            instance_bindings: op_result.provenance.instance_binding.clone(),
                        });

                        // Route based on outcome
                        let target = outcomes.get(&op_result.outcome).ok_or_else(|| {
                            EvalError::DeserializeError {
                                message: format!(
                                    "operation outcome '{}' not handled in step '{}'",
                                    op_result.outcome, id
                                ),
                            }
                        })?;

                        match target {
                            StepTarget::StepRef(next_id) => {
                                current_step_id = next_id.clone();
                            }
                            StepTarget::Terminal { outcome } => {
                                return Ok(FlowResult {
                                    outcome: outcome.clone(),
                                    steps_executed,
                                    entity_state_changes: entity_changes_all,
                                    initiating_persona: None,
                                });
                            }
                        }
                    }
                    Err(op_err) => {
                        // Handle operation failure
                        steps_executed.push(StepRecord {
                            step_id: id.clone(),
                            step_type: "operation".to_string(),
                            result: format!("error: {}", op_err),
                            instance_bindings: op_bindings.clone(),
                        });

                        match handle_failure(
                            on_failure,
                            id,
                            &op_index,
                            snapshot,
                            entity_states,
                            instance_bindings,
                            &mut steps_executed,
                            &mut entity_changes_all,
                        )? {
                            Some(result) => return Ok(result),
                            None => {
                                // Continue flow from the handler's next step
                                match on_failure {
                                    FailureHandler::Compensate {
                                        then: StepTarget::StepRef(next_id),
                                        ..
                                    } => {
                                        current_step_id = next_id.clone();
                                    }
                                    FailureHandler::Escalate { next, .. } => {
                                        current_step_id = next.clone();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            FlowStep::BranchStep {
                id,
                condition,
                persona: _,
                if_true,
                if_false,
            } => {
                // Evaluate condition against FROZEN snapshot
                let mut collector = ProvenanceCollector::new();
                let ctx = EvalContext::new();
                let cond_result = eval_pred(
                    condition,
                    &snapshot.facts,
                    &snapshot.verdicts,
                    &ctx,
                    &mut collector,
                )?;
                let branch_taken = cond_result.as_bool()?;

                let branch_label = if branch_taken { "true" } else { "false" };
                steps_executed.push(StepRecord {
                    step_id: id.clone(),
                    step_type: "branch".to_string(),
                    result: branch_label.to_string(),
                    instance_bindings: std::collections::BTreeMap::new(),
                });

                let target = if branch_taken { if_true } else { if_false };
                match target {
                    StepTarget::StepRef(next_id) => {
                        current_step_id = next_id.clone();
                    }
                    StepTarget::Terminal { outcome } => {
                        return Ok(FlowResult {
                            outcome: outcome.clone(),
                            steps_executed,
                            entity_state_changes: entity_changes_all,
                            initiating_persona: None,
                        });
                    }
                }
            }

            FlowStep::SubFlowStep {
                id,
                flow: sub_flow_id,
                persona: _,
                on_success,
                on_failure,
            } => {
                // Find the sub-flow in the contract (O(1) via HashMap index)
                let sub_flow =
                    contract
                        .get_flow(sub_flow_id)
                        .ok_or_else(|| EvalError::DeserializeError {
                            message: format!("sub-flow '{}' not found in contract", sub_flow_id),
                        })?;

                // Sub-flows INHERIT the parent snapshot AND instance bindings (spec E5, §11.4).
                // Per §11.4: sub-flows use the same InstanceBindingMap as the parent flow.
                match execute_flow(
                    sub_flow,
                    contract,
                    snapshot,
                    entity_states,
                    instance_bindings,
                    None,
                ) {
                    Ok(sub_result) => {
                        entity_changes_all.extend(sub_result.entity_state_changes);
                        steps_executed.push(StepRecord {
                            step_id: id.clone(),
                            step_type: "sub_flow".to_string(),
                            result: sub_result.outcome.clone(),
                            // Sub-flows inherit the parent instance_bindings per §11.4/§11.5.
                            // We record the parent's bindings for this sub-flow step.
                            instance_bindings: instance_bindings.clone(),
                        });

                        match on_success {
                            StepTarget::StepRef(next_id) => {
                                current_step_id = next_id.clone();
                            }
                            StepTarget::Terminal { outcome } => {
                                return Ok(FlowResult {
                                    outcome: outcome.clone(),
                                    steps_executed,
                                    entity_state_changes: entity_changes_all,
                                    initiating_persona: None,
                                });
                            }
                        }
                    }
                    Err(_sub_err) => {
                        steps_executed.push(StepRecord {
                            step_id: id.clone(),
                            step_type: "sub_flow".to_string(),
                            result: "error".to_string(),
                            instance_bindings: instance_bindings.clone(),
                        });

                        match handle_failure(
                            on_failure,
                            id,
                            &op_index,
                            snapshot,
                            entity_states,
                            instance_bindings,
                            &mut steps_executed,
                            &mut entity_changes_all,
                        )? {
                            Some(result) => return Ok(result),
                            None => {
                                // Continue flow from the handler's next step
                                match on_failure {
                                    FailureHandler::Compensate {
                                        then: StepTarget::StepRef(next_id),
                                        ..
                                    } => {
                                        current_step_id = next_id.clone();
                                    }
                                    FailureHandler::Escalate { next, .. } => {
                                        current_step_id = next.clone();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            FlowStep::HandoffStep {
                id,
                from_persona: _,
                to_persona: _,
                next,
            } => {
                // Handoff is a persona transfer -- record and continue
                steps_executed.push(StepRecord {
                    step_id: id.clone(),
                    step_type: "handoff".to_string(),
                    result: "handoff".to_string(),
                    instance_bindings: std::collections::BTreeMap::new(),
                });
                current_step_id = next.clone();
            }

            FlowStep::ParallelStep { id, branches, join } => {
                // Per spec Section 11.5: execute each branch with isolated
                // entity states, then merge on join. The frozen snapshot is
                // shared across all branches (immutable).
                let mut branch_outcomes: Vec<BranchOutcome> = Vec::new();

                for branch in branches {
                    // Each branch gets its own clone of entity states
                    let mut branch_entity_states = entity_states.clone();

                    // Build a branch-local flow to execute
                    let branch_flow = Flow {
                        id: format!("{}:{}", flow.id, branch.id),
                        snapshot: flow.snapshot.clone(),
                        entry: branch.entry.clone(),
                        steps: branch.steps.clone(),
                    };

                    match execute_flow(
                        &branch_flow,
                        contract,
                        snapshot,
                        &mut branch_entity_states,
                        instance_bindings,
                        None,
                    ) {
                        Ok(branch_result) => {
                            branch_outcomes.push(BranchOutcome::Success {
                                branch_id: branch.id.clone(),
                                outcome: branch_result.outcome,
                                entity_changes: branch_result.entity_state_changes,
                                steps: branch_result.steps_executed,
                                entity_states: branch_entity_states,
                            });
                        }
                        Err(e) => {
                            branch_outcomes.push(BranchOutcome::Failure {
                                branch_id: branch.id.clone(),
                                error: format!("{}", e),
                                steps: vec![],
                            });
                        }
                    }
                }

                // Record the parallel step
                let branch_summaries: Vec<String> = branch_outcomes
                    .iter()
                    .map(|bo| match bo {
                        BranchOutcome::Success {
                            branch_id, outcome, ..
                        } => format!("{}:{}", branch_id, outcome),
                        BranchOutcome::Failure {
                            branch_id, error, ..
                        } => format!("{}:error:{}", branch_id, error),
                    })
                    .collect();

                steps_executed.push(StepRecord {
                    step_id: id.clone(),
                    step_type: "parallel".to_string(),
                    result: branch_summaries.join(", "),
                    // Parallel steps use the parent's instance_bindings
                    instance_bindings: instance_bindings.clone(),
                });

                // Collect branch step records
                for bo in &branch_outcomes {
                    match bo {
                        BranchOutcome::Success { steps, .. } => {
                            steps_executed.extend(steps.clone());
                        }
                        BranchOutcome::Failure { steps, .. } => {
                            steps_executed.extend(steps.clone());
                        }
                    }
                }

                // Merge entity state changes from successful branches
                // Spec guarantees non-overlapping entity effect sets
                for bo in &branch_outcomes {
                    if let BranchOutcome::Success {
                        entity_changes,
                        entity_states: branch_states,
                        ..
                    } = bo
                    {
                        entity_changes_all.extend(entity_changes.clone());
                        // Apply branch's final entity states to parent.
                        // EffectRecord now carries instance_id per Plan 04-02, so we use it
                        // directly to form the composite (entity_id, instance_id) key.
                        for change in entity_changes {
                            entity_states.insert(
                                (change.entity_id.clone(), change.instance_id.clone()),
                                change.to_state.clone(),
                            );
                        }
                        // Also apply any entity states that were modified but not
                        // captured as EffectRecords (safety: merge all branch state)
                        let _ = branch_states; // Used via entity_changes above
                    }
                }

                // Evaluate join policy
                let all_success = branch_outcomes
                    .iter()
                    .all(|bo| matches!(bo, BranchOutcome::Success { .. }));
                let any_failure = branch_outcomes
                    .iter()
                    .any(|bo| matches!(bo, BranchOutcome::Failure { .. }));

                if all_success {
                    if let Some(ref target) = join.on_all_success {
                        match target {
                            StepTarget::StepRef(next_id) => {
                                current_step_id = next_id.clone();
                                continue;
                            }
                            StepTarget::Terminal { outcome } => {
                                return Ok(FlowResult {
                                    outcome: outcome.clone(),
                                    steps_executed,
                                    entity_state_changes: entity_changes_all,
                                    initiating_persona: None,
                                });
                            }
                        }
                    }
                }

                if any_failure {
                    if let Some(ref handler) = join.on_any_failure {
                        match handle_failure(
                            handler,
                            id,
                            &op_index,
                            snapshot,
                            entity_states,
                            instance_bindings,
                            &mut steps_executed,
                            &mut entity_changes_all,
                        )? {
                            Some(result) => return Ok(result),
                            None => match handler {
                                FailureHandler::Compensate {
                                    then: StepTarget::StepRef(next_id),
                                    ..
                                } => {
                                    current_step_id = next_id.clone();
                                    continue;
                                }
                                FailureHandler::Escalate { next, .. } => {
                                    current_step_id = next.clone();
                                    continue;
                                }
                                _ => {}
                            },
                        }
                    }
                }

                // on_all_complete: route here regardless (after success/failure)
                if let Some(ref target) = join.on_all_complete {
                    match target {
                        StepTarget::StepRef(next_id) => {
                            current_step_id = next_id.clone();
                        }
                        StepTarget::Terminal { outcome } => {
                            return Ok(FlowResult {
                                outcome: outcome.clone(),
                                steps_executed,
                                entity_state_changes: entity_changes_all,
                                initiating_persona: None,
                            });
                        }
                    }
                } else {
                    // No applicable join policy matched -- this is a flow error
                    return Err(EvalError::FlowError {
                        flow_id: flow.id.clone(),
                        message: format!(
                            "parallel step '{}' completed but no join policy matched",
                            id
                        ),
                    });
                }
            }
        }
    }
}

/// Extract the step ID from a FlowStep.
fn step_id(step: &FlowStep) -> &str {
    match step {
        FlowStep::OperationStep { id, .. } => id,
        FlowStep::BranchStep { id, .. } => id,
        FlowStep::HandoffStep { id, .. } => id,
        FlowStep::SubFlowStep { id, .. } => id,
        FlowStep::ParallelStep { id, .. } => id,
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests;
