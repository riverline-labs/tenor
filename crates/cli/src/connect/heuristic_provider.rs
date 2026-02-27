//! Heuristic matching provider — wraps the existing `matching::match_facts`
//! algorithm behind the `MatchingProvider` trait.

use async_trait::async_trait;
use tenor_interchange::SourceConstruct;

use super::matching::{self, FactMapping};
use super::provider::{
    EnvironmentInventory, FactDeclaration, MappingProposal, MatchingError, MatchingProvider,
};
use super::StructuredFact;

/// A matching provider that uses heuristic path similarity and type
/// compatibility checks (the original `match_facts` algorithm).
pub struct HeuristicMatchingProvider {
    /// Source constructs needed by `match_facts`.
    sources: Vec<SourceConstruct>,
}

impl HeuristicMatchingProvider {
    /// Create a new heuristic provider with the given source constructs.
    pub fn new(sources: Vec<SourceConstruct>) -> Self {
        Self { sources }
    }
}

#[async_trait]
impl MatchingProvider for HeuristicMatchingProvider {
    async fn propose_mappings(
        &self,
        facts: &[FactDeclaration],
        environment: &EnvironmentInventory,
    ) -> Result<Vec<MappingProposal>, MatchingError> {
        if facts.is_empty() {
            return Err(MatchingError::EmptyInput(
                "no fact declarations provided".to_string(),
            ));
        }

        // Convert FactDeclaration -> StructuredFact for the existing algorithm
        let structured_facts: Vec<StructuredFact> = facts
            .iter()
            .map(|fd| StructuredFact {
                id: fd.fact_id.clone(),
                fact_type: fd.full_type.clone(),
                source_id: fd.source_id.clone(),
                path: fd.path.clone(),
            })
            .collect();

        // Call the existing heuristic matcher
        let mappings =
            matching::match_facts(&self.sources, &structured_facts, &environment.schemas);

        // Convert FactMapping -> MappingProposal
        let proposals = mappings.into_iter().map(fact_mapping_to_proposal).collect();

        Ok(proposals)
    }
}

/// Convert a `FactMapping` from the heuristic matcher into a `MappingProposal`.
fn fact_mapping_to_proposal(m: FactMapping) -> MappingProposal {
    // The heuristic matcher stores the endpoint info in the description.
    // Extract what we can; the path field is already available.
    let endpoint = extract_endpoint_from_description(&m.description);
    let field_path = m.path.clone();

    let mut explanation = m.description.clone();
    if let Some(ref note) = m.note {
        explanation = format!("{}; {}", explanation, note);
    }

    MappingProposal {
        fact_id: m.fact_id,
        source_id: m.source_id,
        endpoint,
        field_path,
        confidence: m.confidence,
        explanation,
        alternatives: vec![],
    }
}

/// Try to extract an endpoint path from a heuristic mapping description.
///
/// The heuristic matcher produces descriptions like:
///   "/orders/{id} http -> field 'balance'"
///   "https://api.example.com/orders"
///   "SELECT column FROM table (dialect: postgres)"
fn extract_endpoint_from_description(description: &str) -> String {
    // Look for an HTTP path pattern
    if let Some(space_idx) = description.find(' ') {
        let first_token = &description[..space_idx];
        if first_token.starts_with('/') || first_token.starts_with("http") {
            return first_token.to_string();
        }
    }
    // Fallback: use the full description
    description.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connect::introspect::{
        Endpoint, ExternalSchema, ExternalType, SchemaField, SchemaFormat,
    };
    use crate::connect::matching::Confidence;

    fn make_source(id: &str, protocol: &str) -> SourceConstruct {
        SourceConstruct {
            id: id.to_string(),
            protocol: protocol.to_string(),
            fields: {
                let mut m = BTreeMap::new();
                m.insert(
                    "base_url".to_string(),
                    "https://api.example.com".to_string(),
                );
                m
            },
            description: Some("Test source".to_string()),
            provenance: None,
            tenor: None,
        }
    }

    fn make_fact(id: &str, base_type: &str, source_id: &str, path: &str) -> FactDeclaration {
        FactDeclaration {
            fact_id: id.to_string(),
            base_type: base_type.to_string(),
            source_id: source_id.to_string(),
            path: path.to_string(),
            full_type: serde_json::json!({"base": base_type}),
        }
    }

    fn make_schema_with_field(field_name: &str, field_type: ExternalType) -> ExternalSchema {
        ExternalSchema {
            format: SchemaFormat::OpenApi3,
            endpoints: vec![Endpoint {
                method: "GET".to_string(),
                path: "/orders/{id}".to_string(),
                parameters: vec!["id".to_string()],
                response_fields: vec![SchemaField {
                    path: field_name.to_string(),
                    field_type,
                }],
            }],
        }
    }

    #[tokio::test]
    async fn test_heuristic_provider_with_schema() {
        let source = make_source("order_service", "http");
        let provider = HeuristicMatchingProvider::new(vec![source]);

        let facts = vec![make_fact(
            "order_balance",
            "Int",
            "order_service",
            "balance",
        )];

        let mut schemas = BTreeMap::new();
        schemas.insert(
            "order_service".to_string(),
            make_schema_with_field("balance", ExternalType::Integer),
        );
        let env = EnvironmentInventory { schemas };

        let proposals = provider.propose_mappings(&facts, &env).await.unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].fact_id, "order_balance");
        assert_eq!(proposals[0].confidence, Confidence::High);
    }

    #[tokio::test]
    async fn test_heuristic_provider_empty_facts() {
        let provider = HeuristicMatchingProvider::new(vec![]);
        let env = EnvironmentInventory {
            schemas: BTreeMap::new(),
        };

        let result = provider.propose_mappings(&[], &env).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            MatchingError::EmptyInput(_) => {}
            other => panic!("Expected EmptyInput, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_heuristic_provider_degraded_mode() {
        let source = make_source("order_service", "http");
        let provider = HeuristicMatchingProvider::new(vec![source]);

        let facts = vec![make_fact(
            "order_balance",
            "Int",
            "order_service",
            "balance",
        )];

        // No schemas at all -> degraded mode
        let env = EnvironmentInventory {
            schemas: BTreeMap::new(),
        };

        let proposals = provider.propose_mappings(&facts, &env).await.unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].confidence, Confidence::Low);
    }

    #[test]
    fn test_extract_endpoint_from_description() {
        assert_eq!(
            extract_endpoint_from_description("/orders/{id} http -> field 'balance'"),
            "/orders/{id}"
        );
        assert_eq!(
            extract_endpoint_from_description("https://api.example.com/balance foo"),
            "https://api.example.com/balance"
        );
        assert_eq!(
            extract_endpoint_from_description("no matching field found"),
            "no matching field found"
        );
    }

    #[test]
    fn test_fact_mapping_to_proposal() {
        let mapping = FactMapping {
            fact_id: "order_balance".to_string(),
            source_id: "order_service".to_string(),
            path: "balance".to_string(),
            confidence: Confidence::High,
            description: "/orders/{id} http -> field 'balance'".to_string(),
            note: None,
        };

        let proposal = fact_mapping_to_proposal(mapping);
        assert_eq!(proposal.fact_id, "order_balance");
        assert_eq!(proposal.endpoint, "/orders/{id}");
        assert_eq!(proposal.field_path, "balance");
        assert_eq!(proposal.confidence, Confidence::High);
        assert!(proposal.alternatives.is_empty());
    }

    #[test]
    fn test_fact_mapping_to_proposal_with_note() {
        let mapping = FactMapping {
            fact_id: "order_balance".to_string(),
            source_id: "order_service".to_string(),
            path: "balance".to_string(),
            confidence: Confidence::Medium,
            description: "/orders/{id} http -> field 'balance'".to_string(),
            note: Some("type mismatch: tenor Int vs external string".to_string()),
        };

        let proposal = fact_mapping_to_proposal(mapping);
        assert!(proposal.explanation.contains("type mismatch"));
    }
}
