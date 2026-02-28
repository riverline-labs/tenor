use std::path::Path;
use std::process;

use crate::{report_error, OutputFormat};

static INTERCHANGE_SCHEMA_STR: &str = include_str!("../../../../docs/interchange-schema.json");
static MANIFEST_SCHEMA_STR: &str = include_str!("../../../../docs/manifest-schema.json");

pub(crate) fn cmd_validate(bundle_path: &Path, output: OutputFormat, quiet: bool) {
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
