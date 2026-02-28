//! End-to-end tests for `tenor pack` and pack/unpack round-trip.
//!
//! All tests invoke the `tenor` binary via assert_cmd and verify filesystem
//! output. The round-trip test extracts the archive using `flate2`+`tar`
//! directly (same crates used for packing) to confirm the structure is correct.

use std::fs;

use assert_cmd::Command;
use flate2::read::GzDecoder;
use tar::Archive;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal valid tenor-template.toml string.
fn minimal_manifest(name: &str) -> String {
    format!(
        r#"[template]
name = "{name}"
version = "1.0.0"
description = "A minimal test template"
author = "Test Author"
category = "test"
tags = ["test"]

[template.metadata]
entities = ["Order"]
facts_count = 2
flows_count = 1

[template.requirements]
tenor_version = ">=1.0.0"
"#
    )
}

/// A minimal valid .tenor contract.
fn minimal_tenor_contract() -> &'static str {
    r#"fact is_active {
  type:   Bool
  source: "service.active"
}

rule check_active {
  stratum: 0
  when:    is_active = true
  produce: verdict is_active { payload: Bool = true }
}
"#
}

/// A deliberately invalid .tenor contract (reference to undeclared fact).
fn invalid_tenor_contract() -> &'static str {
    r#"rule bad_rule {
  stratum: 0
  when:    nonexistent_fact = true
  produce: verdict bad { payload: Bool = true }
}
"#
}

/// Build a minimal valid template directory.
///
/// Returns the TempDir — caller must keep it alive for the directory to persist.
fn build_valid_template_dir(name: &str) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();

    fs::write(root.join("tenor-template.toml"), minimal_manifest(name)).expect("write manifest");

    let contract_dir = root.join("contract");
    fs::create_dir_all(&contract_dir).expect("mkdir contract");
    fs::write(contract_dir.join("hello.tenor"), minimal_tenor_contract()).expect("write tenor");

    tmp
}

/// Return a Command for the `tenor` binary.
fn tenor_cmd() -> Command {
    Command::cargo_bin("tenor").expect("tenor binary")
}

/// List entry paths in a `.tar.gz` archive.
fn list_tar_gz_entries(archive_path: &std::path::Path) -> Vec<String> {
    let file = fs::File::open(archive_path).expect("open archive");
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);
    tar.entries()
        .expect("read entries")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.path().ok().map(|p| p.to_string_lossy().to_string()))
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// Pack a valid template; archive exists and has gzip magic bytes.
#[test]
fn test_pack_valid_template() {
    let tmp = build_valid_template_dir("my-template");
    let out_dir = TempDir::new().expect("output tempdir");
    let archive_path = out_dir
        .path()
        .join("my-template-1.0.0.tenor-template.tar.gz");

    tenor_cmd()
        .args([
            "pack",
            tmp.path().to_str().unwrap(),
            "--out",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(archive_path.exists(), "archive not found");

    // Verify gzip magic bytes (1f 8b)
    let bytes = fs::read(&archive_path).expect("read archive");
    assert!(
        bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b,
        "archive does not start with gzip magic bytes"
    );
}

/// Pack then inspect the archive: all required files are present.
#[test]
fn test_pack_unpack_roundtrip() {
    let tmp = build_valid_template_dir("round-trip");
    let out_dir = TempDir::new().expect("output tempdir");
    let archive_path = out_dir
        .path()
        .join("round-trip-1.0.0.tenor-template.tar.gz");

    tenor_cmd()
        .args([
            "pack",
            tmp.path().to_str().unwrap(),
            "--out",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(archive_path.exists(), "archive not found");

    // Inspect the archive contents
    let entries = list_tar_gz_entries(&archive_path);

    assert!(
        entries.iter().any(|e| e == "tenor-template.toml"),
        "tenor-template.toml missing from archive; found: {:?}",
        entries
    );

    assert!(
        entries.iter().any(|e| e.starts_with("contract/")),
        "contract/ directory missing from archive; found: {:?}",
        entries
    );

    assert!(
        entries.iter().any(|e| e.ends_with(".tenor")),
        "no .tenor file in archive; found: {:?}",
        entries
    );

    assert!(
        entries.iter().any(|e| e == "bundle.json"),
        "bundle.json missing from archive; found: {:?}",
        entries
    );

    // Extract to a temp dir and verify the manifest can be parsed
    let unpack_dir = TempDir::new().expect("unpack tempdir");
    let file = fs::File::open(&archive_path).expect("open archive");
    let gz = GzDecoder::new(file);
    let mut tar_archive = Archive::new(gz);
    tar_archive.unpack(unpack_dir.path()).expect("unpack");

    let manifest_str =
        fs::read_to_string(unpack_dir.path().join("tenor-template.toml")).expect("read manifest");

    assert!(
        manifest_str.contains("round-trip"),
        "manifest does not contain template name"
    );
    assert!(
        manifest_str.contains("1.0.0"),
        "manifest does not contain version"
    );
    assert!(
        manifest_str.contains("A minimal test template"),
        "manifest does not contain description"
    );
}

/// Packing a directory without tenor-template.toml fails with a clear error.
#[test]
fn test_pack_missing_manifest() {
    let tmp = TempDir::new().expect("tempdir");
    // No tenor-template.toml created

    let assert = tenor_cmd()
        .args(["pack", tmp.path().to_str().unwrap()])
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    assert!(
        stderr.contains("tenor-template.toml") || stderr.contains("manifest"),
        "expected error to mention tenor-template.toml, got: {}",
        stderr
    );
}

/// Packing a template with an invalid .tenor file fails at elaboration.
#[test]
fn test_pack_invalid_contract() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();

    fs::write(
        root.join("tenor-template.toml"),
        minimal_manifest("bad-contract"),
    )
    .expect("write manifest");

    let contract_dir = root.join("contract");
    fs::create_dir_all(&contract_dir).expect("mkdir");
    fs::write(contract_dir.join("bad.tenor"), invalid_tenor_contract()).expect("write tenor");

    let assert = tenor_cmd()
        .args(["pack", root.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    // The error should indicate the contract failed
    assert!(
        !stderr.is_empty(),
        "expected a non-empty error message, got empty stderr"
    );
}

/// Pack with --output puts the archive at the custom path.
#[test]
fn test_pack_custom_output() {
    let tmp = build_valid_template_dir("custom-output");
    let out_dir = TempDir::new().expect("output tempdir");

    // Use a nested path to verify directory traversal
    let custom_path = out_dir.path().join("nested").join("my-archive.tar.gz");
    fs::create_dir_all(custom_path.parent().unwrap()).expect("mkdir");

    tenor_cmd()
        .args([
            "pack",
            tmp.path().to_str().unwrap(),
            "--out",
            custom_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        custom_path.exists(),
        "archive not found at custom path {}",
        custom_path.display()
    );
}

/// Various invalid manifests are rejected with informative errors.
#[test]
fn test_manifest_validation() {
    // Helper to run pack on a dir with a given manifest string and assert failure.
    let assert_pack_fails = |manifest_content: &str, expected_in_stderr: &str| {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        fs::write(root.join("tenor-template.toml"), manifest_content).expect("write manifest");

        let contract_dir = root.join("contract");
        fs::create_dir_all(&contract_dir).expect("mkdir");
        fs::write(contract_dir.join("c.tenor"), minimal_tenor_contract()).expect("write tenor");

        let assert = tenor_cmd()
            .args(["pack", root.to_str().unwrap()])
            .assert()
            .failure();

        let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
        assert!(
            stderr.contains(expected_in_stderr),
            "expected '{}' in stderr, got: {}",
            expected_in_stderr,
            stderr
        );
    };

    // Invalid name (contains uppercase)
    assert_pack_fails(
        r#"[template]
name = "Bad Name"
version = "1.0.0"
description = "desc"
author = "author"
category = "cat"
"#,
        "name",
    );

    // Invalid version format
    assert_pack_fails(
        r#"[template]
name = "ok-name"
version = "not-semver"
description = "desc"
author = "author"
category = "cat"
"#,
        "version",
    );

    // Empty description
    assert_pack_fails(
        r#"[template]
name = "ok-name"
version = "1.0.0"
description = ""
author = "author"
category = "cat"
"#,
        "description",
    );
}
