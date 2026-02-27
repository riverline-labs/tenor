//! End-to-end tests for `tenor connect` command.
//!
//! Tests the full workflow: elaborate contract, introspect OpenAPI schema,
//! heuristic matching, batch review, apply, and dry-run modes.
//!
//! All tests use `--heuristic` so no API key is required.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
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

/// Helper: create a Command for the `tenor` binary, rooted at workspace.
fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    // Ensure no API key leaks into heuristic tests
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

/// Sample escrow contract with Source declarations and Facts.
const ESCROW_CONTRACT: &str = r#"source escrow_service {
  protocol: http
  base_url: "https://api.escrow.example.com"
  schema_ref: "escrow-api.json"
}

fact escrow_balance {
  type: Int
  source: escrow_service { path: "accounts.balance" }
}

fact order_status {
  type: Text
  source: escrow_service { path: "orders.status" }
}
"#;

/// Sample OpenAPI 3.0 spec matching the escrow contract.
fn escrow_openapi_spec() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "Escrow API", "version": "1.0" },
        "paths": {
            "/accounts/{id}": {
                "get": {
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "balance": { "type": "integer" },
                                            "currency": { "type": "string" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/orders/{id}": {
                "get": {
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": { "type": "string" },
                                            "amount": { "type": "number" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Write the escrow contract and OpenAPI spec into a temp directory.
/// Returns (tmp_dir, contract_path, openapi_path).
fn setup_fixtures() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().expect("temp dir");

    let contract_path = tmp.path().join("escrow.tenor");
    fs::write(&contract_path, ESCROW_CONTRACT).expect("write contract");

    let openapi_path = tmp.path().join("escrow-api.json");
    let spec = escrow_openapi_spec();
    fs::write(&openapi_path, serde_json::to_string_pretty(&spec).unwrap())
        .expect("write openapi spec");

    (tmp, contract_path, openapi_path)
}

// ──────────────────────────────────────────────
// 1. Batch mode: generates review TOML
// ──────────────────────────────────────────────

#[test]
fn connect_batch_heuristic_creates_review_file() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let out_dir = tmp.path().join("batch-output");

    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--batch",
            "--heuristic",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Review TOML must exist
    let review_path = out_dir.join("tenor-connect-review.toml");
    assert!(
        review_path.exists(),
        "review file should exist at {}",
        review_path.display()
    );

    // Parse the TOML and verify it has mapping entries
    let content = fs::read_to_string(&review_path).expect("read review file");
    let table: toml::Value = content.parse().expect("review file must be valid TOML");
    let mappings = table
        .get("mapping")
        .and_then(|v| v.as_array())
        .expect("review file should have [[mapping]] entries");

    // We have 2 facts in the contract, so there should be at least 2 proposals
    assert!(
        mappings.len() >= 2,
        "expected at least 2 mapping proposals, got {}",
        mappings.len()
    );

    // Each mapping should have required fields
    for mapping in mappings {
        assert!(mapping.get("fact_id").is_some(), "mapping missing fact_id");
        assert!(
            mapping.get("source_id").is_some(),
            "mapping missing source_id"
        );
        assert!(mapping.get("status").is_some(), "mapping missing status");
        // All proposals should start as "proposed"
        assert_eq!(
            mapping["status"].as_str().unwrap(),
            "proposed",
            "initial status should be 'proposed'"
        );
    }

    // Verify our fact IDs appear
    let fact_ids: Vec<&str> = mappings
        .iter()
        .filter_map(|m| m.get("fact_id").and_then(|v| v.as_str()))
        .collect();
    assert!(
        fact_ids.contains(&"escrow_balance"),
        "should contain escrow_balance proposal"
    );
    assert!(
        fact_ids.contains(&"order_status"),
        "should contain order_status proposal"
    );
}

// ──────────────────────────────────────────────
// 2. Full workflow: batch -> modify -> apply
// ──────────────────────────────────────────────

#[test]
fn connect_full_workflow_batch_then_apply() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let batch_dir = tmp.path().join("batch-out");
    let apply_dir = tmp.path().join("apply-out");

    // Step 1: Run batch mode to create review file
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--batch",
            "--heuristic",
            "--out",
            batch_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let review_path = batch_dir.join("tenor-connect-review.toml");
    assert!(review_path.exists(), "review file must exist after batch");

    // Step 2: Modify the review file - accept all proposals
    let content = fs::read_to_string(&review_path).expect("read review");
    let modified = content.replace("status = \"proposed\"", "status = \"accepted\"");
    fs::write(&review_path, &modified).expect("write modified review");

    // Step 3: Apply the accepted mappings
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--apply",
            review_path.to_str().unwrap(),
            "--out",
            apply_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Step 4: Verify generated files exist
    let adapter_config = apply_dir.join("tenor-adapters.toml");
    let test_file = apply_dir.join("adapter_tests.rs");
    let mappings_doc = apply_dir.join("MAPPINGS.md");

    assert!(
        adapter_config.exists(),
        "tenor-adapters.toml should exist at {}",
        adapter_config.display()
    );
    assert!(
        test_file.exists(),
        "adapter_tests.rs should exist at {}",
        test_file.display()
    );
    assert!(
        mappings_doc.exists(),
        "MAPPINGS.md should exist at {}",
        mappings_doc.display()
    );

    // Step 5: Verify adapter config is valid TOML
    let config_content = fs::read_to_string(&adapter_config).expect("read adapter config");
    let _: toml::Value = config_content
        .parse()
        .expect("tenor-adapters.toml must be valid TOML");

    // Verify config references our source
    assert!(
        config_content.contains("escrow_service"),
        "adapter config should reference escrow_service"
    );
    assert!(
        config_content.contains("protocol = \"http\""),
        "adapter config should contain protocol"
    );

    // Step 6: Verify test file has content for our facts
    let test_content = fs::read_to_string(&test_file).expect("read test file");
    assert!(
        test_content.contains("escrow_balance"),
        "test file should reference escrow_balance"
    );
    assert!(
        test_content.contains("order_status"),
        "test file should reference order_status"
    );

    // Step 7: Verify MAPPINGS.md has content
    let doc_content = fs::read_to_string(&mappings_doc).expect("read mappings doc");
    assert!(
        doc_content.contains("escrow_service"),
        "MAPPINGS.md should reference escrow_service"
    );
}

// ──────────────────────────────────────────────
// 3. Dry-run mode: outputs proposals without files
// ──────────────────────────────────────────────

#[test]
fn connect_dry_run_outputs_proposals_no_files() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let out_dir = tmp.path().join("dryrun-output");

    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--dry-run",
            "--heuristic",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        // Dry-run should print source and fact info
        .stdout(predicate::str::contains("escrow_service"))
        .stdout(predicate::str::contains("escrow_balance"))
        .stdout(predicate::str::contains("order_status"));

    // Dry-run should NOT create files
    assert!(
        !out_dir.join("tenor-adapters.toml").exists(),
        "dry-run should not create adapter config"
    );
    assert!(
        !out_dir.join("tenor-connect-review.toml").exists(),
        "dry-run should not create review file"
    );
}

// ──────────────────────────────────────────────
// 4. Dry-run JSON output
// ──────────────────────────────────────────────

#[test]
fn connect_dry_run_json_output() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let out_dir = tmp.path().join("dryrun-json-output");

    let output = tenor()
        .args([
            "--output",
            "json",
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--dry-run",
            "--heuristic",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("command execution");

    assert!(output.status.success(), "dry-run json should succeed");

    let stdout = String::from_utf8(output.stdout).expect("valid UTF-8");
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("dry-run json output should be valid JSON");

    // JSON output should have sources, facts, and mappings arrays
    assert!(
        json.get("sources").is_some(),
        "JSON output should have 'sources'"
    );
    assert!(
        json.get("facts").is_some(),
        "JSON output should have 'facts'"
    );
    assert!(
        json.get("mappings").is_some(),
        "JSON output should have 'mappings'"
    );
}

// ──────────────────────────────────────────────
// 5. Heuristic flag works without API key
// ──────────────────────────────────────────────

#[test]
fn connect_heuristic_works_without_api_key() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let out_dir = tmp.path().join("heuristic-output");

    // Explicitly remove ANTHROPIC_API_KEY to ensure heuristic mode
    tenor()
        .env_remove("ANTHROPIC_API_KEY")
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--batch",
            "--heuristic",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Should have created a review file successfully
    assert!(
        out_dir.join("tenor-connect-review.toml").exists(),
        "heuristic mode should produce review file without API key"
    );
}

// ──────────────────────────────────────────────
// 6. Missing contract file fails gracefully
// ──────────────────────────────────────────────

#[test]
fn connect_missing_contract_fails() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("output");

    tenor()
        .args([
            "connect",
            "/nonexistent/contract.tenor",
            "--heuristic",
            "--dry-run",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1);
}

// ──────────────────────────────────────────────
// 7. Apply with partially accepted mappings
// ──────────────────────────────────────────────

#[test]
fn connect_apply_partial_acceptance() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let batch_dir = tmp.path().join("partial-batch");
    let apply_dir = tmp.path().join("partial-apply");

    // Generate review file
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--batch",
            "--heuristic",
            "--out",
            batch_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let review_path = batch_dir.join("tenor-connect-review.toml");
    let content = fs::read_to_string(&review_path).expect("read review");

    // Accept only the first proposal, reject the rest
    let modified = content.replacen("status = \"proposed\"", "status = \"accepted\"", 1);
    let modified = modified.replace("status = \"proposed\"", "status = \"rejected\"");
    fs::write(&review_path, &modified).expect("write partial review");

    // Apply
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--apply",
            review_path.to_str().unwrap(),
            "--out",
            apply_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Generated files should still exist (even with partial acceptance)
    assert!(
        apply_dir.join("tenor-adapters.toml").exists(),
        "should still generate adapter config with partial acceptance"
    );
}

// ──────────────────────────────────────────────
// 8. Apply with no accepted mappings
// ──────────────────────────────────────────────

#[test]
fn connect_apply_all_rejected_no_output() {
    let (tmp, contract_path, openapi_path) = setup_fixtures();
    let batch_dir = tmp.path().join("reject-batch");
    let apply_dir = tmp.path().join("reject-apply");

    // Generate review file
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--environment",
            openapi_path.to_str().unwrap(),
            "--batch",
            "--heuristic",
            "--out",
            batch_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let review_path = batch_dir.join("tenor-connect-review.toml");
    let content = fs::read_to_string(&review_path).expect("read review");

    // Reject all proposals
    let modified = content.replace("status = \"proposed\"", "status = \"rejected\"");
    fs::write(&review_path, &modified).expect("write all-rejected review");

    // Apply should succeed but produce no files (no accepted mappings)
    tenor()
        .args([
            "connect",
            contract_path.to_str().unwrap(),
            "--apply",
            review_path.to_str().unwrap(),
            "--out",
            apply_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // No adapter config should be generated
    assert!(
        !apply_dir.join("tenor-adapters.toml").exists(),
        "all-rejected should not generate adapter config"
    );
}
