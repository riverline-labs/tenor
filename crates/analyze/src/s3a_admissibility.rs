//! S3a — Structural Admissibility Per State analysis.
//!
//! For each Entity state and each persona, determines which Operations
//! are structurally admissible: the persona is authorized, the operation
//! has an effect transitioning from that state, and the precondition is
//! structurally satisfiable by type-level analysis.
//!
//! Spec reference: Section 15, S3a.
//! Complexity: O(|expression tree|) per precondition -- always feasible.

use crate::bundle::{AnalysisBundle, AnalysisFact};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// Key for the admissibility map: (entity_id, state, persona_id).
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdmissibilityKey {
    pub entity_id: String,
    pub state: String,
    pub persona_id: String,
}

impl fmt::Display for AdmissibilityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.entity_id, self.state, self.persona_id)
    }
}

/// Aggregated S3a result.
#[derive(Debug, Clone, Serialize)]
pub struct S3aResult {
    /// Maps (entity, state, persona) -> [admissible operation IDs].
    #[serde(serialize_with = "serialize_admissible_ops")]
    pub admissible_operations: BTreeMap<AdmissibilityKey, Vec<String>>,
    /// Total (entity, state, persona) combinations checked.
    pub total_combinations_checked: usize,
}

/// Custom serializer to convert AdmissibilityKey map keys to strings for JSON compatibility.
fn serialize_admissible_ops<S>(
    map: &BTreeMap<AdmissibilityKey, Vec<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut ser_map = serializer.serialize_map(Some(map.len()))?;
    for (key, value) in map {
        ser_map.serialize_entry(&key.to_string(), value)?;
    }
    ser_map.end()
}

/// S3a — Determine structurally admissible Operations per (entity, state, persona).
///
/// For each (entity E, state S, persona P) triple, finds Operations where:
/// 1. P is in operation.allowed_personas
/// 2. At least one effect has entity_id == E.id AND from_state == S
/// 3. The precondition (if present) is structurally satisfiable
pub fn analyze_admissibility(bundle: &AnalysisBundle) -> S3aResult {
    let mut admissible_operations = BTreeMap::new();
    let mut total_combinations = 0;

    for entity in &bundle.entities {
        for state in &entity.states {
            for persona in &bundle.personas {
                total_combinations += 1;
                let key = AdmissibilityKey {
                    entity_id: entity.id.clone(),
                    state: state.clone(),
                    persona_id: persona.id.clone(),
                };

                let mut ops = Vec::new();

                for operation in &bundle.operations {
                    // Check 1: persona is authorized
                    if !operation.allowed_personas.contains(&persona.id) {
                        continue;
                    }

                    // Check 2: operation has an effect from this state on this entity
                    let has_matching_effect = operation
                        .effects
                        .iter()
                        .any(|e| e.entity_id == entity.id && e.from_state == *state);
                    if !has_matching_effect {
                        continue;
                    }

                    // Check 3: precondition is structurally satisfiable
                    if let Some(ref precondition) = operation.precondition {
                        if !is_structurally_satisfiable(precondition, &bundle.facts) {
                            continue;
                        }
                    }
                    // No precondition = always structurally satisfiable

                    ops.push(operation.id.clone());
                }

                if !ops.is_empty() {
                    admissible_operations.insert(key, ops);
                }
            }
        }
    }

    S3aResult {
        admissible_operations,
        total_combinations_checked: total_combinations,
    }
}

/// Check if a predicate expression is structurally satisfiable by type-level analysis.
///
/// Walks the expression tree checking type compatibility:
/// - Compare: checks if operand types are compatible (e.g., Enum value in declared set)
/// - And: all children must be satisfiable
/// - Or: at least one child must be satisfiable
/// - Not: child must be satisfiable (negation doesn't change structural feasibility)
/// - verdict_present: always satisfiable (runtime-dependent)
/// - fact_ref: always satisfiable (fact exists)
///
/// Conservative: returns true when uncertain (structural satisfiability only).
fn is_structurally_satisfiable(expr: &serde_json::Value, facts: &[AnalysisFact]) -> bool {
    // Dispatch based on expression structure
    if expr.is_null() {
        return true;
    }

    // And expression: {"op": "and", "operands": [...]}
    if let Some(operands) = expr.get("operands") {
        if let Some(op) = expr.get("op").and_then(|o| o.as_str()) {
            if let Some(arr) = operands.as_array() {
                return match op {
                    "and" => arr
                        .iter()
                        .all(|child| is_structurally_satisfiable(child, facts)),
                    "or" => arr
                        .iter()
                        .any(|child| is_structurally_satisfiable(child, facts)),
                    _ => true, // Unknown op: conservative true
                };
            }
        }
    }

    // Not expression: {"op": "not", "operand": {...}}
    if let Some(operand) = expr.get("operand") {
        if expr.get("op").and_then(|o| o.as_str()) == Some("not") {
            return is_structurally_satisfiable(operand, facts);
        }
    }

    // Forall expression: {"quantifier": "forall", "body": {...}}
    if let Some(body) = expr.get("body") {
        if expr.get("quantifier").is_some() {
            return is_structurally_satisfiable(body, facts);
        }
    }

    // verdict_present: always structurally satisfiable (runtime-dependent)
    if expr.get("verdict_present").is_some() {
        return true;
    }

    // fact_ref: always structurally satisfiable
    if expr.get("fact_ref").is_some() {
        return true;
    }

    // Compare expression: has "left", "op", "right"
    if let (Some(left), Some(op), Some(right)) = (
        expr.get("left"),
        expr.get("op").and_then(|o| o.as_str()),
        expr.get("right"),
    ) {
        return is_comparison_satisfiable(left, op, right, facts);
    }

    // Unknown expression structure: conservatively satisfiable
    true
}

/// Check if a comparison is structurally satisfiable.
///
/// Key check: if one side is a fact_ref to an Enum fact and the other
/// is a string literal not in the Enum's declared values, the comparison
/// is structurally unsatisfiable (for equality; for inequality it's always true).
fn is_comparison_satisfiable(
    left: &serde_json::Value,
    op: &str,
    right: &serde_json::Value,
    facts: &[AnalysisFact],
) -> bool {
    // Check for Enum value mismatch: fact_ref to Enum compared with literal not in values
    if let Some(fact_id) = left.get("fact_ref").and_then(|f| f.as_str()) {
        if let Some(literal) = extract_string_literal(right) {
            if let Some(fact) = facts.iter().find(|f| f.id == fact_id) {
                if let Some(enum_values) = extract_enum_values(&fact.fact_type) {
                    // Enum fact compared with string literal
                    if op == "=" || op == "==" {
                        // Equality: unsatisfiable if literal not in enum values
                        return enum_values.contains(&literal);
                    }
                    // For != (not equal): always satisfiable if enum has > 1 value
                    // For other ops: conservatively satisfiable
                }
            }
        }
    }

    // Check the reverse: literal on left, fact_ref on right
    if let Some(fact_id) = right.get("fact_ref").and_then(|f| f.as_str()) {
        if let Some(literal) = extract_string_literal(left) {
            if let Some(fact) = facts.iter().find(|f| f.id == fact_id) {
                if let Some(enum_values) = extract_enum_values(&fact.fact_type) {
                    if op == "=" || op == "==" {
                        return enum_values.contains(&literal);
                    }
                }
            }
        }
    }

    // Check for Int range mismatch
    if let Some(fact_id) = left.get("fact_ref").and_then(|f| f.as_str()) {
        if let Some(lit_val) = extract_int_literal(right) {
            if let Some(fact) = facts.iter().find(|f| f.id == fact_id) {
                if let Some((min, max)) = extract_int_range(&fact.fact_type) {
                    if op == "=" || op == "==" {
                        return lit_val >= min && lit_val <= max;
                    }
                }
            }
        }
    }

    // Default: conservatively satisfiable
    true
}

/// Extract a string literal value from a predicate expression node.
fn extract_string_literal(expr: &serde_json::Value) -> Option<String> {
    // Literal format: {"literal": "value", "type": {...}}
    expr.get("literal")
        .and_then(|l| l.as_str())
        .map(|s| s.to_string())
}

/// Extract an integer literal value from a predicate expression node.
fn extract_int_literal(expr: &serde_json::Value) -> Option<i64> {
    expr.get("literal").and_then(|l| l.as_i64())
}

/// Extract Enum variant values from a type descriptor.
fn extract_enum_values(type_desc: &serde_json::Value) -> Option<Vec<String>> {
    if type_desc.get("base")?.as_str()? != "Enum" {
        return None;
    }
    let values = type_desc.get("values")?.as_array()?;
    Some(
        values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
    )
}

/// Extract Int range (min, max) from a type descriptor.
fn extract_int_range(type_desc: &serde_json::Value) -> Option<(i64, i64)> {
    if type_desc.get("base")?.as_str()? != "Int" {
        return None;
    }
    let min = type_desc.get("min")?.as_i64()?;
    let max = type_desc.get("max")?.as_i64()?;
    Some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::*;
    use serde_json::json;

    fn make_bundle_with(
        entities: Vec<AnalysisEntity>,
        personas: Vec<AnalysisPersona>,
        operations: Vec<AnalysisOperation>,
        facts: Vec<AnalysisFact>,
    ) -> AnalysisBundle {
        AnalysisBundle {
            entities,
            facts,
            rules: vec![],
            operations,
            flows: vec![],
            personas,
            systems: vec![],
        }
    }

    #[test]
    fn test_admissible_operation() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string(), "submitted".to_string()],
                initial: "draft".to_string(),
                transitions: vec![Transition {
                    from: "draft".to_string(),
                    to: "submitted".to_string(),
                }],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "admin".to_string(),
            }],
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
            vec![],
        );

        let result = analyze_admissibility(&bundle);
        let key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "draft".to_string(),
            persona_id: "admin".to_string(),
        };
        assert!(result.admissible_operations.contains_key(&key));
        assert_eq!(result.admissible_operations[&key], vec!["submit"]);
    }

    #[test]
    fn test_persona_not_authorized() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "user".to_string(),
            }],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()], // user NOT authorized
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
            vec![],
        );

        let result = analyze_admissibility(&bundle);
        let key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "draft".to_string(),
            persona_id: "user".to_string(),
        };
        assert!(!result.admissible_operations.contains_key(&key));
    }

    #[test]
    fn test_no_effect_from_state() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string(), "submitted".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "admin".to_string(),
            }],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: None,
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "submitted".to_string(), // Only from submitted, not draft
                    to_state: "approved".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
            vec![],
        );

        let result = analyze_admissibility(&bundle);
        let draft_key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "draft".to_string(),
            persona_id: "admin".to_string(),
        };
        assert!(!result.admissible_operations.contains_key(&draft_key));

        let submitted_key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "submitted".to_string(),
            persona_id: "admin".to_string(),
        };
        assert!(result.admissible_operations.contains_key(&submitted_key));
    }

    #[test]
    fn test_enum_precondition_unsatisfiable() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "admin".to_string(),
            }],
            vec![AnalysisOperation {
                id: "approve".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: Some(json!({
                    "left": {"fact_ref": "status"},
                    "op": "=",
                    "right": {"literal": "approved", "type": {"base": "Enum", "values": ["approved"]}}
                })),
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "approved".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
            // status is an Enum with only "pending" and "confirmed" -- "approved" is NOT a valid value
            vec![AnalysisFact {
                id: "status".to_string(),
                fact_type: json!({"base": "Enum", "values": ["pending", "confirmed"]}),
            }],
        );

        let result = analyze_admissibility(&bundle);
        let key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "draft".to_string(),
            persona_id: "admin".to_string(),
        };
        // Operation should NOT be admissible because precondition is structurally unsatisfiable
        assert!(!result.admissible_operations.contains_key(&key));
    }

    #[test]
    fn test_enum_precondition_satisfiable() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "admin".to_string(),
            }],
            vec![AnalysisOperation {
                id: "approve".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: Some(json!({
                    "left": {"fact_ref": "status"},
                    "op": "=",
                    "right": {"literal": "confirmed", "type": {"base": "Enum", "values": ["confirmed"]}}
                })),
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "approved".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
            vec![AnalysisFact {
                id: "status".to_string(),
                fact_type: json!({"base": "Enum", "values": ["pending", "confirmed"]}),
            }],
        );

        let result = analyze_admissibility(&bundle);
        let key = AdmissibilityKey {
            entity_id: "Order".to_string(),
            state: "draft".to_string(),
            persona_id: "admin".to_string(),
        };
        assert!(result.admissible_operations.contains_key(&key));
    }

    #[test]
    fn test_multiple_personas_cross_product() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![
                AnalysisPersona {
                    id: "admin".to_string(),
                },
                AnalysisPersona {
                    id: "user".to_string(),
                },
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
            vec![],
        );

        let result = analyze_admissibility(&bundle);
        assert_eq!(result.total_combinations_checked, 2); // 1 entity * 1 state * 2 personas
                                                          // Both personas should have the operation admissible
        assert!(result
            .admissible_operations
            .contains_key(&AdmissibilityKey {
                entity_id: "Order".to_string(),
                state: "draft".to_string(),
                persona_id: "admin".to_string(),
            }));
        assert!(result
            .admissible_operations
            .contains_key(&AdmissibilityKey {
                entity_id: "Order".to_string(),
                state: "draft".to_string(),
                persona_id: "user".to_string(),
            }));
    }

    #[test]
    fn test_verdict_present_precondition_satisfiable() {
        let bundle = make_bundle_with(
            vec![AnalysisEntity {
                id: "Order".to_string(),
                states: vec!["draft".to_string()],
                initial: "draft".to_string(),
                transitions: vec![],
                parent: None,
            }],
            vec![AnalysisPersona {
                id: "admin".to_string(),
            }],
            vec![AnalysisOperation {
                id: "submit".to_string(),
                allowed_personas: vec!["admin".to_string()],
                precondition: Some(json!({"verdict_present": "account_active"})),
                effects: vec![Effect {
                    entity_id: "Order".to_string(),
                    from_state: "draft".to_string(),
                    to_state: "submitted".to_string(),
                    outcome: None,
                }],
                outcomes: vec![],
                error_contract: None,
            }],
            vec![],
        );

        let result = analyze_admissibility(&bundle);
        // verdict_present is always structurally satisfiable
        assert!(result
            .admissible_operations
            .contains_key(&AdmissibilityKey {
                entity_id: "Order".to_string(),
                state: "draft".to_string(),
                persona_id: "admin".to_string(),
            }));
    }
}
