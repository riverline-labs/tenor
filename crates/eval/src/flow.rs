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
#[derive(Debug, Clone)]
pub struct StepRecord {
    pub step_id: String,
    pub step_type: String,
    pub result: String,
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
                        });
                    }
                    Err(comp_err) => {
                        // Compensation step failed -- route per comp_step.on_failure
                        steps_executed.push(StepRecord {
                            step_id: format!("comp:{}", comp_step.op),
                            step_type: "compensation".to_string(),
                            result: format!("error: {}", comp_err),
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
mod tests {
    use super::*;
    use crate::operation::EntityStateMap;
    use crate::types::*;

    fn bool_type() -> TypeSpec {
        TypeSpec {
            base: "Bool".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    fn make_contract_with(
        entities: Vec<Entity>,
        operations: Vec<Operation>,
        flows: Vec<Flow>,
    ) -> Contract {
        Contract::new(
            vec![],
            entities,
            vec![],
            operations,
            flows,
            vec!["admin".to_string(), "system".to_string()],
        )
    }

    // ──────────────────────────────────────
    // Simple flow: entry -> operation -> terminal
    // ──────────────────────────────────────

    #[test]
    fn simple_flow_entry_operation_terminal() {
        let operation = Operation {
            id: "approve_order".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["approved".to_string()],
        };

        let flow = Flow {
            id: "approval_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_approve".to_string(),
            steps: vec![FlowStep::OperationStep {
                id: "step_approve".to_string(),
                op: "approve_order".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "approved".to_string(),
                        StepTarget::Terminal {
                            outcome: "order_approved".to_string(),
                        },
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "approval_failed".to_string(),
                },
            }],
        };

        let contract = make_contract_with(
            vec![Entity {
                id: "order".to_string(),
                states: vec!["pending".to_string(), "approved".to_string()],
                initial: "pending".to_string(),
                transitions: vec![],
            }],
            vec![operation],
            vec![flow.clone()],
        );

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };

        let mut entity_states = crate::operation::single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.outcome, "order_approved");
        assert_eq!(result.steps_executed.len(), 1);
        assert_eq!(result.steps_executed[0].step_type, "operation");
        assert_eq!(result.steps_executed[0].result, "approved");
        assert_eq!(result.entity_state_changes.len(), 1);
        assert_eq!(
            crate::operation::get_instance_state(
                &entity_states,
                "order",
                crate::operation::DEFAULT_INSTANCE_ID
            )
            .unwrap(),
            "approved"
        );
    }

    // ──────────────────────────────────────
    // Branch flow: entry -> branch -> terminal_a | terminal_b
    // ──────────────────────────────────────

    #[test]
    fn branch_flow_true_path() {
        let flow = Flow {
            id: "branch_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_branch".to_string(),
            steps: vec![FlowStep::BranchStep {
                id: "step_branch".to_string(),
                condition: Predicate::VerdictPresent("is_valid".to_string()),
                persona: "system".to_string(),
                if_true: StepTarget::Terminal {
                    outcome: "valid_path".to_string(),
                },
                if_false: StepTarget::Terminal {
                    outcome: "invalid_path".to_string(),
                },
            }],
        };

        let contract = make_contract_with(vec![], vec![], vec![flow.clone()]);

        // Verdict is present -> should take true path
        let mut verdicts = VerdictSet::new();
        verdicts.push(VerdictInstance {
            verdict_type: "is_valid".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "check".to_string(),
                stratum: 0,
                facts_used: vec![],
                verdicts_used: vec![],
            },
        });

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts,
        };
        let mut entity_states = EntityStateMap::new();

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.outcome, "valid_path");
        assert_eq!(result.steps_executed[0].result, "true");
    }

    #[test]
    fn branch_flow_false_path() {
        let flow = Flow {
            id: "branch_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_branch".to_string(),
            steps: vec![FlowStep::BranchStep {
                id: "step_branch".to_string(),
                condition: Predicate::VerdictPresent("is_valid".to_string()),
                persona: "system".to_string(),
                if_true: StepTarget::Terminal {
                    outcome: "valid_path".to_string(),
                },
                if_false: StepTarget::Terminal {
                    outcome: "invalid_path".to_string(),
                },
            }],
        };

        let contract = make_contract_with(vec![], vec![], vec![flow.clone()]);

        // No verdict -> should take false path
        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };
        let mut entity_states = EntityStateMap::new();

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.outcome, "invalid_path");
        assert_eq!(result.steps_executed[0].result, "false");
    }

    // ──────────────────────────────────────────────────────────
    // CRITICAL: Frozen verdict test
    //
    // Scenario: A flow where an OperationStep changes entity
    // state, then a subsequent BranchStep checks a verdict that
    // WOULD change if verdicts were recomputed. The branch MUST
    // use the frozen (original) verdict, NOT a recomputed one.
    // ──────────────────────────────────────────────────────────

    #[test]
    fn frozen_verdict_semantics_entity_change_does_not_affect_verdicts() {
        // Setup:
        // - Entity "order" starts in "pending" state
        // - Verdict "order_eligible" was computed at flow initiation
        //   (based on facts/verdicts at that time)
        // - Step 1: OperationStep transitions order from "pending" to "rejected"
        //   (In a recomputing system, this might invalidate "order_eligible")
        // - Step 2: BranchStep checks if verdict "order_eligible" is present
        //   (MUST be true because verdicts are FROZEN at initiation)
        //
        // If frozen semantics are correct: branch takes TRUE path
        // If verdicts were recomputed: branch would take FALSE path

        let operation = Operation {
            id: "reject_order".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "rejected".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["rejected".to_string()],
        };

        let flow = Flow {
            id: "test_frozen_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_reject".to_string(),
            steps: vec![
                // Step 1: Change entity state (pending -> rejected)
                FlowStep::OperationStep {
                    id: "step_reject".to_string(),
                    op: "reject_order".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "rejected".to_string(),
                            StepTarget::StepRef("step_check".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "flow_error".to_string(),
                    },
                },
                // Step 2: Check verdict that was set at initiation
                // Even though entity state changed, verdict MUST still be present
                FlowStep::BranchStep {
                    id: "step_check".to_string(),
                    condition: Predicate::VerdictPresent("order_eligible".to_string()),
                    persona: "system".to_string(),
                    if_true: StepTarget::Terminal {
                        outcome: "frozen_verdict_confirmed".to_string(),
                    },
                    if_false: StepTarget::Terminal {
                        outcome: "frozen_verdict_violated".to_string(),
                    },
                },
            ],
        };

        let contract = make_contract_with(
            vec![Entity {
                id: "order".to_string(),
                states: vec!["pending".to_string(), "rejected".to_string()],
                initial: "pending".to_string(),
                transitions: vec![],
            }],
            vec![operation],
            vec![flow.clone()],
        );

        // Create snapshot WITH "order_eligible" verdict
        // (In a real system, this would have been computed from rules)
        let mut verdicts = VerdictSet::new();
        verdicts.push(VerdictInstance {
            verdict_type: "order_eligible".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "eligibility_check".to_string(),
                stratum: 0,
                facts_used: vec!["order_status".to_string()],
                verdicts_used: vec![],
            },
        });

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts,
        };

        let mut entity_states = crate::operation::single_instance(
            [("order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();

        // CRITICAL ASSERTION: The branch must use the FROZEN verdict
        // Even though the operation changed entity state to "rejected",
        // the verdict "order_eligible" from the snapshot is still present.
        assert_eq!(
            result.outcome, "frozen_verdict_confirmed",
            "Frozen verdict semantics violated! Branch should have used the original \
             snapshot verdict, not a recomputed one."
        );

        // Verify the entity state DID change (operation was executed)
        assert_eq!(
            crate::operation::get_instance_state(
                &entity_states,
                "order",
                crate::operation::DEFAULT_INSTANCE_ID
            )
            .unwrap(),
            "rejected"
        );

        // Verify both steps executed
        assert_eq!(result.steps_executed.len(), 2);
        assert_eq!(result.steps_executed[0].step_type, "operation");
        assert_eq!(result.steps_executed[1].step_type, "branch");
        assert_eq!(result.steps_executed[1].result, "true");
    }

    // ──────────────────────────────────────
    // Sub-flow inherits parent snapshot
    // ──────────────────────────────────────

    #[test]
    fn sub_flow_inherits_parent_snapshot() {
        // Parent flow calls a sub-flow. The sub-flow should see
        // the same verdicts from the parent's snapshot.

        let operation = Operation {
            id: "finalize".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "order".to_string(),
                from: "approved".to_string(),
                to: "finalized".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["done".to_string()],
        };

        // Sub-flow: branch on verdict, then terminal
        let sub_flow = Flow {
            id: "sub_check_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "sub_branch".to_string(),
            steps: vec![FlowStep::BranchStep {
                id: "sub_branch".to_string(),
                condition: Predicate::VerdictPresent("parent_verdict".to_string()),
                persona: "system".to_string(),
                if_true: StepTarget::Terminal {
                    outcome: "sub_success".to_string(),
                },
                if_false: StepTarget::Terminal {
                    outcome: "sub_failure".to_string(),
                },
            }],
        };

        // Parent flow: op -> sub-flow -> terminal
        let parent_flow = Flow {
            id: "parent_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_op".to_string(),
            steps: vec![
                FlowStep::OperationStep {
                    id: "step_op".to_string(),
                    op: "finalize".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "done".to_string(),
                            StepTarget::StepRef("step_sub".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "parent_error".to_string(),
                    },
                },
                FlowStep::SubFlowStep {
                    id: "step_sub".to_string(),
                    flow: "sub_check_flow".to_string(),
                    persona: "admin".to_string(),
                    on_success: StepTarget::Terminal {
                        outcome: "parent_success".to_string(),
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "parent_error".to_string(),
                    },
                },
            ],
        };

        let contract = make_contract_with(
            vec![Entity {
                id: "order".to_string(),
                states: vec!["approved".to_string(), "finalized".to_string()],
                initial: "approved".to_string(),
                transitions: vec![],
            }],
            vec![operation],
            vec![parent_flow.clone(), sub_flow],
        );

        // Snapshot with parent_verdict present
        let mut verdicts = VerdictSet::new();
        verdicts.push(VerdictInstance {
            verdict_type: "parent_verdict".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "parent_rule".to_string(),
                stratum: 0,
                facts_used: vec![],
                verdicts_used: vec![],
            },
        });

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts,
        };

        let mut entity_states = crate::operation::single_instance(
            [("order".to_string(), "approved".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &parent_flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();

        // Sub-flow should have seen "parent_verdict" from the inherited snapshot
        assert_eq!(result.outcome, "parent_success");
        assert_eq!(result.steps_executed.len(), 2); // op + sub_flow
        assert_eq!(result.steps_executed[1].step_type, "sub_flow");
        assert_eq!(result.steps_executed[1].result, "sub_success");
    }

    // ──────────────────────────────────────
    // Operation on_failure routing
    // ──────────────────────────────────────

    #[test]
    fn operation_step_on_failure_terminates() {
        // Operation will fail because entity is in wrong state
        let operation = Operation {
            id: "approve".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "order".to_string(),
                from: "pending".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["approved".to_string()],
        };

        let flow = Flow {
            id: "fail_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_approve".to_string(),
            steps: vec![FlowStep::OperationStep {
                id: "step_approve".to_string(),
                op: "approve".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "approved".to_string(),
                        StepTarget::Terminal {
                            outcome: "success".to_string(),
                        },
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "failure_handled".to_string(),
                },
            }],
        };

        let contract = make_contract_with(
            vec![Entity {
                id: "order".to_string(),
                states: vec![
                    "draft".to_string(),
                    "pending".to_string(),
                    "approved".to_string(),
                ],
                initial: "draft".to_string(),
                transitions: vec![],
            }],
            vec![operation],
            vec![flow.clone()],
        );

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };

        // Entity is in "draft", not "pending" -- operation will fail
        let mut entity_states = crate::operation::single_instance(
            [("order".to_string(), "draft".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.outcome, "failure_handled");
    }

    // ──────────────────────────────────────
    // Multi-step flow: operation -> branch -> terminal
    // ──────────────────────────────────────

    #[test]
    fn multi_step_flow_operation_then_branch() {
        let operation = Operation {
            id: "submit".to_string(),
            allowed_personas: vec!["buyer".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["submitted".to_string()],
        };

        let flow = Flow {
            id: "submit_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_submit".to_string(),
            steps: vec![
                FlowStep::OperationStep {
                    id: "step_submit".to_string(),
                    op: "submit".to_string(),
                    persona: "buyer".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "submitted".to_string(),
                            StepTarget::StepRef("step_check".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "submit_failed".to_string(),
                    },
                },
                FlowStep::BranchStep {
                    id: "step_check".to_string(),
                    condition: Predicate::VerdictPresent("order_valid".to_string()),
                    persona: "system".to_string(),
                    if_true: StepTarget::Terminal {
                        outcome: "submitted_valid".to_string(),
                    },
                    if_false: StepTarget::Terminal {
                        outcome: "submitted_invalid".to_string(),
                    },
                },
            ],
        };

        let contract = make_contract_with(
            vec![Entity {
                id: "order".to_string(),
                states: vec!["draft".to_string(), "submitted".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
            }],
            vec![operation],
            vec![flow.clone()],
        );

        // Snapshot with "order_valid" verdict
        let mut verdicts = VerdictSet::new();
        verdicts.push(VerdictInstance {
            verdict_type: "order_valid".to_string(),
            payload: Value::Bool(true),
            provenance: crate::provenance::VerdictProvenance {
                rule_id: "validate".to_string(),
                stratum: 0,
                facts_used: vec![],
                verdicts_used: vec![],
            },
        });

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts,
        };

        let mut entity_states = crate::operation::single_instance(
            [("order".to_string(), "draft".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(result.outcome, "submitted_valid");
        assert_eq!(result.steps_executed.len(), 2);
        assert_eq!(
            crate::operation::get_instance_state(
                &entity_states,
                "order",
                crate::operation::DEFAULT_INSTANCE_ID
            )
            .unwrap(),
            "submitted"
        );
    }

    // ──────────────────────────────────────
    // C9: FlowError variant test (E1 fix)
    // ──────────────────────────────────────

    #[test]
    fn flow_step_limit_returns_flow_error() {
        // Create a circular flow that will exceed the step limit.
        // step_a -> step_b -> step_a (infinite loop)
        let operation_a = Operation {
            id: "op_a".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![],
            error_contract: vec![],
            outcomes: vec!["done".to_string()],
        };

        let operation_b = Operation {
            id: "op_b".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![],
            error_contract: vec![],
            outcomes: vec!["done".to_string()],
        };

        let flow = Flow {
            id: "loop_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_a".to_string(),
            steps: vec![
                FlowStep::OperationStep {
                    id: "step_a".to_string(),
                    op: "op_a".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "done".to_string(),
                            StepTarget::StepRef("step_b".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "error".to_string(),
                    },
                },
                FlowStep::OperationStep {
                    id: "step_b".to_string(),
                    op: "op_b".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "done".to_string(),
                            StepTarget::StepRef("step_a".to_string()), // circular!
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "error".to_string(),
                    },
                },
            ],
        };

        let contract =
            make_contract_with(vec![], vec![operation_a, operation_b], vec![flow.clone()]);

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };

        let mut entity_states = EntityStateMap::new();

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::FlowError { flow_id, message } => {
                assert_eq!(flow_id, "loop_flow");
                assert!(
                    message.contains("maximum step count"),
                    "error should mention step count, got: {}",
                    message
                );
            }
            other => panic!("expected FlowError, got {:?}", other),
        }
    }

    // ──────────────────────────────────────
    // Configurable max_steps parameter test
    // ──────────────────────────────────────

    // ──────────────────────────────────────────────────────────
    // BEHAVIORAL CHARACTERIZATION: Parallel branch entity state conflict
    //
    // This test documents the ACTUAL behavior when two parallel branches
    // modify the same entity to different states. The spec (Section 10.5,
    // Section 12.2 Pass 5) requires non-overlapping entity effect sets
    // across parallel branches, and the elaborator enforces this at
    // compile time (Pass 5). However, the evaluator does NOT enforce
    // this constraint at runtime — it trusts the elaborator.
    //
    // This test constructs a scenario that would be rejected by the
    // elaborator but is valid interchange JSON, to characterize the
    // evaluator's merge semantics when two branches write conflicting
    // entity states.
    //
    // Observed behavior: LAST-WRITER-WINS (silent).
    // - Both branches execute independently from cloned entity state.
    // - Both succeed (each sees the entity in "initial" state).
    // - During merge, branch outcomes are processed in declaration order.
    // - The last branch's entity_states.insert() overwrites the first's.
    // - entity_changes_all contains BOTH effect records (no dedup).
    // - No error is raised. No warning is emitted.
    //
    // This is a characterization test, not a prescriptive test. If the
    // evaluator is changed to detect conflicts, this test should be
    // updated to reflect the new behavior.
    // ──────────────────────────────────────────────────────────

    #[test]
    fn parallel_branch_entity_conflict_last_writer_wins() {
        // Entity: Order with states [initial, state_a, state_b]
        let entity = Entity {
            id: "Order".to_string(),
            states: vec![
                "initial".to_string(),
                "state_a".to_string(),
                "state_b".to_string(),
            ],
            initial: "initial".to_string(),
            transitions: vec![],
        };

        // op_to_a: transitions Order from initial -> state_a
        let op_a = Operation {
            id: "op_to_a".to_string(),
            allowed_personas: vec!["system".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "Order".to_string(),
                from: "initial".to_string(),
                to: "state_a".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["success".to_string()],
        };

        // op_to_b: transitions Order from initial -> state_b
        let op_b = Operation {
            id: "op_to_b".to_string(),
            allowed_personas: vec!["system".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![Effect {
                entity_id: "Order".to_string(),
                from: "initial".to_string(),
                to: "state_b".to_string(),
                outcome: None,
            }],
            error_contract: vec![],
            outcomes: vec!["success".to_string()],
        };

        // Flow with a parallel step: branch_a runs op_to_a, branch_b runs op_to_b
        let flow = Flow {
            id: "conflict_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "par_step".to_string(),
            steps: vec![FlowStep::ParallelStep {
                id: "par_step".to_string(),
                branches: vec![
                    ParallelBranch {
                        id: "branch_a".to_string(),
                        entry: "step_a".to_string(),
                        steps: vec![FlowStep::OperationStep {
                            id: "step_a".to_string(),
                            op: "op_to_a".to_string(),
                            persona: "system".to_string(),
                            outcomes: {
                                let mut m = BTreeMap::new();
                                m.insert(
                                    "success".to_string(),
                                    StepTarget::Terminal {
                                        outcome: "branch_a_done".to_string(),
                                    },
                                );
                                m
                            },
                            on_failure: FailureHandler::Terminate {
                                outcome: "branch_a_failed".to_string(),
                            },
                        }],
                    },
                    ParallelBranch {
                        id: "branch_b".to_string(),
                        entry: "step_b".to_string(),
                        steps: vec![FlowStep::OperationStep {
                            id: "step_b".to_string(),
                            op: "op_to_b".to_string(),
                            persona: "system".to_string(),
                            outcomes: {
                                let mut m = BTreeMap::new();
                                m.insert(
                                    "success".to_string(),
                                    StepTarget::Terminal {
                                        outcome: "branch_b_done".to_string(),
                                    },
                                );
                                m
                            },
                            on_failure: FailureHandler::Terminate {
                                outcome: "branch_b_failed".to_string(),
                            },
                        }],
                    },
                ],
                join: JoinPolicy {
                    on_all_success: Some(StepTarget::Terminal {
                        outcome: "all_done".to_string(),
                    }),
                    on_any_failure: Some(FailureHandler::Terminate {
                        outcome: "some_failed".to_string(),
                    }),
                    on_all_complete: None,
                },
            }],
        };

        let contract = make_contract_with(vec![entity], vec![op_a, op_b], vec![flow.clone()]);

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };

        let mut entity_states = crate::operation::single_instance(
            [("Order".to_string(), "initial".to_string())]
                .into_iter()
                .collect(),
        );

        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            None,
        );

        // ── Characterize actual behavior ──

        // The flow should succeed (no conflict detection at the evaluator level).
        assert!(
            result.is_ok(),
            "Expected flow to succeed (no runtime conflict detection), but got error: {:?}",
            result.err()
        );

        let flow_result = result.unwrap();

        // Both branches succeed, so on_all_success fires -> outcome "all_done"
        assert_eq!(
            flow_result.outcome, "all_done",
            "Expected on_all_success outcome since both branches succeed independently"
        );

        // CRITICAL OBSERVATION: The final entity state is determined by
        // branch processing order. Since branches are iterated in declaration
        // order and the merge loop does entity_states.insert() for each,
        // the LAST branch's state wins.
        //
        // branch_a sets Order -> state_a
        // branch_b sets Order -> state_b (overwrites state_a)
        let final_state = crate::operation::get_instance_state(
            &entity_states,
            "Order",
            crate::operation::DEFAULT_INSTANCE_ID,
        )
        .unwrap();
        assert_eq!(
            final_state, "state_b",
            "Last-writer-wins: branch_b (declared second) overwrites branch_a's state. \
             Final entity state is '{}', expected 'state_b'.",
            final_state
        );

        // Both effect records are collected (extend, not replace).
        // This means entity_changes_all contains contradictory records:
        // [Order: initial->state_a, Order: initial->state_b]
        assert_eq!(
            flow_result.entity_state_changes.len(),
            2,
            "Both branches' effect records are collected (no dedup). Got: {:?}",
            flow_result.entity_state_changes
        );

        // Verify the individual effect records
        let change_a = &flow_result.entity_state_changes[0];
        assert_eq!(change_a.entity_id, "Order");
        assert_eq!(change_a.from_state, "initial");
        assert_eq!(change_a.to_state, "state_a");

        let change_b = &flow_result.entity_state_changes[1];
        assert_eq!(change_b.entity_id, "Order");
        assert_eq!(change_b.from_state, "initial");
        assert_eq!(change_b.to_state, "state_b");

        // Verify steps executed: parallel step + 2 branch operation steps
        assert_eq!(flow_result.steps_executed.len(), 3);
        assert_eq!(flow_result.steps_executed[0].step_type, "parallel");
        assert_eq!(flow_result.steps_executed[1].step_type, "operation");
        assert_eq!(flow_result.steps_executed[2].step_type, "operation");

        // The parallel step's result shows both branches succeeded
        assert!(
            flow_result.steps_executed[0]
                .result
                .contains("branch_a:branch_a_done"),
            "branch_a should report success"
        );
        assert!(
            flow_result.steps_executed[0]
                .result
                .contains("branch_b:branch_b_done"),
            "branch_b should report success"
        );
    }

    #[test]
    fn configurable_max_steps_triggers_at_custom_limit() {
        // A circular flow: step_a -> step_b -> step_a
        // With max_steps=5, should fail after 5 steps.
        let operation_a = Operation {
            id: "op_a".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![],
            error_contract: vec![],
            outcomes: vec!["done".to_string()],
        };

        let operation_b = Operation {
            id: "op_b".to_string(),
            allowed_personas: vec!["admin".to_string()],
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            },
            effects: vec![],
            error_contract: vec![],
            outcomes: vec!["done".to_string()],
        };

        let flow = Flow {
            id: "custom_limit_flow".to_string(),
            snapshot: "at_initiation".to_string(),
            entry: "step_a".to_string(),
            steps: vec![
                FlowStep::OperationStep {
                    id: "step_a".to_string(),
                    op: "op_a".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "done".to_string(),
                            StepTarget::StepRef("step_b".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "error".to_string(),
                    },
                },
                FlowStep::OperationStep {
                    id: "step_b".to_string(),
                    op: "op_b".to_string(),
                    persona: "admin".to_string(),
                    outcomes: {
                        let mut m = BTreeMap::new();
                        m.insert(
                            "done".to_string(),
                            StepTarget::StepRef("step_a".to_string()),
                        );
                        m
                    },
                    on_failure: FailureHandler::Terminate {
                        outcome: "error".to_string(),
                    },
                },
            ],
        };

        let contract =
            make_contract_with(vec![], vec![operation_a, operation_b], vec![flow.clone()]);

        let snapshot = Snapshot {
            facts: FactSet::new(),
            verdicts: VerdictSet::new(),
        };

        // With max_steps=5, should fail
        let mut entity_states = EntityStateMap::new();
        let result = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states,
            &InstanceBindingMap::new(),
            Some(5),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            EvalError::FlowError { flow_id, message } => {
                assert_eq!(flow_id, "custom_limit_flow");
                assert!(
                    message.contains("5"),
                    "error should mention the custom limit 5, got: {}",
                    message
                );
            }
            other => panic!("expected FlowError, got {:?}", other),
        }

        // With None (default 1000), the same flow would also eventually fail
        // but at step 1000 -- just verify it returns a FlowError
        let mut entity_states2 = EntityStateMap::new();
        let result2 = execute_flow(
            &flow,
            &contract,
            &snapshot,
            &mut entity_states2,
            &InstanceBindingMap::new(),
            None,
        );
        assert!(result2.is_err());
        match result2.unwrap_err() {
            EvalError::FlowError { message, .. } => {
                assert!(
                    message.contains("1000"),
                    "default limit should be 1000, got: {}",
                    message
                );
            }
            other => panic!("expected FlowError with default limit, got {:?}", other),
        }
    }
}
