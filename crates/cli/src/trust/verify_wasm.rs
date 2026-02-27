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

/// Result of an in-memory WASM verification.
#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum VerifyWasmResult {
    Ok,
    Fail(String),
}

/// Verify a WASM binary against its attestation JSON in memory.
#[allow(dead_code)]
pub fn verify_wasm_bytes(
    wasm_bytes: &[u8],
    attestation: &serde_json::Value,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> VerifyWasmResult {
    // Check attestation_format
    let fmt = attestation
        .get("attestation_format")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if fmt != "ed25519-detached" {
        return VerifyWasmResult::Fail(format!("unrecognized attestation format '{}'", fmt));
    }

    // Compute hash of the provided bytes
    let hash_bytes = sha2::Sha256::digest(wasm_bytes);
    let computed_hash: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Compare against stored hash
    let stored_hash = attestation
        .get("wasm_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if stored_hash != computed_hash {
        return VerifyWasmResult::Fail(format!(
            "WASM binary hash mismatch: expected {}, got {}",
            stored_hash, computed_hash
        ));
    }

    // Extract bundle_etag
    let bundle_etag = attestation
        .get("bundle_etag")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Extract and decode signature
    let sig_b64 = attestation
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let sig_bytes = match BASE64.decode(sig_b64) {
        Ok(b) => b,
        Err(e) => return VerifyWasmResult::Fail(format!("error decoding signature: {}", e)),
    };
    if sig_bytes.len() != 64 {
        return VerifyWasmResult::Fail(format!(
            "invalid signature length: expected 64 bytes, got {}",
            sig_bytes.len()
        ));
    }
    let sig_array: [u8; 64] = sig_bytes.try_into().expect("already checked length is 64");
    let signature = Signature::from_bytes(&sig_array);

    // Reconstruct payload and verify
    let payload = format!("{}:{}", computed_hash, bundle_etag);
    if let Err(e) = verifying_key.verify_strict(payload.as_bytes(), &signature) {
        return VerifyWasmResult::Fail(format!("signature invalid: {}", e));
    }

    VerifyWasmResult::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::sign_wasm::sign_wasm_bytes;

    fn generate_keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
        let mut rng = rand::rngs::OsRng;
        let sk = ed25519_dalek::SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    #[test]
    fn test_verify_wasm_roundtrip() {
        let (sk, vk) = generate_keypair();
        let wasm_bytes = b"fake wasm binary for roundtrip test";
        let bundle_etag = "test-etag-123";

        let attestation = sign_wasm_bytes(wasm_bytes, &sk, bundle_etag);
        let result = verify_wasm_bytes(wasm_bytes, &attestation, &vk);
        assert_eq!(
            result,
            VerifyWasmResult::Ok,
            "roundtrip verification failed"
        );
    }

    #[test]
    fn test_verify_wasm_tampered_binary() {
        let (sk, vk) = generate_keypair();
        let wasm_bytes = b"original wasm binary content";
        let tampered = b"tampered wasm binary content!!";
        let bundle_etag = "etag-for-tamper-test";

        let attestation = sign_wasm_bytes(wasm_bytes, &sk, bundle_etag);
        let result = verify_wasm_bytes(tampered, &attestation, &vk);
        assert!(
            matches!(result, VerifyWasmResult::Fail(_)),
            "expected Fail for tampered binary"
        );
        if let VerifyWasmResult::Fail(reason) = result {
            assert!(
                reason.contains("hash mismatch"),
                "expected hash mismatch error, got: {}",
                reason
            );
        }
    }

    #[test]
    fn test_verify_wasm_wrong_key() {
        let (sk, _vk) = generate_keypair();
        let (_, wrong_vk) = generate_keypair();
        let wasm_bytes = b"some wasm content";
        let bundle_etag = "etag-wrong-key";

        let attestation = sign_wasm_bytes(wasm_bytes, &sk, bundle_etag);
        let result = verify_wasm_bytes(wasm_bytes, &attestation, &wrong_vk);
        assert!(
            matches!(result, VerifyWasmResult::Fail(_)),
            "expected Fail for wrong key"
        );
    }

    #[test]
    fn test_verify_wasm_tampered_sig() {
        let (sk, vk) = generate_keypair();
        let wasm_bytes = b"wasm binary for sig tamper test";
        let bundle_etag = "etag-tampered-sig";

        let mut attestation = sign_wasm_bytes(wasm_bytes, &sk, bundle_etag);
        // Replace signature with base64 of 64 zero bytes
        let fake_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 64]);
        attestation["signature"] = serde_json::json!(fake_sig);

        let result = verify_wasm_bytes(wasm_bytes, &attestation, &vk);
        assert!(
            matches!(result, VerifyWasmResult::Fail(_)),
            "expected Fail for tampered signature"
        );
    }
}
