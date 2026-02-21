mod elaborate;
mod error;
mod lexer;
mod parser;
mod runner;
mod tap;

use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  tenor-elaborator run <conformance-suite-dir>");
        eprintln!("  tenor-elaborator elaborate <file.tenor>");
        process::exit(1);
    }

    match args[1].as_str() {
        "run" => {
            let suite_dir = if args.len() >= 3 {
                Path::new(&args[2]).to_path_buf()
            } else {
                Path::new("../conformance").to_path_buf()
            };

            if !suite_dir.exists() {
                eprintln!("error: conformance suite directory not found: {}", suite_dir.display());
                process::exit(1);
            }

            let result = runner::run_suite(&suite_dir);
            if result.failed > 0 {
                process::exit(1);
            }
        }
        "elaborate" => {
            if args.len() < 3 {
                eprintln!("Usage: tenor-elaborator elaborate <file.tenor>");
                process::exit(1);
            }
            let path = Path::new(&args[2]);
            match elaborate::elaborate(path) {
                Ok(bundle) => {
                    let pretty = serde_json::to_string_pretty(&bundle)
                        .unwrap_or_else(|e| format!("serialization error: {}", e));
                    println!("{}", pretty);
                }
                Err(e) => {
                    let err_json = serde_json::to_string_pretty(&e.to_json_value())
                        .unwrap_or_else(|_| format!("{:?}", e));
                    eprintln!("{}", err_json);
                    process::exit(1);
                }
            }
        }
        cmd => {
            eprintln!("unknown command '{}'; use 'run' or 'elaborate'", cmd);
            process::exit(1);
        }
    }
}
