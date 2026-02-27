//! Theme generation for the tenor UI project.
//!
//! Derives a color palette from the contract ID using a deterministic hash,
//! and emits a customizable `theme.ts` file.

/// A derived color palette for a generated UI project.
struct ThemePalette {
    primary: String,
    primary_light: String,
    primary_dark: String,
    secondary: String,
    accent: String,
}

/// Hash the contract_id to a hue value 0–359 using djb2.
fn contract_hue(contract_id: &str) -> u16 {
    let mut hash: u32 = 5381;
    for byte in contract_id.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u32::from(byte));
    }
    (hash % 360) as u16
}

/// Convert HSL (hue 0–360, saturation 0.0–1.0, lightness 0.0–1.0) to a hex color string.
fn hsl_to_hex(h: u16, s: f64, l: f64) -> String {
    let hf = f64::from(h) / 360.0;
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, hf + 1.0 / 3.0),
            hue_to_rgb(p, q, hf),
            hue_to_rgb(p, q, hf - 1.0 / 3.0),
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

/// Derive the full color palette from a hue value.
fn derive_palette(hue: u16) -> ThemePalette {
    ThemePalette {
        primary: hsl_to_hex(hue, 0.65, 0.50),
        primary_light: hsl_to_hex(hue, 0.65, 0.95),
        primary_dark: hsl_to_hex(hue, 0.65, 0.35),
        secondary: hsl_to_hex((u32::from(hue) + 30) as u16 % 360, 0.25, 0.45),
        accent: hsl_to_hex((u32::from(hue) + 180) as u16 % 360, 0.55, 0.55),
    }
}

/// Generate `theme.ts` content for the given contract.
///
/// If `custom_theme` is provided, its top-level keys are merged over the
/// defaults (custom values win; missing keys use the contract-derived defaults).
pub(super) fn emit_theme(contract_id: &str, custom_theme: Option<&serde_json::Value>) -> String {
    let hue = contract_hue(contract_id);
    let palette = derive_palette(hue);

    // Resolve per-color overrides from the custom theme, if any.
    let colors = custom_theme
        .and_then(|v| v.get("colors"))
        .and_then(|v| v.as_object());

    let color = |key: &str, default: &str| -> String {
        colors
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    };

    let primary = color("primary", &palette.primary);
    let primary_light = color("primaryLight", &palette.primary_light);
    let primary_dark = color("primaryDark", &palette.primary_dark);
    let secondary = color("secondary", &palette.secondary);
    let accent = color("accent", &palette.accent);
    let background = color("background", "#f8fafc");
    let surface = color("surface", "#ffffff");
    let text_primary = color("textPrimary", "#0f172a");
    let text_secondary = color("textSecondary", "#64748b");
    let border = color("border", "#e2e8f0");
    let success = color("success", "#16a34a");
    let warning = color("warning", "#d97706");
    let error = color("error", "#dc2626");
    let info = color("info", "#2563eb");

    format!(
        r#"// Theme configuration for {contract_id}
// Edit this file to customize the look and feel of your application.

export const theme = {{
  colors: {{
    primary: '{primary}',
    primaryLight: '{primary_light}',
    primaryDark: '{primary_dark}',
    secondary: '{secondary}',
    accent: '{accent}',
    background: '{background}',
    surface: '{surface}',
    textPrimary: '{text_primary}',
    textSecondary: '{text_secondary}',
    border: '{border}',
    success: '{success}',
    warning: '{warning}',
    error: '{error}',
    info: '{info}',
  }},
  fonts: {{
    body: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
    heading: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
    mono: "ui-monospace, 'Cascadia Code', 'Fira Code', monospace",
  }},
  spacing: {{
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
    xxl: '48px',
  }},
  borderRadius: {{
    sm: '4px',
    md: '8px',
    lg: '12px',
    full: '9999px',
  }},
  shadows: {{
    sm: '0 1px 2px rgba(0,0,0,0.05)',
    md: '0 4px 6px -1px rgba(0,0,0,0.1)',
    lg: '0 10px 15px -3px rgba(0,0,0,0.1)',
  }},
  breakpoints: {{
    tablet: '768px',
    desktop: '1200px',
  }},
}} as const;

export type Theme = typeof theme;
"#,
        contract_id = contract_id,
        primary = primary,
        primary_light = primary_light,
        primary_dark = primary_dark,
        secondary = secondary,
        accent = accent,
        background = background,
        surface = surface,
        text_primary = text_primary,
        text_secondary = text_secondary,
        border = border,
        success = success,
        warning = warning,
        error = error,
        info = info,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_contract_ids_produce_different_hues() {
        let hue1 = contract_hue("integration_escrow");
        let hue2 = contract_hue("loan_approval");
        let hue3 = contract_hue("insurance_claim");
        // All should be valid hues
        assert!(hue1 < 360);
        assert!(hue2 < 360);
        assert!(hue3 < 360);
        // Different contracts should produce different hues
        assert_ne!(hue1, hue2);
        assert_ne!(hue1, hue3);
    }

    #[test]
    fn hsl_to_hex_produces_valid_hex_strings() {
        let result = hsl_to_hex(210, 0.65, 0.50);
        assert_eq!(result.len(), 7, "hex string must be 7 chars");
        assert!(result.starts_with('#'), "hex string must start with #");
        // All chars after # must be valid hex
        assert!(
            result[1..].chars().all(|c| c.is_ascii_hexdigit()),
            "hex string must contain only hex digits after #"
        );
    }

    #[test]
    fn hsl_to_hex_black_and_white() {
        assert_eq!(hsl_to_hex(0, 0.0, 0.0), "#000000");
        assert_eq!(hsl_to_hex(0, 0.0, 1.0), "#ffffff");
    }

    #[test]
    fn emit_theme_contains_all_required_keys() {
        let output = emit_theme("test_contract", None);
        // Colors
        assert!(output.contains("primary:"), "must have primary");
        assert!(output.contains("primaryLight:"), "must have primaryLight");
        assert!(output.contains("primaryDark:"), "must have primaryDark");
        assert!(output.contains("secondary:"), "must have secondary");
        assert!(output.contains("accent:"), "must have accent");
        assert!(output.contains("background:"), "must have background");
        assert!(output.contains("surface:"), "must have surface");
        assert!(output.contains("textPrimary:"), "must have textPrimary");
        assert!(output.contains("textSecondary:"), "must have textSecondary");
        assert!(output.contains("border:"), "must have border");
        assert!(output.contains("success: '#16a34a'"), "fixed success color");
        assert!(output.contains("warning: '#d97706'"), "fixed warning color");
        assert!(output.contains("error: '#dc2626'"), "fixed error color");
        assert!(output.contains("info: '#2563eb'"), "fixed info color");
        // Fonts
        assert!(output.contains("fonts:"), "must have fonts");
        assert!(output.contains("heading:"), "must have heading font");
        assert!(output.contains("mono:"), "must have mono font");
        // Spacing
        assert!(output.contains("xxl:"), "must have xxl spacing");
        // Border radius
        assert!(output.contains("full:"), "must have full border radius");
        // Shadows
        assert!(output.contains("shadows:"), "must have shadows");
        // Breakpoints
        assert!(output.contains("breakpoints:"), "must have breakpoints");
        assert!(output.contains("tablet:"), "must have tablet breakpoint");
        assert!(output.contains("desktop:"), "must have desktop breakpoint");
        // TypeScript export
        assert!(output.contains("export type Theme"), "must export Theme type");
    }

    #[test]
    fn emit_theme_custom_override_applies() {
        let custom = serde_json::json!({
            "colors": {
                "primary": "#ff0000"
            }
        });
        let output = emit_theme("my_contract", Some(&custom));
        assert!(
            output.contains("primary: '#ff0000'"),
            "custom primary color must appear in output"
        );
        // Fixed colors must still be present
        assert!(
            output.contains("success: '#16a34a'"),
            "fixed success color must remain"
        );
    }

    #[test]
    fn emit_theme_contract_id_in_comment() {
        let output = emit_theme("my_test_contract", None);
        assert!(
            output.contains("my_test_contract"),
            "contract_id must appear in theme comment"
        );
    }
}
