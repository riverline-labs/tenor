//! Implementation of the `tenor publish` subcommand.
//!
//! Packs (if needed), validates, and uploads a template archive to the
//! registry API using [`RegistryClient`].

use std::path::Path;

use crate::OutputFormat;

use super::{pack, registry::RegistryClient};

/// Run the `tenor publish` subcommand.
///
/// # Arguments
///
/// * `dir` — template directory (default: `.`)
/// * `registry_url` — registry endpoint override
/// * `token` — auth token; if `None`, the `TENOR_REGISTRY_TOKEN` env var is
///   checked as a fallback
/// * `output` — output format (text / JSON)
/// * `quiet` — suppress non-essential output
pub fn cmd_publish(
    dir: &Path,
    registry_url: Option<&str>,
    token: Option<&str>,
    output: OutputFormat,
    quiet: bool,
) {
    // Resolve auth token: prefer --token flag, fall back to env var.
    let resolved_token = token
        .map(|t| t.to_string())
        .or_else(|| std::env::var("TENOR_REGISTRY_TOKEN").ok());

    if resolved_token.is_none() {
        eprintln!("error: --token or TENOR_REGISTRY_TOKEN required for publishing");
        std::process::exit(1);
    }
    let token_str = resolved_token.unwrap();

    // Pack the template directory into a .tenor-template.tar.gz archive.
    if !quiet {
        if let OutputFormat::Text = output {
            println!("Packing template...");
        }
    }

    let pack_result = match pack::pack_template(dir, None) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let manifest = &pack_result.manifest.template;
    let archive_path = &pack_result.archive_path;

    if !quiet {
        if let OutputFormat::Text = output {
            println!(
                "Packed: {} ({} bytes)",
                archive_path.display(),
                std::fs::metadata(archive_path)
                    .map(|m| m.len())
                    .unwrap_or(0)
            );
        }
    }

    // Upload via the registry client.
    let client = RegistryClient::new(registry_url, Some(&token_str));

    let response = match client.publish(archive_path, manifest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Report success.
    let registry_display = registry_url.unwrap_or(super::registry::DEFAULT_REGISTRY_URL);

    if !quiet {
        match output {
            OutputFormat::Text => {
                println!(
                    "Published {}@{} to {}",
                    response.name, response.version, registry_display
                );
                println!("Status: {}", response.status);
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "name": response.name,
                    "version": response.version,
                    "status": response.status,
                    "registry": registry_display,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            }
        }
    }
}
