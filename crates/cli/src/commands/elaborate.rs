use std::path::Path;
use std::process;

use crate::OutputFormat;

pub(crate) fn cmd_elaborate(file: &Path, manifest: bool, output: OutputFormat, quiet: bool) {
    match tenor_core::elaborate::elaborate(file) {
        Ok(bundle) => {
            let output_value = if manifest {
                crate::manifest::build_manifest(bundle)
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
