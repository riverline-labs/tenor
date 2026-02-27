//! Ed25519 key generation and loading utilities.
//!
//! Key files are stored as base64-encoded raw bytes:
//! - `.secret` — 64-byte Ed25519 secret key (seed || public key)
//! - `.pub`    — 32-byte Ed25519 public key

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{SigningKey, VerifyingKey};

/// Read an Ed25519 signing (secret) key from a `.secret` file.
///
/// The file must contain a base64-encoded 32-byte seed. Returns the
/// `SigningKey` constructed from that seed.
pub fn read_secret_key(path: &Path) -> Result<SigningKey, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading secret key file '{}': {}", path.display(), e))?;
    let bytes = BASE64
        .decode(contents.trim())
        .map_err(|e| format!("error decoding secret key '{}': {}", path.display(), e))?;
    if bytes.len() != 32 {
        return Err(format!(
            "invalid secret key length in '{}': expected 32 bytes, got {}",
            path.display(),
            bytes.len()
        ));
    }
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("failed to read 32-byte seed from '{}'", path.display()))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Read an Ed25519 verifying (public) key from a `.pub` file.
///
/// The file must contain a base64-encoded 32-byte compressed Edwards point.
pub fn read_public_key(path: &Path) -> Result<VerifyingKey, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading public key file '{}': {}", path.display(), e))?;
    let bytes = BASE64
        .decode(contents.trim())
        .map_err(|e| format!("error decoding public key '{}': {}", path.display(), e))?;
    if bytes.len() != 32 {
        return Err(format!(
            "invalid public key length in '{}': expected 32 bytes, got {}",
            path.display(),
            bytes.len()
        ));
    }
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("failed to read 32-byte key from '{}'", path.display()))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid Ed25519 public key in '{}': {}", path.display(), e))
}

/// Compute a short hex fingerprint of a verifying key (first 8 bytes of the key bytes).
pub fn key_fingerprint(key: &VerifyingKey) -> String {
    let bytes = key.to_bytes();
    bytes[..8]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}
