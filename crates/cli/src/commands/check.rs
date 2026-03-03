use std::path::Path;
use std::process;

use crate::{report_error, OutputFormat};

pub(crate) fn cmd_check(file: &Path, analysis: Option<&str>, output: OutputFormat, quiet: bool) {
    // Step 1: Elaborate the .tenor file
    let bundle = match tenor_core::elaborate::elaborate(file) {
        Ok(b) => b,
        Err(e) => {
            match output {
                OutputFormat::Json => {
                    let err_json = serde_json::to_string_pretty(&e.to_json_value())
                        .unwrap_or_else(|_| format!("{{\"error\": \"{:?}\"}}", e));
                    eprintln!("{}", err_json);
                }
                OutputFormat::Text => {
                    if !quiet {
                        eprintln!("elaboration error: {:?}", e);
                    }
                }
            }
            process::exit(1);
        }
    };

    // Step 2: Parse analysis selection
    let valid_analyses = ["s1", "s2", "s3a", "s4", "s5", "s6", "s7", "s8"];
    let selected: Option<Vec<&str>> = analysis.map(|a| {
        let selected: Vec<&str> = a.split(',').map(|s| s.trim()).collect();
        for s in &selected {
            if !valid_analyses.contains(s) {
                let msg = format!(
                    "invalid analysis '{}'. Valid: {}",
                    s,
                    valid_analyses.join(", ")
                );
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        }
        selected
    });

    // Step 3: Run analysis
    let report = match &selected {
        None => tenor_analyze::analyze(&bundle),
        Some(analyses) => tenor_analyze::analyze_selected(&bundle, analyses),
    };

    let report = match report {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("analysis error: {}", e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Step 4: Format output
    if !quiet {
        match output {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&report)
                    .unwrap_or_else(|e| format!("{{\"error\": \"serialization: {}\"}}", e));
                println!("{}", json);
            }
            OutputFormat::Text => {
                println!("Static Analysis Report");
                println!("======================");
                println!();

                if let Some(ref s1) = report.s1_state_space {
                    let total_states: usize = s1.entities.values().map(|e| e.state_count).sum();
                    println!(
                        "  Entities: {} entities, {} total states",
                        s1.entities.len(),
                        total_states
                    );
                }

                if let Some(ref s2) = report.s2_reachability {
                    if s2.has_dead_states {
                        let dead_count: usize = s2
                            .entities
                            .values()
                            .map(|e| e.unreachable_states.len())
                            .sum();
                        println!(
                            "  Reachability: WARNING: {} dead state(s) found",
                            dead_count
                        );
                    } else {
                        println!(
                            "  Reachability: {} entities fully reachable",
                            s2.entities.len()
                        );
                    }
                }

                if let Some(ref s3a) = report.s3a_admissibility {
                    let admissible_count: usize = s3a
                        .admissible_operations
                        .values()
                        .map(|ops| ops.len())
                        .sum();
                    println!(
                        "  Admissibility: {} combinations checked, {} admissible operations",
                        s3a.total_combinations_checked, admissible_count
                    );
                }

                if let Some(ref s4) = report.s4_authority {
                    println!(
                        "  Authority: {} personas, {} authority entries",
                        s4.total_personas, s4.total_authority_entries
                    );
                    if !s4.cross_contract_authorities.is_empty() {
                        // Count unique shared personas
                        let unique_personas: std::collections::BTreeSet<&str> = s4
                            .cross_contract_authorities
                            .iter()
                            .map(|cca| cca.persona_id.as_str())
                            .collect();
                        println!(
                            "  Cross-Contract Authority (S4): {} shared personas, {} cross-contract authority entries",
                            unique_personas.len(),
                            s4.cross_contract_authorities.len()
                        );
                    }
                }

                if let Some(ref s5) = report.s5_verdicts {
                    println!(
                        "  Verdicts: {} verdict types, {} operations with outcomes",
                        s5.total_verdict_types, s5.total_operations_with_outcomes
                    );
                }

                if let Some(ref s6) = report.s6_flow_paths {
                    let truncated_count = s6.flows.values().filter(|f| f.truncated).count();
                    let flow_msg = if truncated_count > 0 {
                        format!(" ({} flow(s) truncated)", truncated_count)
                    } else {
                        String::new()
                    };
                    println!(
                        "  Flow Paths: {} total paths across {} flows{}",
                        s6.total_paths,
                        s6.flows.len(),
                        flow_msg
                    );
                    if !s6.cross_contract_paths.is_empty() {
                        // Count unique trigger targets
                        let unique_triggers: std::collections::BTreeSet<String> = s6
                            .cross_contract_paths
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}.{}->{}.{}",
                                    p.source_contract,
                                    p.source_flow,
                                    p.target_contract,
                                    p.target_flow
                                )
                            })
                            .collect();
                        println!(
                            "  Cross-Contract Flow Paths (S6): {} cross-contract triggers, {} cross-contract paths",
                            unique_triggers.len(),
                            s6.cross_contract_paths.len()
                        );
                    }
                }

                if let Some(ref s7) = report.s7_complexity {
                    println!(
                        "  Complexity: max predicate depth {}, max flow depth {}",
                        s7.max_predicate_depth, s7.max_flow_depth
                    );
                }

                if let Some(ref s8) = report.s8_verdict_uniqueness {
                    if s8.pre_verified {
                        println!("  Verdict Uniqueness: pre-verified (Pass 5)");
                    }
                }

                println!();
                println!("Findings:");

                let has_findings = !report.findings.is_empty();
                if has_findings {
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
                        println!(
                            "  [{}/{}]{}: {}",
                            finding.analysis, severity, context, finding.message
                        );
                    }
                } else {
                    println!("  No findings.");
                }
            }
        }
    }

    // Step 5: Exit code based on findings
    let has_warnings = report
        .findings
        .iter()
        .any(|f| f.severity == tenor_analyze::FindingSeverity::Warning);

    if has_warnings {
        process::exit(1);
    }
}
