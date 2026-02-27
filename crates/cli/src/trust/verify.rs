use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::manifest::compute_etag;
use crate::trust::keygen;

/// Result of an in-memory bundle verification.
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyResult {
    /// Verification succeeded.
    Ok,
    /// Verification failed with a descriptive reason.
    Fail(String),
}

/// Verify a signed bundle JSON value in memory.
///
/// Returns `VerifyResult::Ok` if the bundle is valid, or `VerifyResult::Fail(reason)` otherwise.
pub fn verify_bundle(signed_json: &serde_json::Value, verifying_key_override: Option<&VerifyingKey>) -> VerifyResult {
    // Extract the trust section
    let trust = match signed_json.get("trust") {
        Some(t) if !t.is_null() => t,
        _ => return VerifyResult::Fail("no trust section found".to_string()),
    };

    // Check attestation format
    let attestation_format = match trust.get("attestation_format").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return VerifyResult::Fail("attestation_format missing from trust section".to_string()),
    };

    if attestation_format != "ed25519-detached" {
        return VerifyResult::Fail(format!(
            "unrecognized attestation format: {}",
            attestation_format
        ));
    }

    // Get the verifying key
    let verifying_key: VerifyingKey = if let Some(vk) = verifying_key_override {
        *vk
    } else if let Some(embedded_key) = trust.get("signer_public_key").and_then(|v| v.as_str()) {
        let key_bytes = match BASE64.decode(embedded_key) {
            Ok(b) => b,
            Err(e) => return VerifyResult::Fail(format!("error decoding signer_public_key: {}", e)),
        };
        let key_arr: [u8; 32] = match key_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => return VerifyResult::Fail("signer_public_key has invalid length (expected 32 bytes)".to_string()),
        };
        match VerifyingKey::from_bytes(&key_arr) {
            Ok(k) => k,
            Err(e) => return VerifyResult::Fail(format!("invalid signer_public_key: {}", e)),
        }
    } else {
        return VerifyResult::Fail("no public key available (provide --pubkey or ensure signer_public_key is in trust section)".to_string());
    };

    // Extract the signature
    let sig_b64 = match trust.get("bundle_attestation").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return VerifyResult::Fail("bundle_attestation missing from trust section".to_string()),
    };

    let sig_bytes = match BASE64.decode(sig_b64) {
        Ok(b) => b,
        Err(e) => return VerifyResult::Fail(format!("error decoding bundle_attestation: {}", e)),
    };

    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return VerifyResult::Fail("bundle_attestation has invalid length (expected 64 bytes)".to_string()),
    };

    let signature = Signature::from_bytes(&sig_arr);

    // Extract the bundle content
    let bundle_content = match signed_json.get("bundle") {
        Some(b) if !b.is_null() => b,
        _ => return VerifyResult::Fail("no bundle field found in signed bundle".to_string()),
    };

    // Recompute the etag and compare
    let recomputed_etag = compute_etag(bundle_content);
    let stored_etag = match signed_json.get("etag").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return VerifyResult::Fail("etag field missing from signed bundle".to_string()),
    };

    if recomputed_etag != stored_etag {
        return VerifyResult::Fail(format!(
            "etag mismatch: bundle content has been modified (stored: {}, computed: {})",
            stored_etag, recomputed_etag
        ));
    }

    // Verify the signature
    if let Err(e) = verifying_key.verify_strict(recomputed_etag.as_bytes(), &signature) {
        return VerifyResult::Fail(format!("invalid signature: {}", e));
    }

    VerifyResult::Ok
}

/// Verify a signed interchange bundle.
///
/// Checks:
/// 1. The `trust` section is present and recognizable.
/// 2. The `bundle` field is present and its etag matches the stored `etag`.
/// 3. The Ed25519 signature over the etag bytes is valid.
///
/// Exits 0 on success, exits 1 on any failure.
///
/// # Arguments
/// - `signed_bundle_path`: Path to the signed bundle JSON produced by `tenor sign`.
/// - `pubkey_path`: Optional path to the `.pub` file. If absent, uses the
///   `signer_public_key` embedded in the `trust` section.
pub fn cmd_verify(signed_bundle_path: &Path, pubkey_path: Option<&Path>) {
    // Read and parse the signed bundle
    let bundle_str = match std::fs::read_to_string(signed_bundle_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading '{}': {}", signed_bundle_path.display(), e);
            std::process::exit(1);
        }
    };

    let signed_json: serde_json::Value = match serde_json::from_str(&bundle_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error parsing JSON in '{}': {}",
                signed_bundle_path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    // Load verifying key from file if provided
    let file_key: Option<VerifyingKey> = if let Some(path) = pubkey_path {
        match keygen::read_public_key(path) {
            Ok(k) => Some(k),
            Err(e) => {
                eprintln!("Verification failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    match verify_bundle(&signed_json, file_key.as_ref()) {
        VerifyResult::Ok => {
            let pubkey_bytes = if let Some(k) = &file_key {
                k.to_bytes()
            } else {
                // Extract embedded key for fingerprint display
                let trust = signed_json.get("trust").unwrap();
                let key_b64 = trust["signer_public_key"].as_str().unwrap_or("");
                let key_bytes = BASE64.decode(key_b64).unwrap_or_default();
                let arr: [u8; 32] = key_bytes.try_into().unwrap_or([0u8; 32]);
                arr
            };
            let pubkey_hash = Sha256::digest(pubkey_bytes);
            let fingerprint = format!("{:x}", pubkey_hash)[..16].to_string();
            println!(
                "Bundle verified: etag matches, signature valid, signer: {}...",
                fingerprint
            );
        }
        VerifyResult::Fail(reason) => {
            eprintln!("Verification failed: {}", reason);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::sign::sign_bundle;

    fn generate_keypair() -> (ed25519_dalek::SigningKey, VerifyingKey) {
        let mut rng = rand::rngs::OsRng;
        let sk = ed25519_dalek::SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn minimal_bundle() -> serde_json::Value {
        serde_json::json!({
            "id": "test-bundle",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": []
        })
    }

    #[test]
    fn test_verify_valid_signed_bundle() {
        let (sk, vk) = generate_keypair();
        let signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        assert_eq!(verify_bundle(&signed, Some(&vk)), VerifyResult::Ok);
    }

    #[test]
    fn test_verify_tampered_bundle_fails() {
        let (sk, vk) = generate_keypair();
        let mut signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        // Tamper: change the bundle id
        signed["bundle"]["id"] = serde_json::json!("tampered-id");
        let result = verify_bundle(&signed, Some(&vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail, got Ok"
        );
        if let VerifyResult::Fail(reason) = result {
            assert!(
                reason.contains("etag mismatch"),
                "expected etag mismatch, got: {}",
                reason
            );
        }
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let (sk, _vk) = generate_keypair();
        let (_, wrong_vk) = generate_keypair();
        let signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        let result = verify_bundle(&signed, Some(&wrong_vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail with wrong key"
        );
    }

    #[test]
    fn test_verify_tampered_etag_fails() {
        let (sk, vk) = generate_keypair();
        let mut signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        // Tamper: replace etag with garbage (bundle content unchanged)
        signed["etag"] = serde_json::json!("00000000000000000000000000000000");
        let result = verify_bundle(&signed, Some(&vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail for tampered etag"
        );
    }

    #[test]
    fn test_verify_tampered_signature_fails() {
        let (sk, vk) = generate_keypair();
        let mut signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        // Replace the signature with a base64 encoding of 64 zero bytes
        let fake_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 64]);
        signed["trust"]["bundle_attestation"] = serde_json::json!(fake_sig);
        let result = verify_bundle(&signed, Some(&vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail for tampered signature"
        );
    }

    #[test]
    fn test_verify_unrecognized_format() {
        let (sk, vk) = generate_keypair();
        let mut signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        // Change the format to something unsupported
        signed["trust"]["attestation_format"] = serde_json::json!("rsa-pss");
        let result = verify_bundle(&signed, Some(&vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail for unsupported format"
        );
        if let VerifyResult::Fail(reason) = result {
            assert!(
                reason.contains("unrecognized attestation format"),
                "expected format error, got: {}",
                reason
            );
        }
    }

    #[test]
    fn test_verify_missing_trust_section() {
        let bundle = serde_json::json!({
            "bundle": minimal_bundle(),
            "etag": "someetag",
            "tenor": "1.0"
            // no "trust" field
        });
        let (_, vk) = generate_keypair();
        let result = verify_bundle(&bundle, Some(&vk));
        assert!(
            matches!(result, VerifyResult::Fail(_)),
            "expected Fail for missing trust section"
        );
    }

    #[test]
    fn test_verify_pubkey_from_bundle() {
        // Sign with key, embed public key in trust section, verify without override key
        let (sk, _vk) = generate_keypair();
        let signed = sign_bundle(minimal_bundle(), &sk, "ed25519-detached");
        // signer_public_key is embedded — pass None as override
        assert_eq!(
            verify_bundle(&signed, None),
            VerifyResult::Ok,
            "should verify using embedded signer_public_key"
        );
    }
}
