use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::manifest::compute_etag;
use crate::trust::keygen;

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
            eprintln!("error parsing JSON in '{}': {}", signed_bundle_path.display(), e);
            std::process::exit(1);
        }
    };

    // Extract the trust section
    let trust = match signed_json.get("trust") {
        Some(t) if !t.is_null() => t,
        _ => {
            eprintln!("Verification failed: no trust section found");
            std::process::exit(1);
        }
    };

    // Check attestation format (AL81)
    let attestation_format = match trust.get("attestation_format").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            eprintln!("Verification failed: attestation_format missing from trust section");
            std::process::exit(1);
        }
    };

    if attestation_format != "ed25519-detached" {
        eprintln!(
            "Verification failed: unrecognized attestation format: {}",
            attestation_format
        );
        std::process::exit(1);
    }

    // Get the verifying key — from file or from embedded key
    let verifying_key: VerifyingKey = if let Some(path) = pubkey_path {
        match keygen::read_public_key(path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Verification failed: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(embedded_key) = trust.get("signer_public_key").and_then(|v| v.as_str()) {
        let key_bytes = match BASE64.decode(embedded_key) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Verification failed: error decoding signer_public_key: {}", e);
                std::process::exit(1);
            }
        };
        let key_arr: [u8; 32] = match key_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => {
                eprintln!("Verification failed: signer_public_key has invalid length (expected 32 bytes)");
                std::process::exit(1);
            }
        };
        match VerifyingKey::from_bytes(&key_arr) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Verification failed: invalid signer_public_key: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Verification failed: no public key available (provide --pubkey or ensure signer_public_key is in trust section)");
        std::process::exit(1);
    };

    // Extract the signature
    let sig_b64 = match trust.get("bundle_attestation").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            eprintln!("Verification failed: bundle_attestation missing from trust section");
            std::process::exit(1);
        }
    };

    let sig_bytes = match BASE64.decode(sig_b64) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Verification failed: error decoding bundle_attestation: {}", e);
            std::process::exit(1);
        }
    };

    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => {
            eprintln!("Verification failed: bundle_attestation has invalid length (expected 64 bytes)");
            std::process::exit(1);
        }
    };

    let signature = Signature::from_bytes(&sig_arr);

    // Extract the bundle content
    let bundle_content = match signed_json.get("bundle") {
        Some(b) if !b.is_null() => b,
        _ => {
            eprintln!("Verification failed: no bundle field found in signed bundle");
            std::process::exit(1);
        }
    };

    // Recompute the etag and compare against stored etag
    let recomputed_etag = compute_etag(bundle_content);
    let stored_etag = match signed_json.get("etag").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            eprintln!("Verification failed: etag field missing from signed bundle");
            std::process::exit(1);
        }
    };

    if recomputed_etag != stored_etag {
        eprintln!(
            "Verification failed: etag mismatch: bundle content has been modified\n  stored: {}\n  computed: {}",
            stored_etag, recomputed_etag
        );
        std::process::exit(1);
    }

    // Verify the signature covers the etag bytes
    if let Err(e) = verifying_key.verify_strict(recomputed_etag.as_bytes(), &signature) {
        eprintln!("Verification failed: invalid signature: {}", e);
        std::process::exit(1);
    }

    // Compute signer fingerprint
    let pubkey_hash = Sha256::digest(verifying_key.to_bytes());
    let fingerprint = format!("{:x}", pubkey_hash)[..16].to_string();

    println!(
        "Bundle verified: etag matches, signature valid, signer: {}...",
        fingerprint
    );
}
