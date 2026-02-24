//! Shared AST types for the Tenor elaborator.
//!
//! These types are produced by the parser and consumed throughout all
//! elaboration passes. They live here so that pass modules can import
//! them without depending on the parser.

use std::collections::BTreeMap;

// ──────────────────────────────────────────────
// Provenance
// ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Provenance {
    pub file: String,
    pub line: u32,
}

// ──────────────────────────────────────────────
// Raw types (pre-elaboration)
// ──────────────────────────────────────────────

/// A raw BaseType as it appears in the DSL, before TypeRef resolution.
#[derive(Debug, Clone)]
pub enum RawType {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Decimal {
        precision: u32,
        scale: u32,
    },
    Text {
        max_length: u32,
    },
    Date,
    DateTime,
    Money {
        currency: String,
    },
    Duration {
        unit: String,
        min: i64,
        max: i64,
    },
    Enum {
        values: Vec<String>,
    },
    Record {
        fields: BTreeMap<String, RawType>,
    },
    List {
        element_type: Box<RawType>,
        max: u32,
    },
    /// Named type reference -- resolved during Pass 3/4
    TypeRef(String),
}

// ──────────────────────────────────────────────
// Raw literals
// ──────────────────────────────────────────────

/// A raw literal value
#[derive(Debug, Clone)]
pub enum RawLiteral {
    Bool(bool),
    Int(i64),
    Float(String),
    Str(String),
    Money { amount: String, currency: String },
}

// ──────────────────────────────────────────────
// Raw expressions
// ──────────────────────────────────────────────

/// A raw predicate expression
#[derive(Debug, Clone)]
pub enum RawExpr {
    /// fact_ref op literal -- line is the line of the left operand
    Compare {
        op: String,
        left: RawTerm,
        right: RawTerm,
        line: u32,
    },
    /// verdict_present(id) -- line is the line of the verdict_present token
    VerdictPresent { id: String, line: u32 },
    /// e1 and e2
    And(Box<RawExpr>, Box<RawExpr>),
    /// e1 or e2
    Or(Box<RawExpr>, Box<RawExpr>),
    /// not e
    Not(Box<RawExpr>),
    /// forall var in list_ref . body -- line is the line of the forall token
    Forall {
        var: String,
        domain: String,
        body: Box<RawExpr>,
        line: u32,
    },
    /// exists var in list_ref . body -- line is the line of the exists token
    Exists {
        var: String,
        domain: String,
        body: Box<RawExpr>,
        line: u32,
    },
}

#[derive(Debug, Clone)]
pub enum RawTerm {
    FactRef(String),
    FieldRef {
        var: String,
        field: String,
    },
    Literal(RawLiteral),
    /// Arithmetic multiplication: left * right
    Mul {
        left: Box<RawTerm>,
        right: Box<RawTerm>,
    },
}

// ──────────────────────────────────────────────
// Raw constructs
// ──────────────────────────────────────────────

/// Raw construct from the parser
#[derive(Debug, Clone)]
pub enum RawConstruct {
    Import {
        path: String,
        prov: Provenance,
    },
    TypeDecl {
        id: String,
        fields: BTreeMap<String, RawType>,
        prov: Provenance,
    },
    Fact {
        id: String,
        type_: RawType,
        source: String,
        default: Option<RawLiteral>,
        prov: Provenance,
    },
    Entity {
        id: String,
        states: Vec<String>,
        initial: String,
        /// Line of the `initial:` field keyword
        initial_line: u32,
        /// (from, to, line_of_lparen) -- line is the line of the `(` opening each tuple
        transitions: Vec<(String, String, u32)>,
        parent: Option<String>,
        /// Line of the `parent:` field keyword, when present
        parent_line: Option<u32>,
        prov: Provenance,
    },
    Rule {
        id: String,
        stratum: i64,
        /// Line of the `stratum:` field keyword
        stratum_line: u32,
        when: RawExpr,
        verdict_type: String,
        payload_type: RawType,
        /// Payload value expression (literal or multiplication)
        payload_value: RawTerm,
        /// Line of the `produce:` field keyword
        produce_line: u32,
        prov: Provenance,
    },
    Operation {
        id: String,
        allowed_personas: Vec<String>,
        /// Line of the `allowed_personas:` field keyword
        allowed_personas_line: u32,
        precondition: RawExpr,
        /// (entity, from, to, outcome_label, line_of_lparen) -- line is the line of the `(` opening each tuple
        effects: Vec<(String, String, String, Option<String>, u32)>,
        error_contract: Vec<String>,
        /// Operation-local outcome identifiers (v1.0); empty if not declared
        outcomes: Vec<String>,
        prov: Provenance,
    },
    Persona {
        id: String,
        prov: Provenance,
    },
    Flow {
        id: String,
        snapshot: String,
        entry: String,
        /// Line of the `entry:` field keyword
        entry_line: u32,
        steps: BTreeMap<String, RawStep>,
        prov: Provenance,
    },
    System {
        id: String,
        /// Member contract declarations: (member_id, file_path)
        members: Vec<(String, String)>,
        /// Shared persona bindings: (persona_id, vec of member_ids)
        shared_personas: Vec<(String, Vec<String>)>,
        /// Cross-contract flow triggers
        triggers: Vec<RawTrigger>,
        /// Cross-contract entity relationships: (entity_id, vec of member_ids)
        shared_entities: Vec<(String, Vec<String>)>,
        prov: Provenance,
    },
}

// ──────────────────────────────────────────────
// System sub-types
// ──────────────────────────────────────────────

/// A cross-contract flow trigger declaration within a System.
#[derive(Debug, Clone)]
pub struct RawTrigger {
    pub source_contract: String,
    pub source_flow: String,
    pub on: String,
    pub target_contract: String,
    pub target_flow: String,
    pub persona: String,
}

// ──────────────────────────────────────────────
// Flow step types
// ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RawStep {
    OperationStep {
        op: String,
        persona: String,
        outcomes: BTreeMap<String, RawStepTarget>,
        /// Optional at parse time; absence is a Pass 5 error (not a parse error)
        on_failure: Option<RawFailureHandler>,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    BranchStep {
        condition: RawExpr,
        persona: String,
        if_true: RawStepTarget,
        if_false: RawStepTarget,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    HandoffStep {
        from_persona: String,
        to_persona: String,
        next: String,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    SubFlowStep {
        /// Id of the referenced Flow construct
        flow: String,
        /// Line of the `flow:` field keyword (used in cycle error reporting)
        flow_line: u32,
        persona: String,
        on_success: RawStepTarget,
        on_failure: RawFailureHandler,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    ParallelStep {
        branches: Vec<RawBranch>,
        /// Line of the `branches:` field keyword
        branches_line: u32,
        join: RawJoinPolicy,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
}

#[derive(Debug, Clone)]
pub enum RawStepTarget {
    /// (step_id, line_of_step_id_token)
    StepRef(String, u32),
    Terminal {
        outcome: String,
    },
}

#[derive(Debug, Clone)]
pub enum RawFailureHandler {
    Terminate {
        outcome: String,
    },
    Compensate {
        steps: Vec<RawCompStep>,
        then: String,
    },
    Escalate {
        to_persona: String,
        next: String,
    },
}

#[derive(Debug, Clone)]
pub struct RawCompStep {
    pub op: String,
    pub persona: String,
    pub on_failure: String,
}

#[derive(Debug, Clone)]
pub struct RawBranch {
    pub id: String,
    pub entry: String,
    pub steps: BTreeMap<String, RawStep>,
}

#[derive(Debug, Clone)]
pub struct RawJoinPolicy {
    pub on_all_success: Option<RawStepTarget>,
    pub on_any_failure: Option<RawFailureHandler>,
    pub on_all_complete: Option<RawStepTarget>,
}
