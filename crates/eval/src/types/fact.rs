//! Fact and verdict set types for the Tenor evaluator.

use std::collections::BTreeMap;

use super::values::Value;
use super::TypeSpec;

/// A declared fact with type and optional default.
#[derive(Debug, Clone)]
pub struct FactDecl {
    pub id: String,
    pub fact_type: TypeSpec,
    pub default: Option<Value>,
}

/// A set of fact values keyed by fact id.
#[derive(Debug, Clone)]
pub struct FactSet(pub BTreeMap<String, Value>);

impl Default for FactSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FactSet {
    pub fn new() -> Self {
        FactSet(BTreeMap::new())
    }

    pub fn get(&self, id: &str) -> Option<&Value> {
        self.0.get(id)
    }

    pub fn insert(&mut self, id: String, value: Value) {
        self.0.insert(id, value);
    }
}

/// A set of produced verdicts.
#[derive(Debug, Clone)]
pub struct VerdictSet(pub Vec<VerdictInstance>);

impl Default for VerdictSet {
    fn default() -> Self {
        Self::new()
    }
}

impl VerdictSet {
    pub fn new() -> Self {
        VerdictSet(Vec::new())
    }

    pub fn push(&mut self, verdict: VerdictInstance) {
        self.0.push(verdict);
    }

    /// Check if a verdict of the given type has been produced.
    pub fn has_verdict(&self, verdict_type: &str) -> bool {
        self.0.iter().any(|v| v.verdict_type == verdict_type)
    }

    /// Get the most recent verdict of the given type.
    pub fn get_verdict(&self, verdict_type: &str) -> Option<&VerdictInstance> {
        self.0.iter().rev().find(|v| v.verdict_type == verdict_type)
    }

    /// Serialize to JSON output format.
    pub fn to_json(&self) -> serde_json::Value {
        let verdicts: Vec<serde_json::Value> = self
            .0
            .iter()
            .map(|v| {
                serde_json::json!({
                    "type": v.verdict_type,
                    "payload": v.payload.to_json(),
                    "provenance": {
                        "rule": v.provenance.rule_id,
                        "stratum": v.provenance.stratum,
                        "facts_used": v.provenance.facts_used,
                        "verdicts_used": v.provenance.verdicts_used,
                    }
                })
            })
            .collect();
        serde_json::json!({ "verdicts": verdicts })
    }
}

/// A single verdict instance with provenance.
#[derive(Debug, Clone)]
pub struct VerdictInstance {
    pub verdict_type: String,
    pub payload: Value,
    pub provenance: crate::provenance::VerdictProvenance,
}
