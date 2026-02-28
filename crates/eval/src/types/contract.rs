//! Contract representation and interchange JSON deserialization.

use std::collections::{BTreeMap, HashMap};

use super::fact::FactDecl;
use super::values::{get_str, infer_literal, parse_default_value, parse_literal_value, Value};
use super::{EvalError, Predicate, TypeSpec};

// ──────────────────────────────────────────────
// Contract representation
// ──────────────────────────────────────────────

/// Contract deserialized from interchange JSON bundle.
#[derive(Debug, Clone)]
pub struct Contract {
    pub facts: Vec<FactDecl>,
    pub entities: Vec<Entity>,
    pub rules: Vec<Rule>,
    pub operations: Vec<Operation>,
    pub flows: Vec<Flow>,
    pub personas: Vec<String>,
    // HashMap indexes for O(1) lookups by ID
    pub operation_index: HashMap<String, usize>,
    pub flow_index: HashMap<String, usize>,
    pub entity_index: HashMap<String, usize>,
    pub fact_index: HashMap<String, usize>,
}

impl Contract {
    /// Construct a Contract from its component Vecs, automatically building indexes.
    pub fn new(
        facts: Vec<FactDecl>,
        entities: Vec<Entity>,
        rules: Vec<Rule>,
        operations: Vec<Operation>,
        flows: Vec<Flow>,
        personas: Vec<String>,
    ) -> Self {
        let operation_index: HashMap<String, usize> = operations
            .iter()
            .enumerate()
            .map(|(i, op)| (op.id.clone(), i))
            .collect();
        let flow_index: HashMap<String, usize> = flows
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id.clone(), i))
            .collect();
        let entity_index: HashMap<String, usize> = entities
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();
        let fact_index: HashMap<String, usize> = facts
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id.clone(), i))
            .collect();
        Contract {
            facts,
            entities,
            rules,
            operations,
            flows,
            personas,
            operation_index,
            flow_index,
            entity_index,
            fact_index,
        }
    }

    /// Look up an operation by ID in O(1) via the index.
    pub fn get_operation(&self, id: &str) -> Option<&Operation> {
        self.operation_index.get(id).map(|&i| &self.operations[i])
    }

    /// Look up a flow by ID in O(1) via the index.
    pub fn get_flow(&self, id: &str) -> Option<&Flow> {
        self.flow_index.get(id).map(|&i| &self.flows[i])
    }

    /// Look up an entity by ID in O(1) via the index.
    pub fn get_entity(&self, id: &str) -> Option<&Entity> {
        self.entity_index.get(id).map(|&i| &self.entities[i])
    }

    /// Look up a fact declaration by ID in O(1) via the index.
    pub fn get_fact(&self, id: &str) -> Option<&FactDecl> {
        self.fact_index.get(id).map(|&i| &self.facts[i])
    }
}

impl Contract {
    /// Deserialize a Contract from interchange JSON bundle.
    ///
    /// Uses `tenor_interchange::from_interchange()` for initial JSON parsing
    /// and kind dispatch, then converts shared types to eval-specific domain
    /// types using deep parsers for predicates, flow steps, etc.
    pub fn from_interchange(bundle: &serde_json::Value) -> Result<Contract, EvalError> {
        use tenor_interchange::InterchangeConstruct;

        let parsed = tenor_interchange::from_interchange(bundle).map_err(|e| {
            EvalError::DeserializeError {
                message: e.to_string(),
            }
        })?;

        let mut facts = Vec::new();
        let mut entities = Vec::new();
        let mut rules = Vec::new();
        let mut operations = Vec::new();
        let mut flows = Vec::new();
        let mut personas = Vec::new();

        for construct in &parsed.constructs {
            match construct {
                InterchangeConstruct::Fact(f) => {
                    let fact_type = TypeSpec::from_json(&f.fact_type)?;
                    let default = if let Some(ref def) = f.default {
                        Some(parse_default_value(def, &fact_type)?)
                    } else {
                        None
                    };
                    facts.push(FactDecl {
                        id: f.id.clone(),
                        fact_type,
                        default,
                    });
                }
                InterchangeConstruct::Entity(e) => {
                    entities.push(Entity {
                        id: e.id.clone(),
                        states: e.states.clone(),
                        initial: e.initial.clone(),
                        transitions: e
                            .transitions
                            .iter()
                            .map(|t| Transition {
                                from: t.from.clone(),
                                to: t.to.clone(),
                            })
                            .collect(),
                    });
                }
                InterchangeConstruct::Rule(r) => {
                    let when = r.when().ok_or_else(|| EvalError::DeserializeError {
                        message: format!("Rule '{}' body missing 'when'", r.id),
                    })?;
                    let condition = parse_predicate(when)?;
                    let produce_obj = r.produce().ok_or_else(|| EvalError::DeserializeError {
                        message: format!("Rule '{}' body missing 'produce'", r.id),
                    })?;
                    let produce = parse_produce(produce_obj)?;
                    rules.push(Rule {
                        id: r.id.clone(),
                        stratum: r.stratum as u32,
                        condition,
                        produce,
                    });
                }
                InterchangeConstruct::Operation(op) => {
                    let precondition = if let Some(ref pre) = op.precondition {
                        parse_predicate(pre)?
                    } else {
                        // Null precondition means no precondition -- always true.
                        Predicate::Literal {
                            value: Value::Bool(true),
                            type_spec: TypeSpec {
                                base: "Bool".to_string(),
                                precision: None,
                                scale: None,
                                currency: None,
                                min: None,
                                max: None,
                                max_length: None,
                                values: None,
                                fields: None,
                                element_type: None,
                                unit: None,
                                variants: None,
                            },
                        }
                    };
                    let effects: Vec<Effect> = op
                        .effects
                        .iter()
                        .map(|e| Effect {
                            entity_id: e.entity_id.clone(),
                            from: e.from.clone(),
                            to: e.to.clone(),
                            outcome: e.outcome.clone(),
                        })
                        .collect();
                    let error_contract: Vec<String> = op
                        .error_contract
                        .as_ref()
                        .and_then(|e| e.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|e| e.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    operations.push(Operation {
                        id: op.id.clone(),
                        allowed_personas: op.allowed_personas.clone(),
                        precondition,
                        effects,
                        error_contract,
                        outcomes: op.outcomes.clone(),
                    });
                }
                InterchangeConstruct::Flow(f) => {
                    let steps: Vec<FlowStep> = f
                        .steps
                        .iter()
                        .map(parse_flow_step)
                        .collect::<Result<Vec<_>, EvalError>>()?;
                    flows.push(Flow {
                        id: f.id.clone(),
                        snapshot: f.snapshot.clone(),
                        entry: f.entry.clone(),
                        steps,
                    });
                }
                InterchangeConstruct::Persona(p) => {
                    personas.push(p.id.clone());
                }
                InterchangeConstruct::Source(_)
                | InterchangeConstruct::System(_)
                | InterchangeConstruct::TypeDecl(_) => {
                    // Source, System, and TypeDecl constructs are not used in evaluation
                }
            }
        }

        Ok(Contract::new(
            facts, entities, rules, operations, flows, personas,
        ))
    }
}

// ──────────────────────────────────────────────
// Contract sub-types
// ──────────────────────────────────────────────

/// An entity state machine.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub states: Vec<String>,
    pub initial: String,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
}

/// A verdict-producing rule with stratification.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub stratum: u32,
    pub condition: Predicate,
    pub produce: ProduceClause,
}

/// A produce clause specifying verdict output.
#[derive(Debug, Clone)]
pub struct ProduceClause {
    pub verdict_type: String,
    pub payload_type: TypeSpec,
    pub payload_value: PayloadValue,
}

/// Payload value: either a literal or a computed expression.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum PayloadValue {
    Literal(Value),
    Mul(MulExpr),
}

/// Multiplication expression in a payload.
#[derive(Debug, Clone)]
pub struct MulExpr {
    pub fact_ref: String,
    pub literal: i64,
    pub result_type: TypeSpec,
}

/// An operation (persona-gated state transition).
#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub allowed_personas: Vec<String>,
    pub precondition: Predicate,
    pub effects: Vec<Effect>,
    pub error_contract: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub entity_id: String,
    pub from: String,
    pub to: String,
    pub outcome: Option<String>,
}

/// A flow (DAG of steps).
#[derive(Debug, Clone)]
pub struct Flow {
    pub id: String,
    pub snapshot: String,
    pub entry: String,
    pub steps: Vec<FlowStep>,
}

/// A step within a flow.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum FlowStep {
    OperationStep {
        id: String,
        op: String,
        persona: String,
        outcomes: BTreeMap<String, StepTarget>,
        on_failure: FailureHandler,
    },
    BranchStep {
        id: String,
        condition: Predicate,
        persona: String,
        if_true: StepTarget,
        if_false: StepTarget,
    },
    HandoffStep {
        id: String,
        from_persona: String,
        to_persona: String,
        next: String,
    },
    SubFlowStep {
        id: String,
        flow: String,
        persona: String,
        on_success: StepTarget,
        on_failure: FailureHandler,
    },
    ParallelStep {
        id: String,
        branches: Vec<ParallelBranch>,
        join: JoinPolicy,
    },
}

#[derive(Debug, Clone)]
pub enum StepTarget {
    StepRef(String),
    Terminal { outcome: String },
}

#[derive(Debug, Clone)]
pub enum FailureHandler {
    Terminate {
        outcome: String,
    },
    Compensate {
        steps: Vec<CompStep>,
        then: StepTarget,
    },
    Escalate {
        to_persona: String,
        next: String,
    },
}

#[derive(Debug, Clone)]
pub struct CompStep {
    pub op: String,
    pub persona: String,
    pub on_failure: StepTarget,
}

#[derive(Debug, Clone)]
pub struct ParallelBranch {
    pub id: String,
    pub entry: String,
    pub steps: Vec<FlowStep>,
}

#[derive(Debug, Clone)]
pub struct JoinPolicy {
    pub on_all_success: Option<StepTarget>,
    pub on_any_failure: Option<FailureHandler>,
    pub on_all_complete: Option<StepTarget>,
}

// ──────────────────────────────────────────────
// Interchange JSON parsing helpers
// ──────────────────────────────────────────────

fn parse_produce(v: &serde_json::Value) -> Result<ProduceClause, EvalError> {
    let verdict_type = get_str(v, "verdict_type")?;
    let payload = v
        .get("payload")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce clause missing 'payload'".to_string(),
        })?;
    let type_val = payload
        .get("type")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce payload missing 'type'".to_string(),
        })?;
    let payload_type = TypeSpec::from_json(type_val)?;
    let value = payload
        .get("value")
        .ok_or_else(|| EvalError::DeserializeError {
            message: "produce payload missing 'value'".to_string(),
        })?;

    let payload_value =
        if value.is_object() && value.get("op").and_then(|o| o.as_str()) == Some("*") {
            // MulExpr
            let left = value
                .get("left")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'left'".to_string(),
                })?;
            let fact_ref = get_str(left, "fact_ref")?;
            let literal = value
                .get("literal")
                .and_then(|l| l.as_i64())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'literal'".to_string(),
                })?;
            let rt = value
                .get("result_type")
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "MulExpr missing 'result_type'".to_string(),
                })?;
            let result_type = TypeSpec::from_json(rt)?;
            PayloadValue::Mul(MulExpr {
                fact_ref,
                literal,
                result_type,
            })
        } else {
            // Literal value
            let lit = parse_literal_value(value, &payload_type)?;
            PayloadValue::Literal(lit)
        };

    Ok(ProduceClause {
        verdict_type,
        payload_type,
        payload_value,
    })
}

/// Parse a predicate expression from interchange JSON.
pub fn parse_predicate(v: &serde_json::Value) -> Result<Predicate, EvalError> {
    // Check for verdict_present
    if let Some(vp) = v.get("verdict_present") {
        let id = vp.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "verdict_present must be a string".to_string(),
        })?;
        return Ok(Predicate::VerdictPresent(id.to_string()));
    }

    // Check for fact_ref
    if let Some(fr) = v.get("fact_ref") {
        let id = fr.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "fact_ref must be a string".to_string(),
        })?;
        return Ok(Predicate::FactRef(id.to_string()));
    }

    // Check for field_ref
    if let Some(fr) = v.get("field_ref") {
        let var = get_str(fr, "var")?;
        let field = get_str(fr, "field")?;
        return Ok(Predicate::FieldRef { var, field });
    }

    // Check for op-based expressions BEFORE literal, because Mul nodes
    // have both "op" and "literal" fields -- the op check must come first.
    if let Some(op_val) = v.get("op") {
        let op = op_val.as_str().ok_or_else(|| EvalError::DeserializeError {
            message: "'op' must be a string".to_string(),
        })?;

        match op {
            "and" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "and missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "and missing 'right'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::And {
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
            "or" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "or missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "or missing 'right'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::Or {
                    left: Box::new(left),
                    right: Box::new(right),
                });
            }
            "not" => {
                let operand = parse_predicate(v.get("operand").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "not missing 'operand'".to_string(),
                    }
                })?)?;
                return Ok(Predicate::Not {
                    operand: Box::new(operand),
                });
            }
            "*" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "mul missing 'left'".to_string(),
                    })?)?;
                let literal = v.get("literal").and_then(|l| l.as_i64()).ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "mul missing 'literal'".to_string(),
                    }
                })?;
                let rt = v
                    .get("result_type")
                    .ok_or_else(|| EvalError::DeserializeError {
                        message: "mul missing 'result_type'".to_string(),
                    })?;
                let result_type = TypeSpec::from_json(rt)?;
                return Ok(Predicate::Mul {
                    left: Box::new(left),
                    literal,
                    result_type,
                });
            }
            // Comparison operators
            "=" | "!=" | "<" | "<=" | ">" | ">=" => {
                let left =
                    parse_predicate(v.get("left").ok_or_else(|| EvalError::DeserializeError {
                        message: "compare missing 'left'".to_string(),
                    })?)?;
                let right = parse_predicate(v.get("right").ok_or_else(|| {
                    EvalError::DeserializeError {
                        message: "compare missing 'right'".to_string(),
                    }
                })?)?;
                let comparison_type = if let Some(ct) = v.get("comparison_type") {
                    Some(TypeSpec::from_json(ct)?)
                } else {
                    None
                };
                return Ok(Predicate::Compare {
                    left: Box::new(left),
                    op: op.to_string(),
                    right: Box::new(right),
                    comparison_type,
                });
            }
            _ => {
                return Err(EvalError::DeserializeError {
                    message: format!("unknown operator: {}", op),
                });
            }
        }
    }

    // Check for literal (after op check, since Mul nodes also have "literal")
    if v.get("literal").is_some() {
        let literal_val = v.get("literal").unwrap();
        let (value, type_spec) = if let Some(type_val) = v.get("type") {
            let ts = TypeSpec::from_json(type_val)?;
            let val = parse_literal_value(literal_val, &ts)?;
            (val, ts)
        } else {
            // Infer type from the JSON literal value when "type" is absent
            // (the elaborator omits type for some literal nodes like text comparisons)
            infer_literal(literal_val)?
        };
        return Ok(Predicate::Literal { value, type_spec });
    }

    // Check for forall (quantifier)
    if v.get("quantifier").and_then(|q| q.as_str()) == Some("forall") {
        let variable = get_str(v, "variable")?;
        let vt = v
            .get("variable_type")
            .ok_or_else(|| EvalError::DeserializeError {
                message: "forall missing 'variable_type'".to_string(),
            })?;
        let variable_type = TypeSpec::from_json(vt)?;
        let domain_val = v.get("domain").ok_or_else(|| EvalError::DeserializeError {
            message: "forall missing 'domain'".to_string(),
        })?;
        let domain = parse_predicate(domain_val)?;
        let body_val = v.get("body").ok_or_else(|| EvalError::DeserializeError {
            message: "forall missing 'body'".to_string(),
        })?;
        let body = parse_predicate(body_val)?;
        return Ok(Predicate::Forall {
            variable,
            variable_type,
            domain: Box::new(domain),
            body: Box::new(body),
        });
    }

    // Check for exists (quantifier)
    if v.get("quantifier").and_then(|q| q.as_str()) == Some("exists") {
        let variable = get_str(v, "variable")?;
        let vt = v
            .get("variable_type")
            .ok_or_else(|| EvalError::DeserializeError {
                message: "exists missing 'variable_type'".to_string(),
            })?;
        let variable_type = TypeSpec::from_json(vt)?;
        let domain_val = v.get("domain").ok_or_else(|| EvalError::DeserializeError {
            message: "exists missing 'domain'".to_string(),
        })?;
        let domain = parse_predicate(domain_val)?;
        let body_val = v.get("body").ok_or_else(|| EvalError::DeserializeError {
            message: "exists missing 'body'".to_string(),
        })?;
        let body = parse_predicate(body_val)?;
        return Ok(Predicate::Exists {
            variable,
            variable_type,
            domain: Box::new(domain),
            body: Box::new(body),
        });
    }

    Err(EvalError::DeserializeError {
        message: format!("unrecognized predicate expression: {}", v),
    })
}

fn parse_flow_step(v: &serde_json::Value) -> Result<FlowStep, EvalError> {
    let kind = get_str(v, "kind")?;
    match kind.as_str() {
        "OperationStep" => {
            let id = get_str(v, "id")?;
            let op = get_str(v, "op")?;
            let persona = get_str(v, "persona")?;
            let outcomes_obj = v
                .get("outcomes")
                .and_then(|o| o.as_object())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "OperationStep missing 'outcomes'".to_string(),
                })?;
            let mut outcomes = BTreeMap::new();
            for (k, target_val) in outcomes_obj {
                outcomes.insert(k.clone(), parse_step_target(target_val)?);
            }
            let on_failure = parse_failure_handler(v.get("on_failure").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "OperationStep missing 'on_failure'".to_string(),
                }
            })?)?;
            Ok(FlowStep::OperationStep {
                id,
                op,
                persona,
                outcomes,
                on_failure,
            })
        }
        "BranchStep" => {
            let id = get_str(v, "id")?;
            let persona = get_str(v, "persona")?;
            let condition = parse_predicate(v.get("condition").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'condition'".to_string(),
                }
            })?)?;
            let if_true = parse_step_target(v.get("if_true").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'if_true'".to_string(),
                }
            })?)?;
            let if_false = parse_step_target(v.get("if_false").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "BranchStep missing 'if_false'".to_string(),
                }
            })?)?;
            Ok(FlowStep::BranchStep {
                id,
                condition,
                persona,
                if_true,
                if_false,
            })
        }
        "HandoffStep" => {
            let id = get_str(v, "id")?;
            let from_persona = get_str(v, "from_persona")?;
            let to_persona = get_str(v, "to_persona")?;
            let next = get_str(v, "next")?;
            Ok(FlowStep::HandoffStep {
                id,
                from_persona,
                to_persona,
                next,
            })
        }
        "SubFlowStep" => {
            let id = get_str(v, "id")?;
            let flow = get_str(v, "flow")?;
            let persona = get_str(v, "persona")?;
            let on_success = parse_step_target(v.get("on_success").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "SubFlowStep missing 'on_success'".to_string(),
                }
            })?)?;
            let on_failure = parse_failure_handler(v.get("on_failure").ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "SubFlowStep missing 'on_failure'".to_string(),
                }
            })?)?;
            Ok(FlowStep::SubFlowStep {
                id,
                flow,
                persona,
                on_success,
                on_failure,
            })
        }
        "ParallelStep" => {
            let id = get_str(v, "id")?;
            let branches_arr = v
                .get("branches")
                .and_then(|b| b.as_array())
                .ok_or_else(|| EvalError::DeserializeError {
                    message: "ParallelStep missing 'branches'".to_string(),
                })?;
            let branches: Vec<ParallelBranch> = branches_arr
                .iter()
                .map(|b| {
                    let bid = get_str(b, "id")?;
                    let entry = get_str(b, "entry")?;
                    let steps: Vec<FlowStep> = b
                        .get("steps")
                        .and_then(|s| s.as_array())
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(parse_flow_step)
                        .collect::<Result<Vec<_>, EvalError>>()?;
                    Ok(ParallelBranch {
                        id: bid,
                        entry,
                        steps,
                    })
                })
                .collect::<Result<Vec<_>, EvalError>>()?;
            let join_obj = v.get("join").ok_or_else(|| EvalError::DeserializeError {
                message: "ParallelStep missing 'join'".to_string(),
            })?;
            let on_all_success = if let Some(t) = join_obj.get("on_all_success") {
                Some(parse_step_target(t)?)
            } else {
                None
            };
            let on_any_failure = if let Some(f) = join_obj.get("on_any_failure") {
                Some(parse_failure_handler(f)?)
            } else {
                None
            };
            let on_all_complete = if let Some(t) = join_obj.get("on_all_complete") {
                Some(parse_step_target(t)?)
            } else {
                None
            };
            let join = JoinPolicy {
                on_all_success,
                on_any_failure,
                on_all_complete,
            };
            Ok(FlowStep::ParallelStep { id, branches, join })
        }
        _ => Err(EvalError::DeserializeError {
            message: format!("unknown step kind: {}", kind),
        }),
    }
}

fn parse_step_target(v: &serde_json::Value) -> Result<StepTarget, EvalError> {
    if let Some(s) = v.as_str() {
        Ok(StepTarget::StepRef(s.to_string()))
    } else if let Some(obj) = v.as_object() {
        let outcome = get_str(v, "outcome")?;
        let _ = obj; // already consumed via get_str on v
        Ok(StepTarget::Terminal { outcome })
    } else {
        Err(EvalError::DeserializeError {
            message: "invalid step target".to_string(),
        })
    }
}

fn parse_failure_handler(v: &serde_json::Value) -> Result<FailureHandler, EvalError> {
    let kind = get_str(v, "kind")?;
    match kind.as_str() {
        "Terminate" => {
            let outcome = get_str(v, "outcome")?;
            Ok(FailureHandler::Terminate { outcome })
        }
        "Compensate" => {
            let steps_arr = v.get("steps").and_then(|s| s.as_array()).ok_or_else(|| {
                EvalError::DeserializeError {
                    message: "Compensate handler missing 'steps'".to_string(),
                }
            })?;
            let steps: Vec<CompStep> = steps_arr
                .iter()
                .map(|s| {
                    let op = get_str(s, "op")?;
                    let persona = get_str(s, "persona")?;
                    let on_failure = parse_step_target(s.get("on_failure").ok_or_else(|| {
                        EvalError::DeserializeError {
                            message: "CompensationStep missing 'on_failure'".to_string(),
                        }
                    })?)?;
                    Ok(CompStep {
                        op,
                        persona,
                        on_failure,
                    })
                })
                .collect::<Result<Vec<_>, EvalError>>()?;
            let then =
                parse_step_target(v.get("then").ok_or_else(|| EvalError::DeserializeError {
                    message: "Compensate handler missing 'then'".to_string(),
                })?)?;
            Ok(FailureHandler::Compensate { steps, then })
        }
        "Escalate" => {
            let to_persona = get_str(v, "to_persona")?;
            let next = get_str(v, "next")?;
            Ok(FailureHandler::Escalate { to_persona, next })
        }
        _ => Err(EvalError::DeserializeError {
            message: format!("unknown failure handler kind: {}", kind),
        }),
    }
}
