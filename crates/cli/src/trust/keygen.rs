use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{SigningKey, VerifyingKey};

/// Generate an Ed25519 signing keypair and write to files.
///
/// Writes `<prefix>.secret` (base64-encoded 32-byte seed) and
/// `<prefix>.pub` (base64-encoded 32-byte verifying key).
/// Sets .secret file permissions to 0o600 on Unix.
pub fn cmd_keygen(algorithm: &str, output_prefix: &str) {
    if algorithm != "ed25519" {
        eprintln!(
            "error: unsupported algorithm '{}'; only 'ed25519' is supported in v1",
            algorithm
        );
        std::process::exit(1);
    }

    let mut rng = rand::rngs::OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    // Encode keys as base64
    let secret_b64 = BASE64.encode(signing_key.to_bytes());
    let pub_b64 = BASE64.encode(verifying_key.to_bytes());

    // Write secret key file
    let secret_path = format!("{}.secret", output_prefix);
    if let Err(e) = std::fs::write(&secret_path, &secret_b64) {
        eprintln!("error writing secret key to '{}': {}", secret_path, e);
        std::process::exit(1);
    }

    // Restrict permissions on secret key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        if let Err(e) = std::fs::set_permissions(&secret_path, perms) {
            eprintln!(
                "warning: failed to set permissions on '{}': {}",
                secret_path, e
            );
        }
    }

    // Write public key file
    let pub_path = format!("{}.pub", output_prefix);
    if let Err(e) = std::fs::write(&pub_path, &pub_b64) {
        eprintln!("error writing public key to '{}': {}", pub_path, e);
        std::process::exit(1);
    }

    println!(
        "Generated Ed25519 keypair: {}.secret, {}.pub",
        output_prefix, output_prefix
    );
}

/// Read a secret key file and return the SigningKey.
///
/// The file must contain a base64-encoded 32-byte Ed25519 seed.
pub fn read_secret_key(path: &Path) -> Result<SigningKey, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading secret key '{}': {}", path.display(), e))?;
    let bytes = BASE64
        .decode(contents.trim())
        .map_err(|e| format!("error decoding secret key '{}': {}", path.display(), e))?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
        format!(
            "invalid secret key length in '{}': expected 32 bytes",
            path.display()
        )
    })?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Read a public key file and return the VerifyingKey.
///
/// The file must contain a base64-encoded 32-byte Ed25519 public key.
pub fn read_public_key(path: &Path) -> Result<VerifyingKey, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("error reading public key '{}': {}", path.display(), e))?;
    let bytes = BASE64
        .decode(contents.trim())
        .map_err(|e| format!("error decoding public key '{}': {}", path.display(), e))?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
        format!(
            "invalid public key length in '{}': expected 32 bytes",
            path.display()
        )
    })?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid public key material in '{}': {}", path.display(), e))
}
