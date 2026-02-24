//! Pass 5: Construct validation -- structural checks on Entity, Rule,
//! Operation, Flow, and System constructs.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

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
                validate_entity(
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
                validate_rule(
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
                validate_operation(
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
                validate_flow(id, entry, *entry_line, steps, prov, index)?;
            }
            RawConstruct::System {
                id,
                members,
                shared_personas,
                triggers,
                shared_entities,
                prov,
            } => {
                validate_system(
                    id,
                    members,
                    shared_personas,
                    triggers,
                    shared_entities,
                    prov,
                    constructs,
                )?;
            }
            _ => {}
        }
    }

    validate_entity_dag(constructs, index)?;
    validate_flow_reference_graph(constructs)?;
    validate_parallel_conflicts(constructs)?;

    Ok(())
}

pub fn validate_operation_transitions(
    constructs: &[RawConstruct],
    _index: &Index,
) -> Result<(), ElabError> {
    let mut entity_transitions: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Entity {
            id, transitions, ..
        } = c
        {
            let list = entity_transitions.entry(id.as_str()).or_default();
            for (f, t, _) in transitions {
                list.push((f.as_str(), t.as_str()));
            }
        }
    }

    for c in constructs {
        if let RawConstruct::Operation {
            id, effects, prov, ..
        } = c
        {
            for (entity_id, from, to, _outcome, e_line) in effects {
                if let Some(transitions) = entity_transitions.get(entity_id.as_str()) {
                    if !transitions.iter().any(|(f, t)| f == from && t == to) {
                        let declared: Vec<String> = transitions
                            .iter()
                            .map(|(f, t)| format!("({}, {})", f, t))
                            .collect();
                        return Err(ElabError::new(
                            5, Some("Operation"), Some(id),
                            Some("effects"),
                            &prov.file, *e_line,
                            format!(
                                "effect ({}, {}, {}) is not a declared transition in entity {}; declared transitions are: [{}]",
                                entity_id, from, to, entity_id, declared.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Entity validation ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn validate_entity(
    id: &str,
    states: &[String],
    initial: &str,
    initial_line: u32,
    transitions: &[(String, String, u32)],
    _parent: Option<&str>,
    _parent_line: Option<u32>,
    prov: &Provenance,
    _index: &Index,
) -> Result<(), ElabError> {
    let state_set: HashSet<&str> = states.iter().map(String::as_str).collect();
    let states_list: Vec<&str> = states.iter().map(String::as_str).collect();

    if !state_set.contains(initial) {
        return Err(ElabError::new(
            5,
            Some("Entity"),
            Some(id),
            Some("initial"),
            &prov.file,
            initial_line,
            format!(
                "initial state '{}' is not declared in states: [{}]",
                initial,
                states_list.join(", ")
            ),
        ));
    }

    for (from, to, t_line) in transitions {
        if !state_set.contains(from.as_str()) {
            return Err(ElabError::new(
                5,
                Some("Entity"),
                Some(id),
                Some("transitions"),
                &prov.file,
                *t_line,
                format!(
                    "transition endpoint '{}' is not declared in states: [{}]",
                    from,
                    states_list.join(", ")
                ),
            ));
        }
        if !state_set.contains(to.as_str()) {
            return Err(ElabError::new(
                5,
                Some("Entity"),
                Some(id),
                Some("transitions"),
                &prov.file,
                *t_line,
                format!(
                    "transition endpoint '{}' is not declared in states: [{}]",
                    to,
                    states_list.join(", ")
                ),
            ));
        }
    }

    Ok(())
}

fn validate_entity_dag(constructs: &[RawConstruct], _index: &Index) -> Result<(), ElabError> {
    let mut parents: HashMap<&str, (&str, u32, &Provenance)> = HashMap::new();
    for c in constructs {
        if let RawConstruct::Entity {
            id,
            parent: Some(p),
            parent_line,
            prov,
            ..
        } = c
        {
            parents.insert(
                id.as_str(),
                (p.as_str(), parent_line.unwrap_or(prov.line), prov),
            );
        }
    }

    let mut sorted_ids: Vec<&str> = parents.keys().copied().collect();
    sorted_ids.sort_unstable();

    for start in sorted_ids {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut cur = start;
        visited.insert(cur);
        while let Some((p, p_line, prov)) = parents.get(cur) {
            if visited.contains(p) {
                let mut path = vec![cur.to_string()];
                let mut node = cur;
                while let Some((next, _, _)) = parents.get(node) {
                    path.push(next.to_string());
                    if *next == cur {
                        break;
                    }
                    node = next;
                }
                return Err(ElabError::new(
                    5,
                    Some("Entity"),
                    Some(cur),
                    Some("parent"),
                    prov.file.as_str(),
                    *p_line,
                    format!(
                        "entity hierarchy cycle detected: {}",
                        path.join(" \u{2192} ")
                    ),
                ));
            }
            visited.insert(p);
            let _ = p_line;
            cur = p;
        }
    }
    Ok(())
}

// ── Rule validation ───────────────────────────────────────────────────────────

fn validate_rule(
    id: &str,
    stratum: i64,
    stratum_line: u32,
    when: &RawExpr,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    if stratum < 0 {
        return Err(ElabError::new(
            5,
            Some("Rule"),
            Some(id),
            Some("stratum"),
            &prov.file,
            stratum_line,
            format!("stratum must be a non-negative integer; got {}", stratum),
        ));
    }

    validate_verdict_refs_in_expr(when, id, stratum, prov, index, produced_verdicts)?;

    Ok(())
}

fn validate_verdict_refs_in_expr(
    expr: &RawExpr,
    rule_id: &str,
    rule_stratum: i64,
    prov: &Provenance,
    index: &Index,
    produced_verdicts: &HashSet<String>,
) -> Result<(), ElabError> {
    match expr {
        RawExpr::VerdictPresent { id: vid, line } => {
            if !produced_verdicts.contains(vid.as_str()) {
                return Err(ElabError::new(
                    5, Some("Rule"), Some(rule_id),
                    Some("body.when"),
                    &prov.file, *line,
                    format!("unresolved VerdictType reference: '{}' is not produced by any rule in this contract", vid),
                ));
            }
            if let Some((producing_rule_id, producing_stratum)) = find_producing_rule(vid, index) {
                if producing_stratum >= rule_stratum {
                    return Err(ElabError::new(
                        5, Some("Rule"), Some(rule_id),
                        Some("body.when"),
                        &prov.file, *line,
                        format!(
                            "stratum violation: rule '{}' at stratum {} references verdict '{}' produced by rule '{}' at stratum {}; verdict_refs must reference strata strictly less than the referencing rule's stratum",
                            rule_id, rule_stratum, vid, producing_rule_id, producing_stratum
                        ),
                    ));
                }
            }
        }
        RawExpr::And(a, b) | RawExpr::Or(a, b) => {
            validate_verdict_refs_in_expr(
                a,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
            validate_verdict_refs_in_expr(
                b,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
        }
        RawExpr::Not(e) => {
            validate_verdict_refs_in_expr(
                e,
                rule_id,
                rule_stratum,
                prov,
                index,
                produced_verdicts,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn find_producing_rule(verdict_type: &str, index: &Index) -> Option<(String, i64)> {
    index.verdict_strata.get(verdict_type).cloned()
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

// ── Operation validation ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn validate_operation(
    id: &str,
    allowed_personas: &[String],
    allowed_personas_line: u32,
    effects: &[(String, String, String, Option<String>, u32)],
    outcomes: &[String],
    error_contract: &[String],
    prov: &Provenance,
    index: &Index,
) -> Result<(), ElabError> {
    if allowed_personas.is_empty() {
        return Err(ElabError::new(
            5, Some("Operation"), Some(id),
            Some("allowed_personas"),
            &prov.file, allowed_personas_line,
            "allowed_personas must be non-empty; an Operation with no allowed personas can never be invoked".to_string(),
        ));
    }

    // Validate persona references if persona constructs exist in the index
    if !index.personas.is_empty() {
        for persona_name in allowed_personas {
            if !index.personas.contains_key(persona_name.as_str()) {
                return Err(ElabError::new(
                    5,
                    Some("Operation"),
                    Some(id),
                    Some("allowed_personas"),
                    &prov.file,
                    allowed_personas_line,
                    format!("undeclared persona '{}' in allowed_personas", persona_name),
                ));
            }
        }
    }

    for (entity_id, _from, _to, _outcome, e_line) in effects {
        if !index.entities.contains_key(entity_id.as_str()) {
            return Err(ElabError::new(
                5,
                Some("Operation"),
                Some(id),
                Some("effects"),
                &prov.file,
                *e_line,
                format!("effect references undeclared entity '{}'", entity_id),
            ));
        }
    }

    // Validate effect-to-outcome mapping for multi-outcome operations
    if outcomes.len() >= 2 {
        let outcome_set: HashSet<&str> = outcomes.iter().map(String::as_str).collect();
        for (entity_id, from, to, outcome_label, e_line) in effects {
            match outcome_label {
                None => {
                    return Err(ElabError::new(
                        5,
                        Some("Operation"),
                        Some(id),
                        Some("effects"),
                        &prov.file,
                        *e_line,
                        format!(
                            "effect ({}, {}, {}) is missing an outcome label; multi-outcome operations require every effect to specify which outcome it belongs to",
                            entity_id, from, to
                        ),
                    ));
                }
                Some(label) => {
                    if !outcome_set.contains(label.as_str()) {
                        return Err(ElabError::new(
                            5,
                            Some("Operation"),
                            Some(id),
                            Some("effects"),
                            &prov.file,
                            *e_line,
                            format!(
                                "effect ({}, {}, {}) references undeclared outcome '{}'; declared outcomes are: [{}]",
                                entity_id, from, to, label, outcomes.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }

    // Validate outcomes are disjoint from error_contract names
    if !outcomes.is_empty() {
        let outcome_set: HashSet<&str> = outcomes.iter().map(String::as_str).collect();
        for ec in error_contract {
            if outcome_set.contains(ec.as_str()) {
                return Err(ElabError::new(
                    5, Some("Operation"), Some(id),
                    Some("outcomes"),
                    &prov.file, prov.line,
                    format!("outcome '{}' conflicts with error_contract; outcomes and error_contract must be disjoint", ec),
                ));
            }
        }
    }

    Ok(())
}

// ── Flow validation ───────────────────────────────────────────────────────────

fn validate_flow(
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

fn detect_step_cycle(
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

fn validate_flow_reference_graph(constructs: &[RawConstruct]) -> Result<(), ElabError> {
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

fn validate_parallel_conflicts(constructs: &[RawConstruct]) -> Result<(), ElabError> {
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

// ── System validation ────────────────────────────────────────────────────────

/// Validate System construct structural constraints (C-SYS-01 through C-SYS-17).
///
/// Cross-contract deep validation (C-SYS-06, C-SYS-09, C-SYS-10, C-SYS-12,
/// C-SYS-13, C-SYS-14) requires elaborated member contracts, which are not
/// available in the standard single-file pipeline. Those constraints are
/// validated when the System is elaborated with member contracts loaded.
///
/// This function validates structural constraints that can be checked from the
/// System construct alone:
///   - C-SYS-01: Member id uniqueness
///   - C-SYS-07/08: Trigger source/target contracts are declared members
///   - C-SYS-11: Trigger outcome is a valid terminal outcome
///   - C-SYS-15: Trigger graph acyclicity
///   - C-SYS-16: Shared persona contracts are members
///   - C-SYS-17: Shared entity contracts are members
///   - At least one member declared
///   - Shared persona bindings reference at least 2 contracts
///   - No self-referential triggers (same contract + same flow)
fn validate_system(
    id: &str,
    members: &[(String, String)],
    shared_personas: &[(String, Vec<String>)],
    triggers: &[RawTrigger],
    shared_entities: &[(String, Vec<String>)],
    prov: &Provenance,
    constructs: &[RawConstruct],
) -> Result<(), ElabError> {
    // At least one member required
    if members.is_empty() {
        return Err(ElabError::new(
            5,
            Some("System"),
            Some(id),
            Some("members"),
            &prov.file,
            prov.line,
            "System must declare at least one member contract".to_string(),
        ));
    }

    // C-SYS-01: Member id uniqueness
    let mut member_set: HashSet<&str> = HashSet::new();
    for (mid, _path) in members {
        if !member_set.insert(mid.as_str()) {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("members"),
                &prov.file,
                prov.line,
                format!("duplicate member id '{}' in System '{}'", mid, id),
            ));
        }
    }

    // C-SYS-16: Shared persona contracts are members
    for (persona_id, contracts) in shared_personas {
        for cid in contracts {
            if !member_set.contains(cid.as_str()) {
                return Err(ElabError::new(
                    5,
                    Some("System"),
                    Some(id),
                    Some("shared_personas"),
                    &prov.file,
                    prov.line,
                    format!(
                        "contract '{}' in shared_persona '{}' is not a System member",
                        cid, persona_id
                    ),
                ));
            }
        }
        // Shared persona binding with fewer than 2 contracts is meaningless
        if contracts.len() < 2 {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("shared_personas"),
                &prov.file,
                prov.line,
                format!(
                    "shared_persona '{}' must reference at least 2 member contracts; got {}",
                    persona_id,
                    contracts.len()
                ),
            ));
        }
    }

    // Trigger validation
    let valid_outcomes: HashSet<&str> = ["success", "failure", "escalation"]
        .iter()
        .copied()
        .collect();

    for trigger in triggers {
        // C-SYS-07: Source contract is a member
        if !member_set.contains(trigger.source_contract.as_str()) {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("triggers"),
                &prov.file,
                prov.line,
                format!(
                    "trigger source contract '{}' is not a System member",
                    trigger.source_contract
                ),
            ));
        }

        // C-SYS-08: Target contract is a member
        if !member_set.contains(trigger.target_contract.as_str()) {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("triggers"),
                &prov.file,
                prov.line,
                format!(
                    "trigger target contract '{}' is not a System member",
                    trigger.target_contract
                ),
            ));
        }

        // C-SYS-11: Trigger outcome validity
        if !valid_outcomes.contains(trigger.on.as_str()) {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("triggers"),
                &prov.file,
                prov.line,
                format!(
                    "invalid trigger outcome '{}'; must be 'success', 'failure', or 'escalation'",
                    trigger.on
                ),
            ));
        }

        // Self-referential trigger check (same contract AND same flow)
        if trigger.source_contract == trigger.target_contract
            && trigger.source_flow == trigger.target_flow
        {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("triggers"),
                &prov.file,
                prov.line,
                format!(
                    "self-referential trigger: {}.{} triggers itself",
                    trigger.source_contract, trigger.source_flow
                ),
            ));
        }

        // C-SYS-12: Trigger persona existence + entry operation allowed_personas
        // Look up the target flow in constructs, find its entry step, and if
        // it's an OperationStep, verify the trigger persona is in allowed_personas.
        if let Some(target_flow) = constructs.iter().find_map(|c| match c {
            RawConstruct::Flow {
                id: fid,
                steps,
                entry,
                ..
            } if fid == &trigger.target_flow => Some((entry, steps)),
            _ => None,
        }) {
            let (entry, steps) = target_flow;

            // Check persona existence: look for a Persona construct matching
            // the trigger's persona in the constructs list
            let persona_exists = constructs.iter().any(|c| {
                matches!(
                    c,
                    RawConstruct::Persona { id: pid, .. } if pid == &trigger.persona
                )
            });
            if !persona_exists {
                return Err(ElabError::new(
                    5,
                    Some("System"),
                    Some(id),
                    Some("triggers"),
                    &prov.file,
                    prov.line,
                    format!(
                        "persona '{}' not declared in target contract '{}'",
                        trigger.persona, trigger.target_contract
                    ),
                ));
            }

            // If entry step is an OperationStep, check allowed_personas
            if let Some(RawStep::OperationStep { op, .. }) = steps.get(entry.as_str()) {
                // Look up the Operation's allowed_personas
                let allowed: Option<&Vec<String>> = constructs.iter().find_map(|c| match c {
                    RawConstruct::Operation {
                        id: oid,
                        allowed_personas,
                        ..
                    } if oid == op => Some(allowed_personas),
                    _ => None,
                });
                if let Some(allowed) = allowed {
                    if !allowed.iter().any(|p| p == &trigger.persona) {
                        return Err(ElabError::new(
                            5,
                            Some("System"),
                            Some(id),
                            Some("triggers"),
                            &prov.file,
                            prov.line,
                            format!(
                                "trigger persona '{}' not in allowed_personas of entry operation '{}' in target flow '{}'",
                                trigger.persona, op, trigger.target_flow
                            ),
                        ));
                    }
                }
            }
        }
    }

    // C-SYS-17: Shared entity contracts are members
    for (entity_id, contracts) in shared_entities {
        for cid in contracts {
            if !member_set.contains(cid.as_str()) {
                return Err(ElabError::new(
                    5,
                    Some("System"),
                    Some(id),
                    Some("shared_entities"),
                    &prov.file,
                    prov.line,
                    format!(
                        "contract '{}' in shared_entity '{}' is not a System member",
                        cid, entity_id
                    ),
                ));
            }
        }
        // Shared entity with fewer than 2 contracts is meaningless
        if contracts.len() < 2 {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("shared_entities"),
                &prov.file,
                prov.line,
                format!(
                    "shared_entity '{}' must reference at least 2 member contracts; got {}",
                    entity_id,
                    contracts.len()
                ),
            ));
        }
    }

    // C-SYS-15: Trigger graph acyclicity
    validate_trigger_acyclicity(id, triggers, prov)?;

    Ok(())
}

/// C-SYS-15: Check that the trigger graph (nodes = (contract, flow) pairs,
/// edges = trigger bindings) is acyclic using DFS cycle detection.
fn validate_trigger_acyclicity(
    system_id: &str,
    triggers: &[RawTrigger],
    prov: &Provenance,
) -> Result<(), ElabError> {
    if triggers.is_empty() {
        return Ok(());
    }

    // Build adjacency list: (contract, flow) -> [(contract, flow)]
    let mut adj: HashMap<(&str, &str), Vec<(&str, &str)>> = HashMap::new();
    let mut nodes: HashSet<(&str, &str)> = HashSet::new();
    for t in triggers {
        let src = (t.source_contract.as_str(), t.source_flow.as_str());
        let tgt = (t.target_contract.as_str(), t.target_flow.as_str());
        adj.entry(src).or_default().push(tgt);
        nodes.insert(src);
        nodes.insert(tgt);
    }

    // DFS cycle detection
    let mut visited: HashSet<(&str, &str)> = HashSet::new();
    let mut in_path: HashSet<(&str, &str)> = HashSet::new();
    let mut path: Vec<(&str, &str)> = Vec::new();

    let mut sorted_nodes: Vec<(&str, &str)> = nodes.into_iter().collect();
    sorted_nodes.sort_unstable();

    for &node in &sorted_nodes {
        if !visited.contains(&node) {
            trigger_dfs(
                node,
                &adj,
                &mut visited,
                &mut in_path,
                &mut path,
                system_id,
                prov,
            )?;
        }
    }
    Ok(())
}

fn trigger_dfs<'a>(
    node: (&'a str, &'a str),
    adj: &HashMap<(&'a str, &'a str), Vec<(&'a str, &'a str)>>,
    visited: &mut HashSet<(&'a str, &'a str)>,
    in_path: &mut HashSet<(&'a str, &'a str)>,
    path: &mut Vec<(&'a str, &'a str)>,
    system_id: &str,
    prov: &Provenance,
) -> Result<(), ElabError> {
    path.push(node);
    in_path.insert(node);

    if let Some(neighbors) = adj.get(&node) {
        for &neighbor in neighbors {
            if in_path.contains(&neighbor) {
                let cycle_start = path
                    .iter()
                    .position(|&n| n == neighbor)
                    .ok_or_else(|| {
                        ElabError::new(
                            5,
                            Some("System"),
                            Some(system_id),
                            Some("triggers"),
                            &prov.file,
                            prov.line,
                            format!(
                                "internal error: trigger node ({}, {}) detected in cycle via in_path set but not found in path vector",
                                neighbor.0, neighbor.1
                            ),
                        )
                    })?;
                let mut cycle_nodes: Vec<String> = path[cycle_start..]
                    .iter()
                    .map(|(c, f)| format!("{}.{}", c, f))
                    .collect();
                cycle_nodes.push(format!("{}.{}", neighbor.0, neighbor.1));
                return Err(ElabError::new(
                    5,
                    Some("System"),
                    Some(system_id),
                    Some("triggers"),
                    &prov.file,
                    prov.line,
                    format!("trigger cycle detected: {}", cycle_nodes.join(" \u{2192} ")),
                ));
            }
            if !visited.contains(&neighbor) {
                trigger_dfs(neighbor, adj, visited, in_path, path, system_id, prov)?;
            }
        }
    }

    in_path.remove(&node);
    visited.insert(node);
    path.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(detect_step_cycle("test_flow", "s1", &steps, &prov).is_ok());
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
        let err = detect_step_cycle("test_flow", "s1", &steps, &prov).unwrap_err();
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
        let err = validate_trigger_acyclicity("sys1", &triggers, &prov).unwrap_err();
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
        assert!(validate_trigger_acyclicity("sys1", &triggers, &prov).is_ok());
    }
}
