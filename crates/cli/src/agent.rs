//! `tenor agent` -- interactive REPL for Tenor contracts.
//!
//! Turns any `.tenor` file into an interactive shell session.
//! The REPL discovers the contract's facts, operations, and flows
//! automatically and lets the user set facts, evaluate, run flows,
//! list operations, and get plain-language explanations.

use std::io::{self, BufRead, Write};
use std::path::Path;

use super::explain;

/// Run the interactive agent REPL for the given `.tenor` file.
pub fn run_agent(file: &Path) {
    // Step 1: Elaborate the .tenor file to get the interchange bundle.
    let bundle = match tenor_core::elaborate::elaborate(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: failed to elaborate '{}': {:?}", file.display(), e);
            std::process::exit(1);
        }
    };

    // Step 2: Extract contract metadata from the interchange bundle.
    let contract_id = bundle
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let constructs = bundle
        .get("constructs")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let mut fact_decls: Vec<FactInfo> = Vec::new();
    let mut operations: Vec<OpInfo> = Vec::new();
    let mut flows: Vec<FlowInfo> = Vec::new();
    let mut entity_count = 0;
    let mut persona_count = 0;

    for c in &constructs {
        let kind = c.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let cid = c.get("id").and_then(|i| i.as_str()).unwrap_or("");
        match kind {
            "Fact" => {
                let type_base = c
                    .get("type")
                    .and_then(|t| t.get("base"))
                    .and_then(|b| b.as_str())
                    .unwrap_or("?");
                let has_default =
                    c.get("default").is_some() && !c.get("default").unwrap().is_null();
                fact_decls.push(FactInfo {
                    id: cid.to_string(),
                    type_base: type_base.to_string(),
                    has_default,
                });
            }
            "Operation" => {
                let personas: Vec<String> = c
                    .get("allowed_personas")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let effects: Vec<String> = c
                    .get("effects")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|eff| {
                                let eid =
                                    eff.get("entity_id").and_then(|v| v.as_str()).unwrap_or("?");
                                let from = eff.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                                let to = eff.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                                format!("{}: {} -> {}", eid, from, to)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                operations.push(OpInfo {
                    id: cid.to_string(),
                    personas,
                    effects,
                });
            }
            "Flow" => {
                let entry = c
                    .get("entry")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                let step_count = c
                    .get("steps")
                    .and_then(|s| s.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                flows.push(FlowInfo {
                    id: cid.to_string(),
                    entry,
                    step_count,
                });
            }
            "Entity" => entity_count += 1,
            "Persona" => persona_count += 1,
            _ => {}
        }
    }

    // Step 3: Print welcome banner.
    println!();
    println!("  Tenor Agent: {}", contract_id);
    println!(
        "  {} facts, {} operations, {} flows, {} entities, {} personas",
        fact_decls.len(),
        operations.len(),
        flows.len(),
        entity_count,
        persona_count,
    );
    println!();
    println!("  Commands: help, facts, set, unset, eval, flow, operations, explain, reset, quit");
    println!();

    // Step 4: Enter the REPL loop.
    let mut current_facts: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    loop {
        // Print prompt
        print!("tenor> ");
        if io::stdout().flush().is_err() {
            break;
        }

        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl-D)
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("error reading input: {}", e);
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(3, char::is_whitespace).collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "help" => {
                print_help();
            }
            "facts" => {
                print_facts(&fact_decls, &current_facts);
            }
            "set" => {
                if parts.len() < 3 {
                    eprintln!("usage: set <fact_id> <value>");
                    continue;
                }
                let fact_id = parts[1];
                let raw_value = parts[2];
                set_fact(fact_id, raw_value, &fact_decls, &mut current_facts);
            }
            "unset" => {
                if parts.len() < 2 {
                    eprintln!("usage: unset <fact_id>");
                    continue;
                }
                let fact_id = parts[1];
                if current_facts.remove(fact_id).is_some() {
                    println!("  unset {}", fact_id);
                } else {
                    eprintln!("  {} was not set", fact_id);
                }
            }
            "eval" => {
                run_eval(&bundle, &current_facts);
            }
            "flow" => {
                if parts.len() < 3 {
                    eprintln!("usage: flow <flow_id> <persona>");
                    if !flows.is_empty() {
                        eprintln!("available flows:");
                        for f in &flows {
                            eprintln!("  {} ({} steps, entry: {})", f.id, f.step_count, f.entry);
                        }
                    }
                    continue;
                }
                let flow_id = parts[1];
                let persona = parts[2];
                run_flow(&bundle, &current_facts, flow_id, persona);
            }
            "operations" | "ops" => {
                print_operations(&operations);
            }
            "explain" => {
                run_explain(&bundle);
            }
            "reset" => {
                current_facts.clear();
                println!("  all facts cleared");
            }
            "quit" | "exit" => {
                break;
            }
            _ => {
                eprintln!(
                    "unknown command: {}. Type 'help' for available commands.",
                    cmd
                );
            }
        }
    }
}

// ─── Data structures ─────────────────────────────────────────────────────────

struct FactInfo {
    id: String,
    type_base: String,
    has_default: bool,
}

struct OpInfo {
    id: String,
    personas: Vec<String>,
    effects: Vec<String>,
}

struct FlowInfo {
    id: String,
    entry: String,
    step_count: usize,
}

// ─── Command handlers ────────────────────────────────────────────────────────

fn print_help() {
    println!();
    println!("  help                    Show this help");
    println!("  facts                   List all facts with types and current values");
    println!("  set <fact_id> <value>   Set a fact value (JSON or bare value)");
    println!("  unset <fact_id>         Remove a fact value");
    println!("  eval                    Evaluate the contract with current facts");
    println!("  flow <flow_id> <persona>  Execute a flow as a persona");
    println!("  operations              List all operations with personas and effects");
    println!("  explain                 Show a plain-language explanation of the contract");
    println!("  reset                   Clear all fact values");
    println!("  quit                    Exit the REPL");
    println!();
}

fn print_facts(
    fact_decls: &[FactInfo],
    current_facts: &serde_json::Map<String, serde_json::Value>,
) {
    if fact_decls.is_empty() {
        println!("  no facts declared");
        return;
    }
    println!();
    for f in fact_decls {
        let value_str = if let Some(val) = current_facts.get(&f.id) {
            format!(" = {}", val)
        } else if f.has_default {
            " (has default)".to_string()
        } else {
            " (required, not set)".to_string()
        };
        println!("  {} : {}{}", f.id, f.type_base, value_str);
    }
    println!();
}

fn set_fact(
    fact_id: &str,
    raw_value: &str,
    fact_decls: &[FactInfo],
    current_facts: &mut serde_json::Map<String, serde_json::Value>,
) {
    // Check if fact exists in declarations
    let fact_info = fact_decls.iter().find(|f| f.id == fact_id);
    if fact_info.is_none() {
        eprintln!("  warning: '{}' is not a declared fact", fact_id);
    }

    // Try to parse as JSON first
    let value: serde_json::Value = if let Ok(v) = serde_json::from_str(raw_value) {
        v
    } else {
        // Not valid JSON -- treat as a bare value
        // Check for booleans
        match raw_value {
            "true" => serde_json::Value::Bool(true),
            "false" => serde_json::Value::Bool(false),
            _ => {
                // Try as integer
                if let Ok(n) = raw_value.parse::<i64>() {
                    serde_json::Value::Number(serde_json::Number::from(n))
                } else {
                    // Treat as a bare string (e.g., enum value)
                    serde_json::Value::String(raw_value.to_string())
                }
            }
        }
    };

    println!("  {} = {}", fact_id, value);
    current_facts.insert(fact_id.to_string(), value);
}

fn run_eval(
    bundle: &serde_json::Value,
    current_facts: &serde_json::Map<String, serde_json::Value>,
) {
    let facts_json = serde_json::Value::Object(current_facts.clone());

    match tenor_eval::evaluate(bundle, &facts_json) {
        Ok(result) => {
            let verdicts = &result.verdicts.0;
            if verdicts.is_empty() {
                println!("  no verdicts produced");
            } else {
                println!();
                println!("  {} verdict(s) produced:", verdicts.len());
                for v in verdicts {
                    println!(
                        "  [{}] {} (rule: {}, stratum: {})",
                        v.verdict_type,
                        format_payload(&v.payload),
                        v.provenance.rule_id,
                        v.provenance.stratum,
                    );
                    if !v.provenance.facts_used.is_empty() {
                        println!("    facts used: {}", v.provenance.facts_used.join(", "));
                    }
                    if !v.provenance.verdicts_used.is_empty() {
                        println!(
                            "    verdicts used: {}",
                            v.provenance.verdicts_used.join(", ")
                        );
                    }
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("  evaluation error: {}", e);
        }
    }
}

fn run_flow(
    bundle: &serde_json::Value,
    current_facts: &serde_json::Map<String, serde_json::Value>,
    flow_id: &str,
    persona: &str,
) {
    let facts_json = serde_json::Value::Object(current_facts.clone());

    match tenor_eval::evaluate_flow(bundle, &facts_json, flow_id, persona, None) {
        Ok(result) => {
            println!();
            println!("  Flow: {}", flow_id);
            println!("  Outcome: {}", result.flow_result.outcome);
            if let Some(ref p) = result.flow_result.initiating_persona {
                println!("  Persona: {}", p);
            }

            if !result.flow_result.steps_executed.is_empty() {
                println!(
                    "  Steps executed: {}",
                    result.flow_result.steps_executed.len()
                );
                for s in &result.flow_result.steps_executed {
                    println!("    {} -> {}", s.step_id, s.result);
                }
            }

            if !result.flow_result.entity_state_changes.is_empty() {
                println!("  Entity state changes:");
                for e in &result.flow_result.entity_state_changes {
                    println!("    {} : {} -> {}", e.entity_id, e.from_state, e.to_state);
                }
            }

            let verdicts = &result.verdicts.0;
            if !verdicts.is_empty() {
                println!("  {} verdict(s):", verdicts.len());
                for v in verdicts {
                    println!(
                        "    [{}] {} (rule: {}, stratum: {})",
                        v.verdict_type,
                        format_payload(&v.payload),
                        v.provenance.rule_id,
                        v.provenance.stratum,
                    );
                }
            }
            println!();
        }
        Err(e) => {
            eprintln!("  flow evaluation error: {}", e);
        }
    }
}

fn print_operations(operations: &[OpInfo]) {
    if operations.is_empty() {
        println!("  no operations declared");
        return;
    }
    println!();
    for op in operations {
        println!("  {}", op.id);
        println!("    personas: {}", op.personas.join(", "));
        for eff in &op.effects {
            println!("    effect: {}", eff);
        }
    }
    println!();
}

fn run_explain(bundle: &serde_json::Value) {
    match explain::explain(bundle, explain::ExplainFormat::Terminal, false) {
        Ok(result) => {
            println!();
            print!("{}", result);
        }
        Err(e) => {
            eprintln!("  explain error: {}", e);
        }
    }
}

/// Format a verdict payload for display.
fn format_payload(v: &tenor_eval::Value) -> String {
    match v {
        tenor_eval::Value::Bool(b) => format!("{}", b),
        tenor_eval::Value::Int(i) => format!("{}", i),
        tenor_eval::Value::Decimal(d) => format!("{}", d),
        tenor_eval::Value::Text(t) => format!("\"{}\"", t),
        tenor_eval::Value::Money { amount, currency } => format!("{} {}", amount, currency),
        tenor_eval::Value::Enum(e) => e.clone(),
        _ => format!("{:?}", v),
    }
}
