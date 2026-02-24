//! Agent capabilities data extraction for the webview panel.
//!
//! Computes a structured view of the contract from the agent's perspective:
//! personas, operations grouped by persona, entity state machines, flows,
//! and static analysis findings.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

// ── Response structures ──────────────────────────────────────────────────────

/// Top-level agent capabilities response.
#[derive(Debug, Clone, Serialize)]
pub struct AgentCapabilities {
    pub contract_id: String,
    pub personas: Vec<PersonaView>,
    pub entities: Vec<EntityView>,
    pub operations: Vec<OperationView>,
    pub flows: Vec<FlowView>,
    pub analysis_findings: Vec<Finding>,
    pub error: Option<String>,
}

/// A persona and the operations it can invoke.
#[derive(Debug, Clone, Serialize)]
pub struct PersonaView {
    pub id: String,
    pub operations: Vec<String>,
}

/// An entity with its state machine.
#[derive(Debug, Clone, Serialize)]
pub struct EntityView {
    pub id: String,
    pub states: Vec<String>,
    pub initial_state: String,
    pub transitions: Vec<(String, String)>,
}

/// An operation as seen by an agent.
#[derive(Debug, Clone, Serialize)]
pub struct OperationView {
    pub id: String,
    pub allowed_personas: Vec<String>,
    pub parameters: Vec<ParameterInfo>,
    pub preconditions: Vec<String>,
    pub effects: Vec<EffectInfo>,
    pub outcomes: Vec<String>,
    pub postconditions: Vec<String>,
}

/// A parameter (fact reference) for an operation.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterInfo {
    pub name: String,
    pub fact_type: String,
}

/// An entity state transition triggered by an operation.
#[derive(Debug, Clone, Serialize)]
pub struct EffectInfo {
    pub entity: String,
    pub transition: String,
}

/// A flow summary.
#[derive(Debug, Clone, Serialize)]
pub struct FlowView {
    pub id: String,
    pub entry_point: String,
    pub step_count: usize,
    pub step_summary: Vec<String>,
}

/// An analysis finding.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: String,
    pub analysis: String,
    pub message: String,
}

// ── Computation ──────────────────────────────────────────────────────────────

/// Compute agent capabilities from a .tenor file.
///
/// Runs the full elaboration pipeline and optional static analysis,
/// returning a structured view suitable for the webview panel.
pub fn compute_agent_capabilities(file_path: &Path) -> AgentCapabilities {
    // Step 1: Elaborate
    let bundle = match tenor_core::elaborate::elaborate(file_path) {
        Ok(b) => b,
        Err(e) => {
            return AgentCapabilities {
                contract_id: String::new(),
                personas: Vec::new(),
                entities: Vec::new(),
                operations: Vec::new(),
                flows: Vec::new(),
                analysis_findings: Vec::new(),
                error: Some(e.message.to_string()),
            };
        }
    };

    // Step 2: Extract constructs from interchange JSON
    let contract_id = bundle
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let constructs = bundle
        .get("constructs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut entities = Vec::new();
    let mut operations = Vec::new();
    let mut flows = Vec::new();
    let mut facts_map: BTreeMap<String, String> = BTreeMap::new();
    let mut persona_ops: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for construct in &constructs {
        let kind = construct.get("kind").and_then(|v| v.as_str()).unwrap_or("");

        match kind {
            "Entity" => {
                if let Some(entity) = extract_entity(construct) {
                    entities.push(entity);
                }
            }
            "Operation" => {
                if let Some(op) = extract_operation(construct, &facts_map) {
                    // Track persona -> operations mapping
                    for persona in &op.allowed_personas {
                        persona_ops
                            .entry(persona.clone())
                            .or_default()
                            .push(op.id.clone());
                    }
                    operations.push(op);
                }
            }
            "Flow" => {
                if let Some(flow) = extract_flow(construct) {
                    flows.push(flow);
                }
            }
            "Fact" => {
                let id = construct
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let fact_type = describe_type(construct.get("type"));
                facts_map.insert(id, fact_type);
            }
            "Persona" => {
                let id = construct
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !id.is_empty() {
                    persona_ops.entry(id).or_default();
                }
            }
            _ => {}
        }
    }

    // Also extract operations again with facts now fully populated
    // (facts may appear after operations in the construct list)
    let mut operations_updated = Vec::new();
    for construct in &constructs {
        let kind = construct.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind == "Operation" {
            if let Some(op) = extract_operation(construct, &facts_map) {
                operations_updated.push(op);
            }
        }
    }
    if !operations_updated.is_empty() {
        operations = operations_updated;
    }

    // Build persona views
    let personas: Vec<PersonaView> = persona_ops
        .into_iter()
        .map(|(id, ops)| PersonaView {
            id,
            operations: ops,
        })
        .collect();

    // Step 3: Run analysis (non-fatal)
    let analysis_findings = match tenor_analyze::analyze(&bundle) {
        Ok(report) => extract_findings(&report),
        Err(_) => Vec::new(),
    };

    AgentCapabilities {
        contract_id,
        personas,
        entities,
        operations,
        flows,
        analysis_findings,
        error: None,
    }
}

// ── Extraction helpers ───────────────────────────────────────────────────────

fn extract_entity(construct: &serde_json::Value) -> Option<EntityView> {
    let id = construct.get("id")?.as_str()?.to_string();
    let states: Vec<String> = construct
        .get("states")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let initial_state = construct
        .get("initial")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let transitions: Vec<(String, String)> = construct
        .get("transitions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let from = t.get("from")?.as_str()?.to_string();
                    let to = t.get("to")?.as_str()?.to_string();
                    Some((from, to))
                })
                .collect()
        })
        .unwrap_or_default();

    Some(EntityView {
        id,
        states,
        initial_state,
        transitions,
    })
}

fn extract_operation(
    construct: &serde_json::Value,
    facts_map: &BTreeMap<String, String>,
) -> Option<OperationView> {
    let id = construct.get("id")?.as_str()?.to_string();
    let allowed_personas: Vec<String> = construct
        .get("allowed_personas")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Extract parameters from precondition fact_refs
    let mut parameters = Vec::new();
    if let Some(precondition) = construct.get("precondition") {
        if !precondition.is_null() {
            let fact_refs = collect_fact_refs(precondition);
            for fact_ref in fact_refs {
                let fact_type = facts_map
                    .get(&fact_ref)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                parameters.push(ParameterInfo {
                    name: fact_ref,
                    fact_type,
                });
            }
        }
    }

    // Extract precondition summary
    let preconditions = if let Some(precondition) = construct.get("precondition") {
        if !precondition.is_null() {
            vec![describe_condition(precondition)]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Extract effects
    let effects: Vec<EffectInfo> = construct
        .get("effects")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let entity = e.get("entity_id")?.as_str()?.to_string();
                    let from = e.get("from")?.as_str()?;
                    let to = e.get("to")?.as_str()?;
                    Some(EffectInfo {
                        entity,
                        transition: format!("{} -> {}", from, to),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract outcomes
    let outcomes: Vec<String> = construct
        .get("outcomes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(OperationView {
        id,
        allowed_personas,
        parameters,
        preconditions,
        effects,
        outcomes,
        postconditions: Vec::new(),
    })
}

fn extract_flow(construct: &serde_json::Value) -> Option<FlowView> {
    let id = construct.get("id")?.as_str()?.to_string();
    let entry_point = construct
        .get("entry")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let steps = construct
        .get("steps")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let step_count = steps.len();

    let step_summary: Vec<String> = steps
        .iter()
        .filter_map(|step| {
            let kind = step.get("kind")?.as_str()?;
            let step_id = step.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            match kind {
                "OperationStep" => {
                    let op = step.get("op").and_then(|v| v.as_str()).unwrap_or("?");
                    let persona = step.get("persona").and_then(|v| v.as_str()).unwrap_or("?");
                    Some(format!("{}: {} performs {}", step_id, persona, op))
                }
                "BranchStep" => Some(format!("{}: branch decision", step_id)),
                "HandoffStep" => {
                    let from = step
                        .get("from_persona")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let to = step
                        .get("to_persona")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    Some(format!("{}: handoff {} -> {}", step_id, from, to))
                }
                "SubFlowStep" => {
                    let flow = step.get("flow").and_then(|v| v.as_str()).unwrap_or("?");
                    Some(format!("{}: sub-flow {}", step_id, flow))
                }
                "ParallelStep" => Some(format!("{}: parallel execution", step_id)),
                _ => Some(format!("{}: {}", step_id, kind)),
            }
        })
        .collect();

    Some(FlowView {
        id,
        entry_point,
        step_count,
        step_summary,
    })
}

/// Collect all fact_ref values from a JSON expression tree.
fn collect_fact_refs(expr: &serde_json::Value) -> Vec<String> {
    let mut refs = Vec::new();
    collect_fact_refs_inner(expr, &mut refs);
    // Deduplicate while preserving order
    let mut seen = std::collections::BTreeSet::new();
    refs.retain(|r| seen.insert(r.clone()));
    refs
}

fn collect_fact_refs_inner(expr: &serde_json::Value, refs: &mut Vec<String>) {
    if let Some(fact_ref) = expr.get("fact_ref").and_then(|v| v.as_str()) {
        refs.push(fact_ref.to_string());
    }
    if let Some(left) = expr.get("left") {
        collect_fact_refs_inner(left, refs);
    }
    if let Some(right) = expr.get("right") {
        collect_fact_refs_inner(right, refs);
    }
    if let Some(operand) = expr.get("operand") {
        collect_fact_refs_inner(operand, refs);
    }
}

/// Describe a type JSON value as a human-readable string.
fn describe_type(type_val: Option<&serde_json::Value>) -> String {
    match type_val {
        None => "unknown".to_string(),
        Some(v) => {
            if v.is_null() {
                return "unknown".to_string();
            }
            if let Some(name) = v.get("name").and_then(|v| v.as_str()) {
                return name.to_string();
            }
            let base = v.get("base").and_then(|v| v.as_str()).unwrap_or("unknown");
            match base {
                "Enum" => {
                    if let Some(vals) = v.get("values").and_then(|v| v.as_array()) {
                        let names: Vec<&str> = vals.iter().filter_map(|v| v.as_str()).collect();
                        format!("Enum({})", names.join(", "))
                    } else {
                        "Enum".to_string()
                    }
                }
                "Decimal" => {
                    let p = v.get("precision").and_then(|v| v.as_i64());
                    let s = v.get("scale").and_then(|v| v.as_i64());
                    match (p, s) {
                        (Some(p), Some(s)) => format!("Decimal({}, {})", p, s),
                        _ => "Decimal".to_string(),
                    }
                }
                "Money" => {
                    let cur = v.get("currency").and_then(|v| v.as_str());
                    match cur {
                        Some(c) => format!("Money({})", c),
                        None => "Money".to_string(),
                    }
                }
                _ => base.to_string(),
            }
        }
    }
}

/// Describe a condition expression as human-readable text.
fn describe_condition(cond: &serde_json::Value) -> String {
    if let Some(vp) = cond.get("verdict_present").and_then(|v| v.as_str()) {
        return format!("verdict '{}' is present", vp);
    }
    if let Some(op) = cond.get("op").and_then(|v| v.as_str()) {
        let left = cond.get("left");
        let right = cond.get("right");
        match op {
            "not" => {
                let operand = cond.get("operand");
                let operand_str = operand
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                return format!("not ({})", operand_str);
            }
            "and" | "or" => {
                let left_str = left
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                let right_str = right
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                return format!("({} {} {})", left_str, op, right_str);
            }
            _ => {
                let left_str = describe_expr(left);
                let right_str = describe_expr(right);
                return format!("{} {} {}", left_str, op, right_str);
            }
        }
    }
    if let Some(fact_ref) = cond.get("fact_ref").and_then(|v| v.as_str()) {
        return fact_ref.to_string();
    }
    "?".to_string()
}

fn describe_expr(expr: Option<&serde_json::Value>) -> String {
    match expr {
        None => "?".to_string(),
        Some(v) => {
            if let Some(fact_ref) = v.get("fact_ref").and_then(|v| v.as_str()) {
                return fact_ref.to_string();
            }
            if let Some(lit) = v.get("literal") {
                if let Some(b) = lit.as_bool() {
                    return format!("{}", b);
                }
                if let Some(n) = lit.as_i64() {
                    return format!("{}", n);
                }
                if let Some(s) = lit.as_str() {
                    return format!("\"{}\"", s);
                }
                return format!("{}", lit);
            }
            if let Some(s) = v.as_str() {
                return s.to_string();
            }
            "?".to_string()
        }
    }
}

/// Extract human-readable findings from an analysis report.
fn extract_findings(report: &tenor_analyze::AnalysisReport) -> Vec<Finding> {
    report
        .findings
        .iter()
        .map(|f| {
            let severity = match f.severity {
                tenor_analyze::FindingSeverity::Warning => "warning".to_string(),
                tenor_analyze::FindingSeverity::Info => "info".to_string(),
            };
            Finding {
                severity,
                analysis: f.analysis.clone(),
                message: f.message.clone(),
            }
        })
        .collect()
}
