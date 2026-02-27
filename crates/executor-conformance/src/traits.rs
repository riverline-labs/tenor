use async_trait::async_trait;
use serde_json::Value;

/// Result type for conformance test operations.
pub type ConformanceResult<T> = Result<T, ConformanceError>;

/// Errors from conformance test operations.
#[derive(Debug, Clone)]
pub struct ConformanceError {
    pub message: String,
}

impl std::fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ConformanceError {}

/// Flow execution result from a testable executor.
#[derive(Debug, Clone)]
pub struct FlowResult {
    pub outcome: String,
    pub entity_state_changes: Vec<EntityStateChange>,
    pub steps_executed: Vec<StepResult>,
    pub provenance: Vec<Value>,
}

/// A state change recorded during flow execution.
#[derive(Debug, Clone)]
pub struct EntityStateChange {
    pub entity_id: String,
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
}

/// The result of a single step in a flow execution.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub result: String,
}

/// Action space returned by an executor.
#[derive(Debug, Clone)]
pub struct ActionSpace {
    pub available_operations: Vec<AvailableOperation>,
}

/// An operation available in the current action space.
#[derive(Debug, Clone)]
pub struct AvailableOperation {
    pub operation_id: String,
    pub entity_id: String,
    pub instance_id: String,
    pub persona: String,
}

/// Tenor manifest served by an executor.
#[derive(Debug, Clone)]
pub struct TenorManifest {
    pub bundle: Value,
    pub etag: String,
    pub tenor: String,
    pub capabilities: Value,
    pub trust: Option<Value>,
}

/// Trait that any executor implementation must implement to run the
/// conformance suite. Each method corresponds to a core executor capability.
#[async_trait]
pub trait TestableExecutor: Send + Sync {
    /// Load a contract bundle into the executor.
    async fn load_contract(&self, bundle: &Value) -> ConformanceResult<()>;

    /// Execute a flow and return the result.
    async fn execute_flow(
        &self,
        flow_id: &str,
        persona: &str,
        facts: &Value,
        entity_states: &Value,
    ) -> ConformanceResult<FlowResult>;

    /// Simulate a flow (dry-run, no side effects).
    async fn simulate_flow(
        &self,
        flow_id: &str,
        persona: &str,
        facts: &Value,
        entity_states: &Value,
    ) -> ConformanceResult<FlowResult>;

    /// Get the state of a specific entity instance.
    async fn get_entity_state(
        &self,
        entity_id: &str,
        instance_id: &str,
    ) -> ConformanceResult<Option<String>>;

    /// Get the action space for a persona given current facts and entity states.
    async fn get_action_space(
        &self,
        persona: &str,
        facts: &Value,
        entity_states: &Value,
    ) -> ConformanceResult<ActionSpace>;

    /// Get the manifest served by the executor.
    async fn get_manifest(&self) -> ConformanceResult<TenorManifest>;

    /// Check if trust is configured on this executor.
    async fn is_trust_configured(&self) -> bool;

    /// Create a new entity instance in its initial state.
    async fn create_instance(&self, entity_id: &str, instance_id: &str) -> ConformanceResult<()>;

    /// List all instance IDs for an entity.
    async fn list_instances(&self, entity_id: &str) -> ConformanceResult<Vec<String>>;
}
