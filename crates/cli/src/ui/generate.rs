use std::path::{Path, PathBuf};

use tenor_codegen::bundle::CodegenBundle;

use super::api_client;
use super::components;
use super::hooks;
use super::templates;
use super::theme;
use super::types_gen;

/// Configuration for UI project generation.
pub(super) struct UiConfig {
    pub output_dir: PathBuf,
    pub api_url: String,
    pub contract_id: String,
    pub title: String,
    pub custom_theme: Option<serde_json::Value>,
}

/// Generate a complete React UI project from a contract bundle.
///
/// Returns the list of all generated file paths on success.
pub(super) fn generate_ui_project(
    bundle: &CodegenBundle,
    config: &UiConfig,
) -> Result<Vec<PathBuf>, String> {
    let out = &config.output_dir;

    // Create directory structure
    let src_dir = out.join("src");
    let components_dir = src_dir.join("components");
    let hooks_dir = src_dir.join("hooks");
    let public_dir = out.join("public");

    for dir in &[out, &src_dir, &components_dir, &hooks_dir, &public_dir] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("failed to create directory '{}': {}", dir.display(), e))?;
    }

    // Build list of (path, content) pairs for static files
    let static_files: Vec<(PathBuf, String)> = vec![
        // Root files
        (
            out.join("package.json"),
            templates::package_json(&config.title),
        ),
        (out.join("tsconfig.json"), templates::tsconfig_json()),
        (out.join("vite.config.ts"), templates::vite_config()),
        // public/
        (
            public_dir.join("index.html"),
            templates::index_html(&config.title),
        ),
        // src/
        (src_dir.join("styles.css"), templates::global_css()),
        (src_dir.join("main.tsx"), templates::main_tsx()),
        (
            src_dir.join("App.tsx"),
            templates::app_tsx(&config.contract_id, &config.title),
        ),
        (
            src_dir.join("api.ts"),
            api_client::emit_api_client(&config.api_url, &config.contract_id),
        ),
        (src_dir.join("types.ts"), types_gen::emit_ui_types(bundle)),
        (
            src_dir.join("theme.ts"),
            theme::emit_theme(&config.contract_id, config.custom_theme.as_ref()),
        ),
        // src/components/ — Layout gets a full implementation
        (
            components_dir.join("Layout.tsx"),
            templates::layout_tsx(&config.title),
        ),
    ];

    // Write static files and collect paths
    let mut files: Vec<PathBuf> = static_files
        .into_iter()
        .map(|(path, content)| write_file(&path, &content))
        .collect::<Result<Vec<_>, _>>()?;

    // Contract-driven components (generated from bundle)
    let component_files: Vec<(PathBuf, String)> = vec![
        (
            components_dir.join("Dashboard.tsx"),
            components::emit_dashboard(bundle),
        ),
        (
            components_dir.join("EntityList.tsx"),
            components::emit_entity_list(bundle),
        ),
        (
            components_dir.join("EntityDetail.tsx"),
            components::emit_entity_detail(bundle),
        ),
        (
            components_dir.join("InstanceDetail.tsx"),
            components::emit_instance_detail(bundle),
        ),
        (
            components_dir.join("ActionSpace.tsx"),
            components::emit_action_space(bundle),
        ),
        (
            components_dir.join("BlockedActions.tsx"),
            components::emit_blocked_actions(),
        ),
        (
            components_dir.join("FactInput.tsx"),
            components::emit_fact_input(bundle),
        ),
        (
            components_dir.join("FlowExecution.tsx"),
            components::emit_flow_execution(),
        ),
        (
            components_dir.join("FlowHistory.tsx"),
            components::emit_flow_history(),
        ),
        (
            components_dir.join("ProvenanceDrill.tsx"),
            components::emit_provenance_drill(),
        ),
        (
            components_dir.join("VerdictDisplay.tsx"),
            components::emit_verdict_display(),
        ),
    ];
    for (path, content) in component_files {
        files.push(write_file(&path, &content)?);
    }

    // Contract-driven hooks (generated)
    let hook_files: Vec<(PathBuf, String)> = vec![
        (
            hooks_dir.join("useActionSpace.ts"),
            hooks::emit_use_action_space(),
        ),
        (hooks_dir.join("useEntities.ts"), hooks::emit_use_entities()),
        (
            hooks_dir.join("useExecution.ts"),
            hooks::emit_use_execution(),
        ),
    ];
    for (path, content) in hook_files {
        files.push(write_file(&path, &content)?);
    }

    Ok(files)
}

/// Write content to a file, returning the path on success.
fn write_file(path: &Path, content: &str) -> Result<PathBuf, String> {
    std::fs::write(path, content)
        .map_err(|e| format!("failed to write '{}': {}", path.display(), e))?;
    Ok(path.to_path_buf())
}
