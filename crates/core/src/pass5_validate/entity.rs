//! Entity validation functions.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{HashMap, HashSet};

// ── Entity validation ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_entity(
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

pub(super) fn validate_entity_dag(
    constructs: &[RawConstruct],
    _index: &Index,
) -> Result<(), ElabError> {
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
