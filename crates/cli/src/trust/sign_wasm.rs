//! WASM binary signing — binds a compiled WASM evaluator to a contract bundle etag.
//!
//! Produces a detached signature file (`<wasm>.sig`) containing:
//! - `wasm_hash`         — SHA-256 of the WASM binary bytes (hex)
//! - `bundle_etag`       — the bundle etag this binary was compiled from
//! - `signer_public_key` — base64-encoded Ed25519 public key
//! - `signature`         — base64-encoded Ed25519 signature of `wasm_hash:bundle_etag`
//! - `attestation_format` — always "ed25519-detached"

use std::path::Path;
use std::process;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::Signer as _;
use sha2::{Digest, Sha256};

use super::keygen;

/// Sign a WASM binary, binding it to a specific bundle etag.
///
/// Writes `<wasm_path>.sig` as a JSON attestation file and prints a
/// confirmation to stdout. Exits with code 1 on any error.
pub fn cmd_sign_wasm(wasm_path: &Path, key_path: &Path, bundle_etag: &str) {
    // 1. Read the WASM binary.
    let wasm_bytes = match std::fs::read(wasm_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error reading WASM binary '{}': {}", wasm_path.display(), e);
            process::exit(1);
        }
    };

    // 2. Compute SHA-256 hash of the binary.
    let hash_bytes = Sha256::digest(&wasm_bytes);
    let wasm_hash: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // 3. Read the signing key.
    let signing_key = match keygen::read_secret_key(key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // 4. Derive the verifying (public) key.
    let verifying_key = signing_key.verifying_key();

    // 5. Construct the attestation payload: wasm_hash:bundle_etag
    let payload = format!("{}:{}", wasm_hash, bundle_etag);

    // 6. Sign the payload.
    let signature = signing_key.sign(payload.as_bytes());

    // 7. Base64-encode the signature and public key.
    let sig_b64 = BASE64.encode(signature.to_bytes());
    let pubkey_b64 = BASE64.encode(verifying_key.to_bytes());

    // 8. Compute fingerprint for display.
    let fingerprint = keygen::key_fingerprint(&verifying_key);

    // 9. Build the JSON attestation object (keys sorted lexicographically).
    let attestation = serde_json::json!({
        "attestation_format": "ed25519-detached",
        "bundle_etag": bundle_etag,
        "signature": sig_b64,
        "signer_public_key": pubkey_b64,
        "wasm_hash": wasm_hash,
    });

    // 10. Write detached signature file.
    let sig_path = {
        let mut p = wasm_path.as_os_str().to_owned();
        p.push(".sig");
        std::path::PathBuf::from(p)
    };

    let sig_json = match serde_json::to_string_pretty(&attestation) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("error serializing attestation: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = std::fs::write(&sig_path, sig_json) {
        eprintln!(
            "error writing signature file '{}': {}",
            sig_path.display(),
            e
        );
        process::exit(1);
    }

    // 11. Print confirmation.
    println!("Signed WASM binary: {}", sig_path.display());
    println!("  WASM hash: {}", wasm_hash);
    println!("  Bundle etag: {}", bundle_etag);
    println!("  Signer: {}", fingerprint);
}

/// Core WASM signing — operates on in-memory bytes.
///
/// Returns the attestation JSON value (with all required fields).
pub fn sign_wasm_bytes(
    wasm_bytes: &[u8],
    signing_key: &ed25519_dalek::SigningKey,
    bundle_etag: &str,
) -> serde_json::Value {
    let hash_bytes = Sha256::digest(wasm_bytes);
    let wasm_hash: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let verifying_key = signing_key.verifying_key();
    let payload = format!("{}:{}", wasm_hash, bundle_etag);
    let signature = signing_key.sign(payload.as_bytes());

    let sig_b64 = BASE64.encode(signature.to_bytes());
    let pubkey_b64 = BASE64.encode(verifying_key.to_bytes());

    serde_json::json!({
        "attestation_format": "ed25519-detached",
        "bundle_etag": bundle_etag,
        "signature": sig_b64,
        "signer_public_key": pubkey_b64,
        "wasm_hash": wasm_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn generate_keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
        let mut rng = rand::rngs::OsRng;
        let sk = ed25519_dalek::SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    #[test]
    fn test_sign_wasm_produces_valid_sig_file() {
        let (sk, _vk) = generate_keypair();
        let fake_wasm = b"fake wasm binary content for testing";
        let bundle_etag = "abc123";

        let attestation = sign_wasm_bytes(fake_wasm, &sk, bundle_etag);

        assert!(
            attestation.get("wasm_hash").is_some(),
            "wasm_hash missing"
        );
        assert!(
            attestation.get("bundle_etag").is_some(),
            "bundle_etag missing"
        );
        assert!(
            attestation.get("signature").is_some(),
            "signature missing"
        );
        assert!(
            attestation.get("signer_public_key").is_some(),
            "signer_public_key missing"
        );
        assert!(
            attestation.get("attestation_format").is_some(),
            "attestation_format missing"
        );
        assert_eq!(attestation["attestation_format"], "ed25519-detached");
        assert_eq!(attestation["bundle_etag"], bundle_etag);
    }

    #[test]
    fn test_sign_wasm_hash_is_sha256() {
        let (sk, _vk) = generate_keypair();
        let fake_wasm = b"some deterministic wasm content";
        let bundle_etag = "etag-test";

        // Compute SHA-256 manually
        let expected_hash: String = Sha256::digest(fake_wasm)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        let attestation = sign_wasm_bytes(fake_wasm, &sk, bundle_etag);
        let stored_hash = attestation["wasm_hash"].as_str().unwrap();

        assert_eq!(
            stored_hash, expected_hash,
            "wasm_hash does not match manual SHA-256"
        );
    }
}
