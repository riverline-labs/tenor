//! CLI migrate subcommand.
//!
//! Orchestrates the full migration analysis pipeline:
//! diff -> classify -> analyze -> compatibility -> plan -> confirm.
//!
//! Actual execution requires a TenorStorage backend (database) and is
//! performed via the API server or SDK. The CLI demonstrates the
//! analysis + planning pipeline.

use std::path::Path;
use std::process;

use tenor_eval::migration::{
    analyze_migration, build_migration_plan, check_flow_compatibility_static, MigrationSeverity,
};
use tenor_eval::types::Contract;

use crate::{report_error, OutputFormat};

/// Run the migration analysis and planning pipeline.
pub fn cmd_migrate(v1_path: &Path, v2_path: &Path, yes: bool, output: OutputFormat, quiet: bool) {
    // 1. Load both bundles
    let v1_json = load_interchange_bundle(v1_path, output, quiet);
    let v2_json = load_interchange_bundle(v2_path, output, quiet);

    // 2. Analyze migration
    let analysis = match analyze_migration(&v1_json, &v2_json) {
        Ok(a) => a,
        Err(e) => {
            report_error(&format!("migration analysis error: {}", e), output, quiet);
            process::exit(1);
        }
    };

    let severity = analysis.overall_severity;

    // 3. Check flow compatibility (static, from entry points)
    let v1_contract = Contract::from_interchange(&v1_json).ok();
    let v2_contract = Contract::from_interchange(&v2_json).ok();

    let mut flow_compat_results = Vec::new();
    if let (Some(ref v1c), Some(ref v2c)) = (&v1_contract, &v2_contract) {
        for flow in &v1c.flows {
            let result = check_flow_compatibility_static(v1c, v2c, &flow.id);
            flow_compat_results.push(result);
        }
    }

    // 4. Build migration plan
    let mut plan = match build_migration_plan(&v1_json, &v2_json, analysis) {
        Ok(p) => p,
        Err(e) => {
            report_error(&format!("migration plan error: {}", e), output, quiet);
            process::exit(1);
        }
    };

    // Populate flow compatibility into the plan
    plan.flow_compatibility = flow_compat_results;

    // 5. Display or serialize
    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&plan)
                .unwrap_or_else(|e| format!("{{\"error\": \"serialization: {}\"}}", e));
            println!("{}", json);
        }
        OutputFormat::Text => {
            if !quiet {
                display_plan_text(&plan);
            }
        }
    }

    // 6. Confirmation logic
    if (severity == MigrationSeverity::Breaking || severity == MigrationSeverity::Cautious) && !yes
    {
        eprintln!();
        eprintln!("This migration has {} changes.", severity_label(severity));
        eprintln!("Type 'yes' to confirm the migration plan:");

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim() != "yes" {
            eprintln!("Migration aborted.");
            process::exit(1);
        }
    }

    // 7. Guidance (no live execution in CLI -- requires storage backend)
    if !quiet && output == OutputFormat::Text {
        eprintln!();
        eprintln!("Migration plan complete. Execution requires a storage backend.");
        eprintln!(
            "To execute, use the Tenor API server with `tenor serve` and call the migration endpoint,"
        );
        eprintln!("or use the SDK's `execute_migration` function with a storage backend.");
    }
}

/// Load a bundle from a .tenor or .json file.
fn load_interchange_bundle(path: &Path, output: OutputFormat, quiet: bool) -> serde_json::Value {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "tenor" => {
            // Elaborate .tenor -> interchange JSON
            match tenor_core::elaborate::elaborate(path) {
                Ok(bundle) => bundle,
                Err(e) => {
                    let msg = format!("elaboration error for '{}': {:?}", path.display(), e);
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            }
        }
        "json" => {
            // Read and parse JSON
            let json_str = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("error reading '{}': {}", path.display(), e);
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            };
            match serde_json::from_str(&json_str) {
                Ok(v) => v,
                Err(e) => {
                    let msg = format!("error parsing JSON in '{}': {}", path.display(), e);
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            }
        }
        _ => {
            let msg = format!(
                "unsupported file type for '{}': expected .tenor or .json",
                path.display()
            );
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    }
}

/// Display the migration plan in text format.
fn display_plan_text(plan: &tenor_eval::migration::MigrationPlan) {
    println!("Migration Analysis: {} -> {}", plan.v1_id, plan.v2_id);
    println!("Severity: {}", severity_label(plan.severity));
    println!("Recommended Policy: {:?}", plan.recommended_policy);
    println!();

    // Breaking changes
    if !plan.analysis.breaking_changes.is_empty() {
        println!(
            "Breaking Changes ({}):",
            plan.analysis.breaking_changes.len()
        );
        for bc in &plan.analysis.breaking_changes {
            println!(
                "  - {}/{} field '{}': {} ({:?})",
                bc.construct_kind, bc.construct_id, bc.field, bc.reason, bc.severity
            );
        }
        println!();
    }

    // Entity changes
    if !plan.analysis.entity_changes.is_empty() {
        println!("Entity Changes ({}):", plan.analysis.entity_changes.len());
        for ec in &plan.analysis.entity_changes {
            println!("  - {}: {:?}", ec.entity_id, ec.action);
        }
        println!();
    }

    // Flow compatibility
    if !plan.flow_compatibility.is_empty() {
        println!("Flow Compatibility ({}):", plan.flow_compatibility.len());
        for fc in &plan.flow_compatibility {
            let status = if fc.compatible {
                "compatible"
            } else {
                "INCOMPATIBLE"
            };
            let pos = fc.position.as_deref().unwrap_or("(unknown)");
            println!(
                "  - {} at {}: {} [L1={}, L2={}, L3={}]",
                fc.flow_id,
                pos,
                status,
                fc.layer_results.layer1_verdict_isolation,
                fc.layer_results.layer2_entity_state,
                fc.layer_results.layer3_structure,
            );
            for reason in &fc.reasons {
                println!("    reason: {:?}", reason);
            }
        }
        println!();
    }

    // Entity state mappings
    if !plan.entity_state_mappings.is_empty() {
        println!(
            "Entity State Mappings ({}):",
            plan.entity_state_mappings.len()
        );
        for mapping in &plan.entity_state_mappings {
            println!(
                "  - {}/{}: {} -> {}",
                mapping.entity_id, mapping.instance_id, mapping.from_state, mapping.to_state
            );
        }
        println!();
    }
}

/// Human-readable severity label.
fn severity_label(severity: MigrationSeverity) -> &'static str {
    match severity {
        MigrationSeverity::Safe => "Safe",
        MigrationSeverity::Cautious => "Cautious",
        MigrationSeverity::Breaking => "Breaking",
    }
}
