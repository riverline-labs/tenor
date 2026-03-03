//! Typed structs representing the Tenor interchange JSON schema.
//!
//! These types cover the superset of fields consumed by tenor-eval,
//! tenor-analyze, and tenor-codegen. Fields that only some consumers
//! use are stored as `serde_json::Value` to avoid forcing every consumer
//! to parse deeply nested expression trees.

use serde::{Deserialize, Serialize};

/// Source location provenance recorded by the elaborator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub file: String,
    pub line: u64,
}

/// Trust metadata for signed bundles and manifest trust sections (Section 19.1).
/// All fields are optional — deployments without trust infrastructure omit this entirely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustMetadata {
    /// Base64-encoded signature of the canonical bundle bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_attestation: Option<String>,
    /// Trust domain identifier (e.g. "acme.prod.us-east-1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_domain: Option<String>,
    /// Attestation format identifier (e.g. "ed25519-detached").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_format: Option<String>,
    /// Base64-encoded public key of the signer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_public_key: Option<String>,
}

/// Optional trust fields on provenance records (Section 17.4 E19-E20).
/// When trust is configured, provenance records carry these fields for
/// tamper-evident audit trails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceTrustFields {
    /// Trust domain identifier from executor configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_domain: Option<String>,
    /// Base64-encoded signature of the provenance record content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation: Option<String>,
}

/// Top-level interchange bundle containing all constructs.
#[derive(Debug, Clone)]
pub struct InterchangeBundle {
    /// Bundle identifier (contract id).
    pub id: String,
    /// Tenor spec version (e.g. "1.0").
    pub tenor: String,
    /// Tenor interchange bundle version (e.g. "1.0.0").
    pub tenor_version: String,
    /// All constructs in the bundle.
    pub constructs: Vec<InterchangeConstruct>,
    /// Optional trust metadata for signed bundles (Section 19.1).
    pub trust: Option<TrustMetadata>,
}

/// A single construct from the interchange bundle, dispatched by kind.
#[derive(Debug, Clone)]
pub enum InterchangeConstruct {
    Fact(FactConstruct),
    Entity(EntityConstruct),
    Rule(RuleConstruct),
    Operation(OperationConstruct),
    Flow(FlowConstruct),
    Persona(PersonaConstruct),
    Source(SourceConstruct),
    System(SystemConstruct),
    TypeDecl(TypeDeclConstruct),
}

// ── Fact ────────────────────────────────────────────────────────────

/// A Fact construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct FactConstruct {
    pub id: String,
    /// The full type JSON (base, precision, scale, currency, etc.).
    /// Kept as Value because eval, analyze, and codegen each interpret
    /// type details differently.
    pub fact_type: serde_json::Value,
    /// Source mapping (field, system).
    pub source: Option<serde_json::Value>,
    /// Default value, if declared.
    pub default: Option<serde_json::Value>,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── Entity ──────────────────────────────────────────────────────────

/// A state transition in an Entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transition {
    pub from: String,
    pub to: String,
}

/// An Entity construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct EntityConstruct {
    pub id: String,
    pub states: Vec<String>,
    pub initial: String,
    pub transitions: Vec<Transition>,
    /// Parent entity id for inheritance.
    pub parent: Option<String>,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── Rule ────────────────────────────────────────────────────────────

/// A Rule construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct RuleConstruct {
    pub id: String,
    pub stratum: u64,
    /// The full body JSON object containing `when` and `produce`.
    /// Kept as Value because the predicate tree and produce clause
    /// are deeply nested and interpreted differently by each consumer.
    pub body: serde_json::Value,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

impl RuleConstruct {
    /// Extract the `when` predicate expression from the body.
    pub fn when(&self) -> Option<&serde_json::Value> {
        self.body.get("when")
    }

    /// Extract the `produce` clause from the body.
    pub fn produce(&self) -> Option<&serde_json::Value> {
        self.body.get("produce")
    }

    /// Extract the verdict type string from body.produce.verdict_type.
    pub fn verdict_type(&self) -> Option<&str> {
        self.body
            .get("produce")
            .and_then(|p| p.get("verdict_type"))
            .and_then(|v| v.as_str())
    }

    /// Extract the produce payload from body.produce.payload.
    pub fn produce_payload(&self) -> Option<&serde_json::Value> {
        self.body.get("produce").and_then(|p| p.get("payload"))
    }
}

// ── Operation ───────────────────────────────────────────────────────

/// An effect (entity state transition) in an Operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Effect {
    pub entity_id: String,
    pub from: String,
    pub to: String,
    pub outcome: Option<String>,
}

/// An Operation construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct OperationConstruct {
    pub id: String,
    pub allowed_personas: Vec<String>,
    /// Precondition predicate expression. None if null.
    pub precondition: Option<serde_json::Value>,
    pub effects: Vec<Effect>,
    /// Outcome names declared by the operation.
    pub outcomes: Vec<String>,
    /// Error contract references.
    pub error_contract: Option<serde_json::Value>,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── Flow ────────────────────────────────────────────────────────────

/// A Flow construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct FlowConstruct {
    pub id: String,
    pub entry: String,
    /// Raw step JSON values. Each consumer interprets steps differently:
    /// eval parses into FlowStep enum, analyze preserves as Value for
    /// path enumeration, codegen ignores step details.
    pub steps: Vec<serde_json::Value>,
    pub snapshot: String,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── Persona ─────────────────────────────────────────────────────────

/// A Persona construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct PersonaConstruct {
    pub id: String,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── Source ──────────────────────────────────────────────────────────

/// A Source construct from interchange JSON (§5A).
/// Source declarations are infrastructure metadata — they do not affect
/// evaluation but describe where facts originate.
#[derive(Debug, Clone)]
pub struct SourceConstruct {
    pub id: String,
    pub protocol: String,
    pub fields: std::collections::BTreeMap<String, String>,
    pub description: Option<String>,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── System ──────────────────────────────────────────────────────────

/// A member contract declaration within a System.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemMember {
    pub id: String,
    pub path: String,
}

/// A shared persona binding within a System.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedPersona {
    pub persona: String,
    pub contracts: Vec<String>,
}

/// A cross-contract flow trigger within a System.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowTrigger {
    pub source_contract: String,
    pub source_flow: String,
    pub on: String,
    pub target_contract: String,
    pub target_flow: String,
    pub persona: String,
}

/// A shared entity binding within a System.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedEntity {
    pub entity: String,
    pub contracts: Vec<String>,
}

/// A System construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct SystemConstruct {
    pub id: String,
    pub members: Vec<SystemMember>,
    pub shared_personas: Vec<SharedPersona>,
    pub flow_triggers: Vec<FlowTrigger>,
    pub shared_entities: Vec<SharedEntity>,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}

// ── TypeDecl ────────────────────────────────────────────────────────

/// A TypeDecl construct from interchange JSON.
#[derive(Debug, Clone)]
pub struct TypeDeclConstruct {
    pub id: String,
    /// The type definition JSON.
    pub type_def: serde_json::Value,
    pub provenance: Option<Provenance>,
    pub tenor: Option<String>,
}
