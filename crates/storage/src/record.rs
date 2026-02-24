use serde::{Deserialize, Serialize};

/// A snapshot of an entity's current state as stored in the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStateRecord {
    pub entity_id: String,
    pub instance_id: String,
    pub state: String,
    pub version: i64,
    /// ISO 8601 / RFC 3339 timestamp string.
    pub updated_at: String,
    pub last_flow_id: Option<String>,
    pub last_operation_id: Option<String>,
}

/// A record of a completed flow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecutionRecord {
    pub id: String,
    pub flow_id: String,
    pub contract_id: String,
    pub persona_id: String,
    /// ISO 8601 / RFC 3339 timestamp string.
    pub started_at: String,
    /// ISO 8601 / RFC 3339 timestamp string. None if not yet completed.
    pub completed_at: Option<String>,
    pub outcome: String,
    pub snapshot_facts: serde_json::Value,
    pub snapshot_verdicts: serde_json::Value,
}

/// A record of a single operation execution within a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationExecutionRecord {
    pub id: String,
    pub flow_execution_id: String,
    pub operation_id: String,
    pub persona_id: String,
    pub outcome: String,
    /// ISO 8601 / RFC 3339 timestamp string.
    pub executed_at: String,
    pub step_id: String,
}

/// A record of a single entity state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTransitionRecord {
    pub id: String,
    pub operation_execution_id: String,
    pub entity_id: String,
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
    pub from_version: i64,
    pub to_version: i64,
}

/// A provenance record coupling operation execution to facts + verdicts.
///
/// Per spec C7: every state transition must have an atomically coupled
/// provenance record. No state change without provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub id: String,
    pub operation_execution_id: String,
    pub facts_used: serde_json::Value,
    pub verdicts_used: serde_json::Value,
    pub verdict_set_snapshot: serde_json::Value,
}
