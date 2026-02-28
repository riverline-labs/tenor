//! Template manifest format for `tenor-template.toml`.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Top-level wrapper matching the TOML `[template]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifestFile {
    pub template: TemplateManifest,
}

/// The `[template]` section of a tenor-template.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: TemplateMetadata,
    #[serde(default)]
    pub requirements: TemplateRequirements,
    #[serde(default)]
    pub configuration: TemplateConfiguration,
    #[serde(default)]
    pub screenshots: Vec<TemplateScreenshot>,
}

/// `[template.metadata]` — auto-populated or hand-written summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateMetadata {
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub personas: Vec<String>,
    #[serde(default)]
    pub facts_count: u32,
    #[serde(default)]
    pub flows_count: u32,
}

/// `[template.requirements]` — runtime requirements for the template.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateRequirements {
    #[serde(default)]
    pub tenor_version: Option<String>,
}

/// `[template.configuration]` — deployer-supplied configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateConfiguration {
    #[serde(default)]
    pub required_sources: Vec<String>,
}

/// `[[template.screenshots]]` — optional preview images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateScreenshot {
    pub path: String,
    pub caption: Option<String>,
}

impl TemplateManifest {
    /// Validate the manifest fields.
    ///
    /// Returns `Ok(())` if valid, or `Err(message)` with a human-readable
    /// description of the first validation failure.
    pub fn validate(&self) -> Result<(), String> {
        // name: non-empty, lowercase alphanumeric + hyphens only
        if self.name.is_empty() {
            return Err("template name cannot be empty".to_string());
        }
        for ch in self.name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                return Err(format!(
                    "template name '{}' contains invalid character '{}': only lowercase letters, digits, and hyphens are allowed",
                    self.name, ch
                ));
            }
        }

        // version: simple semver check (MAJOR.MINOR.PATCH with optional pre-release)
        validate_semver(&self.version)?;

        // description: non-empty
        if self.description.is_empty() {
            return Err("template description cannot be empty".to_string());
        }

        // author: non-empty
        if self.author.is_empty() {
            return Err("template author cannot be empty".to_string());
        }

        // category: non-empty
        if self.category.is_empty() {
            return Err("template category cannot be empty".to_string());
        }

        Ok(())
    }
}

/// Validate a version string as basic semver (e.g. "1.0.0", "1.2.3-beta.1").
/// Does not require the full semver spec — just MAJOR.MINOR.PATCH prefix.
fn validate_semver(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("template version cannot be empty".to_string());
    }

    // Strip optional pre-release / build metadata after first '-' or '+'
    let core = version.split(['-', '+']).next().unwrap_or(version);

    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "template version '{}' is not valid semver: expected MAJOR.MINOR.PATCH",
            version
        ));
    }

    for part in &parts {
        if part.is_empty() || part.parse::<u64>().is_err() {
            return Err(format!(
                "template version '{}' is not valid semver: '{}' is not a non-negative integer",
                version, part
            ));
        }
    }

    Ok(())
}

/// Read and parse `tenor-template.toml` from `dir`.
pub fn read_manifest(dir: &Path) -> Result<TemplateManifestFile, String> {
    let toml_path = dir.join("tenor-template.toml");

    let toml_str = std::fs::read_to_string(&toml_path)
        .map_err(|e| format!("could not read '{}': {}", toml_path.display(), e))?;

    let manifest: TemplateManifestFile = toml::from_str(&toml_str)
        .map_err(|e| format!("could not parse '{}': {}", toml_path.display(), e))?;

    Ok(manifest)
}

/// Return the default archive filename for a manifest.
pub fn archive_filename(manifest: &TemplateManifest) -> String {
    format!(
        "{}-{}.tenor-template.tar.gz",
        manifest.name, manifest.version
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest() {
        let m = TemplateManifest {
            name: "my-template".to_string(),
            version: "1.0.0".to_string(),
            description: "A test template".to_string(),
            author: "Test Author".to_string(),
            license: None,
            category: "finance".to_string(),
            tags: vec![],
            metadata: TemplateMetadata::default(),
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration::default(),
            screenshots: vec![],
        };
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_empty_name_rejected() {
        let m = TemplateManifest {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            author: "auth".to_string(),
            license: None,
            category: "cat".to_string(),
            tags: vec![],
            metadata: TemplateMetadata::default(),
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration::default(),
            screenshots: vec![],
        };
        let err = m.validate().unwrap_err();
        assert!(err.contains("name"), "error should mention name: {}", err);
    }

    #[test]
    fn test_invalid_name_chars_rejected() {
        let m = TemplateManifest {
            name: "My Template".to_string(), // uppercase + space
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            author: "auth".to_string(),
            license: None,
            category: "cat".to_string(),
            tags: vec![],
            metadata: TemplateMetadata::default(),
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration::default(),
            screenshots: vec![],
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_bad_version_rejected() {
        for bad in &["1.0", "v1.0.0", "1.0.0.0", "abc", ""] {
            let m = TemplateManifest {
                name: "ok".to_string(),
                version: bad.to_string(),
                description: "desc".to_string(),
                author: "auth".to_string(),
                license: None,
                category: "cat".to_string(),
                tags: vec![],
                metadata: TemplateMetadata::default(),
                requirements: TemplateRequirements::default(),
                configuration: TemplateConfiguration::default(),
                screenshots: vec![],
            };
            assert!(
                m.validate().is_err(),
                "expected error for version '{}'",
                bad
            );
        }
    }

    #[test]
    fn test_semver_with_prerelease() {
        // Pre-release versions should be accepted
        let m = TemplateManifest {
            name: "my-template".to_string(),
            version: "1.0.0-beta.1".to_string(),
            description: "desc".to_string(),
            author: "auth".to_string(),
            license: None,
            category: "cat".to_string(),
            tags: vec![],
            metadata: TemplateMetadata::default(),
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration::default(),
            screenshots: vec![],
        };
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_archive_filename() {
        let m = TemplateManifest {
            name: "escrow-release".to_string(),
            version: "1.0.0".to_string(),
            description: "d".to_string(),
            author: "a".to_string(),
            license: None,
            category: "c".to_string(),
            tags: vec![],
            metadata: TemplateMetadata::default(),
            requirements: TemplateRequirements::default(),
            configuration: TemplateConfiguration::default(),
            screenshots: vec![],
        };
        assert_eq!(
            archive_filename(&m),
            "escrow-release-1.0.0.tenor-template.tar.gz"
        );
    }
}
