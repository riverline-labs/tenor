use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::Map;
use sha2::{Digest, Sha256};

use crate::manifest::compute_etag;
use crate::trust::keygen;

/// Core signing logic — operates on in-memory values.
///
/// Returns the signed bundle JSON value (with `bundle`, `etag`, `tenor`, `trust` fields).
pub fn sign_bundle(
    bundle_content: serde_json::Value,
    signing_key: &SigningKey,
    format: &str,
) -> serde_json::Value {
    let etag = compute_etag(&bundle_content);
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(etag.as_bytes());
    let sig_b64 = BASE64.encode(signature.to_bytes());
    let pubkey_b64 = BASE64.encode(verifying_key.to_bytes());

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

    serde_json::Value::Object(signed_map)
}

/// Extract the bundle content from either a bare bundle or a manifest-wrapped bundle.
pub fn extract_bundle_content(bundle_json: serde_json::Value) -> Option<serde_json::Value> {
    if bundle_json.get("constructs").is_some() {
        Some(bundle_json)
    } else {
        bundle_json.get("bundle").cloned()
    }
}

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
    let bundle_content = match extract_bundle_content(bundle_json) {
        Some(c) => c,
        None => {
            eprintln!(
                "error: '{}' does not appear to be a Tenor interchange bundle or manifest",
                bundle_path.display()
            );
            std::process::exit(1);
        }
    };

    // Load the secret key
    let signing_key = match keygen::read_secret_key(key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // Compute a fingerprint: first 16 hex chars of SHA-256 of pubkey bytes
    let pubkey_hash = Sha256::digest(signing_key.verifying_key().to_bytes());
    let fingerprint = format!("{:x}", pubkey_hash)[..16].to_string();

    // Sign the bundle
    let etag = compute_etag(&bundle_content);
    let signed_bundle = sign_bundle(bundle_content, &signing_key, format);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_bare_bundle() -> serde_json::Value {
        serde_json::json!({
            "id": "test-bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": []
        })
    }

    fn minimal_manifest_bundle() -> serde_json::Value {
        serde_json::json!({
            "bundle": {
                "id": "test-bundle",
                "kind": "Bundle",
                "tenor": "1.0",
                "tenor_version": "1.0.0",
                "constructs": []
            },
            "etag": "existing-etag",
            "tenor": "1.0"
        })
    }

    fn generate_signing_key() -> ed25519_dalek::SigningKey {
        let mut rng = rand::rngs::OsRng;
        ed25519_dalek::SigningKey::generate(&mut rng)
    }

    #[test]
    fn test_sign_produces_valid_trust_section() {
        let bundle = minimal_bare_bundle();
        let signing_key = generate_signing_key();
        let signed = sign_bundle(bundle, &signing_key, "ed25519-detached");

        let trust = signed.get("trust").expect("trust section missing");
        assert!(
            trust.get("bundle_attestation").is_some(),
            "bundle_attestation missing"
        );
        assert!(
            trust.get("attestation_format").is_some(),
            "attestation_format missing"
        );
        assert!(
            trust.get("signer_public_key").is_some(),
            "signer_public_key missing"
        );
        assert_eq!(trust["attestation_format"], "ed25519-detached");
    }

    #[test]
    fn test_sign_etag_matches_content() {
        let bundle = minimal_bare_bundle();
        let signing_key = generate_signing_key();
        let signed = sign_bundle(bundle.clone(), &signing_key, "ed25519-detached");

        let stored_etag = signed["etag"].as_str().expect("etag missing");
        let expected_etag = compute_etag(&bundle);
        assert_eq!(
            stored_etag, expected_etag,
            "etag does not match bundle content"
        );
    }

    #[test]
    fn test_sign_different_bundles_different_signatures() {
        let bundle1 = serde_json::json!({
            "id": "bundle-one",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": []
        });
        let bundle2 = serde_json::json!({
            "id": "bundle-two",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": []
        });
        let signing_key = generate_signing_key();
        let signed1 = sign_bundle(bundle1, &signing_key, "ed25519-detached");
        let signed2 = sign_bundle(bundle2, &signing_key, "ed25519-detached");

        let sig1 = signed1["trust"]["bundle_attestation"].as_str().unwrap();
        let sig2 = signed2["trust"]["bundle_attestation"].as_str().unwrap();
        assert_ne!(
            sig1, sig2,
            "different bundles must produce different signatures"
        );
    }

    #[test]
    fn test_sign_same_bundle_same_signature() {
        let bundle = minimal_bare_bundle();
        let signing_key = generate_signing_key();
        let signed1 = sign_bundle(bundle.clone(), &signing_key, "ed25519-detached");
        let signed2 = sign_bundle(bundle, &signing_key, "ed25519-detached");

        let sig1 = signed1["trust"]["bundle_attestation"].as_str().unwrap();
        let sig2 = signed2["trust"]["bundle_attestation"].as_str().unwrap();
        assert_eq!(
            sig1, sig2,
            "same bundle signed twice must produce identical signatures"
        );

        let etag1 = signed1["etag"].as_str().unwrap();
        let etag2 = signed2["etag"].as_str().unwrap();
        assert_eq!(
            etag1, etag2,
            "same bundle signed twice must produce identical etags"
        );
    }

    #[test]
    fn test_sign_bare_bundle() {
        let bare = minimal_bare_bundle();
        let extracted = extract_bundle_content(bare.clone()).expect("should extract bare bundle");
        // The extracted content should be the bare bundle itself
        assert_eq!(extracted["id"], "test-bundle");
        assert!(extracted.get("constructs").is_some());

        let signing_key = generate_signing_key();
        let signed = sign_bundle(extracted, &signing_key, "ed25519-detached");
        // Result must have bundle, etag, tenor, trust
        assert!(signed.get("bundle").is_some());
        assert!(signed.get("etag").is_some());
        assert!(signed.get("tenor").is_some());
        assert!(signed.get("trust").is_some());
    }

    #[test]
    fn test_sign_manifest_bundle() {
        let manifest = minimal_manifest_bundle();
        let extracted = extract_bundle_content(manifest).expect("should extract from manifest");
        // The extracted content should be the inner bundle
        assert_eq!(extracted["id"], "test-bundle");

        let signing_key = generate_signing_key();
        let signed = sign_bundle(extracted, &signing_key, "ed25519-detached");
        assert!(signed.get("trust").is_some());
        assert_eq!(signed["trust"]["attestation_format"], "ed25519-detached");
    }
}
