//! End-to-end tests for `tenor publish`, `tenor search`, and `tenor install`.
//!
//! These tests do NOT require a running registry server. They verify:
//! - Argument parsing (commands exist and --help works)
//! - Pre-flight validation (missing token, missing manifest)
//! - Graceful error handling for network failures (connection refused)

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Return a Command for the `tenor` binary.
fn tenor_cmd() -> Command {
    Command::cargo_bin("tenor").expect("tenor binary")
}

/// Build a minimal valid template directory.
fn build_valid_template_dir(name: &str) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();

    let manifest = format!(
        r#"[template]
name = "{name}"
version = "1.0.0"
description = "A minimal test template"
author = "Test Author"
category = "test"
tags = ["test"]
"#
    );
    fs::write(root.join("tenor-template.toml"), manifest).expect("write manifest");

    let contract_dir = root.join("contract");
    fs::create_dir_all(&contract_dir).expect("mkdir contract");
    fs::write(
        contract_dir.join("hello.tenor"),
        r#"fact is_active {
  type:   Bool
  source: "service.active"
}

rule check_active {
  stratum: 0
  when:    is_active = true
  produce: verdict is_active { payload: Bool = true }
}
"#,
    )
    .expect("write tenor");

    tmp
}

// ─── Tests: tenor publish ─────────────────────────────────────────────────────

/// `tenor publish` without --token and without TENOR_REGISTRY_TOKEN env var
/// must fail with a clear auth error message.
#[test]
fn test_publish_missing_token() {
    let tmp = build_valid_template_dir("test-template");

    let assert = tenor_cmd()
        .args(["publish", tmp.path().to_str().unwrap()])
        // Ensure env var is not inherited
        .env_remove("TENOR_REGISTRY_TOKEN")
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    assert!(
        stderr.contains("token") || stderr.contains("TENOR_REGISTRY_TOKEN"),
        "expected auth error mentioning token, got: {}",
        stderr
    );
}

/// `tenor publish` in a directory without `tenor-template.toml` must fail
/// with an informative message about the missing manifest.
#[test]
fn test_publish_no_manifest() {
    let tmp = TempDir::new().expect("tempdir");
    // No tenor-template.toml created

    let assert = tenor_cmd()
        .args([
            "publish",
            tmp.path().to_str().unwrap(),
            "--token",
            "fake-token",
        ])
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    assert!(
        stderr.contains("tenor-template.toml") || stderr.contains("manifest"),
        "expected error mentioning tenor-template.toml, got: {}",
        stderr
    );
}

/// `tenor search --help` must exit successfully and show the expected options.
#[test]
fn test_search_help() {
    let output = tenor_cmd()
        .args(["search", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = std::str::from_utf8(&output).unwrap_or("");
    assert!(
        stdout.contains("QUERY") || stdout.contains("query"),
        "help should mention QUERY argument, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--category"),
        "help should mention --category, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--tag"),
        "help should mention --tag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--registry"),
        "help should mention --registry, got: {}",
        stdout
    );
}

/// `tenor install --help` must exit successfully and show the expected options.
#[test]
fn test_install_help() {
    let output = tenor_cmd()
        .args(["install", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = std::str::from_utf8(&output).unwrap_or("");
    assert!(
        stdout.contains("TEMPLATE_NAME") || stdout.contains("template"),
        "help should mention TEMPLATE_NAME, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--version"),
        "help should mention --version, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--out") || stdout.contains("output"),
        "help should mention --out (output directory), got: {}",
        stdout
    );
    assert!(
        stdout.contains("--registry"),
        "help should mention --registry, got: {}",
        stdout
    );
}

/// `tenor publish` against a port with nothing listening should fail with a
/// connection error — not a panic.
#[test]
fn test_publish_connection_refused() {
    let tmp = build_valid_template_dir("conn-refused-pub");

    // Port 19999 — very unlikely to have a server running on CI
    let assert = tenor_cmd()
        .args([
            "publish",
            tmp.path().to_str().unwrap(),
            "--registry",
            "http://localhost:19999",
            "--token",
            "fake-token",
        ])
        .env_remove("TENOR_REGISTRY_TOKEN")
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    // Must not be empty — a clear error message is required
    assert!(
        !stderr.is_empty(),
        "expected a non-empty connection error message"
    );
    // Should mention connection or registry
    assert!(
        stderr.contains("connect") || stderr.contains("error") || stderr.contains("registry"),
        "expected connection error, got: {}",
        stderr
    );
}

/// `tenor search` against a port with nothing listening should fail gracefully.
#[test]
fn test_search_connection_refused() {
    let assert = tenor_cmd()
        .args(["search", "test", "--registry", "http://localhost:19999"])
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    assert!(
        !stderr.is_empty(),
        "expected a non-empty connection error message"
    );
    assert!(
        stderr.contains("connect") || stderr.contains("error") || stderr.contains("registry"),
        "expected connection error, got: {}",
        stderr
    );
}

/// `tenor install` against a port with nothing listening should fail gracefully.
#[test]
fn test_install_connection_refused() {
    let out_dir = TempDir::new().expect("output tempdir");

    let assert = tenor_cmd()
        .args([
            "install",
            "test-template",
            "--registry",
            "http://localhost:19999",
            "--out",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap_or("");
    assert!(
        !stderr.is_empty(),
        "expected a non-empty connection error message"
    );
    assert!(
        stderr.contains("connect") || stderr.contains("error") || stderr.contains("registry"),
        "expected connection error, got: {}",
        stderr
    );
}
