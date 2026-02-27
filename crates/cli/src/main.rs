mod agent;
mod ambiguity;
mod connect;
mod diff;
mod explain;
mod manifest;
mod migrate;
mod runner;
mod serve;
mod tap;

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand, ValueEnum};

/// Output format for CLI responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

/// Output format for the explain subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ExplainOutputFormat {
    Terminal,
    Markdown,
}

/// Tenor contract language toolchain.
#[derive(Parser)]
#[command(name = "tenor", version, about = "Tenor contract language toolchain")]
struct Cli {
    /// Output format (text or json)
    #[arg(long, global = true, default_value = "text", value_enum)]
    output: OutputFormat,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Elaborate a .tenor file to interchange JSON
    Elaborate {
        /// Path to the .tenor source file
        file: PathBuf,
        /// Produce a TenorManifest instead of a bare interchange bundle
        #[arg(long)]
        manifest: bool,
    },

    /// Validate interchange JSON against the formal JSON Schema
    Validate {
        /// Path to the interchange JSON bundle file
        bundle: PathBuf,
    },

    /// Evaluate a contract bundle against a set of facts
    Eval {
        /// Path to the interchange JSON bundle file
        bundle: PathBuf,
        /// Path to the facts JSON file
        #[arg(long)]
        facts: PathBuf,
        /// Flow ID to execute (enables flow evaluation mode)
        #[arg(long)]
        flow: Option<String>,
        /// Persona executing the flow (required when --flow is specified)
        #[arg(long)]
        persona: Option<String>,
    },

    /// Run the conformance test suite
    Test {
        /// Path to the conformance suite directory
        #[arg(default_value = "conformance")]
        suite_dir: PathBuf,
    },

    /// Diff two interchange JSON bundles for structural changes
    Diff {
        /// Path to the first interchange JSON bundle
        t1: PathBuf,
        /// Path to the second interchange JSON bundle
        t2: PathBuf,
        /// Classify changes as breaking/non-breaking using Section 17.2 taxonomy
        #[arg(long)]
        breaking: bool,
    },

    /// Analyze migration between two contract versions
    Migrate {
        /// Path to the v1 contract (.tenor or .json)
        v1: PathBuf,
        /// Path to the v2 contract (.tenor or .json)
        v2: PathBuf,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Run static analysis checks on a .tenor file
    Check {
        /// Path to the .tenor source file
        file: PathBuf,
        /// Comma-separated list of analyses to run (s1,s2,s3a,s4,s5,s6,s7,s8). Default: all.
        #[arg(long)]
        analysis: Option<String>,
    },

    /// Explain a contract bundle in natural language
    Explain {
        /// Path to .tenor source file or interchange JSON bundle
        file: PathBuf,
        /// Output format (terminal or markdown)
        #[arg(long, default_value = "terminal")]
        format: ExplainOutputFormat,
        /// Show technical details (entity states, rule strata, analysis findings)
        #[arg(long)]
        verbose: bool,
    },

    /// Generate code from a contract bundle
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },

    /// Run AI ambiguity testing against the conformance suite
    Ambiguity {
        /// Path to the conformance suite directory
        suite_dir: PathBuf,
        /// Path to the Tenor specification file
        #[arg(long)]
        spec: Option<PathBuf>,
        /// LLM model name to use
        #[arg(long)]
        model: Option<String>,
    },

    /// Start the Tenor HTTP API server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Path to TLS certificate PEM file (requires --tls-key)
        #[arg(long)]
        tls_cert: Option<PathBuf>,
        /// Path to TLS private key PEM file (requires --tls-cert)
        #[arg(long)]
        tls_key: Option<PathBuf>,
        /// .tenor contract files to pre-load
        #[arg()]
        contracts: Vec<PathBuf>,
    },

    /// Start an interactive agent shell for a contract
    Agent {
        /// Path to the .tenor source file
        file: PathBuf,
    },

    /// Introspect sources and generate adapter scaffolding
    Connect {
        /// Path to the .tenor source file or interchange JSON
        contract: PathBuf,
        /// Path to external schema document (OpenAPI, GraphQL SDL, SQL DDL — auto-detected)
        #[arg(long)]
        environment: Option<PathBuf>,
        /// Output directory for generated adapter scaffolding
        #[arg(long, default_value = "./tenor-connect-output")]
        out: PathBuf,
        /// Show proposed mappings without generating files
        #[arg(long)]
        dry_run: bool,
        /// Output review file instead of interactive prompts
        #[arg(long)]
        batch: bool,
        /// Generate adapters from a reviewed mapping file
        #[arg(long)]
        apply: Option<PathBuf>,
        /// LLM model to use (default: claude-sonnet-4-20250514)
        #[arg(long)]
        model: Option<String>,
        /// Force heuristic matching (skip LLM even if API key is set)
        #[arg(long)]
        heuristic: bool,
        /// Show detailed matching reasoning
        #[arg(long)]
        verbose: bool,
    },

    /// Start the Language Server Protocol server over stdio
    Lsp,
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generate TypeScript types and client bindings
    Typescript {
        /// Path to .tenor source file or interchange JSON bundle
        input: PathBuf,
        /// Output directory for generated files
        #[arg(long, default_value = "./generated")]
        out: PathBuf,
        /// SDK import path (default: @tenor/sdk)
        #[arg(long, default_value = "@tenor/sdk")]
        sdk_import: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Elaborate { file, manifest } => {
            cmd_elaborate(&file, manifest, cli.output, cli.quiet);
        }
        Commands::Validate { bundle } => {
            cmd_validate(&bundle, cli.output, cli.quiet);
        }
        Commands::Eval {
            bundle,
            facts,
            flow,
            persona,
        } => {
            cmd_eval(
                &bundle,
                &facts,
                flow.as_deref(),
                persona.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Test { suite_dir } => {
            cmd_test(&suite_dir, cli.quiet);
        }
        Commands::Diff { t1, t2, breaking } => {
            cmd_diff(&t1, &t2, breaking, cli.output, cli.quiet);
        }
        Commands::Migrate { v1, v2, yes } => {
            migrate::cmd_migrate(&v1, &v2, yes, cli.output, cli.quiet);
        }
        Commands::Check { file, analysis } => {
            cmd_check(&file, analysis.as_deref(), cli.output, cli.quiet);
        }
        Commands::Explain {
            file,
            format,
            verbose,
        } => {
            cmd_explain(&file, format, verbose, cli.output, cli.quiet);
        }
        Commands::Generate { command } => {
            cmd_generate(command, cli.output, cli.quiet);
        }
        Commands::Ambiguity {
            suite_dir,
            spec,
            model,
        } => {
            cmd_ambiguity(&suite_dir, spec.as_deref(), model.as_deref());
        }
        Commands::Serve {
            port,
            contracts,
            tls_cert,
            tls_key,
        } => {
            // Validate TLS flags: both must be provided or neither
            if tls_cert.is_some() != tls_key.is_some() {
                eprintln!("error: --tls-cert and --tls-key must both be provided");
                process::exit(1);
            }
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            if let Err(e) = rt.block_on(serve::start_server(port, contracts, tls_cert, tls_key)) {
                eprintln!("Server error: {}", e);
                process::exit(1);
            }
        }
        Commands::Agent { file } => {
            agent::run_agent(&file);
        }
        Commands::Connect {
            contract,
            environment,
            out,
            dry_run,
            batch,
            apply,
            model,
            heuristic,
            verbose,
        } => {
            connect::cmd_connect(connect::ConnectOptions {
                contract: &contract,
                environment: environment.as_deref(),
                output_dir: &out,
                dry_run,
                batch,
                apply: apply.as_deref(),
                model: model.as_deref(),
                heuristic,
                verbose,
                output: cli.output,
                quiet: cli.quiet,
            });
        }
        Commands::Lsp => {
            if let Err(e) = tenor_lsp::run() {
                eprintln!("LSP server error: {}", e);
                process::exit(1);
            }
        }
    }
}

fn cmd_elaborate(file: &Path, manifest: bool, output: OutputFormat, quiet: bool) {
    match tenor_core::elaborate::elaborate(file) {
        Ok(bundle) => {
            let output_value = if manifest {
                manifest::build_manifest(bundle)
            } else {
                bundle
            };
            let pretty = serde_json::to_string_pretty(&output_value)
                .unwrap_or_else(|e| format!("serialization error: {}", e));
            println!("{}", pretty);
        }
        Err(e) => {
            match output {
                OutputFormat::Json => {
                    let err_json = serde_json::to_string_pretty(&e.to_json_value())
                        .unwrap_or_else(|_| format!("{{\"error\": \"{:?}\"}}", e));
                    eprintln!("{}", err_json);
                }
                OutputFormat::Text => {
                    if !quiet {
                        let err_json = serde_json::to_string_pretty(&e.to_json_value())
                            .unwrap_or_else(|_| format!("{:?}", e));
                        eprintln!("{}", err_json);
                    }
                }
            }
            process::exit(1);
        }
    }
}

static INTERCHANGE_SCHEMA_STR: &str = include_str!("../../../docs/interchange-schema.json");
static MANIFEST_SCHEMA_STR: &str = include_str!("../../../docs/manifest-schema.json");

fn cmd_validate(bundle_path: &Path, output: OutputFormat, quiet: bool) {
    // Parse the interchange schema
    let interchange_schema: serde_json::Value = match serde_json::from_str(INTERCHANGE_SCHEMA_STR) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!(
                "internal error: failed to parse embedded interchange schema: {}",
                e
            );
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Read and parse the document file
    let doc_str = match std::fs::read_to_string(bundle_path) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error reading file '{}': {}", bundle_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    let doc: serde_json::Value = match serde_json::from_str(&doc_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error parsing JSON in '{}': {}", bundle_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Auto-detect manifest documents via etag field presence
    let is_manifest = doc.get("etag").is_some();

    let (validator, doc_type) = if is_manifest {
        // Parse manifest schema
        let manifest_schema: serde_json::Value = match serde_json::from_str(MANIFEST_SCHEMA_STR) {
            Ok(s) => s,
            Err(e) => {
                let msg = format!(
                    "internal error: failed to parse embedded manifest schema: {}",
                    e
                );
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        };

        // Build validator with interchange schema registered for $ref resolution
        // The manifest schema $id is https://tenor-lang.org/schemas/manifest/v1.1.0
        // so the relative $ref "interchange-schema.json" resolves to
        // https://tenor-lang.org/schemas/manifest/interchange-schema.json
        let interchange_resource = jsonschema::Resource::from_contents(interchange_schema);
        let v = match jsonschema::options()
            .with_resource(
                "https://tenor-lang.org/schemas/manifest/interchange-schema.json",
                interchange_resource,
            )
            .build(&manifest_schema)
        {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("internal error: failed to compile manifest schema: {}", e);
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        };
        (v, "manifest")
    } else {
        // Validate against interchange schema as before
        let v = match jsonschema::validator_for(&interchange_schema) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("internal error: failed to compile schema: {}", e);
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        };
        (v, "bundle")
    };

    let errors: Vec<String> = validator
        .iter_errors(&doc)
        .map(|e| format!("{}", e))
        .collect();

    if errors.is_empty() {
        if !quiet {
            match output {
                OutputFormat::Text => {
                    if is_manifest {
                        println!("valid manifest");
                    } else {
                        println!("valid");
                    }
                }
                OutputFormat::Json => {
                    if is_manifest {
                        println!("{{\"valid\": true, \"type\": \"manifest\"}}");
                    } else {
                        println!("{{\"valid\": true}}");
                    }
                }
            }
        }
    } else {
        match output {
            OutputFormat::Text => {
                if !quiet {
                    eprintln!("invalid {}", doc_type);
                    for err in &errors {
                        eprintln!("  - {}", err);
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "valid": false,
                    "type": doc_type,
                    "errors": errors
                });
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            }
        }
        process::exit(1);
    }
}

fn cmd_eval(
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

fn cmd_diff(t1_path: &Path, t2_path: &Path, breaking: bool, output: OutputFormat, quiet: bool) {
    // Read and parse the first bundle
    let t1_str = match std::fs::read_to_string(t1_path) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error reading '{}': {}", t1_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };
    let t1: serde_json::Value = match serde_json::from_str(&t1_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error parsing JSON in '{}': {}", t1_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Read and parse the second bundle
    let t2_str = match std::fs::read_to_string(t2_path) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error reading '{}': {}", t2_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };
    let t2: serde_json::Value = match serde_json::from_str(&t2_str) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error parsing JSON in '{}': {}", t2_path.display(), e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Compute the diff
    let bundle_diff = match diff::diff_bundles(&t1, &t2) {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("diff error: {}", e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    };

    // Handle --breaking mode
    if breaking {
        if bundle_diff.is_empty() {
            if !quiet {
                match output {
                    OutputFormat::Json => {
                        println!("{{\"summary\": {{\"total_changes\": 0, \"breaking_count\": 0}}, \"added\": [], \"removed\": [], \"changed\": []}}");
                    }
                    OutputFormat::Text => {
                        println!("0 change(s): 0 breaking, 0 non-breaking, 0 requires analysis");
                    }
                }
            }
            return;
        }

        let classified = diff::classify_diff(&bundle_diff);
        if !quiet {
            match output {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&classified.to_json()).unwrap_or_default()
                    );
                }
                OutputFormat::Text => {
                    println!("{}", classified.to_text());
                }
            }
        }
        if classified.has_breaking() || classified.summary.requires_analysis_count > 0 {
            process::exit(1);
        }
        return;
    }

    // Standard diff output (no --breaking)
    if bundle_diff.is_empty() {
        if !quiet {
            match output {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&bundle_diff.to_json()).unwrap_or_default()
                    );
                }
                OutputFormat::Text => {
                    println!("no differences");
                }
            }
        }
    } else {
        if !quiet {
            match output {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&bundle_diff.to_json()).unwrap_or_default()
                    );
                }
                OutputFormat::Text => {
                    println!("{}", bundle_diff.to_text());
                }
            }
        }
        process::exit(1);
    }
}

fn cmd_test(suite_dir: &Path, quiet: bool) {
    if !suite_dir.exists() {
        eprintln!(
            "error: conformance suite directory not found: {}",
            suite_dir.display()
        );
        process::exit(1);
    }

    let _ = quiet; // TAP output is the primary output; quiet has no effect on test runner
    let result = runner::run_suite(suite_dir);
    if result.failed > 0 {
        process::exit(1);
    }
}

fn cmd_ambiguity(suite_dir: &Path, spec: Option<&Path>, model: Option<&str>) {
    let spec_path = spec.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        suite_dir
            .parent()
            .unwrap_or(std::path::Path::new(".."))
            .join("docs/TENOR.md")
    });

    if !suite_dir.exists() {
        eprintln!(
            "error: conformance suite directory not found: {}",
            suite_dir.display()
        );
        process::exit(1);
    }

    let result = ambiguity::run_ambiguity_suite(suite_dir, &spec_path, model);
    eprintln!(
        "\nAmbiguity test summary: {} total, {} matches, {} mismatches, {} hard errors",
        result.total, result.matches, result.mismatches, result.hard_errors
    );
    if result.hard_errors > 0 {
        process::exit(1);
    }
}

fn cmd_check(file: &Path, analysis: Option<&str>, output: OutputFormat, quiet: bool) {
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

fn cmd_explain(
    file: &Path,
    format: ExplainOutputFormat,
    verbose: bool,
    output: OutputFormat,
    quiet: bool,
) {
    // Determine input type by extension
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

    let bundle: serde_json::Value = if ext == "json" {
        // Parse as interchange JSON
        let json_str = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error reading '{}': {}", file.display(), e);
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        };
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("error parsing JSON in '{}': {}", file.display(), e);
                report_error(&msg, output, quiet);
                process::exit(1);
            }
        }
    } else {
        // Elaborate .tenor file first
        match tenor_core::elaborate::elaborate(file) {
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
        }
    };

    let explain_format = match format {
        ExplainOutputFormat::Terminal => explain::ExplainFormat::Terminal,
        ExplainOutputFormat::Markdown => explain::ExplainFormat::Markdown,
    };

    match explain::explain(&bundle, explain_format, verbose) {
        Ok(result) => {
            if !quiet {
                print!("{}", result);
            }
        }
        Err(e) => {
            let msg = format!("explain error: {}", e);
            report_error(&msg, output, quiet);
            process::exit(1);
        }
    }
}

fn cmd_generate(command: GenerateCommands, output: OutputFormat, quiet: bool) {
    match command {
        GenerateCommands::Typescript {
            input,
            out,
            sdk_import,
        } => {
            // Determine input type by extension
            let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");

            let bundle_json: serde_json::Value = match ext {
                "tenor" => {
                    // Elaborate .tenor file first
                    match tenor_core::elaborate::elaborate(&input) {
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
                    }
                }
                "json" => {
                    // Read and parse interchange JSON
                    let json_str = match std::fs::read_to_string(&input) {
                        Ok(s) => s,
                        Err(e) => {
                            let msg = format!("error reading '{}': {}", input.display(), e);
                            report_error(&msg, output, quiet);
                            process::exit(1);
                        }
                    };
                    match serde_json::from_str(&json_str) {
                        Ok(v) => v,
                        Err(e) => {
                            let msg = format!("error parsing JSON in '{}': {}", input.display(), e);
                            report_error(&msg, output, quiet);
                            process::exit(1);
                        }
                    }
                }
                _ => {
                    let msg = format!(
                        "unsupported input file type '{}': expected .tenor or .json",
                        input.display()
                    );
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            };

            let config = tenor_codegen::TypeScriptConfig {
                out_dir: out,
                sdk_import,
            };

            match tenor_codegen::generate_typescript(&bundle_json, &config) {
                Ok(output_dir) => {
                    if !quiet {
                        match output {
                            OutputFormat::Text => {
                                println!("Generated TypeScript files in {}", output_dir.display());
                            }
                            OutputFormat::Json => {
                                println!("{{\"output_dir\": \"{}\"}}", output_dir.display());
                            }
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("code generation error: {}", e);
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            }
        }
    }
}

pub(crate) fn report_error(msg: &str, output: OutputFormat, quiet: bool) {
    if quiet {
        return;
    }
    match output {
        OutputFormat::Text => eprintln!("{}", msg),
        OutputFormat::Json => {
            eprintln!("{{\"error\": \"{}\"}}", msg.replace('"', "\\\""));
        }
    }
}
