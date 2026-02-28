use std::path::Path;
use std::process;

use crate::{report_error, ExplainOutputFormat, OutputFormat};

pub(crate) fn cmd_explain(
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
        ExplainOutputFormat::Terminal => crate::explain::ExplainFormat::Terminal,
        ExplainOutputFormat::Markdown => crate::explain::ExplainFormat::Markdown,
    };

    match crate::explain::explain(&bundle, explain_format, verbose) {
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
