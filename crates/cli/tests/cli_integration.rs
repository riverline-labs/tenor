//! CLI integration tests for all implemented subcommands.
//!
//! Uses `assert_cmd` to spawn the `tenor` binary and verify
//! exit codes, stdout content, and stderr content.
//!
//! All tests set `current_dir` to the workspace root so that relative
//! paths to conformance fixtures and test fixtures resolve correctly.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use predicates::prelude::*;
use rand::rngs::OsRng;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/cli -> workspace root is two levels up
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Helper: create a Command for the `tenor` binary, rooted at workspace.
fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    cmd
}

// ──────────────────────────────────────────────
// 1. Help and version
// ──────────────────────────────────────────────

#[test]
fn help_exits_0_with_description() {
    tenor()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Tenor contract language toolchain",
        ));
}

#[test]
fn version_exits_0() {
    tenor()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tenor"));
}

#[test]
fn elaborate_help_exits_0() {
    tenor()
        .args(["elaborate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file"));
}

// ──────────────────────────────────────────────
// 2. Elaborate subcommand
// ──────────────────────────────────────────────

#[test]
fn elaborate_valid_file_exits_0() {
    tenor()
        .args(["elaborate", "conformance/positive/fact_basic.tenor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"Bundle\""));
}

#[test]
fn elaborate_nonexistent_file_exits_1() {
    tenor()
        .args(["elaborate", "nonexistent_file_xyz.tenor"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn elaborate_negative_fixture_exits_1() {
    // A file that exists but has elaboration errors
    tenor()
        .args([
            "elaborate",
            "conformance/negative/pass4/unresolved_fact_ref.tenor",
        ])
        .assert()
        .failure()
        .code(1);
}

// ──────────────────────────────────────────────
// 3. Validate subcommand
// ──────────────────────────────────────────────

#[test]
fn validate_valid_bundle_exits_0() {
    tenor()
        .args(["validate", "conformance/positive/fact_basic.expected.json"])
        .assert()
        .success();
}

#[test]
fn validate_invalid_json_exits_1() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bad.json");
    fs::write(&path, r#"{"not": "a bundle"}"#).unwrap();

    tenor()
        .args(["validate", path.to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}

// ──────────────────────────────────────────────
// 4. Test subcommand
// ──────────────────────────────────────────────

#[test]
fn test_conformance_exits_0() {
    tenor()
        .args(["test", "conformance"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn test_nonexistent_dir_exits_1() {
    tenor()
        .args(["test", "nonexistent_suite_dir_xyz"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

// ──────────────────────────────────────────────
// 5. Eval subcommand
// ──────────────────────────────────────────────

#[test]
fn eval_valid_fixtures_exits_0() {
    tenor()
        .args([
            "eval",
            "crates/cli/tests/fixtures/eval_basic_bundle.json",
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("verdict"));
}

#[test]
fn eval_json_output_contains_verdicts() {
    tenor()
        .args([
            "eval",
            "crates/cli/tests/fixtures/eval_basic_bundle.json",
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"verdicts\""));
}

#[test]
fn eval_nonexistent_bundle_exits_1() {
    tenor()
        .args([
            "eval",
            "nonexistent_bundle.json",
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("bundle file not found"));
}

#[test]
fn eval_nonexistent_facts_exits_1() {
    tenor()
        .args([
            "eval",
            "crates/cli/tests/fixtures/eval_basic_bundle.json",
            "--facts",
            "nonexistent_facts.json",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("facts file not found"));
}

#[test]
fn eval_missing_facts_flag_exits_with_clap_error() {
    // Missing the required --facts argument
    tenor()
        .args(["eval", "crates/cli/tests/fixtures/eval_basic_bundle.json"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--facts"));
}

#[test]
fn eval_invalid_json_bundle_exits_1() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bad.json");
    fs::write(&path, "not json at all").unwrap();

    tenor()
        .args([
            "eval",
            path.to_str().unwrap(),
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid JSON"));
}

// ──────────────────────────────────────────────
// 6. Diff subcommand
// ──────────────────────────────────────────────

#[test]
fn diff_identical_files_exits_0() {
    tenor()
        .args([
            "diff",
            "conformance/positive/fact_basic.expected.json",
            "conformance/positive/fact_basic.expected.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("no differences"));
}

#[test]
fn diff_different_files_exits_1() {
    tenor()
        .args([
            "diff",
            "conformance/positive/fact_basic.expected.json",
            "conformance/positive/rule_basic.expected.json",
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn diff_fact_added_shows_addition() {
    // Create two bundles: one minimal, one with an extra fact
    let tmp = TempDir::new().unwrap();
    let bundle1 = serde_json::json!({
        "kind": "Bundle", "id": "test_contract", "tenor": "1.0",
        "constructs": [
            { "kind": "Fact", "id": "amount", "type": { "base": "Int" },
              "source": { "system": "billing", "field": "amt" },
              "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0" }
        ]
    });
    let bundle2 = serde_json::json!({
        "kind": "Bundle", "id": "test_contract", "tenor": "1.0",
        "constructs": [
            { "kind": "Fact", "id": "amount", "type": { "base": "Int" },
              "source": { "system": "billing", "field": "amt" },
              "provenance": { "file": "test.tenor", "line": 1 }, "tenor": "1.0" },
            { "kind": "Fact", "id": "is_active", "type": { "base": "Bool" },
              "source": { "system": "crm", "field": "active" },
              "provenance": { "file": "test.tenor", "line": 5 }, "tenor": "1.0" }
        ]
    });

    let path1 = tmp.path().join("bundle1.json");
    let path2 = tmp.path().join("bundle2.json");
    fs::write(&path1, serde_json::to_string_pretty(&bundle1).unwrap()).unwrap();
    fs::write(&path2, serde_json::to_string_pretty(&bundle2).unwrap()).unwrap();

    // Diff should detect the addition and exit 1
    tenor()
        .args(["diff", path1.to_str().unwrap(), path2.to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn diff_breaking_non_breaking_changes() {
    // Identical bundles should exit 0 with --breaking
    tenor()
        .args([
            "diff",
            "--breaking",
            "conformance/positive/fact_basic.expected.json",
            "conformance/positive/fact_basic.expected.json",
        ])
        .assert()
        .success();
}

#[test]
fn diff_invalid_file_exits_1() {
    let tmp = TempDir::new().unwrap();
    let bad_path = tmp.path().join("not_json.txt");
    fs::write(&bad_path, "this is not JSON at all").unwrap();

    tenor()
        .args([
            "diff",
            bad_path.to_str().unwrap(),
            "conformance/positive/fact_basic.expected.json",
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn diff_nonexistent_file_exits_1() {
    tenor()
        .args([
            "diff",
            "nonexistent_file_xyz.json",
            "conformance/positive/fact_basic.expected.json",
        ])
        .assert()
        .failure()
        .code(1);
}

// ──────────────────────────────────────────────
// 7. Check subcommand
// ──────────────────────────────────────────────

#[test]
fn check_valid_file_exits_0() {
    tenor()
        .args(["check", "conformance/positive/entity_basic.tenor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Static Analysis Report"))
        .stdout(predicate::str::contains("Entities:"))
        .stdout(predicate::str::contains("No findings."));
}

#[test]
fn check_json_output() {
    tenor()
        .args([
            "check",
            "conformance/positive/entity_basic.tenor",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"s1_state_space\""))
        .stdout(predicate::str::contains("\"analyses_run\""));
}

#[test]
fn check_selected_analysis() {
    tenor()
        .args([
            "check",
            "conformance/positive/entity_basic.tenor",
            "--analysis",
            "s1,s2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Entities:"))
        .stdout(predicate::str::contains("Reachability:"));
}

#[test]
fn check_nonexistent_file_exits_1() {
    tenor()
        .args(["check", "nonexistent_file_xyz.tenor"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn check_invalid_analysis_exits_1() {
    tenor()
        .args([
            "check",
            "conformance/positive/entity_basic.tenor",
            "--analysis",
            "s99",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid analysis"));
}

// ──────────────────────────────────────────────
// 8. Explain subcommand
// ──────────────────────────────────────────────

#[test]
fn explain_tenor_file_exits_0() {
    tenor()
        .args(["explain", "domains/saas/saas_subscription.tenor"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("CONTRACT SUMMARY")
                .or(predicate::str::contains("Contract Summary")),
        );
}

#[test]
fn explain_json_bundle_exits_0() {
    // First elaborate to get JSON
    let elaborate_output = tenor()
        .args(["elaborate", "domains/saas/saas_subscription.tenor"])
        .output()
        .expect("elaborate failed");
    assert!(elaborate_output.status.success());

    // Write to temp file
    let tmp = TempDir::new().unwrap();
    let json_path = tmp.path().join("saas.json");
    fs::write(&json_path, &elaborate_output.stdout).unwrap();

    // Explain the JSON bundle
    tenor()
        .args(["explain", json_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("CONTRACT SUMMARY"));
}

#[test]
fn explain_markdown_format() {
    tenor()
        .args([
            "explain",
            "domains/saas/saas_subscription.tenor",
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## CONTRACT SUMMARY"))
        .stdout(predicate::str::contains("## DECISION FLOW NARRATIVE"))
        .stdout(predicate::str::contains("## FACT INVENTORY"))
        .stdout(predicate::str::contains("## RISK / COVERAGE NOTES"));
}

#[test]
fn explain_verbose_flag() {
    tenor()
        .args([
            "explain",
            "domains/saas/saas_subscription.tenor",
            "--verbose",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stratum"))
        .stdout(predicate::str::contains("states:"));
}

#[test]
fn explain_missing_file_exits_1() {
    tenor()
        .args(["explain", "nonexistent_file_xyz.tenor"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn explain_invalid_json_exits_1() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bad.json");
    fs::write(&path, "not valid json at all").unwrap();

    tenor()
        .args(["explain", path.to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}

// ──────────────────────────────────────────────
// 8b. Explain section completeness (drift detection)
// ──────────────────────────────────────────────

/// Verify that explain terminal output contains all expected sections
/// with non-empty content. If a section silently disappears due to
/// interchange format drift, this test will catch it.
#[test]
fn explain_terminal_sections_present_and_nonempty() {
    let output = tenor()
        .args(["explain", "domains/saas/saas_subscription.tenor"])
        .output()
        .expect("explain failed");

    assert!(output.status.success(), "explain should exit 0");
    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");

    // All four sections must be present
    assert!(
        stdout.contains("CONTRACT SUMMARY"),
        "missing CONTRACT SUMMARY section"
    );
    assert!(
        stdout.contains("DECISION FLOW NARRATIVE"),
        "missing DECISION FLOW NARRATIVE section"
    );
    assert!(
        stdout.contains("FACT INVENTORY"),
        "missing FACT INVENTORY section"
    );
    assert!(
        stdout.contains("RISK / COVERAGE NOTES"),
        "missing RISK / COVERAGE NOTES section"
    );

    // Contract summary has content (entity, persona, rule, operation counts)
    assert!(
        stdout.contains("Name:"),
        "CONTRACT SUMMARY missing Name line"
    );
    assert!(
        stdout.contains("Entities:"),
        "CONTRACT SUMMARY missing Entities line"
    );
    assert!(
        stdout.contains("Personas:"),
        "CONTRACT SUMMARY missing Personas line"
    );
    assert!(
        stdout.contains("Rules:"),
        "CONTRACT SUMMARY missing Rules line"
    );
    assert!(
        stdout.contains("Operations:"),
        "CONTRACT SUMMARY missing Operations line"
    );
    assert!(
        stdout.contains("Facts:"),
        "CONTRACT SUMMARY missing Facts line"
    );
    assert!(
        stdout.contains("Flows:"),
        "CONTRACT SUMMARY missing Flows line"
    );

    // Flow narrative has flow content
    assert!(
        stdout.contains("Flow:"),
        "DECISION FLOW NARRATIVE has no flow entries"
    );
    assert!(
        stdout.contains("Entry point:"),
        "DECISION FLOW NARRATIVE missing Entry point"
    );

    // Fact inventory has table content
    assert!(
        stdout.contains("FACT") && stdout.contains("TYPE") && stdout.contains("SOURCE"),
        "FACT INVENTORY missing table header"
    );

    // Risk coverage has analysis results
    assert!(
        stdout.contains("[ok]") || stdout.contains("[!!]"),
        "RISK / COVERAGE NOTES has no analysis results"
    );
}

/// Verify that explain markdown output contains all expected sections
/// with proper markdown formatting.
#[test]
fn explain_markdown_sections_present_and_nonempty() {
    let output = tenor()
        .args([
            "explain",
            "domains/saas/saas_subscription.tenor",
            "--format",
            "markdown",
        ])
        .output()
        .expect("explain failed");

    assert!(
        output.status.success(),
        "explain --format markdown should exit 0"
    );
    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");

    // All four sections as markdown headers
    assert!(
        stdout.contains("## CONTRACT SUMMARY"),
        "missing ## CONTRACT SUMMARY"
    );
    assert!(
        stdout.contains("## DECISION FLOW NARRATIVE"),
        "missing ## DECISION FLOW NARRATIVE"
    );
    assert!(
        stdout.contains("## FACT INVENTORY"),
        "missing ## FACT INVENTORY"
    );
    assert!(
        stdout.contains("## RISK / COVERAGE NOTES"),
        "missing ## RISK / COVERAGE NOTES"
    );

    // Markdown-specific formatting: backtick-quoted names, table
    assert!(
        stdout.contains('`'),
        "markdown output should contain backtick-quoted identifiers"
    );
    assert!(
        stdout.contains("| Fact |"),
        "FACT INVENTORY should use markdown table"
    );
    assert!(
        stdout.contains("|---"),
        "FACT INVENTORY should have markdown table separator"
    );

    // Markdown risk section uses checkbox syntax
    assert!(
        stdout.contains("- [x]") || stdout.contains("- [ ]"),
        "RISK / COVERAGE NOTES should use markdown checkbox syntax"
    );
}

// ──────────────────────────────────────────────
// 9. Generate subcommand
// ──────────────────────────────────────────────

#[test]
fn generate_typescript_from_json_exits_0() {
    let dir = tempfile::tempdir().expect("temp dir");
    tenor()
        .args([
            "generate",
            "typescript",
            "conformance/positive/operation_basic.expected.json",
            "--out",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated TypeScript"));
}

#[test]
fn generate_typescript_from_tenor_exits_0() {
    let dir = tempfile::tempdir().expect("temp dir");
    tenor()
        .args([
            "generate",
            "typescript",
            "conformance/positive/operation_basic.tenor",
            "--out",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated TypeScript"));
}

#[test]
fn generate_typescript_nonexistent_file_exits_1() {
    let dir = tempfile::tempdir().expect("temp dir");
    tenor()
        .args([
            "generate",
            "typescript",
            "nonexistent.json",
            "--out",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn generate_typescript_unsupported_type_exits_1() {
    let dir = tempfile::tempdir().expect("temp dir");
    tenor()
        .args([
            "generate",
            "typescript",
            "README.md",
            "--out",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unsupported input file type"));
}

// ──────────────────────────────────────────────
// 8. Global flags
// ──────────────────────────────────────────────

#[test]
fn elaborate_quiet_suppresses_output_on_error() {
    // With --quiet, errors should not produce output to stderr
    tenor()
        .args([
            "--quiet",
            "elaborate",
            "conformance/negative/pass4/unresolved_fact_ref.tenor",
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty());
}

#[test]
fn elaborate_json_output_format() {
    tenor()
        .args([
            "--output",
            "json",
            "elaborate",
            "conformance/positive/fact_basic.tenor",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"Bundle\""));
}

#[test]
fn eval_quiet_suppresses_output_on_success() {
    tenor()
        .args([
            "--quiet",
            "eval",
            "crates/cli/tests/fixtures/eval_basic_bundle.json",
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn eval_text_output_shows_verdicts() {
    tenor()
        .args([
            "--output",
            "text",
            "eval",
            "crates/cli/tests/fixtures/eval_basic_bundle.json",
            "--facts",
            "crates/cli/tests/fixtures/eval_basic.facts.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("verdict(s) produced"));
}

// ──────────────────────────────────────────────
// Executor conformance — E10, E12 (Phase 5)
// ──────────────────────────────────────────────

/// E10: Manifest Serving — Valid Schema.
/// Validates that `tenor elaborate --manifest` produces a well-formed
/// TenorManifest with the required top-level keys (bundle, etag, tenor),
/// correct etag format, correct tenor version, and valid bundle structure.
#[test]
fn e10_manifest_valid_schema() {
    // Run `tenor elaborate --manifest` on a domain contract
    let output = tenor()
        .args([
            "elaborate",
            "--manifest",
            "domains/saas/saas_subscription.tenor",
        ])
        .output()
        .expect("failed to execute");

    assert!(
        output.status.success(),
        "elaborate --manifest failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    let manifest: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest is not valid JSON");

    // Top-level keys: bundle, etag, tenor (all required by manifest-schema.json)
    assert!(
        manifest.get("bundle").is_some(),
        "manifest missing 'bundle' key"
    );
    assert!(
        manifest.get("etag").is_some(),
        "manifest missing 'etag' key"
    );
    assert!(
        manifest.get("tenor").is_some(),
        "manifest missing 'tenor' key"
    );

    // etag is a lowercase hex string, 64 chars (SHA-256)
    let etag = manifest["etag"].as_str().expect("etag is not a string");
    assert_eq!(
        etag.len(),
        64,
        "etag should be 64 hex chars, got {}",
        etag.len()
    );
    assert!(
        etag.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
        "etag should be lowercase hex, got: {}",
        etag
    );

    // tenor version is "1.0"
    let tenor_version = manifest["tenor"].as_str().expect("tenor is not a string");
    assert_eq!(tenor_version, "1.0", "manifest tenor version should be 1.0");

    // bundle has kind: "Bundle"
    let bundle_kind = manifest["bundle"]["kind"]
        .as_str()
        .expect("bundle.kind is not a string");
    assert_eq!(bundle_kind, "Bundle", "bundle.kind should be 'Bundle'");
}

/// E12: Change Detection — Etag Determinism.
/// Same contract elaborated twice must produce the same etag.
#[test]
fn e12_etag_determinism() {
    let contract = "domains/saas/saas_subscription.tenor";

    // First elaboration
    let out1 = tenor()
        .args(["elaborate", "--manifest", contract])
        .output()
        .expect("first elaboration failed");
    assert!(out1.status.success());

    let json1: serde_json::Value =
        serde_json::from_slice(&out1.stdout).expect("first output not JSON");
    let etag1 = json1["etag"].as_str().expect("first etag missing");

    // Second elaboration
    let out2 = tenor()
        .args(["elaborate", "--manifest", contract])
        .output()
        .expect("second elaboration failed");
    assert!(out2.status.success());

    let json2: serde_json::Value =
        serde_json::from_slice(&out2.stdout).expect("second output not JSON");
    let etag2 = json2["etag"].as_str().expect("second etag missing");

    assert_eq!(
        etag1, etag2,
        "same contract should produce identical etags across elaborations"
    );
}

/// E12: Change Detection — Different contracts produce different etags.
#[test]
fn e12_etag_change_detection() {
    // Elaborate SaaS contract
    let out1 = tenor()
        .args([
            "elaborate",
            "--manifest",
            "domains/saas/saas_subscription.tenor",
        ])
        .output()
        .expect("saas elaboration failed");
    assert!(out1.status.success());

    let json1: serde_json::Value =
        serde_json::from_slice(&out1.stdout).expect("saas output not JSON");
    let etag1 = json1["etag"].as_str().expect("saas etag missing");

    // Elaborate healthcare contract
    let out2 = tenor()
        .args([
            "elaborate",
            "--manifest",
            "domains/healthcare/prior_auth.tenor",
        ])
        .output()
        .expect("healthcare elaboration failed");
    assert!(out2.status.success());

    let json2: serde_json::Value =
        serde_json::from_slice(&out2.stdout).expect("healthcare output not JSON");
    let etag2 = json2["etag"].as_str().expect("healthcare etag missing");

    assert_ne!(
        etag1, etag2,
        "different contracts must produce different etags"
    );
}

// ──────────────────────────────────────────────
// WASM binary signing and verification
// ──────────────────────────────────────────────

/// Helper: generate an Ed25519 keypair and write the .secret and .pub files.
///
/// Returns (secret_key_path, pub_key_path).
fn write_test_keypair(dir: &tempfile::TempDir) -> (PathBuf, PathBuf) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // .secret file: base64-encoded 32-byte seed
    let seed_b64 = BASE64.encode(signing_key.to_bytes());
    let secret_path = dir.path().join("test-key.secret");
    fs::write(&secret_path, &seed_b64).expect("write secret key");

    // .pub file: base64-encoded 32-byte public key
    let pub_b64 = BASE64.encode(verifying_key.to_bytes());
    let pub_path = dir.path().join("test-key.pub");
    fs::write(&pub_path, &pub_b64).expect("write public key");

    (secret_path, pub_path)
}

/// sign-wasm produces a valid .sig file containing all expected fields.
#[test]
fn sign_wasm_produces_sig_file() {
    let dir = TempDir::new().unwrap();
    let (secret_path, _pub_path) = write_test_keypair(&dir);

    // Write a fake WASM binary.
    let wasm_path = dir.path().join("evaluator.wasm");
    fs::write(&wasm_path, b"fake wasm binary content for testing").unwrap();

    let bundle_etag = "abc123etag";

    tenor()
        .args([
            "sign-wasm",
            wasm_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--bundle-etag",
            bundle_etag,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Signed WASM binary:"))
        .stdout(predicate::str::contains("WASM hash:"))
        .stdout(predicate::str::contains("Bundle etag: abc123etag"));

    // Verify the .sig file was created and has expected fields.
    let sig_path = dir.path().join("evaluator.wasm.sig");
    assert!(sig_path.exists(), ".sig file must exist");

    let sig_str = fs::read_to_string(&sig_path).unwrap();
    let sig_json: serde_json::Value = serde_json::from_str(&sig_str).unwrap();
    assert_eq!(sig_json["attestation_format"], "ed25519-detached");
    assert_eq!(sig_json["bundle_etag"], bundle_etag);
    assert!(
        sig_json["wasm_hash"].is_string(),
        "wasm_hash must be present"
    );
    assert!(
        sig_json["signature"].is_string(),
        "signature must be present"
    );
    assert!(
        sig_json["signer_public_key"].is_string(),
        "signer_public_key must be present"
    );
}

/// verify-wasm succeeds for a valid signed binary.
#[test]
fn verify_wasm_succeeds_for_valid_binary() {
    let dir = TempDir::new().unwrap();
    let (secret_path, pub_path) = write_test_keypair(&dir);

    let wasm_path = dir.path().join("evaluator.wasm");
    fs::write(&wasm_path, b"fake wasm binary content for testing").unwrap();

    // Sign first.
    tenor()
        .args([
            "sign-wasm",
            wasm_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--bundle-etag",
            "etag-v1",
        ])
        .assert()
        .success();

    let sig_path = dir.path().join("evaluator.wasm.sig");

    // Verify.
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
        .success()
        .stdout(predicate::str::contains("WASM binary verified:"))
        .stdout(predicate::str::contains("Signature: valid"));
}

/// verify-wasm fails when the WASM binary has been tampered with.
#[test]
fn verify_wasm_fails_for_tampered_binary() {
    let dir = TempDir::new().unwrap();
    let (secret_path, pub_path) = write_test_keypair(&dir);

    let wasm_path = dir.path().join("evaluator.wasm");
    fs::write(&wasm_path, b"original wasm binary content").unwrap();

    // Sign the original.
    tenor()
        .args([
            "sign-wasm",
            wasm_path.to_str().unwrap(),
            "--key",
            secret_path.to_str().unwrap(),
            "--bundle-etag",
            "etag-v1",
        ])
        .assert()
        .success();

    let sig_path = dir.path().join("evaluator.wasm.sig");

    // Tamper with the binary.
    fs::write(&wasm_path, b"tampered wasm binary content!!").unwrap();

    // Verify should fail.
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
        .failure()
        .stderr(predicate::str::contains("hash mismatch"));
}

/// sign-wasm help shows expected options.
#[test]
fn sign_wasm_help_shows_options() {
    tenor()
        .args(["sign-wasm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--key"))
        .stdout(predicate::str::contains("--bundle-etag"));
}

/// verify-wasm help shows expected options.
#[test]
fn verify_wasm_help_shows_options() {
    tenor()
        .args(["verify-wasm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--sig"))
        .stdout(predicate::str::contains("--pubkey"));
}
