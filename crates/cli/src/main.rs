mod agent;
mod ambiguity;
mod builder;
mod commands;
mod connect;
mod diff;
mod explain;
mod manifest;
mod migrate;
mod runner;
mod serve;
mod tap;
mod template;
mod trust;
mod ui;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};

/// Output format for CLI responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

/// Output format for the explain subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ExplainOutputFormat {
    Terminal,
    Markdown,
}

/// Tenor contract language toolchain.
#[derive(Parser)]
#[command(name = "tenor", version, about = "Tenor contract language toolchain")]
struct Cli {
    /// Output format (text or json)
    #[arg(long, global = true, default_value = "text", value_enum)]
    output: OutputFormat,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Elaborate a .tenor file to interchange JSON
    Elaborate {
        /// Path to the .tenor source file
        file: PathBuf,
        /// Produce a TenorManifest instead of a bare interchange bundle
        #[arg(long)]
        manifest: bool,
    },

    /// Validate interchange JSON against the formal JSON Schema
    Validate {
        /// Path to the interchange JSON bundle file
        bundle: PathBuf,
    },

    /// Evaluate a contract bundle against a set of facts
    Eval {
        /// Path to the interchange JSON bundle file
        bundle: PathBuf,
        /// Path to the facts JSON file
        #[arg(long)]
        facts: PathBuf,
        /// Flow ID to execute (enables flow evaluation mode)
        #[arg(long)]
        flow: Option<String>,
        /// Persona executing the flow (required when --flow is specified)
        #[arg(long)]
        persona: Option<String>,
    },

    /// Run the conformance test suite
    Test {
        /// Path to the conformance suite directory
        #[arg(default_value = "conformance")]
        suite_dir: PathBuf,
    },

    /// Diff two interchange JSON bundles for structural changes
    Diff {
        /// Path to the first interchange JSON bundle
        t1: PathBuf,
        /// Path to the second interchange JSON bundle
        t2: PathBuf,
        /// Classify changes as breaking/non-breaking using Section 17.2 taxonomy
        #[arg(long)]
        breaking: bool,
    },

    /// Analyze migration between two contract versions
    Migrate {
        /// Path to the v1 contract (.tenor or .json)
        v1: PathBuf,
        /// Path to the v2 contract (.tenor or .json)
        v2: PathBuf,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Run static analysis checks on a .tenor file
    Check {
        /// Path to the .tenor source file
        file: PathBuf,
        /// Comma-separated list of analyses to run (s1,s2,s3a,s4,s5,s6,s7,s8). Default: all.
        #[arg(long)]
        analysis: Option<String>,
    },

    /// Explain a contract bundle in natural language
    Explain {
        /// Path to .tenor source file or interchange JSON bundle
        file: PathBuf,
        /// Output format (terminal or markdown)
        #[arg(long, default_value = "terminal")]
        format: ExplainOutputFormat,
        /// Show technical details (entity states, rule strata, analysis findings)
        #[arg(long)]
        verbose: bool,
    },

    /// Generate code from a contract bundle
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },

    /// Run AI ambiguity testing against the conformance suite
    Ambiguity {
        /// Path to the conformance suite directory
        suite_dir: PathBuf,
        /// Path to the Tenor specification file
        #[arg(long)]
        spec: Option<PathBuf>,
        /// LLM model name to use
        #[arg(long)]
        model: Option<String>,
    },

    /// Start the Tenor HTTP API server
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Path to TLS certificate PEM file (requires --tls-key)
        #[arg(long)]
        tls_cert: Option<PathBuf>,
        /// Path to TLS private key PEM file (requires --tls-cert)
        #[arg(long)]
        tls_key: Option<PathBuf>,
        /// .tenor contract files to pre-load
        #[arg()]
        contracts: Vec<PathBuf>,
    },

    /// Start an interactive agent shell for a contract
    Agent {
        /// Path to the .tenor source file
        file: PathBuf,
    },

    /// Introspect sources and generate adapter scaffolding
    Connect {
        /// Path to the .tenor source file or interchange JSON
        contract: PathBuf,
        /// Path to external schema document (OpenAPI, GraphQL SDL, SQL DDL — auto-detected)
        #[arg(long)]
        environment: Option<PathBuf>,
        /// Output directory for generated adapter scaffolding
        #[arg(long, default_value = "./tenor-connect-output")]
        out: PathBuf,
        /// Show proposed mappings without generating files
        #[arg(long)]
        dry_run: bool,
        /// Output review file instead of interactive prompts
        #[arg(long)]
        batch: bool,
        /// Generate adapters from a reviewed mapping file
        #[arg(long)]
        apply: Option<PathBuf>,
        /// LLM model to use (default: claude-sonnet-4-20250514)
        #[arg(long)]
        model: Option<String>,
        /// Force heuristic matching (skip LLM even if API key is set)
        #[arg(long)]
        heuristic: bool,
        /// Show detailed matching reasoning
        #[arg(long)]
        verbose: bool,
    },

    /// Generate a React application from a contract
    Ui {
        /// Path to .tenor source file or interchange JSON bundle
        contract: PathBuf,
        /// Output directory (default: ./tenor-ui/)
        #[arg(long = "out", default_value = "./tenor-ui")]
        out_dir: PathBuf,
        /// Executor API base URL
        #[arg(long, default_value = "http://localhost:3000")]
        api_url: String,
        /// Contract ID for multi-contract executors
        #[arg(long)]
        contract_id: Option<String>,
        /// Custom theme file (JSON)
        #[arg(long)]
        theme: Option<PathBuf>,
        /// Application title (default: contract id)
        #[arg(long)]
        title: Option<String>,
    },

    /// Start or build the Tenor Builder SPA
    Builder {
        /// Builder subcommand (omit to start dev server)
        #[command(subcommand)]
        command: Option<BuilderCommands>,
        /// Port for the dev server
        #[arg(long, default_value = "5173")]
        port: u16,
        /// Open browser after starting
        #[arg(long)]
        open: bool,
        /// Pre-load a .tenor or .json contract file
        #[arg(long)]
        contract: Option<PathBuf>,
    },

    /// Package a contract template for publishing
    Pack {
        /// Template directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Output archive path
        #[arg(long = "out", name = "pack_out")]
        out: Option<PathBuf>,
    },

    /// Publish a contract template to the registry
    Publish {
        /// Template directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Registry URL
        #[arg(long)]
        registry: Option<String>,
        /// Auth token for publishing
        #[arg(long)]
        token: Option<String>,
    },

    /// Search for contract templates in the registry
    Search {
        /// Search query
        query: String,
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
        /// Registry URL
        #[arg(long)]
        registry: Option<String>,
    },

    /// Install a contract template from the registry
    Install {
        /// Template name
        template_name: String,
        /// Specific version (default: latest)
        #[arg(long)]
        version: Option<String>,
        /// Output directory
        #[arg(long = "out", name = "install_out", default_value = ".")]
        out: PathBuf,
        /// Registry URL
        #[arg(long)]
        registry: Option<String>,
    },

    /// Deploy a contract template to the hosted platform
    Deploy {
        /// Template name (from registry)
        template_name: String,
        /// Organization ID (overrides org_id in config file)
        #[arg(long)]
        org: Option<String>,
        /// Template version (default: latest)
        #[arg(long)]
        version: Option<String>,
        /// Deployment configuration file (sources, persona mappings)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Registry URL
        #[arg(long)]
        registry: Option<String>,
        /// Platform API URL
        #[arg(long)]
        platform: Option<String>,
        /// Platform auth token (or set TENOR_PLATFORM_TOKEN)
        #[arg(long)]
        token: Option<String>,
    },

    /// Start the Language Server Protocol server over stdio
    Lsp,

    /// Generate an Ed25519 signing keypair
    Keygen {
        /// Signing algorithm (default: ed25519, only option for v1)
        #[arg(long, default_value = "ed25519")]
        algorithm: String,
        /// Output file prefix (default: tenor-key)
        #[arg(long = "prefix", default_value = "tenor-key")]
        prefix: String,
    },

    /// Sign an interchange bundle with a detached attestation
    Sign {
        /// Path to the interchange JSON bundle or manifest
        bundle: PathBuf,
        /// Path to the Ed25519 secret key file
        #[arg(long)]
        key: PathBuf,
        /// Output file path (default: <bundle>.signed.json)
        #[arg(long = "out", name = "out")]
        out: Option<PathBuf>,
        /// Attestation format identifier
        #[arg(long, default_value = "ed25519-detached")]
        format: String,
    },

    /// Verify a signed interchange bundle
    Verify {
        /// Path to the signed bundle JSON
        bundle: PathBuf,
        /// Path to the public key file (optional if signer_public_key in bundle)
        #[arg(long)]
        pubkey: Option<PathBuf>,
    },

    /// Sign a WASM evaluator binary with bundle binding
    SignWasm {
        /// Path to the WASM binary file
        wasm: PathBuf,
        /// Path to the Ed25519 secret key file
        #[arg(long)]
        key: PathBuf,
        /// Bundle etag to bind the WASM binary to
        #[arg(long)]
        bundle_etag: String,
    },

    /// Verify a signed WASM evaluator binary
    VerifyWasm {
        /// Path to the WASM binary file
        wasm: PathBuf,
        /// Path to the detached signature file
        #[arg(long)]
        sig: PathBuf,
        /// Path to the public key file
        #[arg(long)]
        pubkey: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum GenerateCommands {
    /// Generate TypeScript types and client bindings
    Typescript {
        /// Path to .tenor source file or interchange JSON bundle
        input: PathBuf,
        /// Output directory for generated files
        #[arg(long, default_value = "./generated")]
        out: PathBuf,
        /// SDK import path (default: @tenor/sdk)
        #[arg(long, default_value = "@tenor/sdk")]
        sdk_import: String,
    },
}

#[derive(Subcommand)]
enum BuilderCommands {
    /// Build the Tenor Builder SPA for production
    Build {
        /// Output directory for the production build
        #[arg(long = "out", default_value = "./builder-dist")]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Elaborate { file, manifest } => {
            commands::elaborate::cmd_elaborate(&file, manifest, cli.output, cli.quiet);
        }
        Commands::Validate { bundle } => {
            commands::validate::cmd_validate(&bundle, cli.output, cli.quiet);
        }
        Commands::Eval {
            bundle,
            facts,
            flow,
            persona,
        } => {
            commands::eval::cmd_eval(
                &bundle,
                &facts,
                flow.as_deref(),
                persona.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Test { suite_dir } => {
            commands::test::cmd_test(&suite_dir, cli.quiet);
        }
        Commands::Diff { t1, t2, breaking } => {
            commands::diff::cmd_diff(&t1, &t2, breaking, cli.output, cli.quiet);
        }
        Commands::Migrate { v1, v2, yes } => {
            migrate::cmd_migrate(&v1, &v2, yes, cli.output, cli.quiet);
        }
        Commands::Check { file, analysis } => {
            commands::check::cmd_check(&file, analysis.as_deref(), cli.output, cli.quiet);
        }
        Commands::Explain {
            file,
            format,
            verbose,
        } => {
            commands::explain::cmd_explain(&file, format, verbose, cli.output, cli.quiet);
        }
        Commands::Generate { command } => {
            commands::generate::cmd_generate(command, cli.output, cli.quiet);
        }
        Commands::Ambiguity {
            suite_dir,
            spec,
            model,
        } => {
            commands::ambiguity::cmd_ambiguity(&suite_dir, spec.as_deref(), model.as_deref());
        }
        Commands::Serve {
            port,
            contracts,
            tls_cert,
            tls_key,
        } => {
            // Validate TLS flags: both must be provided or neither
            if tls_cert.is_some() != tls_key.is_some() {
                eprintln!("error: --tls-cert and --tls-key must both be provided");
                process::exit(1);
            }
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            if let Err(e) = rt.block_on(serve::start_server(port, contracts, tls_cert, tls_key)) {
                eprintln!("Server error: {}", e);
                process::exit(1);
            }
        }
        Commands::Agent { file } => {
            agent::run_agent(&file);
        }
        Commands::Connect {
            contract,
            environment,
            out,
            dry_run,
            batch,
            apply,
            model,
            heuristic,
            verbose,
        } => {
            connect::cmd_connect(connect::ConnectOptions {
                contract: &contract,
                environment: environment.as_deref(),
                output_dir: &out,
                dry_run,
                batch,
                apply: apply.as_deref(),
                model: model.as_deref(),
                heuristic,
                verbose,
                output: cli.output,
                quiet: cli.quiet,
            });
        }
        Commands::Ui {
            contract,
            out_dir,
            api_url,
            contract_id,
            theme,
            title,
        } => {
            ui::cmd_ui(ui::UiOptions {
                contract: &contract,
                output_dir: &out_dir,
                api_url: &api_url,
                contract_id: contract_id.as_deref(),
                theme: theme.as_deref(),
                title: title.as_deref(),
                output: cli.output,
                quiet: cli.quiet,
            });
        }
        Commands::Builder {
            command,
            port,
            open,
            contract,
        } => match command {
            Some(BuilderCommands::Build { output }) => {
                builder::cmd_builder_build(builder::BuilderBuildOptions {
                    output_dir: &output,
                    quiet: cli.quiet,
                });
            }
            None => {
                builder::cmd_builder(builder::BuilderOptions {
                    port,
                    open,
                    contract: contract.as_deref(),
                    quiet: cli.quiet,
                });
            }
        },
        Commands::Pack { dir, out } => {
            template::cmd_pack(&dir, out.as_deref(), cli.quiet);
        }
        Commands::Publish {
            dir,
            registry,
            token,
        } => {
            template::publish::cmd_publish(
                &dir,
                registry.as_deref(),
                token.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Search {
            query,
            category,
            tag,
            registry,
        } => {
            template::search::cmd_search(
                &query,
                category.as_deref(),
                tag.as_deref(),
                registry.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Install {
            template_name,
            version,
            out,
            registry,
        } => {
            template::install::cmd_install(
                &template_name,
                version.as_deref(),
                &out,
                registry.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Deploy {
            template_name,
            org,
            version,
            config,
            registry,
            platform,
            token,
        } => {
            template::deploy::cmd_deploy(
                &template_name,
                org.as_deref(),
                version.as_deref(),
                config.as_deref(),
                registry.as_deref(),
                platform.as_deref(),
                token.as_deref(),
                cli.output,
                cli.quiet,
            );
        }
        Commands::Lsp => {
            if let Err(e) = tenor_lsp::run() {
                eprintln!("LSP server error: {}", e);
                process::exit(1);
            }
        }
        Commands::Keygen { algorithm, prefix } => {
            trust::keygen::cmd_keygen(&algorithm, &prefix);
        }
        Commands::Sign {
            bundle,
            key,
            out,
            format,
        } => {
            trust::sign::cmd_sign(&bundle, &key, out.as_deref(), &format);
        }
        Commands::Verify { bundle, pubkey } => {
            trust::verify::cmd_verify(&bundle, pubkey.as_deref());
        }
        Commands::SignWasm {
            wasm,
            key,
            bundle_etag,
        } => {
            trust::sign_wasm::cmd_sign_wasm(&wasm, &key, &bundle_etag);
        }
        Commands::VerifyWasm { wasm, sig, pubkey } => {
            trust::verify_wasm::cmd_verify_wasm(&wasm, &sig, &pubkey);
        }
    }
}

pub(crate) fn report_error(msg: &str, output: OutputFormat, quiet: bool) {
    if quiet {
        return;
    }
    match output {
        OutputFormat::Text => eprintln!("{}", msg),
        OutputFormat::Json => {
            eprintln!("{{\"error\": \"{}\"}}", msg.replace('"', "\\\""));
        }
    }
}
