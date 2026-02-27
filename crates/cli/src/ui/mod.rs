//! `tenor ui` — Generate a React application from a Tenor contract.

mod api_client;
mod generate;
mod templates;

use std::path::Path;

use crate::{report_error, OutputFormat};
use tenor_codegen::bundle::CodegenBundle;

/// Options for the `tenor ui` command.
pub(crate) struct UiOptions<'a> {
    pub contract: &'a Path,
    pub output_dir: &'a Path,
    pub api_url: &'a str,
    pub contract_id: Option<&'a str>,
    pub theme: Option<&'a Path>,
    pub title: Option<&'a str>,
    pub output: OutputFormat,
    pub quiet: bool,
}

/// Run the `tenor ui` command.
pub(crate) fn cmd_ui(opts: UiOptions<'_>) {
    // Step 1: Load the contract (elaborate .tenor or parse .json)
    let ext = opts
        .contract
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let bundle_json: serde_json::Value = if ext == "json" {
        let json_str = match std::fs::read_to_string(opts.contract) {
            Ok(s) => s,
            Err(e) => {
                report_error(
                    &format!("error reading '{}': {}", opts.contract.display(), e),
                    opts.output,
                    opts.quiet,
                );
                std::process::exit(1);
            }
        };
        match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                report_error(
                    &format!("error parsing JSON in '{}': {}", opts.contract.display(), e),
                    opts.output,
                    opts.quiet,
                );
                std::process::exit(1);
            }
        }
    } else {
        match tenor_core::elaborate::elaborate(opts.contract) {
            Ok(b) => b,
            Err(e) => {
                report_error(
                    &format!("elaboration error: {:?}", e),
                    opts.output,
                    opts.quiet,
                );
                std::process::exit(1);
            }
        }
    };

    // Step 2: Parse the interchange JSON into CodegenBundle
    let bundle = match CodegenBundle::from_interchange(&bundle_json) {
        Ok(b) => b,
        Err(e) => {
            report_error(
                &format!("interchange parse error: {}", e),
                opts.output,
                opts.quiet,
            );
            std::process::exit(1);
        }
    };

    // Step 3: Determine contract_id
    let contract_id = opts.contract_id.unwrap_or(&bundle.id).to_string();

    // Step 4: Determine title
    let title = opts
        .title
        .map(|t| t.to_string())
        .unwrap_or_else(|| snake_to_title(&contract_id));

    // Step 5: Load custom theme JSON if --theme is provided
    let custom_theme: Option<serde_json::Value> = if let Some(theme_path) = opts.theme {
        let theme_str = match std::fs::read_to_string(theme_path) {
            Ok(s) => s,
            Err(e) => {
                report_error(
                    &format!("error reading theme file '{}': {}", theme_path.display(), e),
                    opts.output,
                    opts.quiet,
                );
                std::process::exit(1);
            }
        };
        match serde_json::from_str(&theme_str) {
            Ok(v) => Some(v),
            Err(e) => {
                report_error(
                    &format!(
                        "error parsing theme JSON in '{}': {}",
                        theme_path.display(),
                        e
                    ),
                    opts.output,
                    opts.quiet,
                );
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Step 6: Generate the UI project
    let config = generate::UiConfig {
        output_dir: opts.output_dir.to_path_buf(),
        api_url: opts.api_url.to_string(),
        contract_id,
        title,
        custom_theme,
    };

    match generate::generate_ui_project(&bundle, &config) {
        Ok(files) => {
            if !opts.quiet {
                match opts.output {
                    OutputFormat::Text => {
                        println!(
                            "UI generated at {}. Run `cd {} && npm install && npm run dev` to start.",
                            opts.output_dir.display(),
                            opts.output_dir.display()
                        );
                        println!("Generated {} file(s):", files.len());
                        for f in &files {
                            println!("  {}", f.display());
                        }
                    }
                    OutputFormat::Json => {
                        let file_list: Vec<String> =
                            files.iter().map(|f| f.display().to_string()).collect();
                        let json = serde_json::json!({
                            "output_dir": opts.output_dir.display().to_string(),
                            "files": file_list,
                            "next_steps": format!(
                                "cd {} && npm install && npm run dev",
                                opts.output_dir.display()
                            ),
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
            report_error(
                &format!("UI generation error: {}", e),
                opts.output,
                opts.quiet,
            );
            std::process::exit(1);
        }
    }
}

/// Convert a snake_case or kebab-case string to Title Case.
fn snake_to_title(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper = c.to_uppercase().to_string();
                    upper + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
