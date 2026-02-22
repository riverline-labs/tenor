//! `tenor explain` — human-readable contract summary.
//!
//! Produces a 4-section contract summary:
//! 1. Contract Summary — what the contract contains
//! 2. Decision Flow Narrative — step-by-step process description
//! 3. Fact Inventory — all facts with types and sources
//! 4. Risk / Coverage Notes — analysis findings from S1-S8

use std::collections::BTreeMap;

/// Output format for the explain command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplainFormat {
    Terminal,
    Markdown,
}

/// Produce a human-readable contract summary.
///
/// `bundle` is the interchange JSON value (kind: "Bundle").
/// Returns the formatted string (styled terminal text or markdown).
pub fn explain(bundle: &serde_json::Value, format: ExplainFormat, verbose: bool) -> String {
    let mut out = String::new();

    let contract_id = bundle
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let constructs = bundle
        .get("constructs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Classify constructs by kind
    let mut facts: Vec<&serde_json::Value> = Vec::new();
    let mut entities: Vec<&serde_json::Value> = Vec::new();
    let mut personas: Vec<&serde_json::Value> = Vec::new();
    let mut rules: Vec<&serde_json::Value> = Vec::new();
    let mut operations: Vec<&serde_json::Value> = Vec::new();
    let mut flows: Vec<&serde_json::Value> = Vec::new();

    for c in &constructs {
        match c.get("kind").and_then(|v| v.as_str()).unwrap_or("") {
            "Fact" => facts.push(c),
            "Entity" => entities.push(c),
            "Persona" => personas.push(c),
            "Rule" => rules.push(c),
            "Operation" => operations.push(c),
            "Flow" => flows.push(c),
            _ => {}
        }
    }

    // Build an operation lookup by id for flow narrative
    let op_map: BTreeMap<&str, &serde_json::Value> = operations
        .iter()
        .filter_map(|op| op.get("id").and_then(|v| v.as_str()).map(|id| (id, *op)))
        .collect();

    // Section 1: Contract Summary
    section_contract_summary(
        &mut out,
        format,
        contract_id,
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
    section_risk_coverage(&mut out, format, bundle, verbose);

    out
}

// ─── Section 1: Contract Summary ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn section_contract_summary(
    out: &mut String,
    format: ExplainFormat,
    contract_id: &str,
    facts: &[&serde_json::Value],
    entities: &[&serde_json::Value],
    personas: &[&serde_json::Value],
    rules: &[&serde_json::Value],
    operations: &[&serde_json::Value],
    flows: &[&serde_json::Value],
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
            .map(|e| {
                let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let state_count = e
                    .get("states")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                format!("{} ({} states)", styled_name(format, id), state_count)
            })
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
            let names: Vec<&str> = personas
                .iter()
                .filter_map(|p| p.get("id").and_then(|v| v.as_str()))
                .collect();
            emit_line(
                out,
                format,
                &format!("  Persona list: {}", names.join(", ")),
            );
        }

        // Verbose: list entity states
        for e in entities {
            let id = e.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            if let Some(states) = e.get("states").and_then(|v| v.as_array()) {
                let state_names: Vec<&str> = states.iter().filter_map(|s| s.as_str()).collect();
                emit_line(
                    out,
                    format,
                    &format!("  {} states: {}", id, state_names.join(", ")),
                );
            }
        }

        // Verbose: rule strata breakdown
        let mut strata_counts: BTreeMap<i64, usize> = BTreeMap::new();
        for r in rules {
            let s = r.get("stratum").and_then(|v| v.as_i64()).unwrap_or(0);
            *strata_counts.entry(s).or_insert(0) += 1;
        }
        for (s, count) in &strata_counts {
            emit_line(out, format, &format!("  Stratum {}: {} rule(s)", s, count));
        }
    }

    out.push('\n');
}

fn count_strata(rules: &[&serde_json::Value]) -> usize {
    let mut strata = std::collections::BTreeSet::new();
    for r in rules {
        if let Some(s) = r.get("stratum").and_then(|v| v.as_i64()) {
            strata.insert(s);
        }
    }
    strata.len()
}

// ─── Section 2: Decision Flow Narrative ──────────────────────────────────────

fn section_flow_narrative(
    out: &mut String,
    format: ExplainFormat,
    flows: &[&serde_json::Value],
    op_map: &BTreeMap<&str, &serde_json::Value>,
    verbose: bool,
) {
    heading(out, format, "DECISION FLOW NARRATIVE");

    if flows.is_empty() {
        emit_line(out, format, "No flows defined in this contract.");
        out.push('\n');
        return;
    }

    for flow in flows {
        let flow_id = flow.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let entry = flow.get("entry").and_then(|v| v.as_str()).unwrap_or("?");

        emit_line(
            out,
            format,
            &format!("Flow: {}", styled_name(format, flow_id)),
        );
        emit_line(out, format, &format!("  Entry point: {}", entry));
        out.push('\n');

        // Build step index for ordered walk
        let steps = flow
            .get("steps")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let step_map: BTreeMap<&str, &serde_json::Value> = steps
            .iter()
            .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(|id| (id, s)))
            .collect();

        // Walk from entry
        let mut visited = std::collections::HashSet::new();
        walk_steps(
            out,
            format,
            entry,
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
    op_map: &BTreeMap<&str, &serde_json::Value>,
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
            // Follow outcomes
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
            // Follow both branches
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
                    // Create a new visited set for the else branch so both paths can walk independently
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
                    // Merge else_visited back
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
            // on_success may route to another step
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
            // Follow join on_all_success
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

/// Resolve a step target (can be a string step_id or a Terminal/Terminate object).
fn resolve_step_target(target: &serde_json::Value) -> Option<String> {
    if let Some(s) = target.as_str() {
        return Some(s.to_string());
    }
    // Terminal or Terminate objects end the flow -- no next step
    None
}

fn describe_operation_step(
    out: &mut String,
    format: ExplainFormat,
    step: &serde_json::Value,
    op_map: &BTreeMap<&str, &serde_json::Value>,
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
        // Show precondition
        if let Some(op) = op_map.get(op_id) {
            if let Some(precondition) = op.get("precondition") {
                if !precondition.is_null() {
                    let pre_str = describe_condition(precondition);
                    emit_line(
                        out,
                        format,
                        &format!("{}  Precondition: {}", indent(depth), pre_str),
                    );
                }
            }
            // Show effects
            if let Some(effects) = op.get("effects").and_then(|v| v.as_array()) {
                for eff in effects {
                    let entity = eff.get("entity_id").and_then(|v| v.as_str()).unwrap_or("?");
                    let from = eff.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                    let to = eff.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                    emit_line(
                        out,
                        format,
                        &format!(
                            "{}  Effect: {} transitions {} -> {}",
                            indent(depth),
                            entity,
                            from,
                            to,
                        ),
                    );
                }
            }
        }
    }

    // Note on_failure if present
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

    // Note outcomes leading to Terminal
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

    // Describe branches
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

    // Note on_success and on_failure
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
    _op_map: &BTreeMap<&str, &serde_json::Value>,
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
    // verdict_present
    if let Some(vp) = cond.get("verdict_present").and_then(|v| v.as_str()) {
        return format!("verdict '{}' is present", vp);
    }
    // Operators with left/right (or operand for unary)
    if let Some(op) = cond.get("op").and_then(|v| v.as_str()) {
        let left = cond.get("left");
        let right = cond.get("right");

        match op {
            "not" => {
                // Unary not: uses "operand" field
                let operand = cond.get("operand");
                let operand_str = operand
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                return format!("not ({})", operand_str);
            }
            "and" | "or" => {
                // Logical operators: recurse into both sides as conditions
                let left_str = left
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                let right_str = right
                    .map(describe_condition)
                    .unwrap_or_else(|| "?".to_string());
                return format!("({} {} {})", left_str, op, right_str);
            }
            _ => {
                // Comparison operators: describe as expressions
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
    facts: &[&serde_json::Value],
    verbose: bool,
) {
    heading(out, format, "FACT INVENTORY");

    if facts.is_empty() {
        emit_line(out, format, "No facts defined in this contract.");
        out.push('\n');
        return;
    }

    // Group facts by type category
    let mut grouped: BTreeMap<&str, Vec<&serde_json::Value>> = BTreeMap::new();
    for fact in facts {
        let category = categorize_fact_type(fact);
        grouped.entry(category).or_default().push(fact);
    }

    match format {
        ExplainFormat::Markdown => {
            out.push_str("| Fact | Type | Source | Default |\n");
            out.push_str("|------|------|--------|---------|\n");
            for group_facts in grouped.values() {
                for fact in group_facts {
                    let id = fact.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let type_str = describe_fact_type(fact, verbose);
                    let source_str = describe_source(fact);
                    let default_str = describe_default(fact);
                    out.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        id, type_str, source_str, default_str
                    ));
                }
            }
        }
        ExplainFormat::Terminal => {
            // Calculate column widths
            let mut max_id = 4;
            let mut max_type = 4;
            let mut max_source = 6;
            for fact in facts {
                let id = fact.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let type_str = describe_fact_type(fact, verbose);
                let source_str = describe_source(fact);
                max_id = max_id.max(id.len());
                max_type = max_type.max(type_str.len());
                max_source = max_source.max(source_str.len());
            }

            // Header
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
                // Category sub-header
                out.push_str(&format!("  [{}]\n", category));
                for fact in group_facts {
                    let id = fact.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let type_str = describe_fact_type(fact, verbose);
                    let source_str = describe_source(fact);
                    let default_str = describe_default(fact);
                    out.push_str(&format!(
                        "  {:<id_w$}  {:<type_w$}  {:<src_w$}  {}\n",
                        id,
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

fn categorize_fact_type(fact: &serde_json::Value) -> &'static str {
    let base = fact
        .get("type")
        .and_then(|t| t.get("base"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match base {
        "Int" | "Decimal" => "numeric",
        "Bool" => "boolean",
        "Enum" => "enum",
        "Money" => "numeric",
        "Text" | "Date" | "Duration" => "text/temporal",
        "Record" => "record",
        "List" => "list",
        _ => {
            // Named type (Record-like)
            if fact.get("type").and_then(|t| t.get("name")).is_some() {
                "record"
            } else {
                "other"
            }
        }
    }
}

fn describe_fact_type(fact: &serde_json::Value, verbose: bool) -> String {
    let type_val = match fact.get("type") {
        Some(t) => t,
        None => return "?".to_string(),
    };

    // Named type
    if let Some(name) = type_val.get("name").and_then(|v| v.as_str()) {
        return name.to_string();
    }

    let base = type_val.get("base").and_then(|v| v.as_str()).unwrap_or("?");

    if verbose {
        // Include constraints
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
            "Money" => {
                let cur = type_val.get("currency").and_then(|v| v.as_str());
                match cur {
                    Some(c) => format!("Money({})", c),
                    None => "Money".to_string(),
                }
            }
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
        // Simplified type names
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

fn describe_source(fact: &serde_json::Value) -> String {
    match fact.get("source") {
        None => "-".to_string(),
        Some(source) => {
            let system = source.get("system").and_then(|v| v.as_str()).unwrap_or("?");
            let field = source.get("field").and_then(|v| v.as_str()).unwrap_or("?");
            format!("{}.{}", system, field)
        }
    }
}

fn describe_default(fact: &serde_json::Value) -> String {
    match fact.get("default") {
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

    // Run analysis
    let report = match tenor_analyze::analyze(bundle) {
        Ok(r) => r,
        Err(e) => {
            emit_line(out, format, &format!("Analysis error: {}", e));
            return;
        }
    };

    // S1: State space
    if let Some(ref s1) = report.s1_state_space {
        let total_states: usize = s1.entities.values().map(|e| e.state_count).sum();
        emit_checkmark(
            out,
            format,
            &format!(
                "{} entities, {} total states",
                s1.entities.len(),
                total_states,
            ),
        );
    }

    // S2: Reachability
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
                &format!("All {} entities fully reachable", s2.entities.len(),),
            );
        }
    }

    // S5: Verdicts
    if let Some(ref s5) = report.s5_verdicts {
        emit_checkmark(
            out,
            format,
            &format!("{} verdict types defined", s5.total_verdict_types),
        );
    }

    // S6: Flow paths
    if let Some(ref s6) = report.s6_flow_paths {
        let truncated_count = s6.flows.values().filter(|f| f.truncated).count();
        if truncated_count > 0 {
            emit_warning(
                out,
                format,
                &format!(
                    "{} flow paths ({} flow(s) truncated)",
                    s6.total_paths, truncated_count,
                ),
            );
        } else {
            emit_checkmark(
                out,
                format,
                &format!(
                    "{} flow paths across {} flows",
                    s6.total_paths,
                    s6.flows.len(),
                ),
            );
        }
    }

    // S8: Verdict uniqueness
    if let Some(ref s8) = report.s8_verdict_uniqueness {
        if s8.pre_verified {
            emit_checkmark(out, format, "Verdict uniqueness pre-verified (Pass 5)");
        }
    }

    // Verbose: list all findings with severity
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
                        finding.analysis, severity, context, finding.message,
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
