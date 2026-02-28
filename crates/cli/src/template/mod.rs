//! Template packaging for the Tenor marketplace.
//!
//! Provides the `tenor-template.toml` manifest format and the `tenor pack`
//! command that produces `.tenor-template.tar.gz` archives.

pub mod manifest;
pub mod pack;

use std::path::Path;

/// Dispatch function for the `tenor pack` CLI subcommand.
pub fn cmd_pack(dir: &Path, output: Option<&Path>, quiet: bool) {
    match pack::pack_template(dir, output) {
        Ok(result) => {
            if !quiet {
                println!("Packed: {}", result.archive_path.display());
                if let Ok(meta) = std::fs::metadata(&result.archive_path) {
                    println!("Size:   {} bytes", meta.len());
                }
                println!("SHA256: {}", result.archive_hash);
                println!(
                    "Template: {}-{}",
                    result.manifest.template.name, result.manifest.template.version
                );
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
