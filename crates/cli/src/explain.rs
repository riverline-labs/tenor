//! `tenor explain` — human-readable contract summary.
//!
//! Produces a 4-section contract summary:
//! 1. Contract Summary — what the contract contains
//! 2. Decision Flow Narrative — step-by-step process description
//! 3. Fact Inventory — all facts with types and sources
//! 4. Risk / Coverage Notes — analysis findings from S1-S8
//!
//! Uses typed structs from `tenor-interchange` for deserialization so that
//! interchange format changes cause compile errors across all consumers
//! (eval, analyze, codegen, explain) instead of silently dropping output.

use std::collections::BTreeMap;
use tenor_interchange::{
    EntityConstruct, FactConstruct, FlowConstruct, InterchangeConstruct, OperationConstruct,
    PersonaConstruct, RuleConstruct,
};

/// Output format for the explain command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplainFormat {
    Terminal,
    Markdown,
}

/// Produce a human-readable contract summary.
///
/// `raw_bundle` is the interchange JSON value (kind: "Bundle").
/// Returns the formatted string (styled terminal text or markdown),
/// or an error if the bundle cannot be deserialized into the expected structure.
pub fn explain(
    raw_bundle: &serde_json::Value,
    format: ExplainFormat,
    verbose: bool,
) -> Result<String, String> {
    let bundle = tenor_interchange::from_interchange(raw_bundle)
        .map_err(|e| format!("failed to parse interchange bundle: {}", e))?;

    let mut out = String::new();

    // Classify constructs by kind
    let mut facts: Vec<&FactConstruct> = Vec::new();
    let mut entities: Vec<&EntityConstruct> = Vec::new();
    let mut personas: Vec<&PersonaConstruct> = Vec::new();
    let mut rules: Vec<&RuleConstruct> = Vec::new();
    let mut operations: Vec<&OperationConstruct> = Vec::new();
    let mut flows: Vec<&FlowConstruct> = Vec::new();

    for c in &bundle.constructs {
        match c {
            InterchangeConstruct::Fact(f) => facts.push(f),
            InterchangeConstruct::Entity(e) => entities.push(e),
            InterchangeConstruct::Persona(p) => personas.push(p),
            InterchangeConstruct::Rule(r) => rules.push(r),
            InterchangeConstruct::Operation(o) => operations.push(o),
            InterchangeConstruct::Flow(f) => flows.push(f),
            // Source, TypeDecl, and System are not rendered in explain output
            InterchangeConstruct::Source(_)
            | InterchangeConstruct::TypeDecl(_)
            | InterchangeConstruct::System(_) => {}
        }
    }

    // Build an operation lookup by id for flow narrative
    let op_map: BTreeMap<&str, &OperationConstruct> =
        operations.iter().map(|op| (op.id.as_str(), *op)).collect();

    // Section 1: Contract Summary
    section_contract_summary(
        &mut out,
        format,
        &bundle.id,
        &facts,
        &entities,
        &personas,
        &rules,
        &operations,
        &flows,
        verbose,
    );

    // Section 2: Decision Flow Narrative
    section_flow_narrative(&mut out, format, &flows, &op_map, verbose);

    // Section 3: Fact Inventory
    section_fact_inventory(&mut out, format, &facts, verbose);

    // Section 4: Risk / Coverage Notes
    // This section uses tenor_analyze which requires the raw serde_json::Value
    section_risk_coverage(&mut out, format, raw_bundle, verbose);

    Ok(out)
}

// ─── Section 1: Contract Summary ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn section_contract_summary(
    out: &mut String,
    format: ExplainFormat,
    contract_id: &str,
    facts: &[&FactConstruct],
    entities: &[&EntityConstruct],
    personas: &[&PersonaConstruct],
    rules: &[&RuleConstruct],
    operations: &[&OperationConstruct],
    flows: &[&FlowConstruct],
    verbose: bool,
) {
    heading(out, format, "CONTRACT SUMMARY");

    emit_line(
        out,
        format,
        &format!("Name: {}", styled_name(format, contract_id)),
    );

    // Entity summary with state counts
    if !entities.is_empty() {
        let entity_parts: Vec<String> = entities
            .iter()
            .map(|e| format!("{} ({} states)", styled_name(format, &e.id), e.states.len()))
            .collect();
        emit_line(
            out,
            format,
            &format!("Entities: {}", entity_parts.join(", ")),
        );
    }

    emit_line(out, format, &format!("Personas: {}", personas.len()));

    // Rule summary with strata
    let strata = count_strata(rules);
    emit_line(
        out,
        format,
        &format!(
            "Rules: {} across {} strat{}",
            rules.len(),
            strata,
            if strata == 1 { "um" } else { "a" }
        ),
    );

    emit_line(out, format, &format!("Operations: {}", operations.len()));

    emit_line(out, format, &format!("Facts: {}", facts.len()));

    emit_line(out, format, &format!("Flows: {}", flows.len()));

    if verbose {
        // Verbose: list persona names
        if !personas.is_empty() {
            let names: Vec<&str> = personas.iter().map(|p| p.id.as_str()).collect();
            emit_line(
                out,
                format,
                &format!("  Persona list: {}", names.join(", ")),
            );
        }

        // Verbose: list entity states
        for e in entities {
            let state_names = &e.states;
            emit_line(
                out,
                format,
                &format!("  {} states: {}", e.id, state_names.join(", ")),
            );
        }

        // Verbose: rule strata breakdown
        let mut strata_counts: BTreeMap<u64, usize> = BTreeMap::new();
        for r in rules {
            *strata_counts.entry(r.stratum).or_insert(0) += 1;
        }
        for (s, count) in &strata_counts {
            emit_line(out, format, &format!("  Stratum {}: {} rule(s)", s, count));
        }
    }

    out.push('\n');
}

fn count_strata(rules: &[&RuleConstruct]) -> usize {
    let mut strata = std::collections::BTreeSet::new();
    for r in rules {
        strata.insert(r.stratum);
    }
    strata.len()
}

// ─── Section 2: Decision Flow Narrative ──────────────────────────────────────

fn section_flow_narrative(
    out: &mut String,
    format: ExplainFormat,
    flows: &[&FlowConstruct],
    op_map: &BTreeMap<&str, &OperationConstruct>,
    verbose: bool,
) {
    heading(out, format, "DECISION FLOW NARRATIVE");

    if flows.is_empty() {
        emit_line(out, format, "No flows defined in this contract.");
        out.push('\n');
        return;
    }

    for flow in flows {
        emit_line(
            out,
            format,
            &format!("Flow: {}", styled_name(format, &flow.id)),
        );
        emit_line(out, format, &format!("  Entry point: {}", flow.entry));
        out.push('\n');

        // Build step index for ordered walk
        // Steps remain as serde_json::Value because their structure is highly
        // polymorphic (OperationStep, BranchStep, HandoffStep, SubFlowStep,
        // ParallelStep) with deeply nested condition/outcome trees.
        let step_map: BTreeMap<&str, &serde_json::Value> = flow
            .steps
            .iter()
            .filter_map(|s: &serde_json::Value| {
                s.get("id").and_then(|v| v.as_str()).map(|id| (id, s))
            })
            .collect();

        // Walk from entry
        let mut visited = std::collections::HashSet::new();
        walk_steps(
            out,
            format,
            &flow.entry,
            &step_map,
            op_map,
            verbose,
            &mut visited,
            1,
        );

        out.push('\n');
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_steps(
    out: &mut String,
    format: ExplainFormat,
    step_id: &str,
    step_map: &BTreeMap<&str, &serde_json::Value>,
    op_map: &BTreeMap<&str, &OperationConstruct>,
    verbose: bool,
    visited: &mut std::collections::HashSet<String>,
    depth: usize,
) {
    if visited.contains(step_id) {
        emit_line(
            out,
            format,
            &format!("{}(back to {})", indent(depth), step_id),
        );
        return;
    }
    visited.insert(step_id.to_string());

    let step = match step_map.get(step_id) {
        Some(s) => *s,
        None => {
            emit_line(
                out,
                format,
                &format!("{}[unknown step: {}]", indent(depth), step_id),
            );
            return;
        }
    };

    let kind = step.get("kind").and_then(|v| v.as_str()).unwrap_or("?");

    match kind {
        "OperationStep" => {
            describe_operation_step(out, format, step, op_map, verbose, depth);
            if let Some(outcomes) = step.get("outcomes").and_then(|v| v.as_object()) {
                for (_outcome_name, target) in outcomes {
                    if let Some(next_id) = resolve_step_target(target) {
                        walk_steps(
                            out, format, &next_id, step_map, op_map, verbose, visited, depth,
                        );
                    }
                }
            }
        }
        "BranchStep" => {
            describe_branch_step(out, format, step, verbose, depth);
            let if_true = step.get("if_true");
            let if_false = step.get("if_false");

            if let Some(target) = if_true {
                if let Some(next_id) = resolve_step_target(target) {
                    walk_steps(
                        out,
                        format,
                        &next_id,
                        step_map,
                        op_map,
                        verbose,
                        visited,
                        depth + 1,
                    );
                }
            }
            if let Some(target) = if_false {
                if let Some(next_id) = resolve_step_target(target) {
                    let mut else_visited = visited.clone();
                    walk_steps(
                        out,
                        format,
                        &next_id,
                        step_map,
                        op_map,
                        verbose,
                        &mut else_visited,
                        depth + 1,
                    );
                    visited.extend(else_visited);
                }
            }
        }
        "HandoffStep" => {
            describe_handoff_step(out, format, step, depth);
            if let Some(next_id) = step.get("next").and_then(|v| v.as_str()) {
                walk_steps(
                    out, format, next_id, step_map, op_map, verbose, visited, depth,
                );
            }
        }
        "SubFlowStep" => {
            describe_subflow_step(out, format, step, depth);
            if let Some(target) = step.get("on_success") {
                if let Some(next_id) = resolve_step_target(target) {
                    walk_steps(
                        out, format, &next_id, step_map, op_map, verbose, visited, depth,
                    );
                }
            }
        }
        "ParallelStep" => {
            describe_parallel_step(out, format, step, op_map, verbose, depth);
            if let Some(join) = step.get("join") {
                if let Some(next_id) = join.get("on_all_success").and_then(|v| v.as_str()) {
                    walk_steps(
                        out, format, next_id, step_map, op_map, verbose, visited, depth,
                    );
                }
            }
        }
        _ => {
            emit_line(
                out,
                format,
                &format!("{}[{}: {}]", indent(depth), kind, step_id),
            );
        }
    }
}

fn resolve_step_target(target: &serde_json::Value) -> Option<String> {
    if let Some(s) = target.as_str() {
        return Some(s.to_string());
    }
    None
}

fn describe_operation_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    op_map: &BTreeMap<&str, &OperationConstruct>,
    verbose: bool,
    depth: usize,
) {
    let _step_id = step.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let op_id = step.get("op").and_then(|v| v.as_str()).unwrap_or("?");
    let persona = step.get("persona").and_then(|v| v.as_str()).unwrap_or("?");

    let op_desc = humanize_id(op_id);
    emit_line(
        out,
        format,
        &format!(
            "{}{} performs: {}",
            indent(depth),
            styled_name(format, persona),
            op_desc,
        ),
    );

    if verbose {
        if let Some(op) = op_map.get(op_id) {
            if let Some(ref precondition) = op.precondition {
                let pre_str = describe_condition(precondition);
                emit_line(
                    out,
                    format,
                    &format!("{}  Precondition: {}", indent(depth), pre_str),
                );
            }
            for eff in &op.effects {
                emit_line(
                    out,
                    format,
                    &format!(
                        "{}  Effect: {} transitions {} -> {}",
                        indent(depth),
                        eff.entity_id,
                        eff.from,
                        eff.to,
                    ),
                );
            }
        }
    }

    if let Some(on_failure) = step.get("on_failure") {
        let fail_kind = on_failure
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        if fail_kind == "Terminate" {
            let outcome = on_failure
                .get("outcome")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            emit_line(
                out,
                format,
                &format!(
                    "{}  On failure: process ends ({})",
                    indent(depth),
                    humanize_id(outcome),
                ),
            );
        } else if fail_kind == "Escalate" {
            let to_persona = on_failure
                .get("to_persona")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            emit_line(
                out,
                format,
                &format!(
                    "{}  On failure: escalate to {}",
                    indent(depth),
                    styled_name(format, to_persona),
                ),
            );
        }
    }

    if let Some(outcomes) = step.get("outcomes").and_then(|v| v.as_object()) {
        for (outcome_name, target) in outcomes {
            if let Some(obj) = target.as_object() {
                if obj.get("kind").and_then(|v| v.as_str()) == Some("Terminal") {
                    let terminal_outcome =
                        obj.get("outcome").and_then(|v| v.as_str()).unwrap_or("?");
                    emit_line(
                        out,
                        format,
                        &format!(
                            "{}  On {}: process completes ({})",
                            indent(depth),
                            outcome_name,
                            humanize_id(terminal_outcome),
                        ),
                    );
                }
            }
        }
    }
}

fn describe_branch_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    verbose: bool,
    depth: usize,
) {
    let condition = step.get("condition");
    let cond_str = condition
        .map(describe_condition)
        .unwrap_or_else(|| "?".to_string());

    emit_line(
        out,
        format,
        &format!("{}Decision: {}", indent(depth), cond_str),
    );

    let if_true_desc = describe_target(step.get("if_true"));
    let if_false_desc = describe_target(step.get("if_false"));

    emit_line(
        out,
        format,
        &format!("{}  If yes: {}", indent(depth), if_true_desc),
    );
    emit_line(
        out,
        format,
        &format!("{}  If no:  {}", indent(depth), if_false_desc),
    );

    if verbose {
        if let Some(persona) = step.get("persona").and_then(|v| v.as_str()) {
            emit_line(
                out,
                format,
                &format!("{}  Decided by: {}", indent(depth), persona),
            );
        }
    }
}

fn describe_handoff_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    depth: usize,
) {
    let from = step
        .get("from_persona")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let to = step
        .get("to_persona")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    emit_line(
        out,
        format,
        &format!(
            "{}Handed off from {} to {}",
            indent(depth),
            styled_name(format, from),
            styled_name(format, to),
        ),
    );
}

fn describe_subflow_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    depth: usize,
) {
    let sub_flow_id = step.get("flow").and_then(|v| v.as_str()).unwrap_or("?");

    emit_line(
        out,
        format,
        &format!(
            "{}Sub-process: {}",
            indent(depth),
            styled_name(format, sub_flow_id),
        ),
    );

    if let Some(on_success) = step.get("on_success") {
        let desc = describe_inline_target(on_success);
        emit_line(
            out,
            format,
            &format!("{}  On success: {}", indent(depth), desc),
        );
    }
    if let Some(on_failure) = step.get("on_failure") {
        let desc = describe_inline_target(on_failure);
        emit_line(
            out,
            format,
            &format!("{}  On failure: {}", indent(depth), desc),
        );
    }
}

fn describe_parallel_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    _op_map: &BTreeMap<&str, &OperationConstruct>,
    _verbose: bool,
    depth: usize,
) {
    emit_line(out, format, &format!("{}Concurrently:", indent(depth)));

    if let Some(branches) = step.get("branches").and_then(|v| v.as_array()) {
        for branch in branches {
            let branch_id = branch.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let entry = branch.get("entry").and_then(|v| v.as_str()).unwrap_or("?");
            emit_line(
                out,
                format,
                &format!(
                    "{}  Branch {}: starts at {}",
                    indent(depth),
                    styled_name(format, branch_id),
                    entry,
                ),
            );
        }
    }

    if let Some(join) = step.get("join") {
        if let Some(next) = join.get("on_all_success").and_then(|v| v.as_str()) {
            emit_line(
                out,
                format,
                &format!("{}  When all complete: continue to {}", indent(depth), next),
            );
        }
        if let Some(on_fail) = join.get("on_any_failure") {
            let desc = describe_inline_target(on_fail);
            emit_line(
                out,
                format,
                &format!("{}  If any fails: {}", indent(depth), desc),
            );
        }
    }
}

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
    format!("{}", cond)
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
            format!("{}", v)
        }
    }
}

fn describe_target(target: Option<&serde_json::Value>) -> String {
    match target {
        None => "?".to_string(),
        Some(v) => {
            if let Some(s) = v.as_str() {
                return format!("continue to {}", s);
            }
            describe_inline_target(v)
        }
    }
}

fn describe_inline_target(target: &serde_json::Value) -> String {
    if let Some(s) = target.as_str() {
        return format!("continue to {}", s);
    }
    if let Some(obj) = target.as_object() {
        let kind = obj.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
        match kind {
            "Terminal" => {
                let outcome = obj.get("outcome").and_then(|v| v.as_str()).unwrap_or("?");
                return format!("process completes ({})", humanize_id(outcome));
            }
            "Terminate" => {
                let outcome = obj.get("outcome").and_then(|v| v.as_str()).unwrap_or("?");
                return format!("process ends ({})", humanize_id(outcome));
            }
            _ => return format!("{}", target),
        }
    }
    format!("{}", target)
}

// ─── Section 3: Fact Inventory ───────────────────────────────────────────────

fn section_fact_inventory(
    out: &mut String,
    format: ExplainFormat,
    facts: &[&FactConstruct],
    verbose: bool,
) {
    heading(out, format, "FACT INVENTORY");

    if facts.is_empty() {
        emit_line(out, format, "No facts defined in this contract.");
        out.push('\n');
        return;
    }

    let mut grouped: BTreeMap<&str, Vec<&FactConstruct>> = BTreeMap::new();
    for fact in facts {
        let category = categorize_fact_type(&fact.fact_type);
        grouped.entry(category).or_default().push(fact);
    }

    match format {
        ExplainFormat::Markdown => {
            out.push_str("| Fact | Type | Source | Default |\n");
            out.push_str("|------|------|--------|---------|\n");
            for group_facts in grouped.values() {
                for fact in group_facts {
                    let type_str = describe_fact_type(&fact.fact_type, verbose);
                    let source_str = describe_source(&fact.source);
                    let default_str = describe_default(fact.default.as_ref());
                    out.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        fact.id, type_str, source_str, default_str
                    ));
                }
            }
        }
        ExplainFormat::Terminal => {
            let mut max_id = 4;
            let mut max_type = 4;
            let mut max_source = 6;
            for fact in facts {
                let type_str = describe_fact_type(&fact.fact_type, verbose);
                let source_str = describe_source(&fact.source);
                max_id = max_id.max(fact.id.len());
                max_type = max_type.max(type_str.len());
                max_source = max_source.max(source_str.len());
            }

            out.push_str(&format!(
                "  {:<id_w$}  {:<type_w$}  {:<src_w$}  {}\n",
                "FACT",
                "TYPE",
                "SOURCE",
                "DEFAULT",
                id_w = max_id,
                type_w = max_type,
                src_w = max_source,
            ));
            out.push_str(&format!(
                "  {:<id_w$}  {:<type_w$}  {:<src_w$}  {}\n",
                "-".repeat(max_id),
                "-".repeat(max_type),
                "-".repeat(max_source),
                "-------",
                id_w = max_id,
                type_w = max_type,
                src_w = max_source,
            ));

            for (category, group_facts) in &grouped {
                out.push_str(&format!("  [{}]\n", category));
                for fact in group_facts {
                    let type_str = describe_fact_type(&fact.fact_type, verbose);
                    let source_str = describe_source(&fact.source);
                    let default_str = describe_default(fact.default.as_ref());
                    out.push_str(&format!(
                        "  {:<id_w$}  {:<type_w$}  {:<src_w$}  {}\n",
                        fact.id,
                        type_str,
                        source_str,
                        default_str,
                        id_w = max_id,
                        type_w = max_type,
                        src_w = max_source,
                    ));
                }
            }
        }
    }

    out.push('\n');
}

fn categorize_fact_type(type_val: &serde_json::Value) -> &'static str {
    let base = type_val.get("base").and_then(|v| v.as_str()).unwrap_or("");
    match base {
        "Int" | "Decimal" => "numeric",
        "Bool" => "boolean",
        "Enum" => "enum",
        "Money" => "numeric",
        "Text" | "Date" | "Duration" => "text/temporal",
        "Record" => "record",
        "List" => "list",
        _ => {
            if type_val.get("name").is_some() {
                "record"
            } else {
                "other"
            }
        }
    }
}

fn describe_fact_type(type_val: &serde_json::Value, verbose: bool) -> String {
    if type_val.is_null() {
        return "?".to_string();
    }
    if let Some(name) = type_val.get("name").and_then(|v| v.as_str()) {
        return name.to_string();
    }
    let base = type_val.get("base").and_then(|v| v.as_str()).unwrap_or("?");
    if verbose {
        match base {
            "Int" => {
                let min = type_val.get("min").and_then(|v| v.as_i64());
                let max = type_val.get("max").and_then(|v| v.as_i64());
                match (min, max) {
                    (Some(mn), Some(mx)) => format!("Int({} .. {})", mn, mx),
                    (Some(mn), None) => format!("Int(min: {})", mn),
                    (None, Some(mx)) => format!("Int(max: {})", mx),
                    _ => "Int".to_string(),
                }
            }
            "Decimal" => {
                let p = type_val.get("precision").and_then(|v| v.as_i64());
                let s = type_val.get("scale").and_then(|v| v.as_i64());
                match (p, s) {
                    (Some(p), Some(s)) => format!("Decimal({}, {})", p, s),
                    _ => "Decimal".to_string(),
                }
            }
            "Money" => match type_val.get("currency").and_then(|v| v.as_str()) {
                Some(c) => format!("Money({})", c),
                None => "Money".to_string(),
            },
            "Enum" => {
                if let Some(vals) = type_val.get("values").and_then(|v| v.as_array()) {
                    let names: Vec<&str> = vals.iter().filter_map(|v| v.as_str()).collect();
                    format!("Enum({})", names.join(", "))
                } else {
                    "Enum".to_string()
                }
            }
            _ => base.to_string(),
        }
    } else {
        match base {
            "Enum" => {
                if let Some(vals) = type_val.get("values").and_then(|v| v.as_array()) {
                    format!("Enum({} values)", vals.len())
                } else {
                    "Enum".to_string()
                }
            }
            _ => base.to_string(),
        }
    }
}

fn describe_source(source: &Option<serde_json::Value>) -> String {
    match source {
        None => "-".to_string(),
        Some(s) => {
            let system = s.get("system").and_then(|v| v.as_str()).unwrap_or("?");
            let field = s.get("field").and_then(|v| v.as_str()).unwrap_or("?");
            format!("{}.{}", system, field)
        }
    }
}

fn describe_default(default: Option<&serde_json::Value>) -> String {
    match default {
        None => "-".to_string(),
        Some(default) => {
            if let Some(b) = default.get("value") {
                if let Some(bv) = b.as_bool() {
                    return format!("{}", bv);
                }
                if let Some(iv) = b.as_i64() {
                    return format!("{}", iv);
                }
                if let Some(sv) = b.as_str() {
                    return format!("\"{}\"", sv);
                }
                return format!("{}", b);
            }
            format!("{}", default)
        }
    }
}

// ─── Section 4: Risk / Coverage Notes ────────────────────────────────────────

fn section_risk_coverage(
    out: &mut String,
    format: ExplainFormat,
    bundle: &serde_json::Value,
    verbose: bool,
) {
    heading(out, format, "RISK / COVERAGE NOTES");
    let report = match tenor_analyze::analyze(bundle) {
        Ok(r) => r,
        Err(e) => {
            emit_line(out, format, &format!("Analysis error: {}", e));
            return;
        }
    };
    if let Some(ref s1) = report.s1_state_space {
        let total_states: usize = s1.entities.values().map(|e| e.state_count).sum();
        emit_checkmark(
            out,
            format,
            &format!(
                "{} entities, {} total states",
                s1.entities.len(),
                total_states
            ),
        );
    }
    if let Some(ref s2) = report.s2_reachability {
        if s2.has_dead_states {
            let dead_count: usize = s2
                .entities
                .values()
                .map(|e| e.unreachable_states.len())
                .sum();
            emit_warning(out, format, &format!("{} dead state(s) found", dead_count));
        } else {
            emit_checkmark(
                out,
                format,
                &format!("All {} entities fully reachable", s2.entities.len()),
            );
        }
    }
    if let Some(ref s5) = report.s5_verdicts {
        emit_checkmark(
            out,
            format,
            &format!("{} verdict types defined", s5.total_verdict_types),
        );
    }
    if let Some(ref s6) = report.s6_flow_paths {
        let truncated_count = s6.flows.values().filter(|f| f.truncated).count();
        if truncated_count > 0 {
            emit_warning(
                out,
                format,
                &format!(
                    "{} flow paths ({} flow(s) truncated)",
                    s6.total_paths, truncated_count
                ),
            );
        } else {
            emit_checkmark(
                out,
                format,
                &format!(
                    "{} flow paths across {} flows",
                    s6.total_paths,
                    s6.flows.len()
                ),
            );
        }
    }
    if let Some(ref s8) = report.s8_verdict_uniqueness {
        if s8.pre_verified {
            emit_checkmark(out, format, "Verdict uniqueness pre-verified (Pass 5)");
        }
    }
    if verbose {
        out.push('\n');
        if report.findings.is_empty() {
            emit_line(out, format, "  No analysis findings.");
        } else {
            emit_line(
                out,
                format,
                &format!("  Analysis findings ({}):", report.findings.len()),
            );
            for finding in &report.findings {
                let severity = match finding.severity {
                    tenor_analyze::FindingSeverity::Warning => "WARNING",
                    tenor_analyze::FindingSeverity::Info => "INFO",
                };
                let context = finding
                    .entity_id
                    .as_ref()
                    .map(|id| format!(" [{}]", id))
                    .unwrap_or_default();
                emit_line(
                    out,
                    format,
                    &format!(
                        "    [{}/{}]{}: {}",
                        finding.analysis, severity, context, finding.message
                    ),
                );
            }
        }
    }
    out.push('\n');
}

// ─── Formatting helpers ──────────────────────────────────────────────────────

fn heading(out: &mut String, format: ExplainFormat, title: &str) {
    match format {
        ExplainFormat::Terminal => {
            out.push_str(&format!("\x1b[1m{}\x1b[0m\n", title));
            out.push_str(&"\u{2550}".repeat(title.len()));
            out.push('\n');
        }
        ExplainFormat::Markdown => {
            out.push_str(&format!("## {}\n\n", title));
        }
    }
}

fn emit_line(out: &mut String, _format: ExplainFormat, text: &str) {
    out.push_str(text);
    out.push('\n');
}

fn emit_checkmark(out: &mut String, format: ExplainFormat, text: &str) {
    match format {
        ExplainFormat::Terminal => {
            out.push_str(&format!("  \x1b[32m[ok]\x1b[0m {}\n", text));
        }
        ExplainFormat::Markdown => {
            out.push_str(&format!("- [x] {}\n", text));
        }
    }
}

fn emit_warning(out: &mut String, format: ExplainFormat, text: &str) {
    match format {
        ExplainFormat::Terminal => {
            out.push_str(&format!("  \x1b[33m[!!]\x1b[0m WARNING: {}\n", text));
        }
        ExplainFormat::Markdown => {
            out.push_str(&format!("- [ ] WARNING: {}\n", text));
        }
    }
}

fn styled_name(format: ExplainFormat, name: &str) -> String {
    match format {
        ExplainFormat::Terminal => format!("\x1b[36m{}\x1b[0m", name),
        ExplainFormat::Markdown => format!("`{}`", name),
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn humanize_id(id: &str) -> String {
    id.replace('_', " ")
}

/// Produce a JSON explanation of a contract bundle.
pub fn explain_bundle(bundle: &serde_json::Value) -> Result<serde_json::Value, String> {
    let markdown = explain(bundle, ExplainFormat::Markdown, false)?;
    let verbose_markdown = explain(bundle, ExplainFormat::Markdown, true)?;
    Ok(serde_json::json!({ "summary": markdown, "verbose": verbose_markdown }))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_bundle() {
        let bundle_json = serde_json::json!({
            "kind": "Bundle", "id": "test_contract", "tenor": "1.0",
            "constructs": [
                { "kind": "Fact", "id": "my_fact", "type": { "base": "Bool" },
                  "source": { "system": "sys", "field": "fld" },
                  "default": { "kind": "bool_literal", "value": true },
                  "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0" },
                { "kind": "Entity", "id": "my_entity", "states": ["active", "inactive"],
                  "initial": "active", "transitions": [],
                  "provenance": { "file": "test.tenor", "line": 5 }, "tenor": "1.0" },
                { "kind": "Persona", "id": "admin",
                  "provenance": { "file": "test.tenor", "line": 10 }, "tenor": "1.0" },
                { "kind": "Rule", "id": "my_rule", "stratum": 0,
                  "body": { "when": {}, "produce": {} },
                  "provenance": { "file": "test.tenor", "line": 15 }, "tenor": "1.0" },
                { "kind": "Operation", "id": "my_op", "precondition": null,
                  "effects": [{ "entity_id": "my_entity", "from": "active", "to": "inactive" }],
                  "allowed_personas": ["admin"], "error_contract": ["precondition_failed"],
                  "provenance": { "file": "test.tenor", "line": 20 }, "tenor": "1.0" },
                { "kind": "Flow", "id": "my_flow", "entry": "step_one",
                  "steps": [{ "kind": "OperationStep", "id": "step_one", "op": "my_op", "persona": "admin", "outcomes": {} }],
                  "provenance": { "file": "test.tenor", "line": 25 }, "tenor": "1.0" }
            ]
        });

        let bundle = tenor_interchange::from_interchange(&bundle_json)
            .expect("interchange deserialization failed");
        assert_eq!(bundle.id, "test_contract");
        assert_eq!(bundle.constructs.len(), 6);

        let mut fact_count = 0;
        let mut entity_count = 0;
        let mut persona_count = 0;
        let mut rule_count = 0;
        let mut op_count = 0;
        let mut flow_count = 0;

        for c in &bundle.constructs {
            match c {
                InterchangeConstruct::Fact(f) => {
                    assert_eq!(f.id, "my_fact");
                    assert!(f.source.is_some());
                    let src = f.source.as_ref().unwrap();
                    assert_eq!(src.get("system").and_then(|v| v.as_str()), Some("sys"));
                    assert_eq!(src.get("field").and_then(|v| v.as_str()), Some("fld"));
                    assert!(f.default.is_some());
                    fact_count += 1;
                }
                InterchangeConstruct::Entity(e) => {
                    assert_eq!(e.id, "my_entity");
                    assert_eq!(e.states, vec!["active", "inactive"]);
                    entity_count += 1;
                }
                InterchangeConstruct::Persona(p) => {
                    assert_eq!(p.id, "admin");
                    persona_count += 1;
                }
                InterchangeConstruct::Rule(r) => {
                    assert_eq!(r.id, "my_rule");
                    assert_eq!(r.stratum, 0);
                    rule_count += 1;
                }
                InterchangeConstruct::Operation(o) => {
                    assert_eq!(o.id, "my_op");
                    assert_eq!(o.effects.len(), 1);
                    assert_eq!(o.effects[0].entity_id, "my_entity");
                    assert_eq!(o.effects[0].from, "active");
                    assert_eq!(o.effects[0].to, "inactive");
                    assert_eq!(o.allowed_personas, vec!["admin"]);
                    op_count += 1;
                }
                InterchangeConstruct::Flow(f) => {
                    assert_eq!(f.id, "my_flow");
                    assert_eq!(f.entry, "step_one");
                    assert_eq!(f.steps.len(), 1);
                    flow_count += 1;
                }
                _ => {}
            }
        }
        assert_eq!(fact_count, 1);
        assert_eq!(entity_count, 1);
        assert_eq!(persona_count, 1);
        assert_eq!(rule_count, 1);
        assert_eq!(op_count, 1);
        assert_eq!(flow_count, 1);
    }

    #[test]
    fn explain_produces_output_for_minimal_bundle() {
        let bundle_json = serde_json::json!({
            "kind": "Bundle", "id": "test_contract", "tenor": "1.0",
            "constructs": [
                { "kind": "Fact", "id": "payment_ok", "type": { "base": "Bool" },
                  "default": { "kind": "bool_literal", "value": true },
                  "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0" },
                { "kind": "Entity", "id": "Order", "states": ["pending", "completed"],
                  "initial": "pending", "transitions": [],
                  "provenance": { "file": "test.tenor", "line": 5 }, "tenor": "1.0" },
                { "kind": "Persona", "id": "user",
                  "provenance": { "file": "test.tenor", "line": 10 }, "tenor": "1.0" }
            ]
        });
        let result = explain(&bundle_json, ExplainFormat::Markdown, false);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("## CONTRACT SUMMARY"));
        assert!(output.contains("`test_contract`"));
        assert!(output.contains("## FACT INVENTORY"));
    }

    #[test]
    fn deserialization_fails_on_wrong_type() {
        let bad_json = serde_json::json!({
            "kind": "Bundle", "id": "test",
            "constructs": [{
                "kind": "Entity", "id": "my_entity", "states": "not_an_array",
                "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0"
            }]
        });
        let result = tenor_interchange::from_interchange(&bad_json);
        assert!(
            result.is_err(),
            "should fail when states is a string instead of array"
        );
    }

    /// Helper: build a rich test bundle for Markdown format testing.
    fn make_rich_bundle() -> serde_json::Value {
        serde_json::json!({
            "kind": "Bundle", "id": "test_contract", "tenor": "1.0",
            "constructs": [
                { "kind": "Fact", "id": "amount", "type": { "base": "Int", "min": 0, "max": 10000 },
                  "source": { "system": "billing", "field": "total" },
                  "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0" },
                { "kind": "Fact", "id": "is_active",
                  "type": { "base": "Bool" },
                  "default": { "kind": "bool_literal", "value": true },
                  "provenance": { "file": "test.tenor", "line": 3 }, "tenor": "1.0" },
                { "kind": "Entity", "id": "Order", "states": ["draft", "submitted", "approved"],
                  "initial": "draft", "transitions": [
                    {"from": "draft", "to": "submitted"},
                    {"from": "submitted", "to": "approved"}
                  ],
                  "provenance": { "file": "test.tenor", "line": 10 }, "tenor": "1.0" },
                { "kind": "Persona", "id": "admin",
                  "provenance": { "file": "test.tenor", "line": 20 }, "tenor": "1.0" },
                { "kind": "Rule", "id": "check_amount", "stratum": 0,
                  "body": {
                    "when": { "left": {"fact_ref": "amount"}, "op": ">", "right": {"literal": 100, "type": {"base": "Int"}} },
                    "produce": { "verdict_type": "high_value", "payload": {"type": {"base": "Bool"}, "value": true} }
                  },
                  "provenance": { "file": "test.tenor", "line": 25 }, "tenor": "1.0" },
                { "kind": "Operation", "id": "submit_order",
                  "allowed_personas": ["admin"], "precondition": null,
                  "effects": [{ "entity_id": "Order", "from": "draft", "to": "submitted" }],
                  "error_contract": ["precondition_failed"],
                  "provenance": { "file": "test.tenor", "line": 30 }, "tenor": "1.0" },
                { "kind": "Flow", "id": "approval_flow", "entry": "step_submit",
                  "steps": [
                    { "kind": "OperationStep", "id": "step_submit", "op": "submit_order",
                      "persona": "admin", "outcomes": { "success": "Terminal" } }
                  ],
                  "provenance": { "file": "test.tenor", "line": 40 }, "tenor": "1.0" }
            ]
        })
    }

    #[test]
    fn markdown_format_uses_headings_and_backticks() {
        let bundle = make_rich_bundle();
        let result = explain(&bundle, ExplainFormat::Markdown, false);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Markdown headings (##) for all four sections
        assert!(
            output.contains("## CONTRACT SUMMARY"),
            "should have ## heading for CONTRACT SUMMARY"
        );
        assert!(
            output.contains("## DECISION FLOW NARRATIVE"),
            "should have ## heading for DECISION FLOW NARRATIVE"
        );
        assert!(
            output.contains("## FACT INVENTORY"),
            "should have ## heading for FACT INVENTORY"
        );
        assert!(
            output.contains("## RISK / COVERAGE NOTES"),
            "should have ## heading for RISK / COVERAGE NOTES"
        );

        // Backtick-quoted construct names in markdown mode
        assert!(
            output.contains("`test_contract`"),
            "contract name should be backtick-quoted in markdown"
        );
        assert!(
            output.contains("`Order`"),
            "entity name should be backtick-quoted in markdown"
        );

        // Markdown table in fact inventory
        assert!(
            output.contains("| Fact |"),
            "fact inventory should use markdown table headers"
        );
        assert!(
            output.contains("|---"),
            "fact inventory should have markdown table separator"
        );

        // Markdown checkbox in risk section (checkmark or warning)
        assert!(
            output.contains("- [x]") || output.contains("- [ ]"),
            "risk section should use markdown checkbox syntax"
        );

        // Should NOT contain ANSI escape codes
        assert!(
            !output.contains("\x1b["),
            "markdown format should not contain ANSI escape codes"
        );
    }

    #[test]
    fn markdown_vs_terminal_format_differences() {
        let bundle = make_rich_bundle();
        let md_result = explain(&bundle, ExplainFormat::Markdown, false).unwrap();
        let term_result = explain(&bundle, ExplainFormat::Terminal, false).unwrap();

        // Markdown uses ## headings, terminal uses bold ANSI codes
        assert!(md_result.contains("## CONTRACT SUMMARY"));
        assert!(!term_result.contains("## CONTRACT SUMMARY"));
        assert!(term_result.contains("\x1b[1m")); // ANSI bold
        assert!(!md_result.contains("\x1b[1m"));

        // Markdown uses backticks for names, terminal uses cyan ANSI
        assert!(md_result.contains("`test_contract`"));
        assert!(term_result.contains("\x1b[36m")); // ANSI cyan

        // Markdown uses checkboxes, terminal uses [ok]
        let md_has_checkbox = md_result.contains("- [x]") || md_result.contains("- [ ]");
        let term_has_ok = term_result.contains("[ok]") || term_result.contains("[!!]");
        assert!(md_has_checkbox, "markdown should use checkbox syntax");
        assert!(term_has_ok, "terminal should use [ok]/[!!] syntax");

        // Both should contain the same key content (contract name, facts, etc.)
        assert!(md_result.contains("amount"));
        assert!(term_result.contains("amount"));
        assert!(md_result.contains("is_active"));
        assert!(term_result.contains("is_active"));
    }
}
