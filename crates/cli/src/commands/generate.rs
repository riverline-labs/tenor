use std::process;

use crate::{report_error, GenerateCommands, OutputFormat};

pub(crate) fn cmd_generate(command: GenerateCommands, output: OutputFormat, quiet: bool) {
    match command {
        GenerateCommands::Typescript {
            input,
            out,
            sdk_import,
        } => {
            // Determine input type by extension
            let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");

            let bundle_json: serde_json::Value = match ext {
                "tenor" => {
                    // Elaborate .tenor file first
                    match tenor_core::elaborate::elaborate(&input) {
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
                }
                "json" => {
                    // Read and parse interchange JSON
                    let json_str = match std::fs::read_to_string(&input) {
                        Ok(s) => s,
                        Err(e) => {
                            let msg = format!("error reading '{}': {}", input.display(), e);
                            report_error(&msg, output, quiet);
                            process::exit(1);
                        }
                    };
                    match serde_json::from_str(&json_str) {
                        Ok(v) => v,
                        Err(e) => {
                            let msg = format!("error parsing JSON in '{}': {}", input.display(), e);
                            report_error(&msg, output, quiet);
                            process::exit(1);
                        }
                    }
                }
                _ => {
                    let msg = format!(
                        "unsupported input file type '{}': expected .tenor or .json",
                        input.display()
                    );
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            };

            let config = tenor_codegen::TypeScriptConfig {
                out_dir: out,
                sdk_import,
            };

            match tenor_codegen::generate_typescript(&bundle_json, &config) {
                Ok(output_dir) => {
                    if !quiet {
                        match output {
                            OutputFormat::Text => {
                                println!("Generated TypeScript files in {}", output_dir.display());
                            }
                            OutputFormat::Json => {
                                println!("{{\"output_dir\": \"{}\"}}", output_dir.display());
                            }
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("code generation error: {}", e);
                    report_error(&msg, output, quiet);
                    process::exit(1);
                }
            }
        }
    }
}
