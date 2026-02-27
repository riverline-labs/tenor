//! `tenor connect` — Source introspection, fact-to-source matching,
//! and adapter scaffolding generation.

#[allow(dead_code)]
mod generate;
#[allow(dead_code)]
mod introspect;
#[allow(dead_code)]
mod matching;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{report_error, OutputFormat};
use tenor_interchange::{
    from_interchange, FactConstruct, InterchangeBundle, InterchangeConstruct, SourceConstruct,
};

/// Run the `tenor connect` command.
pub(crate) fn cmd_connect(
    contract: &Path,
    environment: Option<&Path>,
    output_dir: &Path,
    dry_run: bool,
    output: OutputFormat,
    quiet: bool,
) {
    // Step 1: Elaborate or load the contract
    let bundle = load_bundle(contract, output, quiet);

    // Step 2: Extract Sources and Facts with structured sources
    let sources = extract_sources(&bundle);
    let facts = extract_structured_facts(&bundle);

    if sources.is_empty() {
        if !quiet {
            report_error("no Source declarations found in contract", output, quiet);
        }
        std::process::exit(1);
    }

    // Step 3: Introspect external schemas where schema_ref is available
    let mut schemas: BTreeMap<String, introspect::ExternalSchema> = BTreeMap::new();
    for source in &sources {
        if let Some(schema_ref) = source.fields.get("schema_ref") {
            // Use --environment override if provided and source has schema_ref
            let schema_path = environment
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(schema_ref));

            match introspect::introspect_schema(&source.protocol, &schema_path) {
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

    // Step 4: Match facts to external schema fields
    let mappings = matching::match_facts(&sources, &facts, &schemas);

    // Step 5: Output
    if dry_run {
        print_dry_run(&sources, &facts, &mappings, &schemas, output, quiet);
        return;
    }

    // Step 6: Generate scaffolding
    match generate::generate_scaffolding(&sources, &facts, &mappings, &schemas, output_dir) {
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

/// Print dry-run output showing proposed mappings.
fn print_dry_run(
    sources: &[SourceConstruct],
    facts: &[StructuredFact],
    mappings: &[matching::FactMapping],
    schemas: &BTreeMap<String, introspect::ExternalSchema>,
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

            println!("Proposed mappings ({}):", mappings.len());
            for m in mappings {
                let confidence = match m.confidence {
                    matching::Confidence::High => "HIGH",
                    matching::Confidence::Medium => "MEDIUM",
                    matching::Confidence::Low => "LOW",
                };
                println!("  {} -> {} [{}]", m.fact_id, m.description, confidence);
                if let Some(ref note) = m.note {
                    println!("    {}", note);
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
                "mappings": mappings.iter().map(|m| {
                    serde_json::json!({
                        "fact_id": m.fact_id,
                        "source_id": m.source_id,
                        "path": m.path,
                        "confidence": format!("{:?}", m.confidence).to_lowercase(),
                        "description": m.description,
                        "note": m.note,
                    })
                }).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&json).unwrap_or_default()
            );
        }
    }
}
