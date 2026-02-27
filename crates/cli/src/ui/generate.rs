use std::path::{Path, PathBuf};

use tenor_codegen::bundle::CodegenBundle;

use super::api_client;
use super::components;
use super::hooks;
use super::templates;
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
            emit_theme(&config.contract_id, config.custom_theme.as_ref()),
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

/// Generate theme.ts with contract-derived color palette.
fn emit_theme(contract_id: &str, custom_theme: Option<&serde_json::Value>) -> String {
    // If a custom theme was provided, serialize it directly
    if let Some(theme) = custom_theme {
        let theme_str = serde_json::to_string_pretty(theme).unwrap_or_default();
        return format!(
            "// Auto-generated by tenor ui (custom theme).\nexport const theme = {} as const;\n",
            theme_str
        );
    }

    // Derive primary hue from contract_id hash
    let hue = contract_id_to_hue(contract_id);
    let primary = hsl_to_hex(hue, 0.55, 0.45);
    let primary_light = hsl_to_hex(hue, 0.55, 0.93);
    let secondary = hsl_to_hex((hue + 30) % 360, 0.40, 0.50);
    let accent = hsl_to_hex((hue + 180) % 360, 0.55, 0.45);

    format!(
        r#"// Auto-generated by tenor ui. Do not edit.
// Theme derived from contract: {contract_id}

export const theme = {{
  colors: {{
    primary: '{primary}',
    primaryLight: '{primary_light}',
    secondary: '{secondary}',
    accent: '{accent}',
    background: '#f8f9fa',
    surface: '#ffffff',
    sidebar: '#f1f3f5',
    border: '#dee2e6',
    text: '#212529',
    textMuted: '#6c757d',
    success: '#16a34a',
    warning: '#d97706',
    error: '#dc2626',
    info: '#2563eb',
  }},
  fonts: {{
    body: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif",
    mono: "'Fira Code', 'Cascadia Code', 'JetBrains Mono', Consolas, monospace",
  }},
  spacing: {{
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
  }},
  borderRadius: {{
    sm: '4px',
    md: '8px',
    lg: '12px',
  }},
}} as const;

export type Theme = typeof theme;
"#,
        contract_id = contract_id,
        primary = primary,
        primary_light = primary_light,
        secondary = secondary,
        accent = accent,
    )
}

/// Hash the contract_id to a hue value 0-359.
fn contract_id_to_hue(contract_id: &str) -> u32 {
    let mut hash: u32 = 5381;
    for byte in contract_id.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash % 360
}

/// Convert HSL to a hex color string.
fn hsl_to_hex(h: u32, s: f64, l: f64) -> String {
    let h = h as f64 / 360.0;
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    let ri = (r * 255.0).round() as u8;
    let gi = (g * 255.0).round() as u8;
    let bi = (b * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", ri, gi, bi)
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}
