//! System validation functions.

use crate::ast::*;
use crate::error::ElabError;
use std::collections::{HashMap, HashSet};

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
pub(super) fn validate_system(
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

    // C-SYS-03: Member file must not be a System file (no nested Systems).
    // Collect file paths of all System constructs in the bundle.
    let system_files: HashSet<&str> = constructs
        .iter()
        .filter_map(|c| match c {
            RawConstruct::System { prov, .. } => Some(prov.file.as_str()),
            _ => None,
        })
        .collect();
    for (mid, path) in members {
        if system_files.contains(path.as_str()) {
            return Err(ElabError::new(
                5,
                Some("System"),
                Some(id),
                Some("members"),
                &prov.file,
                prov.line,
                format!(
                    "member '{}' is a System file; nested Systems are not permitted",
                    mid
                ),
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
pub(super) fn validate_trigger_acyclicity(
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
