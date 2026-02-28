//! Flow validation functions.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// ── Flow validation ───────────────────────────────────────────────────────────

pub(super) fn validate_flow(
    id: &str,
    entry: &str,
    entry_line: u32,
    steps: &BTreeMap<String, RawStep>,
    prov: &Provenance,
    _index: &Index,
) -> Result<(), ElabError> {
    if !steps.contains_key(entry) {
        return Err(ElabError::new(
            5,
            Some("Flow"),
            Some(id),
            Some("entry"),
            &prov.file,
            entry_line,
            format!("entry step '{}' is not declared in steps", entry),
        ));
    }

    for (step_id, step) in steps {
        if let RawStep::OperationStep {
            on_failure: None,
            line,
            ..
        } = step
        {
            return Err(ElabError::new(
                5,
                Some("Flow"),
                Some(id),
                Some(&format!("steps.{}.on_failure", step_id)),
                &prov.file,
                *line,
                format!("OperationStep '{}' must declare a FailureHandler", step_id),
            ));
        }
    }

    for (step_id, step) in steps {
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for (label, target) in outcomes {
                    if let RawStepTarget::StepRef(r, ref_line) = target {
                        if !steps.contains_key(r.as_str()) {
                            return Err(ElabError::new(
                                5,
                                Some("Flow"),
                                Some(id),
                                Some(&format!("steps.{}.outcomes.{}", step_id, label)),
                                &prov.file,
                                *ref_line,
                                format!("step reference '{}' is not declared in steps", r),
                            ));
                        }
                    }
                }
            }
            RawStep::BranchStep {
                if_true, if_false, ..
            } => {
                if let RawStepTarget::StepRef(r, ref_line) = if_true {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5,
                            Some("Flow"),
                            Some(id),
                            Some(&format!("steps.{}.if_true", step_id)),
                            &prov.file,
                            *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
                if let RawStepTarget::StepRef(r, ref_line) = if_false {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5,
                            Some("Flow"),
                            Some(id),
                            Some(&format!("steps.{}.if_false", step_id)),
                            &prov.file,
                            *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
            }
            RawStep::HandoffStep { next, line, .. } => {
                if !steps.contains_key(next.as_str()) {
                    return Err(ElabError::new(
                        5,
                        Some("Flow"),
                        Some(id),
                        Some(&format!("steps.{}.next", step_id)),
                        &prov.file,
                        *line,
                        format!("step reference '{}' is not declared in steps", next),
                    ));
                }
            }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, ref_line) = on_success {
                    if !steps.contains_key(r.as_str()) {
                        return Err(ElabError::new(
                            5,
                            Some("Flow"),
                            Some(id),
                            Some(&format!("steps.{}.on_success", step_id)),
                            &prov.file,
                            *ref_line,
                            format!("step reference '{}' is not declared in steps", r),
                        ));
                    }
                }
            }
            RawStep::ParallelStep { .. } => {}
        }
    }

    detect_step_cycle(id, entry, steps, prov)?;

    Ok(())
}

pub(super) fn detect_step_cycle(
    flow_id: &str,
    _entry: &str,
    steps: &BTreeMap<String, RawStep>,
    prov: &Provenance,
) -> Result<(), ElabError> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sid, step) in steps {
        let mut neighbors: Vec<&str> = Vec::new();
        match step {
            RawStep::OperationStep { outcomes, .. } => {
                for t in outcomes.values() {
                    if let RawStepTarget::StepRef(r, _) = t {
                        neighbors.push(r.as_str());
                    }
                }
            }
            RawStep::BranchStep {
                if_true, if_false, ..
            } => {
                if let RawStepTarget::StepRef(r, _) = if_true {
                    neighbors.push(r.as_str());
                }
                if let RawStepTarget::StepRef(r, _) = if_false {
                    neighbors.push(r.as_str());
                }
            }
            RawStep::HandoffStep { next, .. } => {
                neighbors.push(next.as_str());
            }
            RawStep::SubFlowStep { on_success, .. } => {
                if let RawStepTarget::StepRef(r, _) = on_success {
                    neighbors.push(r.as_str());
                }
            }
            RawStep::ParallelStep { .. } => {}
        }
        adj.insert(sid.as_str(), neighbors);
    }

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for sid in steps.keys() {
        in_degree.entry(sid.as_str()).or_insert(0);
    }
    for neighbors in adj.values() {
        for &n in neighbors {
            *in_degree.entry(n).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut processed: HashSet<&str> = HashSet::new();
    while let Some(node) = queue.pop_front() {
        processed.insert(node);
        for &neighbor in adj.get(node).unwrap_or(&vec![]) {
            let deg = in_degree.get_mut(neighbor).ok_or_else(|| {
                ElabError::new(
                    5,
                    Some("Flow"),
                    Some(flow_id),
                    Some("steps"),
                    &prov.file,
                    prov.line,
                    format!(
                        "internal error: step neighbor '{}' not found in in_degree map",
                        neighbor
                    ),
                )
            })?;
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(neighbor);
            }
        }
    }

    if processed.len() < steps.len() {
        let mut cyclic: Vec<&str> = steps
            .keys()
            .map(String::as_str)
            .filter(|s| !processed.contains(*s))
            .collect();
        cyclic.sort_unstable();
        let report_line = cyclic
            .first()
            .and_then(|s| steps.get(*s))
            .map(|step| match step {
                RawStep::OperationStep { line, .. } => *line,
                RawStep::BranchStep { line, .. } => *line,
                RawStep::HandoffStep { line, .. } => *line,
                RawStep::SubFlowStep { line, .. } => *line,
                RawStep::ParallelStep { line, .. } => *line,
            })
            .unwrap_or(prov.line);
        let cycle_list = cyclic.join(", ");
        return Err(ElabError::new(
            5,
            Some("Flow"),
            Some(flow_id),
            Some("steps"),
            &prov.file,
            report_line,
            format!(
                "flow step graph is not acyclic: cycle detected involving steps [{}]",
                cycle_list
            ),
        ));
    }

    Ok(())
}

// ── Flow reference graph cycle detection ──────────────────────────────────────

pub(super) fn validate_flow_reference_graph(constructs: &[RawConstruct]) -> Result<(), ElabError> {
    let mut flows: HashMap<&str, (&Provenance, &BTreeMap<String, RawStep>)> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Flow {
            id, steps, prov, ..
        } = c
        {
            flows.insert(id.as_str(), (prov, steps));
        }
    }

    let mut flow_ids: Vec<&str> = flows.keys().copied().collect();
    flow_ids.sort_unstable();

    let mut visited: HashSet<&str> = HashSet::new();
    let mut in_path: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for &fid in &flow_ids {
        if !visited.contains(fid) {
            dfs_flow_refs(fid, &flows, &mut visited, &mut in_path, &mut path)?;
        }
    }
    Ok(())
}

fn collect_subflow_refs<'a>(
    step: &'a RawStep,
    step_id: &'a str,
    out: &mut Vec<(&'a str, u32, &'a str)>,
) {
    match step {
        RawStep::SubFlowStep {
            flow, flow_line, ..
        } => {
            out.push((step_id, *flow_line, flow.as_str()));
        }
        RawStep::ParallelStep { branches, .. } => {
            for branch in branches {
                for (branch_step_id, branch_step) in &branch.steps {
                    collect_subflow_refs(branch_step, branch_step_id.as_str(), out);
                }
            }
        }
        _ => {}
    }
}

fn dfs_flow_refs<'a>(
    flow_id: &'a str,
    flows: &HashMap<&'a str, (&'a Provenance, &'a BTreeMap<String, RawStep>)>,
    visited: &mut HashSet<&'a str>,
    in_path: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Result<(), ElabError> {
    path.push(flow_id);
    in_path.insert(flow_id);

    if let Some((prov, steps)) = flows.get(flow_id) {
        let mut sub_refs: Vec<(&str, u32, &str)> = Vec::new();
        for (step_id, step) in steps.iter() {
            collect_subflow_refs(step, step_id.as_str(), &mut sub_refs);
        }
        for (step_id, flow_line, ref_flow) in sub_refs {
            if in_path.contains(ref_flow) {
                let cycle_start = path
                    .iter()
                    .position(|&s| s == ref_flow)
                    .ok_or_else(|| {
                        ElabError::new(
                            5,
                            Some("Flow"),
                            Some(flow_id),
                            Some("steps"),
                            &prov.file,
                            prov.line,
                            format!(
                                "internal error: flow '{}' detected in cycle path via in_path set but not found in path vector",
                                ref_flow
                            ),
                        )
                    })?;
                let mut cycle_nodes: Vec<&str> = path[cycle_start..].to_vec();
                cycle_nodes.push(ref_flow);
                let cycle_str = cycle_nodes.join(" \u{2192} ");
                return Err(ElabError::new(
                    5,
                    Some("Flow"),
                    Some(flow_id),
                    Some(&format!("steps.{}.flow", step_id)),
                    &prov.file,
                    flow_line,
                    format!("flow reference cycle detected: {}", cycle_str),
                ));
            }
            if !visited.contains(ref_flow) && flows.contains_key(ref_flow) {
                dfs_flow_refs(ref_flow, flows, visited, in_path, path)?;
            }
        }
    }

    in_path.remove(flow_id);
    visited.insert(flow_id);
    path.pop();
    Ok(())
}
