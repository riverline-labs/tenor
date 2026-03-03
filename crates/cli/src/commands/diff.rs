use std::path::Path;
use std::process;

use crate::{report_error, OutputFormat};

pub(crate) fn cmd_diff(
    t1_path: &Path,
    t2_path: &Path,
    breaking: bool,
    output: OutputFormat,
    quiet: bool,
) {
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
    let bundle_diff = match crate::diff::diff_bundles(&t1, &t2) {
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

        let classified = crate::diff::classify_diff(&bundle_diff);
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
