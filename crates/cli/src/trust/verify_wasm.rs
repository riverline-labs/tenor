//! WASM binary verification — checks binary integrity and bundle binding.
//!
//! Reads the detached `.sig` file produced by `sign_wasm`, recomputes the
//! SHA-256 of the binary, and verifies the Ed25519 signature. Reports a
//! clear failure reason if any check fails, and exits with code 1.

use std::path::Path;
use std::process;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::Signature;
use sha2::{Digest, Sha256};

use super::keygen;

/// Verify a signed WASM binary against its detached signature file.
///
/// Checks:
/// 1. The attestation format is "ed25519-detached".
/// 2. The SHA-256 hash of the binary matches `wasm_hash` in the sig file.
/// 3. The Ed25519 signature over `wasm_hash:bundle_etag` is valid.
///
/// Exits with code 1 on any verification failure.
pub fn cmd_verify_wasm(wasm_path: &Path, sig_path: &Path, pubkey_path: &Path) {
    // 1. Read the WASM binary.
    let wasm_bytes = match std::fs::read(wasm_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "Verification failed: error reading WASM binary '{}': {}",
                wasm_path.display(),
                e
            );
            process::exit(1);
        }
    };

    // 2. Compute SHA-256 hash.
    let hash_bytes = Sha256::digest(&wasm_bytes);
    let computed_hash: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // 3. Read and parse the .sig JSON file.
    let sig_str = match std::fs::read_to_string(sig_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Verification failed: error reading signature file '{}': {}",
                sig_path.display(),
                e
            );
            process::exit(1);
        }
    };

    let sig_json: serde_json::Value = match serde_json::from_str(&sig_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Verification failed: invalid JSON in '{}': {}",
                sig_path.display(),
                e
            );
            process::exit(1);
        }
    };

    // 4. Check attestation_format.
    let fmt = sig_json
        .get("attestation_format")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if fmt != "ed25519-detached" {
        eprintln!(
            "Verification failed: unrecognized attestation format '{}'",
            fmt
        );
        process::exit(1);
    }

    // 5. Extract and compare wasm_hash.
    let stored_hash = sig_json
        .get("wasm_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if stored_hash != computed_hash {
        eprintln!("Verification failed: WASM binary hash mismatch: binary has been modified");
        eprintln!("  Expected: {}", stored_hash);
        eprintln!("  Computed: {}", computed_hash);
        process::exit(1);
    }

    // 6. Extract bundle_etag.
    let bundle_etag = sig_json
        .get("bundle_etag")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 7. Read the public key from the --pubkey file.
    let verifying_key = match keygen::read_public_key(pubkey_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Verification failed: {}", e);
            process::exit(1);
        }
    };

    // 8. Extract and decode the signature.
    let sig_b64 = sig_json
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let sig_bytes = match BASE64.decode(sig_b64) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Verification failed: error decoding signature: {}", e);
            process::exit(1);
        }
    };
    if sig_bytes.len() != 64 {
        eprintln!(
            "Verification failed: invalid signature length: expected 64 bytes, got {}",
            sig_bytes.len()
        );
        process::exit(1);
    }
    let sig_array: [u8; 64] = sig_bytes.try_into().expect("already checked length is 64");
    let signature = Signature::from_bytes(&sig_array);

    // 9. Reconstruct the attestation payload.
    let payload = format!("{}:{}", computed_hash, bundle_etag);

    // 10. Verify the signature.
    if let Err(e) = verifying_key.verify_strict(payload.as_bytes(), &signature) {
        eprintln!("Verification failed: signature invalid: {}", e);
        process::exit(1);
    }

    // 11. Print success.
    let fingerprint = keygen::key_fingerprint(&verifying_key);
    println!("WASM binary verified:");
    println!("  WASM hash: {} (matches)", computed_hash);
    println!("  Bundle etag: {}", bundle_etag);
    println!("  Signature: valid");
    println!("  Signer: {}", fingerprint);
}
