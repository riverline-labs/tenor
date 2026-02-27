use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::Signer;
use serde_json::Map;
use sha2::{Digest, Sha256};

use crate::manifest::compute_etag;
use crate::trust::keygen;

/// Sign an interchange bundle with a detached Ed25519 attestation.
///
/// Reads the bundle JSON, computes the etag, signs it with the given secret key,
/// and produces a signed bundle JSON with a `trust` section.
///
/// # Arguments
/// - `bundle_path`: Path to the interchange JSON bundle or manifest.
/// - `key_path`: Path to the `.secret` file produced by `tenor keygen`.
/// - `output_path`: Where to write the signed bundle (default: `<bundle>.signed.json`).
/// - `format`: Attestation format identifier (default: `"ed25519-detached"`).
pub fn cmd_sign(bundle_path: &Path, key_path: &Path, output_path: Option<&Path>, format: &str) {
    // Read and parse the bundle JSON
    let bundle_str = match std::fs::read_to_string(bundle_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading bundle '{}': {}", bundle_path.display(), e);
            std::process::exit(1);
        }
    };

    let bundle_json: serde_json::Value = match serde_json::from_str(&bundle_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error parsing JSON in '{}': {}", bundle_path.display(), e);
            std::process::exit(1);
        }
    };

    // Detect bundle content — support both bare bundle and manifest-wrapped formats
    let bundle_content = if bundle_json.get("constructs").is_some() {
        // Bare bundle format: the whole JSON is the bundle
        bundle_json.clone()
    } else if let Some(inner) = bundle_json.get("bundle") {
        // Manifest format: extract the bundle field
        inner.clone()
    } else {
        eprintln!(
            "error: '{}' does not appear to be a Tenor interchange bundle or manifest",
            bundle_path.display()
        );
        std::process::exit(1);
    };

    // Compute the etag (SHA-256 of canonical compact JSON)
    let etag = compute_etag(&bundle_content);

    // Load the secret key
    let signing_key = match keygen::read_secret_key(key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Derive verifying key and sign the etag bytes
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(etag.as_bytes());

    // Encode signature and public key as base64
    let sig_b64 = BASE64.encode(signature.to_bytes());
    let pubkey_b64 = BASE64.encode(verifying_key.to_bytes());

    // Compute a fingerprint: first 16 hex chars of SHA-256 of pubkey bytes
    let pubkey_hash = Sha256::digest(verifying_key.to_bytes());
    let fingerprint = format!("{:x}", pubkey_hash)[..16].to_string();

    // Build the signed bundle JSON with lexicographically sorted keys
    let mut trust_map = Map::new();
    trust_map.insert("attestation_format".to_string(), serde_json::json!(format));
    trust_map.insert("bundle_attestation".to_string(), serde_json::json!(sig_b64));
    trust_map.insert(
        "signer_public_key".to_string(),
        serde_json::json!(pubkey_b64),
    );

    let mut signed_map = Map::new();
    signed_map.insert("bundle".to_string(), bundle_content);
    signed_map.insert("etag".to_string(), serde_json::json!(etag));
    signed_map.insert("tenor".to_string(), serde_json::json!("1.0"));
    signed_map.insert("trust".to_string(), serde_json::Value::Object(trust_map));

    let signed_bundle = serde_json::Value::Object(signed_map);

    // Determine output path
    let default_output;
    let out_path = match output_path {
        Some(p) => p,
        None => {
            default_output = bundle_path.with_extension("signed.json");
            &default_output
        }
    };

    // Write the signed bundle
    let pretty = serde_json::to_string_pretty(&signed_bundle)
        .unwrap_or_else(|e| panic!("serialization error: {}", e));
    if let Err(e) = std::fs::write(out_path, &pretty) {
        eprintln!(
            "error writing signed bundle to '{}': {}",
            out_path.display(),
            e
        );
        std::process::exit(1);
    }

    println!(
        "Signed bundle written to '{}'\netag: {}\nsigner: {}...",
        out_path.display(),
        etag,
        fingerprint
    );
}
