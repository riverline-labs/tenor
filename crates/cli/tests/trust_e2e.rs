//! End-to-end tests for `tenor keygen`, `tenor sign`, `tenor verify`,
//! `tenor sign-wasm`, and `tenor verify-wasm` CLI commands.
//!
//! Tests exercise the full trust pipeline via `assert_cmd`, writing
//! temporary files and checking exit codes and output.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Helper: create a Command for the `tenor` binary.
fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    cmd
}

/// Generate a keypair in a temp dir, returning paths to .secret and .pub.
fn keygen_in(tmp: &TempDir, name: &str) -> (PathBuf, PathBuf) {
    let prefix = tmp.path().join(name);
    tenor()
        .args(["keygen", "--prefix", prefix.to_str().unwrap()])
        .assert()
        .success();
    let secret = tmp.path().join(format!("{}.secret", name));
    let pub_key = tmp.path().join(format!("{}.pub", name));
    assert!(secret.exists(), "{}.secret not created", name);
    assert!(pub_key.exists(), "{}.pub not created", name);
    (secret, pub_key)
}

/// Write a minimal interchange JSON bundle to a temp file.
fn write_minimal_bundle(tmp: &TempDir, filename: &str) -> PathBuf {
    let path = tmp.path().join(filename);
    let content = serde_json::json!({
        "id": "e2e-test-bundle",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": []
    });
    fs::write(&path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
    path
}

// ── Test 1: keygen ─────────────────────────────────────────────────

#[test]
fn test_cli_keygen() {
    let tmp = TempDir::new().unwrap();
    let prefix = tmp.path().join("test-key");
    let prefix_str = prefix.to_str().unwrap();

    tenor()
        .args(["keygen", "--prefix", prefix_str])
        .assert()
        .success();

    // Both files must exist
    let secret_path = tmp.path().join("test-key.secret");
    let pub_path = tmp.path().join("test-key.pub");
    assert!(secret_path.exists(), ".secret file not created");
    assert!(pub_path.exists(), ".pub file not created");

    // Contents must be valid base64
    let secret_contents = fs::read_to_string(&secret_path).unwrap();
    let pub_contents = fs::read_to_string(&pub_path).unwrap();

    use base64::Engine as _;
    let secret_bytes = base64::engine::general_purpose::STANDARD
        .decode(secret_contents.trim())
        .expect(".secret file is not valid base64");
    let pub_bytes = base64::engine::general_purpose::STANDARD
        .decode(pub_contents.trim())
        .expect(".pub file is not valid base64");

    assert_eq!(secret_bytes.len(), 32, ".secret must be 32 bytes (Ed25519 seed)");
    assert_eq!(pub_bytes.len(), 32, ".pub must be 32 bytes (Ed25519 verifying key)");
}

// ── Test 2: sign and verify ─────────────────────────────────────────

#[test]
fn test_cli_sign_and_verify() {
    let tmp = TempDir::new().unwrap();
    let (secret_path, _pub_path) = keygen_in(&tmp, "sign-key");
    let bundle_path = write_minimal_bundle(&tmp, "bundle.json");
    let signed_path = tmp.path().join("bundle.signed.json");

    // Sign the bundle
    tenor()
        .args([
            "sign",
            bundle_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--out",
            signed_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(signed_path.exists(), "signed bundle not created");

    // Verify the signed bundle (using embedded public key)
    tenor()
        .args([
            "verify",
            signed_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

// ── Test 3: verify fails on tampered bundle ─────────────────────────

#[test]
fn test_cli_verify_fails_on_tampered() {
    let tmp = TempDir::new().unwrap();
    let (secret_path, _pub_path) = keygen_in(&tmp, "tamper-key");
    let bundle_path = write_minimal_bundle(&tmp, "bundle.json");
    let signed_path = tmp.path().join("bundle.signed.json");

    // Sign
    tenor()
        .args([
            "sign",
            bundle_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--out",
            signed_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Tamper: read, modify, write
    let mut signed_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&signed_path).unwrap()).unwrap();
    signed_json["bundle"]["id"] = serde_json::json!("tampered-id");
    fs::write(
        &signed_path,
        serde_json::to_string_pretty(&signed_json).unwrap(),
    )
    .unwrap();

    // Verify must fail (exit code 1)
    tenor()
        .args(["verify", signed_path.to_str().unwrap()])
        .assert()
        .failure();
}

// ── Test 4: verify fails with wrong key ────────────────────────────

#[test]
fn test_cli_verify_fails_wrong_key() {
    let tmp = TempDir::new().unwrap();
    let (secret_path, _pub_path) = keygen_in(&tmp, "signer-key");
    let (_wrong_secret, wrong_pub) = keygen_in(&tmp, "wrong-key");
    let bundle_path = write_minimal_bundle(&tmp, "bundle.json");
    let signed_path = tmp.path().join("bundle.signed.json");

    // Sign with signer-key
    tenor()
        .args([
            "sign",
            bundle_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--out",
            signed_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify with wrong key — must fail
    tenor()
        .args([
            "verify",
            signed_path.to_str().unwrap(),
            "--pubkey",
            wrong_pub.to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// ── Test 5: sign-wasm and verify-wasm ──────────────────────────────

#[test]
fn test_cli_sign_wasm_and_verify() {
    let tmp = TempDir::new().unwrap();
    let (secret_path, pub_path) = keygen_in(&tmp, "wasm-key");

    // Create a fake WASM binary
    let wasm_path = tmp.path().join("evaluator.wasm");
    fs::write(&wasm_path, b"\x00asm\x01\x00\x00\x00fake wasm content").unwrap();

    // Sign WASM
    tenor()
        .args([
            "sign-wasm",
            wasm_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--bundle-etag",
            "test-etag-for-e2e",
        ])
        .assert()
        .success();

    // .sig file must exist
    let sig_path_str = format!("{}.sig", wasm_path.to_str().unwrap());
    let sig_path = PathBuf::from(&sig_path_str);
    assert!(sig_path.exists(), ".sig file not created");

    // Verify WASM
    tenor()
        .args([
            "verify-wasm",
            wasm_path.to_str().unwrap(),
            "--sig",
            sig_path.to_str().unwrap(),
            "--pubkey",
            pub_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

// ── Test 6: verify-wasm fails on tampered binary ───────────────────

#[test]
fn test_cli_verify_wasm_fails_tampered() {
    let tmp = TempDir::new().unwrap();
    let (secret_path, pub_path) = keygen_in(&tmp, "wasm-tamper-key");

    // Create and sign a fake WASM binary
    let wasm_path = tmp.path().join("evaluator.wasm");
    fs::write(&wasm_path, b"\x00asm\x01\x00\x00\x00original wasm content").unwrap();

    tenor()
        .args([
            "sign-wasm",
            wasm_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--bundle-etag",
            "tamper-test-etag",
        ])
        .assert()
        .success();

    let sig_path_str = format!("{}.sig", wasm_path.to_str().unwrap());
    let sig_path = PathBuf::from(&sig_path_str);

    // Tamper: overwrite the WASM file with different content
    fs::write(&wasm_path, b"\x00asm\x01\x00\x00\x00tampered wasm content!!").unwrap();

    // Verify must fail
    tenor()
        .args([
            "verify-wasm",
            wasm_path.to_str().unwrap(),
            "--sig",
            sig_path.to_str().unwrap(),
            "--pubkey",
            pub_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
}
