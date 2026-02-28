//! Deployment configuration format for `tenor deploy`.
//!
//! The deploy config TOML maps template requirements (required sources, personas)
//! to concrete connection details and API keys needed to provision the contract
//! on the hosted platform.
//!
//! # Example
//!
//! ```toml
//! [deploy]
//! org_id = "org-uuid-here"
//!
//! [sources.payment_service]
//! protocol = "rest"
//! base_url = "https://api.payments.example.com"
//! auth_header = "Authorization"
//! auth_value = "Bearer sk_live_..."
//!
//! [sources.delivery_service]
//! protocol = "rest"
//! base_url = "https://api.delivery.example.com"
//! auth_header = "X-API-Key"
//! auth_value = "dk_..."
//!
//! [personas.buyer]
//! api_key = "buyer-api-key-uuid"
//!
//! [personas.seller]
//! api_key = "seller-api-key-uuid"
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::manifest::TemplateManifest;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Top-level deploy configuration.
///
/// Loaded from a TOML file passed via `tenor deploy --config deploy-config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    pub deploy: DeploySettings,
    /// Source adapter configurations, keyed by source ID.
    #[serde(default)]
    pub sources: BTreeMap<String, SourceConfig>,
    /// Persona API key mappings, keyed by persona ID.
    #[serde(default)]
    pub personas: BTreeMap<String, PersonaConfig>,
}

/// `[deploy]` section — global deployment settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploySettings {
    /// Organization ID on the hosted platform.
    pub org_id: String,
}

/// Configuration for a single source adapter.
///
/// REST sources use `protocol = "rest"` with `base_url` / `auth_header` / `auth_value`.
/// Database sources use `protocol = "database"` with `connection_string` / `query`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Adapter protocol: `"rest"` or `"database"`.
    pub protocol: String,
    /// Base URL for REST sources.
    pub base_url: Option<String>,
    /// HTTP header name used for authentication (REST sources).
    pub auth_header: Option<String>,
    /// HTTP header value used for authentication (REST sources).
    pub auth_value: Option<String>,
    /// Database connection string (database sources).
    pub connection_string: Option<String>,
    /// SQL query template (database sources).
    pub query: Option<String>,
}

/// Configuration for a single persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaConfig {
    /// API key assigned to this persona on the hosted platform.
    pub api_key: String,
}

// ── Functions ─────────────────────────────────────────────────────────────────

/// Read and parse a deploy config TOML file from `path`.
///
/// Returns a human-readable error string on failure.
pub fn read_deploy_config(path: &Path) -> Result<DeployConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read '{}': {}", path.display(), e))?;

    toml::from_str(&content).map_err(|e| format!("could not parse '{}': {}", path.display(), e))
}

/// Validate that a `DeployConfig` satisfies all requirements declared in `manifest`.
///
/// Checks:
/// - Every `required_sources` entry has a corresponding `[sources.*]` config.
/// - Every persona in `template.metadata.personas` has a `[personas.*]` mapping.
///
/// Returns `Ok(())` if valid, or `Err(Vec<String>)` listing all missing entries.
pub fn validate_deploy_config(
    config: &DeployConfig,
    manifest: &TemplateManifest,
) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    // Check required sources.
    for source_id in &manifest.configuration.required_sources {
        if !config.sources.contains_key(source_id.as_str()) {
            errors.push(format!(
                "missing source configuration for '{}' — add a [sources.{}] section",
                source_id, source_id
            ));
        }
    }

    // Check persona mappings.
    for persona_id in &manifest.metadata.personas {
        if !config.personas.contains_key(persona_id.as_str()) {
            errors.push(format!(
                "missing persona mapping for '{}' — add a [personas.{}] section",
                persona_id, persona_id
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Generate a skeleton deploy config TOML for the user to fill in.
///
/// Produces placeholder sections for each required source and persona declared
/// in the manifest. The output TOML is ready to save to `deploy-config.toml`.
pub fn generate_deploy_config_template(manifest: &TemplateManifest) -> String {
    let mut out = String::new();

    // [deploy] section
    out.push_str("[deploy]\n");
    out.push_str("org_id = \"<your-org-id>\"  # Find this in your Tenor platform dashboard\n");
    out.push('\n');

    // [sources.*] sections
    if manifest.configuration.required_sources.is_empty() {
        out.push_str("# This template has no required sources.\n");
        out.push('\n');
    } else {
        out.push_str("# Source adapter configuration\n");
        out.push_str("# Fill in connection details for each required source.\n");
        out.push('\n');
        for source_id in &manifest.configuration.required_sources {
            out.push_str(&format!("[sources.{}]\n", source_id));
            out.push_str("protocol = \"rest\"  # or \"database\"\n");
            out.push_str("# REST source fields:\n");
            out.push_str("base_url = \"https://api.example.com\"\n");
            out.push_str("auth_header = \"Authorization\"\n");
            out.push_str("auth_value = \"Bearer <token>\"\n");
            out.push_str(
                "# Database source fields (comment out REST fields and uncomment these):\n",
            );
            out.push_str("# connection_string = \"postgres://user:pass@host/db\"\n");
            out.push_str("# query = \"SELECT * FROM ...\"\n");
            out.push('\n');
        }
    }

    // [personas.*] sections
    if manifest.metadata.personas.is_empty() {
        out.push_str("# This template has no personas to configure.\n");
        out.push('\n');
    } else {
        out.push_str("# Persona API key mappings\n");
        out.push_str("# Each persona needs an API key from your Tenor platform dashboard.\n");
        out.push('\n');
        for persona_id in &manifest.metadata.personas {
            out.push_str(&format!("[personas.{}]\n", persona_id));
            out.push_str(&format!(
                "api_key = \"<api-key-for-{}>\"  # Create this key in your dashboard\n",
                persona_id
            ));
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::manifest::{
        TemplateConfiguration, TemplateMetadata, TemplateRequirements,
    };

    fn make_manifest(sources: Vec<&str>, personas: Vec<&str>) -> TemplateManifest {
        TemplateManifest {
            name: "test-template".to_string(),
            version: "1.0.0".to_string(),
            description: "A test template".to_string(),
            author: "Tester".to_string(),
            license: None,
            category: "finance".to_string(),
            tags: vec![],
            metadata: TemplateMetadata {
                entities: vec![],
                personas: personas.into_iter().map(|s| s.to_string()).collect(),
                facts_count: 0,
                flows_count: 0,
            },
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration {
                required_sources: sources.into_iter().map(|s| s.to_string()).collect(),
            },
            screenshots: vec![],
        }
    }

    #[test]
    fn test_validate_missing_source() {
        let manifest = make_manifest(vec!["payment_service", "delivery_service"], vec![]);
        let config = DeployConfig {
            deploy: DeploySettings {
                org_id: "org-123".to_string(),
            },
            sources: BTreeMap::from([(
                "payment_service".to_string(),
                SourceConfig {
                    protocol: "rest".to_string(),
                    base_url: Some("https://api.payments.example.com".to_string()),
                    auth_header: None,
                    auth_value: None,
                    connection_string: None,
                    query: None,
                },
            )]),
            personas: BTreeMap::new(),
        };

        let errors = validate_deploy_config(&config, &manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("delivery_service"));
    }

    #[test]
    fn test_validate_missing_persona() {
        let manifest = make_manifest(vec![], vec!["buyer", "seller"]);
        let config = DeployConfig {
            deploy: DeploySettings {
                org_id: "org-123".to_string(),
            },
            sources: BTreeMap::new(),
            personas: BTreeMap::from([(
                "buyer".to_string(),
                PersonaConfig {
                    api_key: "key-1".to_string(),
                },
            )]),
        };

        let errors = validate_deploy_config(&config, &manifest).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("seller"));
    }

    #[test]
    fn test_validate_complete_config() {
        let manifest = make_manifest(vec!["api"], vec!["admin"]);
        let config = DeployConfig {
            deploy: DeploySettings {
                org_id: "org-123".to_string(),
            },
            sources: BTreeMap::from([(
                "api".to_string(),
                SourceConfig {
                    protocol: "rest".to_string(),
                    base_url: Some("https://api.example.com".to_string()),
                    auth_header: None,
                    auth_value: None,
                    connection_string: None,
                    query: None,
                },
            )]),
            personas: BTreeMap::from([(
                "admin".to_string(),
                PersonaConfig {
                    api_key: "admin-key".to_string(),
                },
            )]),
        };

        assert!(validate_deploy_config(&config, &manifest).is_ok());
    }

    #[test]
    fn test_generate_template_with_sources_and_personas() {
        let manifest = make_manifest(vec!["payment_service"], vec!["buyer"]);
        let tmpl = generate_deploy_config_template(&manifest);

        assert!(tmpl.contains("[deploy]"));
        assert!(tmpl.contains("[sources.payment_service]"));
        assert!(tmpl.contains("[personas.buyer]"));
        assert!(tmpl.contains("org_id"));
        assert!(tmpl.contains("api_key"));
    }

    #[test]
    fn test_generate_template_no_sources_no_personas() {
        let manifest = make_manifest(vec![], vec![]);
        let tmpl = generate_deploy_config_template(&manifest);

        assert!(tmpl.contains("[deploy]"));
        assert!(tmpl.contains("no required sources"));
        assert!(tmpl.contains("no personas"));
    }

    #[test]
    fn test_read_deploy_config_roundtrip() {
        use std::io::Write;
        let toml_content = r#"
[deploy]
org_id = "org-abc-123"

[sources.payment]
protocol = "rest"
base_url = "https://payments.example.com"
auth_header = "Authorization"
auth_value = "Bearer sk_live"

[personas.buyer]
api_key = "buyer-key-uuid"
"#;
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        tmp.as_file().write_all(toml_content.as_bytes()).unwrap();

        let config = read_deploy_config(tmp.path()).expect("parse config");
        assert_eq!(config.deploy.org_id, "org-abc-123");
        assert!(config.sources.contains_key("payment"));
        assert_eq!(
            config.sources["payment"].base_url.as_deref(),
            Some("https://payments.example.com")
        );
        assert!(config.personas.contains_key("buyer"));
        assert_eq!(config.personas["buyer"].api_key, "buyer-key-uuid");
    }
}
