//! HTTP client for the Tenor marketplace registry API.
//!
//! Provides [`RegistryClient`] which encapsulates all HTTP interactions with
//! the registry. Commands (`publish`, `search`, `install`) delegate to this
//! client rather than constructing HTTP requests themselves.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::manifest::TemplateManifest;

/// Default registry endpoint used when no `--registry` flag is supplied.
pub const DEFAULT_REGISTRY_URL: &str = "https://registry.tenor.dev";

// ─── Response types ───────────────────────────────────────────────────────────

/// Response returned by a successful `publish` call.
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishResponse {
    pub name: String,
    pub version: String,
    pub status: String,
}

/// A single search result entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub downloads: u64,
    pub rating: Option<f32>,
}

/// Full template metadata returned by `get_template`.
// Used by `tenor install --info` (planned future subcommand).
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub latest_version: String,
    pub versions: Vec<String>,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
}

// ─── RegistryClient ───────────────────────────────────────────────────────────

/// HTTP client for the Tenor registry API.
pub struct RegistryClient {
    base_url: String,
    token: Option<String>,
}

impl RegistryClient {
    /// Create a new client.
    ///
    /// Uses `registry_url` if provided, otherwise falls back to
    /// [`DEFAULT_REGISTRY_URL`]. `token` is required only for `publish`.
    pub fn new(registry_url: Option<&str>, token: Option<&str>) -> Self {
        let base_url = registry_url
            .unwrap_or(DEFAULT_REGISTRY_URL)
            .trim_end_matches('/')
            .to_string();

        Self {
            base_url,
            token: token.map(|t| t.to_string()),
        }
    }

    /// Publish a template archive to the registry.
    ///
    /// POST `/api/v1/registry/templates`
    ///
    /// The archive and the manifest JSON are uploaded as a multipart form.
    /// An auth token is required.
    pub fn publish(
        &self,
        archive_path: &Path,
        manifest: &TemplateManifest,
    ) -> Result<PublishResponse, String> {
        let token = self
            .token
            .as_deref()
            .ok_or_else(|| "error: --token or TENOR_REGISTRY_TOKEN required for publishing".to_string())?;

        let archive_bytes = std::fs::read(archive_path)
            .map_err(|e| format!("could not read archive '{}': {}", archive_path.display(), e))?;

        let manifest_json = serde_json::to_string(manifest)
            .map_err(|e| format!("could not serialize manifest: {}", e))?;

        let url = format!("{}/api/v1/registry/templates", self.base_url);

        let agent = ureq::Agent::new_with_defaults();

        // Build a simple multipart body using a custom boundary.
        // ureq v3 does not bundle multipart support, so we construct the body manually.
        let boundary = "tenor-boundary-a1b2c3d4";
        let mut body: Vec<u8> = Vec::new();

        // Part 1: archive file
        let archive_filename = archive_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "archive.tar.gz".to_string());

        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"archive\"; filename=\"{archive_filename}\"\r\nContent-Type: application/gzip\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&archive_bytes);
        body.extend_from_slice(b"\r\n");

        // Part 2: manifest JSON
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"manifest\"\r\nContent-Type: application/json\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(manifest_json.as_bytes());
        body.extend_from_slice(b"\r\n");

        // Closing boundary
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let content_type = format!("multipart/form-data; boundary={boundary}");

        let response = agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .header("Content-Type", &content_type)
            .send(&body)
            .map_err(|e| classify_http_error(e, "publish"))?;

        response
            .into_body()
            .read_json::<PublishResponse>()
            .map_err(|e| format!("could not parse publish response: {}", e))
    }

    /// Search for templates in the registry.
    ///
    /// GET `/api/v1/registry/templates?q=...&category=...&tag=...`
    pub fn search(
        &self,
        query: &str,
        category: Option<&str>,
        tag: Option<&str>,
    ) -> Result<Vec<SearchResult>, String> {
        let mut url = format!(
            "{}/api/v1/registry/templates?q={}",
            self.base_url,
            urlencoded(query)
        );

        if let Some(cat) = category {
            url.push_str(&format!("&category={}", urlencoded(cat)));
        }
        if let Some(t) = tag {
            url.push_str(&format!("&tag={}", urlencoded(t)));
        }

        let agent = ureq::Agent::new_with_defaults();

        let response = agent
            .get(&url)
            .call()
            .map_err(|e| classify_http_error(e, "search"))?;

        response
            .into_body()
            .read_json::<Vec<SearchResult>>()
            .map_err(|e| format!("could not parse search response: {}", e))
    }

    /// Download a template archive.
    ///
    /// GET `/api/v1/registry/templates/{name}/{version}/download`
    ///
    /// Uses `"latest"` if no version is specified.
    pub fn download(&self, name: &str, version: Option<&str>) -> Result<Vec<u8>, String> {
        let version_str = version.unwrap_or("latest");
        let url = format!(
            "{}/api/v1/registry/templates/{}/{}/download",
            self.base_url, name, version_str
        );

        let agent = ureq::Agent::new_with_defaults();

        let response = agent
            .get(&url)
            .call()
            .map_err(|e| classify_download_error(e, name, version))?;

        let bytes = response
            .into_body()
            .read_to_vec()
            .map_err(|e| format!("error reading download response: {}", e))?;

        Ok(bytes)
    }

    /// Retrieve full metadata for a single template.
    ///
    /// GET `/api/v1/registry/templates/{name}`
    // Planned for `tenor info <template>` subcommand in a future plan.
    #[allow(dead_code)]
    pub fn get_template(&self, name: &str) -> Result<TemplateInfo, String> {
        let url = format!("{}/api/v1/registry/templates/{}", self.base_url, name);

        let agent = ureq::Agent::new_with_defaults();

        let response = agent
            .get(&url)
            .call()
            .map_err(|e| classify_http_error(e, "get template"))?;

        response
            .into_body()
            .read_json::<TemplateInfo>()
            .map_err(|e| format!("could not parse template info response: {}", e))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Percent-encode a query string value (spaces → %20, etc.).
///
/// Only encodes characters that must be encoded in a query parameter value.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push_str("%20"),
            _ => {
                for byte in ch.to_string().as_bytes() {
                    out.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    out
}

/// Convert a ureq error into a human-readable string for general operations.
fn classify_http_error(err: ureq::Error, operation: &str) -> String {
    let msg = err.to_string();

    // Connection-level errors
    if msg.contains("Connection refused")
        || msg.contains("connection refused")
        || msg.contains("os error 61")
        || msg.contains("os error 111")
    {
        return format!(
            "error: could not connect to registry ({operation}): {msg}"
        );
    }

    // Parse the HTTP status if present
    if let Some(status) = extract_status_from_ureq_error(&msg) {
        match status {
            401 => return format!("error: registry returned 401 Unauthorized — check your token"),
            403 => return format!("error: registry returned 403 Forbidden — insufficient permissions"),
            404 => return format!("error: registry returned 404 Not Found"),
            500..=599 => return format!("error: registry server error ({status})"),
            _ => {}
        }
    }

    format!("error: registry {operation} failed: {msg}")
}

/// Convert a ureq error into a human-readable string for download operations.
///
/// Provides template-specific 404 messages.
fn classify_download_error(err: ureq::Error, name: &str, version: Option<&str>) -> String {
    let msg = err.to_string();

    if msg.contains("Connection refused")
        || msg.contains("connection refused")
        || msg.contains("os error 61")
        || msg.contains("os error 111")
    {
        return format!("error: could not connect to registry: {msg}");
    }

    if let Some(status) = extract_status_from_ureq_error(&msg) {
        match status {
            404 => {
                return if let Some(v) = version {
                    format!("error: version '{v}' of '{name}' not found")
                } else {
                    format!("error: template '{name}' not found in registry")
                };
            }
            401 => return "error: registry returned 401 Unauthorized — check your token".to_string(),
            500..=599 => return format!("error: registry server error ({status})"),
            _ => {}
        }
    }

    format!("error: download failed: {msg}")
}

/// Extract an HTTP status code from a ureq error string.
///
/// ureq v3 formats status errors as "http status: NNN ..." or similar.
fn extract_status_from_ureq_error(msg: &str) -> Option<u16> {
    for word in msg.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_ascii_digit());
        if clean.len() == 3 {
            if let Ok(code) = clean.parse::<u16>() {
                if (100..=599).contains(&code) {
                    return Some(code);
                }
            }
        }
    }
    None
}
