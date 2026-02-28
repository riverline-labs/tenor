//! Pass 5: Construct validation -- structural checks on Entity, Rule,
//! Operation, Flow, and System constructs.

mod entity;
mod flow;
mod operation;
mod parallel;
mod rule;
mod source;
mod system;

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{HashMap, HashSet};

pub fn validate(constructs: &[RawConstruct], index: &Index) -> Result<(), ElabError> {
    validate_verdict_uniqueness(constructs)?;

    let produced_verdicts: HashSet<String> = index.rule_verdicts.values().cloned().collect();

    for c in constructs {
        match c {
            RawConstruct::Entity {
                id,
                states,
                initial,
                initial_line,
                transitions,
                parent,
                parent_line,
                prov,
            } => {
                entity::validate_entity(
                    id,
                    states,
                    initial,
                    *initial_line,
                    transitions,
                    parent.as_deref(),
                    *parent_line,
                    prov,
                    index,
                )?;
            }
            RawConstruct::Rule {
                id,
                stratum,
                stratum_line,
                when,
                prov,
                ..
            } => {
                rule::validate_rule(
                    id,
                    *stratum,
                    *stratum_line,
                    when,
                    prov,
                    index,
                    &produced_verdicts,
                )?;
            }
            RawConstruct::Operation {
                id,
                allowed_personas,
                allowed_personas_line,
                effects,
                outcomes,
                error_contract,
                prov,
                ..
            } => {
                operation::validate_operation(
                    id,
                    allowed_personas,
                    *allowed_personas_line,
                    effects,
                    outcomes,
                    error_contract,
                    prov,
                    index,
                )?;
            }
            RawConstruct::Flow {
                id,
                entry,
                entry_line,
                steps,
                prov,
                ..
            } => {
                flow::validate_flow(id, entry, *entry_line, steps, prov, index)?;
            }
            RawConstruct::System {
                id,
                members,
                shared_personas,
                triggers,
                shared_entities,
                prov,
            } => {
                system::validate_system(
                    id,
                    members,
                    shared_personas,
                    triggers,
                    shared_entities,
                    prov,
                    constructs,
                )?;
            }
            RawConstruct::Source {
                id,
                protocol,
                fields,
                prov,
                ..
            } => {
                source::validate_source(id, protocol, fields, prov, index)?;
            }
            RawConstruct::Fact {
                id,
                source: crate::ast::RawSourceDecl::Structured { source_id, .. },
                prov,
                ..
            } => {
                if !index.sources.contains_key(source_id) {
                    return Err(ElabError::new(
                        5,
                        Some("Fact"),
                        Some(id),
                        Some("source"),
                        &prov.file,
                        prov.line,
                        format!("fact '{}' references undeclared source '{}'", id, source_id),
                    ));
                }
            }
            _ => {}
        }
    }

    entity::validate_entity_dag(constructs, index)?;
    flow::validate_flow_reference_graph(constructs)?;
    parallel::validate_parallel_conflicts(constructs)?;

    Ok(())
}

pub fn validate_operation_transitions(
    constructs: &[RawConstruct],
    index: &Index,
) -> Result<(), ElabError> {
    operation::validate_operation_transitions(constructs, index)
}

/// S8: each VerdictType may be produced by at most one Rule.
fn validate_verdict_uniqueness(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    // verdict_type_name -> first rule id
    let mut seen: HashMap<&str, &str> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Rule {
            id,
            verdict_type,
            produce_line,
            prov,
            ..
        } = c
        {
            if let Some(first_rule_id) = seen.get(verdict_type.as_str()) {
                return Err(ElabError::new(
                    5,
                    Some("Rule"),
                    Some(id),
                    Some("produce"),
                    &prov.file,
                    *produce_line,
                    format!(
                        "VerdictType '{}' is already produced by rule '{}'. Each VerdictType may be produced by at most one rule (S8).",
                        verdict_type, first_rule_id
                    ),
                ));
            }
            seen.insert(verdict_type.as_str(), id.as_str());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_prov() -> Provenance {
        Provenance {
            file: "test.tenor".to_string(),
            line: 1,
        }
    }

    fn test_failure_handler() -> RawFailureHandler {
        RawFailureHandler::Terminate {
            outcome: "fail".to_string(),
        }
    }

    #[test]
    fn detect_step_cycle_in_degree_missing_neighbor() {
        // Construct a flow with steps that reference each other, exercising
        // the in_degree lookup. The normal code path populates in_degree for
        // all neighbors via or_insert(0), so the ok_or_else should never fire
        // in practice. This test verifies a simple acyclic flow works correctly.
        let mut steps = BTreeMap::new();
        let mut outcomes1 = BTreeMap::new();
        outcomes1.insert(
            "ok".to_string(),
            RawStepTarget::StepRef("s2".to_string(), 2),
        );
        steps.insert(
            "s1".to_string(),
            RawStep::OperationStep {
                op: "op1".to_string(),
                persona: "admin".to_string(),
                outcomes: outcomes1,
                on_failure: Some(test_failure_handler()),
                line: 2,
            },
        );
        steps.insert(
            "s2".to_string(),
            RawStep::OperationStep {
                op: "op2".to_string(),
                persona: "admin".to_string(),
                outcomes: BTreeMap::new(),
                on_failure: Some(test_failure_handler()),
                line: 3,
            },
        );
        let prov = test_prov();
        // Should succeed -- no cycle
        assert!(flow::detect_step_cycle("test_flow", "s1", &steps, &prov).is_ok());
    }

    #[test]
    fn detect_step_cycle_finds_cycle() {
        // Two steps that point to each other => cycle
        let mut steps = BTreeMap::new();
        let mut outcomes1 = BTreeMap::new();
        outcomes1.insert(
            "ok".to_string(),
            RawStepTarget::StepRef("s2".to_string(), 2),
        );
        steps.insert(
            "s1".to_string(),
            RawStep::OperationStep {
                op: "op1".to_string(),
                persona: "admin".to_string(),
                outcomes: outcomes1,
                on_failure: Some(test_failure_handler()),
                line: 2,
            },
        );
        let mut outcomes2 = BTreeMap::new();
        outcomes2.insert(
            "ok".to_string(),
            RawStepTarget::StepRef("s1".to_string(), 3),
        );
        steps.insert(
            "s2".to_string(),
            RawStep::OperationStep {
                op: "op2".to_string(),
                persona: "admin".to_string(),
                outcomes: outcomes2,
                on_failure: Some(test_failure_handler()),
                line: 3,
            },
        );
        let prov = test_prov();
        let err = flow::detect_step_cycle("test_flow", "s1", &steps, &prov).unwrap_err();
        assert_eq!(err.pass, 5);
        assert!(err.message.contains("cycle detected"));
    }

    #[test]
    fn trigger_acyclicity_detects_cycle() {
        // A -> B -> A triggers form a cycle
        let triggers = vec![
            RawTrigger {
                source_contract: "c1".to_string(),
                source_flow: "f1".to_string(),
                on: "success".to_string(),
                target_contract: "c2".to_string(),
                target_flow: "f2".to_string(),
                persona: "p".to_string(),
            },
            RawTrigger {
                source_contract: "c2".to_string(),
                source_flow: "f2".to_string(),
                on: "success".to_string(),
                target_contract: "c1".to_string(),
                target_flow: "f1".to_string(),
                persona: "p".to_string(),
            },
        ];
        let prov = test_prov();
        let err = system::validate_trigger_acyclicity("sys1", &triggers, &prov).unwrap_err();
        assert_eq!(err.pass, 5);
        assert!(err.message.contains("trigger cycle detected"));
    }

    #[test]
    fn trigger_acyclicity_no_cycle() {
        // A -> B (no cycle)
        let triggers = vec![RawTrigger {
            source_contract: "c1".to_string(),
            source_flow: "f1".to_string(),
            on: "success".to_string(),
            target_contract: "c2".to_string(),
            target_flow: "f2".to_string(),
            persona: "p".to_string(),
        }];
        let prov = test_prov();
        assert!(system::validate_trigger_acyclicity("sys1", &triggers, &prov).is_ok());
    }
}
