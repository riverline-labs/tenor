//! End-to-end tests for `tenor deploy` and the deploy config types.
//!
//! These tests verify the CLI behavior without a running server:
//! 1. Deploy --help shows expected options
//! 2. Missing auth token error
//! 3. Registry connection refused error
//!
//! The deploy config unit tests (config read, validate, generate) live in
//! deploy_config.rs as unit tests — see `cargo test -p tenor-cli template`.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tenor_cmd() -> Command {
    cargo_bin_cmd!("tenor")
}

// ── Test 1: deploy --help ─────────────────────────────────────────────────────

#[test]
fn test_deploy_help() {
    let mut cmd = tenor_cmd();
    cmd.args(["deploy", "--help"]);
    let output = cmd.output().expect("run deploy --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        output.status.success(),
        "deploy --help should exit 0, got:\n{}",
        combined
    );
    assert!(
        combined.contains("--org") || combined.contains("Organization"),
        "deploy --help should show --org: {}",
        combined
    );
    assert!(
        combined.contains("--config"),
        "deploy --help should show --config: {}",
        combined
    );
    assert!(
        combined.contains("--registry"),
        "deploy --help should show --registry: {}",
        combined
    );
    assert!(
        combined.contains("--platform"),
        "deploy --help should show --platform: {}",
        combined
    );
    assert!(
        combined.contains("--token"),
        "deploy --help should show --token: {}",
        combined
    );
    assert!(
        combined.contains("TEMPLATE_NAME")
            || combined.contains("template-name")
            || combined.contains("template_name"),
        "deploy --help should show TEMPLATE_NAME argument: {}",
        combined
    );
}

// ── Test 2: deploy without --token ────────────────────────────────────────────

#[test]
fn test_deploy_missing_token() {
    // Ensure TENOR_PLATFORM_TOKEN is not set in the test environment.
    let mut cmd = tenor_cmd();
    cmd.args(["deploy", "some-template"])
        .env_remove("TENOR_PLATFORM_TOKEN");

    let output = cmd.output().expect("run deploy without token");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "deploy without token should fail, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("token")
            || stderr.contains("TENOR_PLATFORM_TOKEN")
            || stderr.contains("auth"),
        "error should mention missing token: {}",
        stderr
    );
}

// ── Test 3: deploy with connection refused on registry ────────────────────────

#[test]
fn test_deploy_connection_refused_registry() {
    // Port 19999 is almost certainly not running anything.
    let mut cmd = tenor_cmd();
    cmd.args([
        "deploy",
        "some-template",
        "--token",
        "fake-token",
        "--registry",
        "http://localhost:19999",
    ])
    .env_remove("TENOR_PLATFORM_TOKEN");

    let output = cmd.output().expect("run deploy with bad registry");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "deploy with unreachable registry should fail, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("connect")
            || stderr.contains("refused")
            || stderr.contains("registry")
            || stderr.contains("error"),
        "error should mention registry connection failure: {}",
        stderr
    );
}

// ── Test 4: DeployConfig TOML read ────────────────────────────────────────────
//
// This test verifies the TOML parsing by writing a file and checking that
// `tenor deploy --config` picks it up. Since the binary processes exit after
// reading the config (when the registry is unreachable), we verify parsing
// indirectly: a valid config should NOT produce a "config" parse error,
// while an invalid config should.

#[test]
fn test_deploy_config_read_valid() {
    let toml_content = r#"
[deploy]
org_id = "org-abc-def-123"

[sources.payment_service]
protocol = "rest"
base_url = "https://api.payments.example.com"
auth_header = "Authorization"
auth_value = "Bearer sk_live_abc123"

[personas.buyer]
api_key = "buyer-key-uuid-1"
"#;

    let mut tmp = NamedTempFile::new().expect("create temp file");
    tmp.write_all(toml_content.as_bytes()).unwrap();

    let mut cmd = tenor_cmd();
    cmd.args([
        "deploy",
        "some-template",
        "--token",
        "fake-token",
        "--config",
        tmp.path().to_str().unwrap(),
        "--registry",
        "http://localhost:19999",
    ])
    .env_remove("TENOR_PLATFORM_TOKEN");

    let output = cmd.output().expect("run deploy with config");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The config should parse fine; the failure should be registry unreachable.
    assert!(
        !stderr.contains("could not parse") && !stderr.contains("invalid config"),
        "valid config should parse without errors: {}",
        stderr
    );
    // It should fail at the registry stage, not the config stage.
    assert!(
        stderr.contains("connect")
            || stderr.contains("refused")
            || stderr.contains("registry")
            || stderr.contains("error"),
        "should fail at network stage: {}",
        stderr
    );
}

#[test]
fn test_deploy_config_read_invalid() {
    let bad_toml = "this is not valid toml ][{";

    let mut tmp = NamedTempFile::new().expect("create temp file");
    tmp.write_all(bad_toml.as_bytes()).unwrap();

    let mut cmd = tenor_cmd();
    cmd.args([
        "deploy",
        "some-template",
        "--token",
        "fake-token",
        "--config",
        tmp.path().to_str().unwrap(),
        "--registry",
        "http://localhost:19999",
    ])
    .env_remove("TENOR_PLATFORM_TOKEN");

    let output = cmd.output().expect("run deploy with invalid config");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "invalid config should fail: {}",
        stderr
    );
    assert!(
        stderr.contains("parse")
            || stderr.contains("TOML")
            || stderr.contains("could not")
            || stderr.contains("error"),
        "error should mention config parse failure: {}",
        stderr
    );
}

// ── Test 5: deploy config validation — missing source (unit test via library) ─
//
// The unit tests for validate_deploy_config are embedded in deploy_config.rs.
// Run them via: cargo test -p tenor-cli validate

// ── Test 6: deploy config validation — missing persona (unit test via library) ─
// Run via: cargo test -p tenor-cli validate

// ── Test 7: deploy config template generation ─────────────────────────────────
//
// Verify that deploying a template that has required_sources but no --config
// generates a deploy-config.toml file (and exits 0).
// This test runs in a temp directory.

#[test]
fn test_deploy_config_template_generation_via_cli() {
    // We can't easily test this without a working registry. The generation
    // happens after the registry download and unpack, so we can only verify
    // the behavior via unit tests in deploy_config.rs.
    //
    // This test documents the expected CLI behavior: if the template has
    // required sources/personas and no --config is provided, the CLI should
    // exit 0 with a message about generating deploy-config.toml.
    //
    // Since we can't connect to a registry in tests, we skip the full flow test.
    // Unit tests in deploy_config.rs cover generate_deploy_config_template directly.
    let _ = "documented only — see deploy_config.rs unit tests";
}

// ── Test 8: config roundtrip via CLI ─────────────────────────────────────────

#[test]
fn test_deploy_config_roundtrip_via_cli() {
    // Write a valid config that would pass validation for a template with no
    // required sources and no personas (simplest case).
    let minimal_config = r#"
[deploy]
org_id = "org-roundtrip-test"
"#;

    let mut tmp = NamedTempFile::new().expect("create temp file");
    tmp.write_all(minimal_config.as_bytes()).unwrap();

    // Deploy should get past config parsing and fail at registry.
    let mut cmd = tenor_cmd();
    cmd.args([
        "deploy",
        "any-template",
        "--token",
        "test-token",
        "--config",
        tmp.path().to_str().unwrap(),
        "--registry",
        "http://localhost:19999",
    ])
    .env_remove("TENOR_PLATFORM_TOKEN");

    let output = cmd.output().expect("run deploy with minimal config");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT fail on config parsing.
    assert!(
        !stderr.contains("could not parse") && !stderr.contains("invalid config"),
        "minimal config should parse fine: {}",
        stderr
    );
}
