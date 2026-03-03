use std::path::Path;
use std::process;

use crate::{report_error, OutputFormat};

pub(crate) fn cmd_eval(
    bundle_path: &Path,
    facts_path: &Path,
    flow_id: Option<&str>,
    persona: Option<&str>,
    output: OutputFormat,
    quiet: bool,
) {
    // Read bundle file
    let bundle_str = match std::fs::read_to_string(bundle_path) {
        Ok(s) => s,
        Err(_) => {
            let msg = format!("error: bundle file not found: {}", bundle_path.display());
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Parse bundle JSON
    let bundle: serde_json::Value = match serde_json::from_str(&bundle_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error: invalid JSON in {}: {}", bundle_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Read facts file
    let facts_str = match std::fs::read_to_string(facts_path) {
        Ok(s) => s,
        Err(_) => {
            let msg = format!("error: facts file not found: {}", facts_path.display());
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Parse facts JSON
    let facts: serde_json::Value = match serde_json::from_str(&facts_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error: invalid JSON in {}: {}", facts_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Flow evaluation mode
    if let Some(fid) = flow_id {
        let p = match persona {
            Some(p) => p,
            None => {
                let msg = "error: --persona is required when --flow is specified";
                report_error(msg, output, quiet);
                process::exit(1);
            }
        };

        match tenor_eval::evaluate_flow(
            &bundle,
            &facts,
            fid,
            p,
            None,
            &tenor_eval::InstanceBindingMap::new(),
        ) {
            Ok(result) => {
                if !quiet {
                    match output {
                        OutputFormat::Json => {
                            let mut json_output = serde_json::Map::new();
                            json_output.insert("flow_id".to_string(), serde_json::json!(fid));
                            json_output.insert(
                                "outcome".to_string(),
                                serde_json::json!(result.flow_result.outcome),
                            );
                            json_output.insert(
                                "initiating_persona".to_string(),
                                serde_json::json!(result.flow_result.initiating_persona),
                            );
                            let entity_changes: serde_json::Value = result
                                .flow_result
                                .entity_state_changes
                                .iter()
                                .map(|e| {
                                    serde_json::json!({
                                        "entity_id": e.entity_id,
                                        "from": e.from_state,
                                        "to": e.to_state
                                    })
                                })
                                .collect();
                            json_output.insert("entity_state_changes".to_string(), entity_changes);
                            let steps: serde_json::Value = result
                                .flow_result
                                .steps_executed
                                .iter()
                                .map(|s| {
                                    serde_json::json!({
                                        "step_id": s.step_id,
                                        "result": s.result
                                    })
                                })
                                .collect();
                            json_output.insert("steps_executed".to_string(), steps);
                            json_output.insert("verdicts".to_string(), result.verdicts.to_json());
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::Value::Object(
                                    json_output
                                ))
                                .unwrap_or_else(|e| format!("serialization error: {}", e))
                            );
                        }
                        OutputFormat::Text => {
                            println!("Flow: {}", fid);
                            println!("Outcome: {}", result.flow_result.outcome);
                            if let Some(ref p) = result.flow_result.initiating_persona {
                                println!("Persona: {}", p);
                            }
                            if !result.flow_result.steps_executed.is_empty() {
                                println!(
                                    "Steps executed: {}",
                                    result.flow_result.steps_executed.len()
                                );
                                for s in &result.flow_result.steps_executed {
                                    println!("  {} -> {}", s.step_id, s.result);
                                }
                            }
                            if !result.flow_result.entity_state_changes.is_empty() {
                                println!("Entity state changes:");
                                for e in &result.flow_result.entity_state_changes {
                                    println!(
                                        "  {} : {} -> {}",
                                        e.entity_id, e.from_state, e.to_state
                                    );
                                }
                            }
                            let verdicts = &result.verdicts.0;
                            if !verdicts.is_empty() {
                                println!("{} verdict(s):", verdicts.len());
                                for v in verdicts {
                                    println!(
                                        "  [{}] {} (rule: {}, stratum: {})",
                                        v.verdict_type,
                                        format_verdict_payload(&v.payload),
                                        v.provenance.rule_id,
                                        v.provenance.stratum,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                match output {
                    OutputFormat::Json => {
                        if !quiet {
                            let err_json = serde_json::json!({
                                "error": format!("{}", e),
                            });
                            eprintln!(
                                "{}",
                                serde_json::to_string_pretty(&err_json).unwrap_or_default()
                            );
                        }
                    }
                    OutputFormat::Text => {
                        if !quiet {
                            eprintln!("flow evaluation error: {}", e);
                        }
                    }
                }
                process::exit(1);
            }
        }
        return;
    }

    // Rule-only evaluation (default)
    match tenor_eval::evaluate(&bundle, &facts) {
        Ok(result) => {
            if !quiet {
                match output {
                    OutputFormat::Json => {
                        let json_output = result.verdicts.to_json();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json_output)
                                .unwrap_or_else(|e| format!("serialization error: {}", e))
                        );
                    }
                    OutputFormat::Text => {
                        let verdicts = &result.verdicts.0;
                        if verdicts.is_empty() {
                            println!("no verdicts produced");
                        } else {
                            println!("{} verdict(s) produced:", verdicts.len());
                            for v in verdicts {
                                println!(
                                    "  [{}] {} (rule: {}, stratum: {})",
                                    v.verdict_type,
                                    format_verdict_payload(&v.payload),
                                    v.provenance.rule_id,
                                    v.provenance.stratum,
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            match output {
                OutputFormat::Json => {
                    if !quiet {
                        let err_json = serde_json::json!({
                            "error": format!("{}", e),
                            "details": {
                                "type": format!("{:?}", e).split('{').next().unwrap_or("Unknown").trim().to_string(),
                            }
                        });
                        eprintln!(
                            "{}",
                            serde_json::to_string_pretty(&err_json).unwrap_or_default()
                        );
                    }
                }
                OutputFormat::Text => {
                    if !quiet {
                        eprintln!("evaluation error: {}", e);
                    }
                }
            }
            process::exit(1);
        }
    }
}

/// Format a verdict payload for text output.
fn format_verdict_payload(v: &tenor_eval::Value) -> String {
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
