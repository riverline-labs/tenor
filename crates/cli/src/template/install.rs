//! Implementation of the `tenor install` subcommand.
//!
//! Downloads a template archive from the registry and unpacks it to the
//! output directory using [`pack::unpack_template`].

use std::path::Path;

use crate::OutputFormat;

use super::{pack, registry::RegistryClient};

/// Run the `tenor install` subcommand.
///
/// # Arguments
///
/// * `template_name` — template name to install (e.g. `escrow-release`)
/// * `version` — specific version to install; `None` means "latest"
/// * `output_dir` — directory to unpack the template into
/// * `registry_url` — registry endpoint override
/// * `output` — output format (text / JSON)
/// * `quiet` — suppress non-essential output
pub fn cmd_install(
    template_name: &str,
    version: Option<&str>,
    output_dir: &Path,
    registry_url: Option<&str>,
    output: OutputFormat,
    quiet: bool,
) {
    let client = RegistryClient::new(registry_url, None);

    if !quiet {
        if let OutputFormat::Text = output {
            let version_display = version.unwrap_or("latest");
            println!(
                "Downloading {}@{} from registry...",
                template_name, version_display
            );
        }
    }

    // Download the archive bytes.
    let archive_bytes = match client.download(template_name, version) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Write the archive to a temporary file.
    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: could not create temp directory: {}", e);
            std::process::exit(1);
        }
    };

    let version_str = version.unwrap_or("latest");
    let archive_filename = format!("{}-{}.tenor-template.tar.gz", template_name, version_str);
    let tmp_archive = tmp_dir.path().join(&archive_filename);

    if let Err(e) = std::fs::write(&tmp_archive, &archive_bytes) {
        eprintln!(
            "error: could not write archive to temp file: {}",
            e
        );
        std::process::exit(1);
    }

    // Warn if output directory already exists and has content, but proceed.
    if output_dir.exists() {
        let is_nonempty = std::fs::read_dir(output_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

        if is_nonempty && !quiet {
            eprintln!(
                "warning: output directory '{}' already exists — contents will be overwritten",
                output_dir.display()
            );
        }
    }

    // Unpack the archive.
    let manifest_file = match pack::unpack_template(&tmp_archive, output_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not unpack template: {}", e);
            std::process::exit(1);
        }
    };

    // Report success.
    let installed_version = &manifest_file.template.version;

    if !quiet {
        match output {
            OutputFormat::Text => {
                println!(
                    "Installed {}@{} to {}",
                    template_name,
                    installed_version,
                    output_dir.display()
                );
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "name": template_name,
                    "version": installed_version,
                    "output_dir": output_dir.display().to_string(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            }
        }
    }
}
