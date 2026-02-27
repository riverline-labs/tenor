//! `tenor connect` — Source introspection, fact-to-source matching,
//! and adapter scaffolding generation.

mod generate;
mod heuristic_provider;
mod introspect;
mod llm_provider;
mod matching;
mod provider;
mod workflow;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{report_error, OutputFormat};
use tenor_interchange::{
    from_interchange, FactConstruct, InterchangeBundle, InterchangeConstruct, SourceConstruct,
};

/// Options for the `tenor connect` command.
pub(crate) struct ConnectOptions<'a> {
    pub contract: &'a Path,
    pub environment: Option<&'a Path>,
    pub output_dir: &'a Path,
    pub dry_run: bool,
    pub batch: bool,
    pub apply: Option<&'a Path>,
    pub model: Option<&'a str>,
    pub heuristic: bool,
    pub verbose: bool,
    pub output: OutputFormat,
    pub quiet: bool,
}

/// Run the `tenor connect` command.
pub(crate) fn cmd_connect(opts: ConnectOptions<'_>) {
    // --apply mode: read review file + contract, generate from accepted mappings
    if let Some(review_path) = opts.apply {
        run_apply_mode(
            review_path,
            opts.contract,
            opts.output_dir,
            opts.output,
            opts.quiet,
        );
        return;
    }

    // Step 1: Elaborate or load the contract
    let bundle = load_bundle(opts.contract, opts.output, opts.quiet);

    // Step 2: Extract Sources and Facts with structured sources
    let sources = extract_sources(&bundle);
    let facts = extract_structured_facts(&bundle);

    if sources.is_empty() {
        report_error(
            "no Source declarations found in contract",
            opts.output,
            opts.quiet,
        );
        std::process::exit(1);
    }

    // Step 3: Introspect external schemas
    let schemas = introspect_schemas(&sources, opts.environment, opts.quiet);

    // Step 4: Run matching via provider
    let proposals = run_matching(
        &sources,
        &facts,
        &schemas,
        opts.heuristic,
        opts.model,
        opts.verbose,
        opts.output,
        opts.quiet,
    );

    // Step 5: Route based on mode
    if opts.dry_run {
        print_dry_run(
            &sources,
            &facts,
            &proposals,
            &schemas,
            opts.verbose,
            opts.output,
            opts.quiet,
        );
        return;
    }

    if opts.batch {
        run_batch_mode(&proposals, opts.output_dir, opts.output, opts.quiet);
        return;
    }

    // Interactive mode (default)
    run_interactive_mode(
        &proposals,
        &sources,
        &facts,
        &schemas,
        opts.output_dir,
        opts.output,
        opts.quiet,
    );
}

/// Apply mode: read a review file, load the contract, generate from accepted mappings.
fn run_apply_mode(
    review_path: &Path,
    contract: &Path,
    output_dir: &Path,
    output: OutputFormat,
    quiet: bool,
) {
    let accepted = match workflow::read_review_file(review_path) {
        Ok(a) => a,
        Err(e) => {
            report_error(&format!("apply error: {}", e), output, quiet);
            std::process::exit(1);
        }
    };

    if accepted.is_empty() {
        if !quiet {
            eprintln!("No accepted mappings found in '{}'", review_path.display());
        }
        return;
    }

    // Load contract for sources/facts
    let bundle = load_bundle(contract, output, quiet);
    let sources = extract_sources(&bundle);
    let facts = extract_structured_facts(&bundle);
    let schemas: BTreeMap<String, introspect::ExternalSchema> = BTreeMap::new();

    // Convert ReviewedMappings to FactMappings for generation
    let mappings: Vec<matching::FactMapping> = accepted
        .iter()
        .map(|r| matching::FactMapping {
            fact_id: r.fact_id.clone(),
            source_id: r.source_id.clone(),
            path: r.field_path.clone(),
            confidence: parse_confidence(&r.confidence),
            description: format!("{} -> {}", r.endpoint, r.field_path),
            note: None,
        })
        .collect();

    generate_and_report(
        &sources, &facts, &mappings, &schemas, output_dir, output, quiet,
    );

    if !quiet {
        eprintln!(
            "Applied {} accepted mapping(s) from '{}'",
            accepted.len(),
            review_path.display()
        );
    }
}

/// Introspect external schemas for all sources that have a schema_ref.
fn introspect_schemas(
    sources: &[SourceConstruct],
    environment: Option<&Path>,
    quiet: bool,
) -> BTreeMap<String, introspect::ExternalSchema> {
    let mut schemas = BTreeMap::new();

    for source in sources {
        // Use --environment if provided (overrides schema_ref for all sources)
        let schema_path = if let Some(env_path) = environment {
            Some(env_path.to_path_buf())
        } else {
            source.fields.get("schema_ref").map(PathBuf::from)
        };

        if let Some(path) = schema_path {
            // Try auto-detection first, fall back to protocol-based
            let result = introspect::detect_and_introspect(&path)
                .or_else(|_| introspect::introspect_schema(&source.protocol, &path));

            match result {
                Ok(schema) => {
                    schemas.insert(source.id.clone(), schema);
                }
                Err(e) => {
                    if !quiet {
                        eprintln!(
                            "warning: could not introspect schema for source '{}': {}",
                            source.id, e
                        );
                    }
                }
            }
        }
    }

    schemas
}

/// Run matching and return proposals. Uses LLM or heuristic based on config.
#[allow(clippy::too_many_arguments)]
fn run_matching(
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    schemas: &BTreeMap<String, introspect::ExternalSchema>,
    force_heuristic: bool,
    model: Option<&str>,
    verbose: bool,
    output: OutputFormat,
    quiet: bool,
) -> Vec<provider::MappingProposal> {
    // Build provider-compatible types
    let fact_decls: Vec<provider::FactDeclaration> = facts
        .iter()
        .map(|f| provider::FactDeclaration {
            fact_id: f.id.clone(),
            base_type: f
                .fact_type
                .get("base")
                .and_then(|b| b.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            source_id: f.source_id.clone(),
            path: f.path.clone(),
            full_type: f.fact_type.clone(),
        })
        .collect();

    let environment = provider::EnvironmentInventory {
        schemas: schemas.clone(),
    };

    // Determine provider
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let use_llm = !force_heuristic && api_key.is_some();

    if !use_llm && !force_heuristic && !quiet {
        eprintln!("No ANTHROPIC_API_KEY set. Using heuristic matching. Set ANTHROPIC_API_KEY for LLM-powered matching.");
    }

    if use_llm {
        if verbose && !quiet {
            eprintln!(
                "Using LLM matching (model: {})",
                model.unwrap_or("claude-sonnet-4-20250514")
            );
        }

        let api_key = api_key.unwrap();
        let config = match model {
            Some(m) => llm_provider::LlmMatchingConfig::with_model(api_key, m.to_string()),
            None => llm_provider::LlmMatchingConfig::new(api_key),
        };
        let provider = llm_provider::LlmMatchingProvider::new(config);

        // Run async matching in a tokio runtime
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        match rt.block_on(provider::MatchingProvider::propose_mappings(
            &provider,
            &fact_decls,
            &environment,
        )) {
            Ok(proposals) => proposals,
            Err(e) => {
                if !quiet {
                    eprintln!("LLM matching failed: {}. Falling back to heuristic.", e);
                }
                run_heuristic_matching(sources, &fact_decls, &environment, verbose, output, quiet)
            }
        }
    } else {
        if verbose && !quiet {
            eprintln!("Using heuristic matching");
        }
        run_heuristic_matching(sources, &fact_decls, &environment, verbose, output, quiet)
    }
}

/// Run heuristic matching.
fn run_heuristic_matching(
    sources: &[SourceConstruct],
    fact_decls: &[provider::FactDeclaration],
    environment: &provider::EnvironmentInventory,
    _verbose: bool,
    _output: OutputFormat,
    _quiet: bool,
) -> Vec<provider::MappingProposal> {
    let provider = heuristic_provider::HeuristicMatchingProvider::new(sources.to_vec());
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    match rt.block_on(provider::MatchingProvider::propose_mappings(
        &provider,
        fact_decls,
        environment,
    )) {
        Ok(proposals) => proposals,
        Err(e) => {
            eprintln!("heuristic matching error: {}", e);
            vec![]
        }
    }
}

/// Convert provider proposals to workflow proposals for interactive/batch use.
fn to_workflow_proposals(
    proposals: &[provider::MappingProposal],
    facts: &[StructuredFact],
) -> Vec<workflow::MappingProposal> {
    proposals
        .iter()
        .map(|p| {
            let fact_type = facts
                .iter()
                .find(|f| f.id == p.fact_id)
                .map(|f| {
                    let base = f
                        .fact_type
                        .get("base")
                        .and_then(|b| b.as_str())
                        .unwrap_or("?");
                    base.to_string()
                })
                .unwrap_or_else(|| "?".to_string());

            workflow::MappingProposal {
                fact_id: p.fact_id.clone(),
                fact_type,
                source_id: p.source_id.clone(),
                endpoint: p.endpoint.clone(),
                field_path: p.field_path.clone(),
                confidence: p.confidence,
                explanation: p.explanation.clone(),
            }
        })
        .collect()
}

/// Interactive mode: prompt user, generate from accepted.
fn run_interactive_mode(
    proposals: &[provider::MappingProposal],
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    schemas: &BTreeMap<String, introspect::ExternalSchema>,
    output_dir: &Path,
    output: OutputFormat,
    quiet: bool,
) {
    let wf_proposals = to_workflow_proposals(proposals, facts);
    let reviewed = workflow::run_interactive(&wf_proposals, quiet);

    // Filter to accepted mappings
    let accepted_fact_ids: std::collections::HashSet<&str> = reviewed
        .iter()
        .filter(|r| r.status == workflow::MappingStatus::Accepted)
        .map(|r| r.fact_id.as_str())
        .collect();

    if accepted_fact_ids.is_empty() {
        if !quiet {
            eprintln!("No mappings accepted. Nothing to generate.");
        }
        return;
    }

    // Build FactMappings from the reviewed (may have edited endpoints)
    let mappings: Vec<matching::FactMapping> = reviewed
        .iter()
        .filter(|r| r.status == workflow::MappingStatus::Accepted)
        .map(|r| matching::FactMapping {
            fact_id: r.fact_id.clone(),
            source_id: r.source_id.clone(),
            path: r.field_path.clone(),
            confidence: parse_confidence(&r.confidence),
            description: format!("{} -> {}", r.endpoint, r.field_path),
            note: None,
        })
        .collect();

    generate_and_report(
        sources, facts, &mappings, schemas, output_dir, output, quiet,
    );
}

/// Batch mode: write review TOML file.
fn run_batch_mode(
    proposals: &[provider::MappingProposal],
    output_dir: &Path,
    output: OutputFormat,
    quiet: bool,
) {
    // Ensure output dir exists
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        report_error(
            &format!("could not create output directory: {}", e),
            output,
            quiet,
        );
        std::process::exit(1);
    }

    let review_path = output_dir.join("tenor-connect-review.toml");

    // Convert to workflow proposals (we need fact_type info but we don't have facts here)
    // For batch mode, build workflow proposals from the provider proposals directly
    let wf_proposals: Vec<workflow::MappingProposal> = proposals
        .iter()
        .map(|p| workflow::MappingProposal {
            fact_id: p.fact_id.clone(),
            fact_type: format!("{:?}", p.confidence), // Will be overridden below
            source_id: p.source_id.clone(),
            endpoint: p.endpoint.clone(),
            field_path: p.field_path.clone(),
            confidence: p.confidence,
            explanation: p.explanation.clone(),
        })
        .collect();

    match workflow::write_review_file(&wf_proposals, &review_path) {
        Ok(()) => {
            if !quiet {
                match output {
                    OutputFormat::Text => {
                        println!("Review file written to: {}", review_path.display());
                        println!(
                            "Edit the file, then run: tenor connect <contract> --apply {}",
                            review_path.display()
                        );
                    }
                    OutputFormat::Json => {
                        let json = serde_json::json!({
                            "review_file": review_path.display().to_string(),
                            "proposals": proposals.len(),
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json).unwrap_or_default()
                        );
                    }
                }
            }
        }
        Err(e) => {
            report_error(&format!("batch error: {}", e), output, quiet);
            std::process::exit(1);
        }
    }
}

/// Generate scaffolding and report results.
fn generate_and_report(
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    mappings: &[matching::FactMapping],
    schemas: &BTreeMap<String, introspect::ExternalSchema>,
    output_dir: &Path,
    output: OutputFormat,
    quiet: bool,
) {
    match generate::generate_scaffolding(sources, facts, mappings, schemas, output_dir) {
        Ok(files) => {
            if !quiet {
                match output {
                    OutputFormat::Text => {
                        println!(
                            "Generated {} file(s) in {}",
                            files.len(),
                            output_dir.display()
                        );
                        for f in &files {
                            println!("  {}", f.display());
                        }
                    }
                    OutputFormat::Json => {
                        let file_list: Vec<String> =
                            files.iter().map(|f| f.display().to_string()).collect();
                        let json = serde_json::json!({
                            "output_dir": output_dir.display().to_string(),
                            "files": file_list,
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json).unwrap_or_default()
                        );
                    }
                }
            }
        }
        Err(e) => {
            report_error(&format!("generation error: {}", e), output, quiet);
            std::process::exit(1);
        }
    }
}

/// Parse a confidence string back to enum.
fn parse_confidence(s: &str) -> matching::Confidence {
    match s.to_uppercase().as_str() {
        "HIGH" => matching::Confidence::High,
        "MEDIUM" => matching::Confidence::Medium,
        _ => matching::Confidence::Low,
    }
}

/// Print dry-run output showing proposed mappings.
fn print_dry_run(
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    proposals: &[provider::MappingProposal],
    schemas: &BTreeMap<String, introspect::ExternalSchema>,
    verbose: bool,
    output: OutputFormat,
    quiet: bool,
) {
    if quiet {
        return;
    }

    match output {
        OutputFormat::Text => {
            println!("Sources ({}):", sources.len());
            for s in sources {
                let has_schema = schemas.contains_key(&s.id);
                let schema_status = if has_schema { " [schema loaded]" } else { "" };
                println!("  {} ({}){}", s.id, s.protocol, schema_status);
            }
            println!();

            println!("Facts with structured sources ({}):", facts.len());
            for f in facts {
                let base = f
                    .fact_type
                    .get("base")
                    .and_then(|b| b.as_str())
                    .unwrap_or("?");
                println!("  {} : {} -> {}.{}", f.id, base, f.source_id, f.path);
            }
            println!();

            println!("Proposed mappings ({}):", proposals.len());
            for p in proposals {
                let confidence = match p.confidence {
                    matching::Confidence::High => "HIGH",
                    matching::Confidence::Medium => "MEDIUM",
                    matching::Confidence::Low => "LOW",
                };
                println!(
                    "  {} -> {} -> {} [{}]",
                    p.fact_id, p.endpoint, p.field_path, confidence
                );
                if verbose {
                    println!("    Reason: {}", p.explanation);
                    for alt in &p.alternatives {
                        let alt_conf = match alt.confidence {
                            matching::Confidence::High => "HIGH",
                            matching::Confidence::Medium => "MEDIUM",
                            matching::Confidence::Low => "LOW",
                        };
                        println!(
                            "    Alt: {} -> {} [{}] {}",
                            alt.endpoint, alt.field_path, alt_conf, alt.explanation
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "sources": sources.iter().map(|s| {
                    serde_json::json!({
                        "id": s.id,
                        "protocol": s.protocol,
                        "schema_loaded": schemas.contains_key(&s.id),
                    })
                }).collect::<Vec<_>>(),
                "facts": facts.iter().map(|f| {
                    serde_json::json!({
                        "id": f.id,
                        "type": f.fact_type,
                        "source_id": f.source_id,
                        "path": f.path,
                    })
                }).collect::<Vec<_>>(),
                "mappings": proposals.iter().map(|p| {
                    let mut entry = serde_json::json!({
                        "fact_id": p.fact_id,
                        "source_id": p.source_id,
                        "endpoint": p.endpoint,
                        "field_path": p.field_path,
                        "confidence": format!("{:?}", p.confidence).to_lowercase(),
                        "explanation": p.explanation,
                    });
                    if verbose && !p.alternatives.is_empty() {
                        entry["alternatives"] = serde_json::json!(
                            p.alternatives.iter().map(|a| {
                                serde_json::json!({
                                    "endpoint": a.endpoint,
                                    "field_path": a.field_path,
                                    "confidence": format!("{:?}", a.confidence).to_lowercase(),
                                    "explanation": a.explanation,
                                })
                            }).collect::<Vec<_>>()
                        );
                    }
                    entry
                }).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&json).unwrap_or_default()
            );
        }
    }
}

/// Load a bundle from a .tenor file or interchange JSON.
fn load_bundle(path: &Path, output: OutputFormat, quiet: bool) -> InterchangeBundle {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let bundle_json: serde_json::Value = if ext == "json" {
        let json_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                report_error(
                    &format!("error reading '{}': {}", path.display(), e),
                    output,
                    quiet,
                );
                std::process::exit(1);
            }
        };
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                report_error(
                    &format!("error parsing JSON in '{}': {}", path.display(), e),
                    output,
                    quiet,
                );
                std::process::exit(1);
            }
        }
    } else {
        match tenor_core::elaborate::elaborate(path) {
            Ok(b) => b,
            Err(e) => {
                report_error(&format!("elaboration error: {:?}", e), output, quiet);
                std::process::exit(1);
            }
        }
    };

    match from_interchange(&bundle_json) {
        Ok(b) => b,
        Err(e) => {
            report_error(&format!("interchange parse error: {}", e), output, quiet);
            std::process::exit(1);
        }
    }
}

/// Extract all Source constructs from the bundle.
fn extract_sources(bundle: &InterchangeBundle) -> Vec<SourceConstruct> {
    bundle
        .constructs
        .iter()
        .filter_map(|c| {
            if let InterchangeConstruct::Source(s) = c {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect()
}

/// A fact with a structured source reference.
#[derive(Debug, Clone)]
pub(crate) struct StructuredFact {
    pub id: String,
    pub fact_type: serde_json::Value,
    pub source_id: String,
    pub path: String,
}

/// Extract facts that have structured source references.
fn extract_structured_facts(bundle: &InterchangeBundle) -> Vec<StructuredFact> {
    bundle
        .constructs
        .iter()
        .filter_map(|c| {
            if let InterchangeConstruct::Fact(f) = c {
                extract_structured_source(f)
            } else {
                None
            }
        })
        .collect()
}

/// Try to extract a structured source reference from a Fact.
fn extract_structured_source(fact: &FactConstruct) -> Option<StructuredFact> {
    let source = fact.source.as_ref()?;
    let source_id = source.get("source_id")?.as_str()?;
    let path = source.get("path")?.as_str()?;
    Some(StructuredFact {
        id: fact.id.clone(),
        fact_type: fact.fact_type.clone(),
        source_id: source_id.to_string(),
        path: path.to_string(),
    })
}
