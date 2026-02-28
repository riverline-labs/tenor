//! Implementation of the `tenor deploy` subcommand.
//!
//! Orchestrates the full deploy workflow:
//!   1. Resolve auth token
//!   2. Download template archive from registry
//!   3. Extract interchange bundle from archive
//!   4. Load and validate the deploy config TOML
//!   5. POST to the platform's deployment API
//!   6. Report the live endpoint URL

use std::path::Path;

use crate::OutputFormat;

use super::{
    deploy_config::{self, DeployConfig},
    pack, registry,
};

/// Default platform API URL used when `--platform` is not provided.
pub const DEFAULT_PLATFORM_URL: &str = "https://api.tenor.dev";

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct DeployBundleResponse {
    contract_id: String,
    endpoint_url: String,
}

// ── cmd_deploy ─────────────────────────────────────────────────────────────────

/// Run the `tenor deploy` subcommand.
///
/// # Arguments
///
/// * `template_name` — template name to deploy (looked up in the registry)
/// * `org_id` — organization ID override (also settable in deploy config)
/// * `version` — specific version to deploy; `None` means "latest"
/// * `config_path` — path to the deploy config TOML; `None` triggers config template generation
/// * `registry_url` — registry endpoint override (`TENOR_REGISTRY_URL` env fallback)
/// * `platform_url` — platform API endpoint override (`TENOR_PLATFORM_URL` env fallback)
/// * `token` — platform auth token (`TENOR_PLATFORM_TOKEN` env fallback)
/// * `output` — output format (text / JSON)
/// * `quiet` — suppress non-essential output
#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy(
    template_name: &str,
    org_id: Option<&str>,
    version: Option<&str>,
    config_path: Option<&Path>,
    registry_url: Option<&str>,
    platform_url: Option<&str>,
    token: Option<&str>,
    output: OutputFormat,
    quiet: bool,
) {
    // ── Step 1: Resolve auth token ────────────────────────────────────────────

    let token = resolve_token(token);
    let token = match token {
        Some(t) => t,
        None => {
            eprintln!("error: platform auth token required — provide --token or set TENOR_PLATFORM_TOKEN");
            std::process::exit(1);
        }
    };

    // ── Step 2: Resolve URLs ──────────────────────────────────────────────────

    let registry_url = registry_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("TENOR_REGISTRY_URL").ok())
        .unwrap_or_else(|| registry::DEFAULT_REGISTRY_URL.to_string());

    let platform_url = platform_url
        .map(|s| s.trim_end_matches('/').to_string())
        .or_else(|| {
            std::env::var("TENOR_PLATFORM_URL")
                .ok()
                .map(|s| s.trim_end_matches('/').to_string())
        })
        .unwrap_or_else(|| DEFAULT_PLATFORM_URL.to_string());

    // ── Step 3: Download template from registry ───────────────────────────────

    let client = registry::RegistryClient::new(Some(&registry_url), None);

    if !quiet {
        if let OutputFormat::Text = output {
            let version_display = version.unwrap_or("latest");
            println!(
                "Downloading {}@{} from registry...",
                template_name, version_display
            );
        }
    }

    let archive_bytes = match client.download(template_name, version) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // ── Step 4: Unpack to temp directory ─────────────────────────────────────

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
        eprintln!("error: could not write archive to temp file: {}", e);
        std::process::exit(1);
    }

    let unpack_dir = tmp_dir.path().join("unpacked");
    let manifest_file = match pack::unpack_template(&tmp_archive, &unpack_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not unpack template: {}", e);
            std::process::exit(1);
        }
    };

    let manifest = &manifest_file.template;
    let resolved_version = &manifest.version;

    // Read the interchange bundle from the unpacked archive.
    let bundle_path = unpack_dir.join("bundle.json");
    let bundle_json = match std::fs::read_to_string(&bundle_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "error: could not read bundle.json from archive: {}",
                e
            );
            std::process::exit(1);
        }
    };

    let bundle: serde_json::Value = match serde_json::from_str(&bundle_json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: invalid bundle JSON in archive: {}", e);
            std::process::exit(1);
        }
    };

    // ── Step 5: Load deploy config ────────────────────────────────────────────

    let deploy_config: DeployConfig = match config_path {
        Some(path) => {
            // Load from the provided file.
            match deploy_config::read_deploy_config(path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // No config provided: generate a template and tell the user.
            let needs_config = !manifest.configuration.required_sources.is_empty()
                || !manifest.metadata.personas.is_empty();

            if needs_config {
                let template_toml =
                    deploy_config::generate_deploy_config_template(manifest);

                // Write the config template to deploy-config.toml.
                let out_path = std::path::Path::new("deploy-config.toml");
                if let Err(e) = std::fs::write(out_path, &template_toml) {
                    eprintln!("error: could not write deploy-config.toml: {}", e);
                    std::process::exit(1);
                }

                println!(
                    "Deploy config required. Generated template at deploy-config.toml — fill in values and re-run with --config deploy-config.toml"
                );
                std::process::exit(0);
            }

            // Template has no required sources or personas — use a minimal config.
            let resolved_org_id = org_id.unwrap_or("").to_string();
            DeployConfig {
                deploy: super::deploy_config::DeploySettings {
                    org_id: resolved_org_id,
                },
                sources: std::collections::BTreeMap::new(),
                personas: std::collections::BTreeMap::new(),
            }
        }
    };

    // Apply --org override if provided.
    let effective_org_id = org_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| deploy_config.deploy.org_id.clone());

    if effective_org_id.is_empty() {
        eprintln!(
            "error: org_id is required — provide --org or set org_id in your deploy config"
        );
        std::process::exit(1);
    }

    // ── Step 6: Validate deploy config ────────────────────────────────────────

    if let Err(errors) = deploy_config::validate_deploy_config(&deploy_config, manifest) {
        eprintln!("error: deploy config validation failed:");
        for err in &errors {
            eprintln!("  - {}", err);
        }
        std::process::exit(1);
    }

    // ── Step 7: POST to platform deployment API ───────────────────────────────

    if !quiet {
        if let OutputFormat::Text = output {
            println!(
                "Deploying {}@{} to {}...",
                template_name, resolved_version, platform_url
            );
        }
    }

    // Build the adapter config from deploy config sources.
    let adapter_config: serde_json::Value = serde_json::json!(
        deploy_config.sources.iter().map(|(id, cfg)| {
            (id.clone(), serde_json::json!({
                "protocol": cfg.protocol,
                "base_url": cfg.base_url,
                "auth_header": cfg.auth_header,
                "auth_value": cfg.auth_value,
                "connection_string": cfg.connection_string,
                "query": cfg.query,
            }))
        }).collect::<serde_json::Map<_, _>>()
    );

    // Build persona mappings from deploy config.
    let persona_mappings: serde_json::Value = serde_json::json!(
        deploy_config.personas.iter().map(|(id, cfg)| {
            (id.clone(), serde_json::json!({ "api_key": cfg.api_key }))
        }).collect::<serde_json::Map<_, _>>()
    );

    let request_body = serde_json::json!({
        "bundle": bundle,
        "org_id": effective_org_id,
        "adapter_config": adapter_config,
        "persona_mappings": persona_mappings,
    });

    let deploy_url = format!("{}/api/v1/contracts/deploy-bundle", platform_url);

    let agent = ureq::Agent::new_with_defaults();

    let response = match agent
        .post(&deploy_url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send_json(&request_body)
    {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Connection refused")
                || msg.contains("connection refused")
                || msg.contains("os error 61")
                || msg.contains("os error 111")
            {
                eprintln!(
                    "error: could not reach platform at {}",
                    platform_url
                );
            } else if msg.contains("401") || msg.contains("Unauthorized") {
                eprintln!("error: platform returned 401 Unauthorized — check your token");
            } else if msg.contains("403") || msg.contains("Forbidden") {
                eprintln!("error: platform returned 403 Forbidden — check permissions");
            } else {
                eprintln!("error: deployment failed: {}", msg);
            }
            std::process::exit(1);
        }
    };

    let deploy_response: DeployBundleResponse = match response.into_body().read_json() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: could not parse deployment response: {}", e);
            std::process::exit(1);
        }
    };

    // ── Step 8: Report success ────────────────────────────────────────────────

    if !quiet {
        match output {
            OutputFormat::Text => {
                println!(
                    "Deployed {}@{} to {}",
                    template_name, resolved_version, platform_url
                );
                println!("  Endpoint:    {}", deploy_response.endpoint_url);
                println!("  Contract ID: {}", deploy_response.contract_id);
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "contract_id": deploy_response.contract_id,
                    "endpoint_url": deploy_response.endpoint_url,
                    "template": template_name,
                    "version": resolved_version,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                );
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve the platform auth token from the argument or environment variable.
fn resolve_token(arg: Option<&str>) -> Option<String> {
    arg.map(|s| s.to_string())
        .or_else(|| std::env::var("TENOR_PLATFORM_TOKEN").ok())
}
