//! Matching provider abstraction for fact-to-source mapping.
//!
//! Defines the `MatchingProvider` trait that unifies heuristic and LLM-based
//! matching behind a common async interface.

use std::collections::BTreeMap;
use std::fmt;

use async_trait::async_trait;

use super::introspect::ExternalSchema;
use super::matching::Confidence;

/// A declaration of a fact to be matched against external schemas.
#[derive(Debug, Clone)]
pub struct FactDeclaration {
    /// The fact identifier (e.g. "order_balance").
    pub fact_id: String,
    /// The base type name (e.g. "Int", "Money", "Text").
    pub base_type: String,
    /// The source this fact references.
    pub source_id: String,
    /// The declared path within the source (e.g. "orders.balance").
    pub path: String,
    /// Full type descriptor as JSON (includes constraints, precision, etc.).
    pub full_type: serde_json::Value,
}

/// An inventory of external schemas keyed by source ID.
#[derive(Debug, Clone)]
pub struct EnvironmentInventory {
    pub schemas: BTreeMap<String, ExternalSchema>,
}

/// A proposed mapping from a fact to an external field.
#[derive(Debug, Clone)]
pub struct MappingProposal {
    /// The fact being mapped.
    pub fact_id: String,
    /// The source being mapped to.
    pub source_id: String,
    /// The external endpoint (e.g. "/orders/{id}").
    pub endpoint: String,
    /// The field path within the endpoint response.
    pub field_path: String,
    /// Confidence level of this mapping.
    pub confidence: Confidence,
    /// Human-readable explanation of why this mapping was proposed.
    pub explanation: String,
    /// Alternative mappings that were considered.
    pub alternatives: Vec<AlternativeProposal>,
}

/// An alternative mapping that was considered but not selected as primary.
#[derive(Debug, Clone)]
pub struct AlternativeProposal {
    pub endpoint: String,
    pub field_path: String,
    pub confidence: Confidence,
    pub explanation: String,
}

/// Error type for matching operations.
#[derive(Debug)]
pub enum MatchingError {
    /// The API call failed (network, auth, rate limit).
    ApiError(String),
    /// The response could not be parsed.
    ParseError(String),
    /// No facts or schemas were provided.
    EmptyInput(String),
    /// An internal error occurred.
    Internal(String),
}

impl fmt::Display for MatchingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchingError::ApiError(msg) => write!(f, "API error: {}", msg),
            MatchingError::ParseError(msg) => write!(f, "parse error: {}", msg),
            MatchingError::EmptyInput(msg) => write!(f, "empty input: {}", msg),
            MatchingError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for MatchingError {}

/// Trait for matching providers that propose fact-to-source mappings.
#[async_trait]
pub trait MatchingProvider: Send + Sync {
    /// Propose mappings between fact declarations and external environment schemas.
    async fn propose_mappings(
        &self,
        facts: &[FactDeclaration],
        environment: &EnvironmentInventory,
    ) -> Result<Vec<MappingProposal>, MatchingError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_declaration_construction() {
        let decl = FactDeclaration {
            fact_id: "order_balance".to_string(),
            base_type: "Int".to_string(),
            source_id: "order_service".to_string(),
            path: "orders.balance".to_string(),
            full_type: serde_json::json!({"base": "Int", "min": 0}),
        };
        assert_eq!(decl.fact_id, "order_balance");
        assert_eq!(decl.base_type, "Int");
    }

    #[test]
    fn test_environment_inventory_construction() {
        let inv = EnvironmentInventory {
            schemas: BTreeMap::new(),
        };
        assert!(inv.schemas.is_empty());
    }

    #[test]
    fn test_mapping_proposal_construction() {
        let proposal = MappingProposal {
            fact_id: "order_balance".to_string(),
            source_id: "order_service".to_string(),
            endpoint: "/orders/{id}".to_string(),
            field_path: "balance".to_string(),
            confidence: Confidence::High,
            explanation: "Exact type and path match".to_string(),
            alternatives: vec![],
        };
        assert_eq!(proposal.fact_id, "order_balance");
        assert_eq!(proposal.confidence, Confidence::High);
        assert!(proposal.alternatives.is_empty());
    }

    #[test]
    fn test_matching_error_display() {
        let err = MatchingError::ApiError("connection refused".to_string());
        assert_eq!(format!("{}", err), "API error: connection refused");

        let err = MatchingError::ParseError("invalid JSON".to_string());
        assert_eq!(format!("{}", err), "parse error: invalid JSON");

        let err = MatchingError::EmptyInput("no facts".to_string());
        assert_eq!(format!("{}", err), "empty input: no facts");

        let err = MatchingError::Internal("unexpected".to_string());
        assert_eq!(format!("{}", err), "internal error: unexpected");
    }
}
