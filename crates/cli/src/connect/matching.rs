//! Fact-to-source matching: propose mappings between declared Facts
//! and external schema fields.

use std::collections::BTreeMap;

use super::introspect::{ExternalSchema, ExternalType, SchemaField};
use super::StructuredFact;
use tenor_interchange::SourceConstruct;

/// Confidence level for a proposed mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Type match and path match.
    High,
    /// Partial match (type or path, not both).
    Medium,
    /// Inferred from path only, no schema to validate.
    Low,
}

/// A proposed mapping between a Fact and an external field.
#[derive(Debug, Clone)]
pub struct FactMapping {
    pub fact_id: String,
    pub source_id: String,
    pub path: String,
    pub confidence: Confidence,
    pub description: String,
    pub note: Option<String>,
}

/// Match facts against external schemas to produce proposed mappings.
pub fn match_facts(
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    schemas: &BTreeMap<String, ExternalSchema>,
) -> Vec<FactMapping> {
    let source_map: BTreeMap<&str, &SourceConstruct> =
        sources.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut mappings = Vec::new();

    for fact in facts {
        let source = match source_map.get(fact.source_id.as_str()) {
            Some(s) => s,
            None => continue,
        };

        if let Some(schema) = schemas.get(&fact.source_id) {
            // Rich mode: match against introspected schema
            let mapping = match_with_schema(fact, source, schema);
            mappings.push(mapping);
        } else {
            // Degraded mode: infer from protocol and path
            let mapping = match_degraded(fact, source);
            mappings.push(mapping);
        }
    }

    mappings
}

/// Match a fact against an introspected external schema.
fn match_with_schema(
    fact: &StructuredFact,
    source: &SourceConstruct,
    schema: &ExternalSchema,
) -> FactMapping {
    let tenor_type = extract_tenor_base_type(&fact.fact_type);

    // Try to find a matching field in any endpoint's response
    let mut best_match: Option<(&SchemaField, &str, Confidence)> = None;

    for endpoint in &schema.endpoints {
        for field in &endpoint.response_fields {
            let path_similarity = path_similarity(&fact.path, &field.path);
            let type_compat = type_compatible(&tenor_type, &field.field_type);

            let confidence = match (path_similarity >= 0.5, type_compat) {
                (true, true) => Confidence::High,
                (true, false) | (false, true) => Confidence::Medium,
                (false, false) => continue,
            };

            let dominated = best_match
                .as_ref()
                .map(|(_, _, c)| confidence_ord(confidence) > confidence_ord(*c))
                .unwrap_or(true);
            if dominated {
                best_match = Some((field, &endpoint.path, confidence));
            }
        }
    }

    if let Some((field, endpoint_path, confidence)) = best_match {
        FactMapping {
            fact_id: fact.id.clone(),
            source_id: fact.source_id.clone(),
            path: fact.path.clone(),
            confidence,
            description: format!(
                "{} {} -> field '{}'",
                endpoint_path, source.protocol, field.path
            ),
            note: if confidence == Confidence::Medium {
                Some(format!(
                    "type mismatch or partial path match: tenor {} vs external {}",
                    tenor_type, field.field_type
                ))
            } else {
                None
            },
        }
    } else {
        FactMapping {
            fact_id: fact.id.clone(),
            source_id: fact.source_id.clone(),
            path: fact.path.clone(),
            confidence: Confidence::Low,
            description: format!(
                "no matching field found in {} schema for path '{}'",
                source.protocol, fact.path
            ),
            note: Some("manual mapping required".to_string()),
        }
    }
}

/// Match a fact without an external schema (degraded mode).
fn match_degraded(fact: &StructuredFact, source: &SourceConstruct) -> FactMapping {
    let description = match source.protocol.as_str() {
        "http" => {
            let base = source
                .fields
                .get("base_url")
                .map(|s| s.as_str())
                .unwrap_or("?");
            let url_path = fact.path.replace('.', "/");
            format!("{}/{}", base, url_path)
        }
        "database" => {
            let parts: Vec<&str> = fact.path.splitn(2, '.').collect();
            if parts.len() == 2 {
                format!(
                    "SELECT {} FROM {} (dialect: {})",
                    parts[1],
                    parts[0],
                    source
                        .fields
                        .get("dialect")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown")
                )
            } else {
                format!("query path: {}", fact.path)
            }
        }
        "graphql" => {
            format!("GraphQL query for {}", fact.path)
        }
        "grpc" => {
            format!("gRPC call for {}", fact.path)
        }
        "static" => {
            format!("static value: {}", fact.path)
        }
        "manual" => {
            format!("manual input: {}", fact.path)
        }
        _ => {
            format!("{}: {}", source.protocol, fact.path)
        }
    };

    FactMapping {
        fact_id: fact.id.clone(),
        source_id: fact.source_id.clone(),
        path: fact.path.clone(),
        confidence: Confidence::Low,
        description,
        note: Some("no schema_ref available; mapping inferred from protocol and path".to_string()),
    }
}

/// Extract the base type name from a Tenor fact type JSON.
fn extract_tenor_base_type(fact_type: &serde_json::Value) -> String {
    fact_type
        .get("base")
        .and_then(|b| b.as_str())
        .unwrap_or("Unknown")
        .to_string()
}

/// Compute a simple path similarity score (0.0 to 1.0).
/// Compares the leaf segments of both paths.
fn path_similarity(tenor_path: &str, schema_path: &str) -> f64 {
    let tenor_parts: Vec<&str> = tenor_path.split('.').collect();
    let schema_parts: Vec<&str> = schema_path.split('.').collect();

    if tenor_parts.is_empty() || schema_parts.is_empty() {
        return 0.0;
    }

    // Check if the leaf (last segment) matches
    let tenor_leaf = tenor_parts.last().unwrap().to_lowercase();
    let schema_leaf = schema_parts.last().unwrap().to_lowercase();

    if tenor_leaf == schema_leaf {
        // Exact leaf match
        if tenor_parts.len() == schema_parts.len() {
            1.0
        } else {
            0.75
        }
    } else if tenor_leaf.contains(&schema_leaf) || schema_leaf.contains(&tenor_leaf) {
        0.5
    } else {
        0.0
    }
}

/// Check if a Tenor type is compatible with an external schema type.
fn type_compatible(tenor_type: &str, external_type: &ExternalType) -> bool {
    match (tenor_type, external_type) {
        ("Int", ExternalType::Integer) => true,
        ("Int", ExternalType::Number) => true,
        ("Decimal", ExternalType::Number) => true,
        ("Decimal", ExternalType::Integer) => true,
        ("Money", ExternalType::Number) => true,
        ("Money", ExternalType::Integer) => true,
        ("Money", ExternalType::Object) => true, // Money often serialized as { amount, currency }
        ("Text", ExternalType::String) => true,
        ("Bool", ExternalType::Boolean) => true,
        ("Date", ExternalType::String) => true, // Dates often transmitted as strings
        ("DateTime", ExternalType::String) => true,
        (_, ExternalType::Unknown(_)) => true, // Unknown external type — assume compatible
        _ => false,
    }
}

/// Numeric ordering for confidence levels.
fn confidence_ord(c: Confidence) -> u8 {
    match c {
        Confidence::High => 3,
        Confidence::Medium => 2,
        Confidence::Low => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_similarity_exact_leaf() {
        assert!(path_similarity("orders.balance", "balance") >= 0.75);
        assert!(path_similarity("balance", "balance") >= 1.0);
    }

    #[test]
    fn test_path_similarity_no_match() {
        assert_eq!(path_similarity("orders.balance", "status"), 0.0);
    }

    #[test]
    fn test_type_compatible() {
        assert!(type_compatible("Int", &ExternalType::Integer));
        assert!(type_compatible("Bool", &ExternalType::Boolean));
        assert!(type_compatible("Text", &ExternalType::String));
        assert!(!type_compatible("Int", &ExternalType::String));
        assert!(!type_compatible("Bool", &ExternalType::Integer));
    }

    #[test]
    fn test_extract_tenor_base_type() {
        let ft = serde_json::json!({"base": "Int", "min": 0, "max": 100});
        assert_eq!(extract_tenor_base_type(&ft), "Int");
    }
}
