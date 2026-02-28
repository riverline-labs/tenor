//! Pass 2: Construct indexing -- build a lookup index by (kind, id)
//! and detect duplicate ids within the same file.

use crate::ast::*;
use crate::error::ElabError;
use std::collections::HashMap;

/// A lookup index over the construct list, built in Pass 2.
pub struct Index {
    pub facts: HashMap<String, Provenance>,
    pub entities: HashMap<String, Provenance>,
    pub rules: HashMap<String, Provenance>,
    pub operations: HashMap<String, Provenance>,
    pub flows: HashMap<String, Provenance>,
    pub type_decls: HashMap<String, Provenance>,
    pub personas: HashMap<String, Provenance>,
    pub systems: HashMap<String, Provenance>,
    pub sources: HashMap<String, Provenance>,
    /// Map from rule_id -> verdict_type name produced by that rule
    pub rule_verdicts: HashMap<String, String>,
    /// Map from verdict_type -> (rule_id, stratum) of the producing rule
    pub verdict_strata: HashMap<String, (String, i64)>,
    /// Map from operation_id -> declared outcomes (empty vec = implicit ["success"])
    pub operation_outcomes: HashMap<String, Vec<String>>,
    /// Map from operation_id -> allowed_personas list
    pub operation_allowed_personas: HashMap<String, Vec<String>>,
}

pub fn build_index(constructs: &[RawConstruct]) -> Result<Index, ElabError> {
    let mut idx = Index {
        facts: HashMap::new(),
        entities: HashMap::new(),
        rules: HashMap::new(),
        operations: HashMap::new(),
        flows: HashMap::new(),
        type_decls: HashMap::new(),
        personas: HashMap::new(),
        systems: HashMap::new(),
        sources: HashMap::new(),
        rule_verdicts: HashMap::new(),
        verdict_strata: HashMap::new(),
        operation_outcomes: HashMap::new(),
        operation_allowed_personas: HashMap::new(),
    };

    for c in constructs {
        match c {
            RawConstruct::Fact { id, prov, .. } => {
                if let Some(first) = idx.facts.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Fact"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Fact id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.facts.insert(id.clone(), prov.clone());
            }
            RawConstruct::Entity { id, prov, .. } => {
                if let Some(first) = idx.entities.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Entity"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Entity id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.entities.insert(id.clone(), prov.clone());
            }
            RawConstruct::Rule {
                id,
                verdict_type,
                stratum,
                prov,
                ..
            } => {
                if let Some(first) = idx.rules.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Rule"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Rule id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.rules.insert(id.clone(), prov.clone());
                idx.rule_verdicts.insert(id.clone(), verdict_type.clone());
                idx.verdict_strata
                    .insert(verdict_type.clone(), (id.clone(), *stratum));
            }
            RawConstruct::Operation {
                id,
                outcomes,
                allowed_personas,
                prov,
                ..
            } => {
                if let Some(first) = idx.operations.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Operation"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Operation id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.operations.insert(id.clone(), prov.clone());
                idx.operation_outcomes.insert(id.clone(), outcomes.clone());
                idx.operation_allowed_personas
                    .insert(id.clone(), allowed_personas.clone());
            }
            RawConstruct::Flow { id, prov, .. } => {
                if let Some(first) = idx.flows.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Flow"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Flow id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.flows.insert(id.clone(), prov.clone());
            }
            RawConstruct::TypeDecl { id, prov, .. } => {
                if let Some(first) = idx.type_decls.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("TypeDecl"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate TypeDecl id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.type_decls.insert(id.clone(), prov.clone());
            }
            RawConstruct::Persona { id, prov, .. } => {
                if let Some(first) = idx.personas.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Persona"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Persona id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.personas.insert(id.clone(), prov.clone());
            }
            RawConstruct::System { id, prov, .. } => {
                if let Some(first) = idx.systems.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("System"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate System id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.systems.insert(id.clone(), prov.clone());
            }
            RawConstruct::Source { id, prov, .. } => {
                if let Some(first) = idx.sources.get(id) {
                    return Err(ElabError::new(
                        2,
                        Some("Source"),
                        Some(id),
                        Some("id"),
                        &prov.file,
                        prov.line,
                        format!(
                            "duplicate Source id '{}': first declared at line {}",
                            id, first.line
                        ),
                    ));
                }
                idx.sources.insert(id.clone(), prov.clone());
            }
            RawConstruct::Import { .. } => {}
        }
    }

    Ok(idx)
}
