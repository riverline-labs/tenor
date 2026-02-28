//! Parallel entity conflict validation functions.

use crate::ast::*;
use crate::error::ElabError;
use std::collections::{BTreeMap, HashMap};

// ── Parallel entity conflict validation ───────────────────────────────────────

fn collect_branch_entity_effects<'a>(
    branch: &'a RawBranch,
    op_entities: &HashMap<&'a str, Vec<&'a str>>,
    flow_map: &HashMap<&'a str, &'a BTreeMap<String, RawStep>>,
) -> HashMap<String, Option<String>> {
    let mut effects: HashMap<String, Option<String>> = HashMap::new();

    for step in branch.steps.values() {
        match step {
            RawStep::OperationStep { op, .. } => {
                if let Some(entities) = op_entities.get(op.as_str()) {
                    for &entity in entities {
                        effects.entry(entity.to_owned()).or_insert(None);
                    }
                }
            }
            RawStep::SubFlowStep { flow: flow_id, .. } => {
                if let Some(flow_steps) = flow_map.get(flow_id.as_str()) {
                    for (_sub_step_id, sub_step) in flow_steps.iter() {
                        if let RawStep::OperationStep { op, .. } = sub_step {
                            if let Some(entities) = op_entities.get(op.as_str()) {
                                for &entity in entities {
                                    let trace =
                                        format!("SubFlowStep \u{2192} {} \u{2192} {}", flow_id, op);
                                    effects.entry(entity.to_owned()).or_insert(Some(trace));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    effects
}

pub(super) fn validate_parallel_conflicts(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut op_entities: HashMap<&str, Vec<&str>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Operation { id, effects, .. } = c {
            let entities: Vec<&str> = effects.iter().map(|(e, _, _, _, _)| e.as_str()).collect();
            op_entities.insert(id.as_str(), entities);
        }
    }

    let mut flow_map: HashMap<&str, &BTreeMap<String, RawStep>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Flow { id, steps, .. } = c {
            flow_map.insert(id.as_str(), steps);
        }
    }

    for c in constructs {
        if let RawConstruct::Flow {
            id: flow_id,
            steps,
            prov,
            ..
        } = c
        {
            for (step_id, step) in steps {
                if let RawStep::ParallelStep {
                    branches,
                    branches_line,
                    ..
                } = step
                {
                    let branch_effects: Vec<(&str, HashMap<String, Option<String>>)> = branches
                        .iter()
                        .map(|b| {
                            (
                                b.id.as_str(),
                                collect_branch_entity_effects(b, &op_entities, &flow_map),
                            )
                        })
                        .collect();

                    for i in 0..branch_effects.len() {
                        for j in (i + 1)..branch_effects.len() {
                            let (b1_id, b1_effects) = &branch_effects[i];
                            let (b2_id, b2_effects) = &branch_effects[j];

                            let mut b1_sorted: Vec<&String> = b1_effects.keys().collect();
                            b1_sorted.sort_unstable();

                            for entity in b1_sorted {
                                if let Some(b2_trace) = b2_effects.get(entity) {
                                    // SAFETY: entity came from b1_effects.keys(), so get() is guaranteed
                                    if let Some(b1_trace) = b1_effects.get(entity) {
                                        let msg = if b1_trace.is_none() && b2_trace.is_none() {
                                            format!(
                                                "parallel branches '{}' and '{}' both declare effects on entity '{}'; parallel branch entity effect sets must be disjoint",
                                                b1_id, b2_id, entity
                                            )
                                        } else {
                                            let (transitive_id, trace) = if let Some(t) = b1_trace {
                                                (*b1_id, t.as_str())
                                            } else if let Some(t) = b2_trace.as_ref() {
                                                (*b2_id, t.as_str())
                                            } else {
                                                // Both are None -- unreachable since we checked !both-none above
                                                (*b1_id, "direct")
                                            };
                                            format!(
                                                "parallel branches '{}' and '{}' both affect entity '{}' ({} transitively through {}); parallel branch entity effect sets must be disjoint",
                                                b1_id, b2_id, entity, transitive_id, trace
                                            )
                                        };
                                        return Err(ElabError::new(
                                            5,
                                            Some("Flow"),
                                            Some(flow_id),
                                            Some(&format!("steps.{}.branches", step_id)),
                                            &prov.file,
                                            *branches_line,
                                            msg,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
