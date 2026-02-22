//! S4 — Authority Topology analysis.
//!
//! For any declared Persona P and Entity state S, derives the set of
//! Operations P can invoke in S. Also derives whether a persona can
//! cause a specific state transition S -> S'.
//!
//! Spec reference: Section 15, S4.

use crate::bundle::AnalysisBundle;
use crate::s3a_admissibility::S3aResult;
use serde::Serialize;
use std::collections::BTreeMap;

/// Authority map for a single persona: entity -> state -> [operation_ids].
#[derive(Debug, Clone, Serialize)]
pub struct AuthorityMap {
    pub by_entity: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// A specific transition authority: persona can cause entity state change via operation.
#[derive(Debug, Clone, Serialize)]
pub struct TransitionAuthority {
    pub persona_id: String,
    pub entity_id: String,
    pub from_state: String,
    pub to_state: String,
    pub via_operation: String,
}

/// Aggregated S4 result.
#[derive(Debug, Clone, Serialize)]
pub struct S4Result {
    /// persona_id -> AuthorityMap
    pub persona_authority: BTreeMap<String, AuthorityMap>,
    /// All derivable transition authorities.
    pub transition_authorities: Vec<TransitionAuthority>,
    pub total_personas: usize,
    pub total_authority_entries: usize,
}

/// S4 — Derive the authority topology from S3a results.
///
/// Reorganizes S3a's (entity, state, persona) -> [ops] into
/// persona -> entity -> state -> [ops], and derives all
/// transition authorities.
pub fn analyze_authority(bundle: &AnalysisBundle, s3a: &S3aResult) -> S4Result {
    let mut persona_authority: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<String>>>> =
        BTreeMap::new();
    let mut transition_authorities = Vec::new();
    let mut total_authority_entries = 0;

    // Reorganize S3a: (entity, state, persona) -> [ops] => persona -> entity -> state -> [ops]
    for (key, ops) in &s3a.admissible_operations {
        let persona_map = persona_authority
            .entry(key.persona_id.clone())
            .or_default();
        let entity_map = persona_map.entry(key.entity_id.clone()).or_default();
        entity_map.insert(key.state.clone(), ops.clone());
        total_authority_entries += ops.len();
    }

    // Derive transition authorities
    for (key, ops) in &s3a.admissible_operations {
        for op_id in ops {
            // Find the operation and its effects
            if let Some(operation) = bundle.operations.iter().find(|o| o.id == *op_id) {
                for effect in &operation.effects {
                    if effect.entity_id == key.entity_id && effect.from_state == key.state {
                        transition_authorities.push(TransitionAuthority {
                            persona_id: key.persona_id.clone(),
                            entity_id: key.entity_id.clone(),
                            from_state: key.state.clone(),
                            to_state: effect.to_state.clone(),
                            via_operation: op_id.clone(),
                        });
                    }
                }
            }
        }
    }

    // Sort transition authorities for deterministic output
    transition_authorities.sort_by(|a, b| {
        (&a.persona_id, &a.entity_id, &a.from_state, &a.to_state, &a.via_operation)
            .cmp(&(&b.persona_id, &b.entity_id, &b.from_state, &b.to_state, &b.via_operation))
    });

    let total_personas = persona_authority.len();

    // Convert to AuthorityMap structs
    let persona_authority = persona_authority
        .into_iter()
        .map(|(persona_id, entity_map)| {
            (
                persona_id,
                AuthorityMap {
                    by_entity: entity_map,
                },
            )
        })
        .collect();

    S4Result {
        persona_authority,
        transition_authorities,
        total_personas,
        total_authority_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::*;
    use crate::s3a_admissibility::analyze_admissibility;

    fn make_bundle_with(
        entities: Vec<AnalysisEntity>,
        personas: Vec<AnalysisPersona>,
        operations: Vec<AnalysisOperation>,
    ) -> AnalysisBundle {
        AnalysisBundle {
            entities,
            facts: vec![],
            rules: vec![],
            operations,
            flows: vec![],
            personas,
        }
    }

    #[test]
    fn test_single_persona_authority() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string(), "submitted".to_string()],
                initial: "draft".to_string(),
                transitions: vec![Transition { from: "draft".to_string(), to: "submitted".to_string() }],
                parent: None,
            }],
            vec![AnalysisPersona { id: "admin".to_string() }],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: None,
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "submitted".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
        );

        let s3a = analyze_admissibility(&bundle);
        let result = analyze_authority(&bundle, &s3a);

        assert_eq!(result.total_personas, 1);
        assert!(result.persona_authority.contains_key("admin"));

        let admin = &result.persona_authority["admin"];
        assert!(admin.by_entity.contains_key("Order"));
        assert!(admin.by_entity["Order"].contains_key("draft"));
        assert_eq!(admin.by_entity["Order"]["draft"], vec!["submit"]);
    }

    #[test]
    fn test_transition_authority_derived() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string(), "submitted".to_string()],
                initial: "draft".to_string(),
                transitions: vec![Transition { from: "draft".to_string(), to: "submitted".to_string() }],
                parent: None,
            }],
            vec![AnalysisPersona { id: "admin".to_string() }],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: None,
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "submitted".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
        );

        let s3a = analyze_admissibility(&bundle);
        let result = analyze_authority(&bundle, &s3a);

        assert_eq!(result.transition_authorities.len(), 1);
        let ta = &result.transition_authorities[0];
        assert_eq!(ta.persona_id, "admin");
        assert_eq!(ta.entity_id, "Order");
        assert_eq!(ta.from_state, "draft");
        assert_eq!(ta.to_state, "submitted");
        assert_eq!(ta.via_operation, "submit");
    }

    #[test]
    fn test_multiple_personas_overlapping() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![
                AnalysisPersona { id: "admin".to_string() },
                AnalysisPersona { id: "user".to_string() },
            ],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string(), "user".to_string()],
                precondition: None,
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "submitted".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
        );

        let s3a = analyze_admissibility(&bundle);
        let result = analyze_authority(&bundle, &s3a);

        assert_eq!(result.total_personas, 2);
        assert!(result.persona_authority.contains_key("admin"));
        assert!(result.persona_authority.contains_key("user"));
    }

    #[test]
    fn test_persona_no_authority() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![
                AnalysisPersona { id: "admin".to_string() },
                AnalysisPersona { id: "viewer".to_string() },
            ],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()], // viewer NOT authorized
                precondition: None,
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "submitted".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
        );

        let s3a = analyze_admissibility(&bundle);
        let result = analyze_authority(&bundle, &s3a);

        assert!(result.persona_authority.contains_key("admin"));
        assert!(!result.persona_authority.contains_key("viewer"));
    }
}
