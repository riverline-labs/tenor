//! Operation validation functions.

use crate::ast::*;
use crate::error::ElabError;
use crate::pass2_index::Index;
use std::collections::{HashMap, HashSet};

// ── Operation validation ──────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_operation(
    id: &str,
    allowed_personas: &[String],
    allowed_personas_line: u32,
    effects: &[(String, String, String, Option<String>, u32)],
    outcomes: &[String],
    error_contract: &[String],
    prov: &Provenance,
    index: &Index,
) -> Result<(), ElabError> {
    // Validate outcome uniqueness (§9 — outcome labels must be unique)
    {
        let mut seen: HashSet<&str> = HashSet::new();
        for outcome in outcomes {
            if !seen.insert(outcome.as_str()) {
                return Err(ElabError::new(
                    5,
                    Some("Operation"),
                    Some(id),
                    Some("outcomes"),
                    &prov.file,
                    prov.line,
                    format!(
                        "duplicate outcome '{}'; outcome labels must be unique within an Operation",
                        outcome
                    ),
                ));
            }
        }
    }

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

pub(super) fn validate_operation_transitions(
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
