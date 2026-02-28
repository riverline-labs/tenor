//! Integration tests for the `tenor ui` command.
//!
//! Verifies that `tenor ui` generates correct, complete, and compilable
//! React applications from various contracts.
//!
//! All tests set `current_dir` to the workspace root so that relative
//! paths to conformance fixtures resolve correctly.
//!
//! Node.js-requiring tests are marked `#[ignore]` and can be run with:
//!   cargo test -p tenor-cli -- --ignored test_generated_typescript_compiles

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

/// Helper: create a Command for the `tenor` binary, rooted at workspace.
fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    cmd
}

/// The escrow contract path (relative to workspace root).
const ESCROW_CONTRACT: &str = "conformance/positive/integration_escrow.tenor";

/// Minimal contract content for testing a simple single-entity contract.
const MINIMAL_CONTRACT: &str = r#"fact is_active {
  type: Bool
  source: "system.active"
}

entity Order {
  states: [draft, submitted]
  initial: draft
  transitions: [(draft, submitted)]
}

rule check_active {
  stratum: 0
  when: is_active = true
  produce: verdict active { payload: Bool = true }
}

operation submit_order {
  allowed_personas: [buyer]
  precondition: verdict_present(active)
  effects: [(Order, draft, submitted)]
  error_contract: [precondition_failed]
}
"#;

// ──────────────────────────────────────────────
// Task 1: Escrow contract generation test
// ──────────────────────────────────────────────

#[test]
fn test_generate_escrow_ui() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("escrow-ui");

    // 1. Run tenor ui on the escrow contract
    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    // 2. Assert all expected files exist
    let expected_files = [
        "package.json",
        "tsconfig.json",
        "vite.config.ts",
        "public/index.html",
        "src/main.tsx",
        "src/App.tsx",
        "src/api.ts",
        "src/types.ts",
        "src/theme.ts",
        "src/styles.css",
        "src/components/Layout.tsx",
        "src/components/Dashboard.tsx",
        "src/components/EntityList.tsx",
        "src/components/EntityDetail.tsx",
        "src/components/InstanceDetail.tsx",
        "src/components/ActionSpace.tsx",
        "src/components/BlockedActions.tsx",
        "src/components/FactInput.tsx",
        "src/components/FlowExecution.tsx",
        "src/components/FlowHistory.tsx",
        "src/components/ProvenanceDrill.tsx",
        "src/components/VerdictDisplay.tsx",
        "src/hooks/useActionSpace.ts",
        "src/hooks/useEntities.ts",
        "src/hooks/useExecution.ts",
    ];

    for rel_path in &expected_files {
        let full_path = out_dir.join(rel_path);
        assert!(
            full_path.exists(),
            "expected file missing: {rel_path} (checked at {})",
            full_path.display()
        );
    }

    // 3. Read types.ts and assert content correctness
    let types_ts = fs::read_to_string(out_dir.join("src/types.ts")).expect("read types.ts");

    // Entity state types
    assert!(
        types_ts.contains("EscrowAccountState"),
        "types.ts must contain EscrowAccountState"
    );
    assert!(
        types_ts.contains("\"held\""),
        "EscrowAccountState must include held"
    );
    assert!(
        types_ts.contains("\"released\""),
        "EscrowAccountState must include released"
    );
    assert!(
        types_ts.contains("\"refunded\""),
        "EscrowAccountState must include refunded"
    );
    assert!(
        types_ts.contains("\"disputed\""),
        "EscrowAccountState must include disputed"
    );

    assert!(
        types_ts.contains("DeliveryRecordState"),
        "types.ts must contain DeliveryRecordState"
    );
    assert!(
        types_ts.contains("\"pending\""),
        "DeliveryRecordState must include pending"
    );
    assert!(
        types_ts.contains("\"confirmed\""),
        "DeliveryRecordState must include confirmed"
    );
    assert!(
        types_ts.contains("\"failed\""),
        "DeliveryRecordState must include failed"
    );

    // Persona type
    assert!(
        types_ts.contains("Persona"),
        "types.ts must contain Persona type"
    );
    assert!(
        types_ts.contains("\"escrow_agent\""),
        "Persona must include escrow_agent"
    );
    assert!(
        types_ts.contains("\"compliance_officer\""),
        "Persona must include compliance_officer"
    );
    assert!(types_ts.contains("\"buyer\""), "Persona must include buyer");
    assert!(
        types_ts.contains("\"seller\""),
        "Persona must include seller"
    );

    // Facts interface
    assert!(
        types_ts.contains("Facts"),
        "types.ts must contain Facts interface"
    );
    assert!(
        types_ts.contains("escrowAmount") || types_ts.contains("escrow_amount"),
        "Facts must contain escrow_amount"
    );

    // OPERATIONS and FLOWS arrays
    assert!(
        types_ts.contains("OPERATIONS"),
        "types.ts must contain OPERATIONS array"
    );
    assert!(
        types_ts.contains("release_escrow"),
        "OPERATIONS must contain release_escrow"
    );
    assert!(
        types_ts.contains("FLOWS"),
        "types.ts must contain FLOWS array"
    );
    assert!(
        types_ts.contains("standard_release"),
        "FLOWS must contain standard_release"
    );
    assert!(
        types_ts.contains("refund_flow"),
        "FLOWS must contain refund_flow"
    );

    // 4. Read api.ts and assert content
    let api_ts = fs::read_to_string(out_dir.join("src/api.ts")).expect("read api.ts");
    assert!(
        api_ts.contains("class TenorClient"),
        "api.ts must contain TenorClient class"
    );
    assert!(
        api_ts.contains("getManifest"),
        "api.ts must have getManifest"
    );
    assert!(
        api_ts.contains("getActionSpace"),
        "api.ts must have getActionSpace"
    );
    assert!(
        api_ts.contains("executeFlow"),
        "api.ts must have executeFlow"
    );
    assert!(
        api_ts.contains("simulateFlow"),
        "api.ts must have simulateFlow"
    );
    assert!(
        api_ts.contains("getEntityInstances"),
        "api.ts must have getEntityInstances"
    );
    assert!(
        api_ts.contains("API_BASE"),
        "api.ts must have API_BASE constant"
    );
    assert!(
        api_ts.contains("CONTRACT_ID"),
        "api.ts must have CONTRACT_ID constant"
    );

    // 5. Read theme.ts and assert content
    let theme_ts = fs::read_to_string(out_dir.join("src/theme.ts")).expect("read theme.ts");
    assert!(
        theme_ts.contains("export const theme"),
        "theme.ts must export theme"
    );
    assert!(
        theme_ts.contains("primary:"),
        "theme.ts must have primary color"
    );
    assert!(
        theme_ts.contains("success: '#16a34a'"),
        "theme.ts must have success = #16a34a"
    );
    assert!(
        theme_ts.contains("system-ui"),
        "theme.ts fonts.body must contain system-ui"
    );
}

// ──────────────────────────────────────────────
// Task 2: Minimal contract generation test
// ──────────────────────────────────────────────

#[test]
fn test_generate_minimal_ui() {
    let tmp = TempDir::new().expect("temp dir");
    let contract_path = tmp.path().join("minimal.tenor");
    let out_dir = tmp.path().join("minimal-ui");

    // Write minimal contract to temp file
    fs::write(&contract_path, MINIMAL_CONTRACT).expect("write minimal contract");

    // 1. Generate UI from the minimal contract
    tenor()
        .args([
            "ui",
            contract_path.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 2. Assert core files exist
    let core_files = [
        "package.json",
        "tsconfig.json",
        "vite.config.ts",
        "src/App.tsx",
        "src/api.ts",
        "src/types.ts",
        "src/theme.ts",
        "src/styles.css",
        "src/components/Dashboard.tsx",
    ];
    for rel_path in &core_files {
        let full_path = out_dir.join(rel_path);
        assert!(full_path.exists(), "expected core file missing: {rel_path}");
    }

    // 3. Read types.ts and assert content
    let types_ts = fs::read_to_string(out_dir.join("src/types.ts")).expect("read types.ts");

    // OrderState must contain draft and submitted
    assert!(
        types_ts.contains("OrderState"),
        "types.ts must contain OrderState"
    );
    assert!(
        types_ts.contains("\"draft\""),
        "OrderState must include draft"
    );
    assert!(
        types_ts.contains("\"submitted\""),
        "OrderState must include submitted"
    );

    // Persona type must contain buyer
    assert!(
        types_ts.contains("Persona"),
        "types.ts must contain Persona type"
    );
    assert!(types_ts.contains("\"buyer\""), "Persona must include buyer");

    // Facts interface must contain is_active (camelCase: isActive)
    assert!(
        types_ts.contains("Facts"),
        "types.ts must contain Facts interface"
    );
    assert!(
        types_ts.contains("isActive") || types_ts.contains("is_active"),
        "Facts must contain is_active"
    );

    // Transitions must be populated (not empty)
    assert!(
        types_ts.contains(r#"["draft", "submitted"]"#),
        "ENTITIES.Order transitions must include draft->submitted"
    );

    // 4. No obvious duplication: OrderState should appear as a type but not repeated
    let order_state_count = types_ts.matches("OrderState").count();
    // It will appear in the type declaration and in EntityStates interface = 2 occurrences minimum
    assert!(
        order_state_count >= 1,
        "OrderState should appear at least once"
    );
    assert!(
        order_state_count <= 5,
        "OrderState should not be excessively duplicated (found {order_state_count} occurrences)"
    );
}

// ──────────────────────────────────────────────
// Task 3: Fact type matching tests
// ──────────────────────────────────────────────

#[test]
fn test_fact_input_types_match() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("fact-types-ui");

    // Generate UI from escrow contract
    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    let types_ts = fs::read_to_string(out_dir.join("src/types.ts")).expect("read types.ts");
    let fact_input_tsx = fs::read_to_string(out_dir.join("src/components/FactInput.tsx"))
        .expect("read FactInput.tsx");

    // Assert FACTS metadata has correct type strings in types.ts
    // escrow_amount: Money with currency USD
    assert!(
        types_ts.contains("\"Money\"") || types_ts.contains("type: \"Money\""),
        "FACTS must include Money type"
    );
    assert!(
        types_ts.contains("\"USD\""),
        "FACTS must include USD currency for escrow_amount"
    );

    // delivery_status: Enum with values pending, confirmed, failed
    assert!(
        types_ts.contains("\"Enum\"") || types_ts.contains("type: \"Enum\""),
        "FACTS must include Enum type"
    );
    assert!(
        types_ts.contains("\"pending\"") && types_ts.contains("\"confirmed\""),
        "Enum delivery_status must have pending and confirmed values"
    );

    // line_items: List with elementType containing Record fields
    assert!(
        types_ts.contains("\"List\"") || types_ts.contains("type: \"List\""),
        "FACTS must include List type"
    );
    assert!(
        types_ts.contains("elementType"),
        "List fact must have elementType in FACTS metadata"
    );

    // buyer_requested_refund: Bool
    assert!(
        types_ts.contains("\"Bool\"") || types_ts.contains("type: \"Bool\""),
        "FACTS must include Bool type"
    );

    // Assert FactInput.tsx dispatches on type strings for rendering
    // The component should handle at least Bool, Money, Enum, List cases
    assert!(
        fact_input_tsx.contains("Bool") || fact_input_tsx.contains("bool"),
        "FactInput must handle Bool type"
    );
    assert!(
        fact_input_tsx.contains("Money") || fact_input_tsx.contains("money"),
        "FactInput must handle Money type"
    );
    assert!(
        fact_input_tsx.contains("Enum") || fact_input_tsx.contains("enum"),
        "FactInput must handle Enum type"
    );
    assert!(
        fact_input_tsx.contains("List") || fact_input_tsx.contains("list"),
        "FactInput must handle List type"
    );
}

#[test]
fn test_persona_list_matches() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("persona-ui");

    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    let types_ts = fs::read_to_string(out_dir.join("src/types.ts")).expect("read types.ts");

    // PERSONAS array must contain all four personas
    assert!(
        types_ts.contains("\"buyer\""),
        "PERSONAS must contain buyer"
    );
    assert!(
        types_ts.contains("\"seller\""),
        "PERSONAS must contain seller"
    );
    assert!(
        types_ts.contains("\"escrow_agent\""),
        "PERSONAS must contain escrow_agent"
    );
    assert!(
        types_ts.contains("\"compliance_officer\""),
        "PERSONAS must contain compliance_officer"
    );

    // PERSONAS array should be declared
    assert!(
        types_ts.contains("PERSONAS"),
        "types.ts must export PERSONAS array"
    );
}

#[test]
fn test_entity_states_match() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("states-ui");

    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    let types_ts = fs::read_to_string(out_dir.join("src/types.ts")).expect("read types.ts");

    // EscrowAccountState must have exactly: held, released, refunded, disputed
    assert!(
        types_ts.contains("EscrowAccountState"),
        "must have EscrowAccountState"
    );
    assert!(types_ts.contains("\"held\""), "must have held");
    assert!(types_ts.contains("\"released\""), "must have released");
    assert!(types_ts.contains("\"refunded\""), "must have refunded");
    assert!(types_ts.contains("\"disputed\""), "must have disputed");

    // DeliveryRecordState must have exactly: pending, confirmed, failed
    assert!(
        types_ts.contains("DeliveryRecordState"),
        "must have DeliveryRecordState"
    );
    assert!(types_ts.contains("\"pending\""), "must have pending");
    assert!(types_ts.contains("\"confirmed\""), "must have confirmed");
    assert!(types_ts.contains("\"failed\""), "must have failed");

    // ENTITIES const must be present
    assert!(
        types_ts.contains("ENTITIES"),
        "types.ts must export ENTITIES const"
    );
    // ENTITIES should reference EscrowAccount and DeliveryRecord
    assert!(
        types_ts.contains("EscrowAccount"),
        "ENTITIES must reference EscrowAccount"
    );
    assert!(
        types_ts.contains("DeliveryRecord"),
        "ENTITIES must reference DeliveryRecord"
    );

    // ENTITIES must contain transitions (not empty arrays)
    // EscrowAccount has transitions like held->released, held->refunded, etc.
    assert!(
        types_ts.contains(r#"["held", "released"]"#),
        "ENTITIES.EscrowAccount transitions must include held->released"
    );
    assert!(
        types_ts.contains(r#"["held", "refunded"]"#),
        "ENTITIES.EscrowAccount transitions must include held->refunded"
    );
    // DeliveryRecord has transitions like pending->confirmed
    assert!(
        types_ts.contains(r#"["pending", "confirmed"]"#),
        "ENTITIES.DeliveryRecord transitions must include pending->confirmed"
    );
}

// ──────────────────────────────────────────────
// Task 4: TypeScript compilation tests (ignored)
// ──────────────────────────────────────────────

/// Run with: cargo test -p tenor-cli -- --ignored test_generated_typescript_compiles
/// Requires Node.js and npm in PATH.
#[test]
#[ignore]
fn test_generated_typescript_compiles() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("tsc-ui");

    // 1. Generate UI from escrow contract
    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    // 2. Run npm install
    let npm_install = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&out_dir)
        .output()
        .expect("failed to run npm install — is Node.js installed?");

    assert!(
        npm_install.status.success(),
        "npm install failed:\n{}",
        String::from_utf8_lossy(&npm_install.stderr)
    );

    // 3. Run tsc --noEmit to type-check
    let tsc = std::process::Command::new("npx")
        .args(["tsc", "--noEmit"])
        .current_dir(&out_dir)
        .output()
        .expect("failed to run npx tsc");

    assert!(
        tsc.status.success(),
        "TypeScript compilation failed:\n{}\n{}",
        String::from_utf8_lossy(&tsc.stdout),
        String::from_utf8_lossy(&tsc.stderr)
    );
}

/// Run with: cargo test -p tenor-cli -- --ignored test_generated_app_builds
/// Requires Node.js and npm in PATH.
#[test]
#[ignore]
fn test_generated_app_builds() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("build-ui");

    // 1. Generate UI from escrow contract
    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir.to_str().unwrap()])
        .assert()
        .success();

    // 2. Run npm install
    let npm_install = std::process::Command::new("npm")
        .arg("install")
        .current_dir(&out_dir)
        .output()
        .expect("failed to run npm install — is Node.js installed?");

    assert!(
        npm_install.status.success(),
        "npm install failed:\n{}",
        String::from_utf8_lossy(&npm_install.stderr)
    );

    // 3. Run npm run build (Vite build)
    let npm_build = std::process::Command::new("npm")
        .args(["run", "build"])
        .current_dir(&out_dir)
        .output()
        .expect("failed to run npm run build");

    assert!(
        npm_build.status.success(),
        "npm run build failed:\n{}\n{}",
        String::from_utf8_lossy(&npm_build.stdout),
        String::from_utf8_lossy(&npm_build.stderr)
    );

    // 4. Assert dist/ directory was created
    let dist_dir = out_dir.join("dist");
    assert!(
        dist_dir.exists() && dist_dir.is_dir(),
        "dist/ directory should exist after build"
    );
}

// ──────────────────────────────────────────────
// Task 5: CLI flag tests and edge cases
// ──────────────────────────────────────────────

#[test]
fn test_ui_custom_api_url() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("api-url-ui");

    tenor()
        .args([
            "ui",
            ESCROW_CONTRACT,
            "--out",
            out_dir.to_str().unwrap(),
            "--api-url",
            "https://api.example.com",
        ])
        .assert()
        .success();

    let api_ts = fs::read_to_string(out_dir.join("src/api.ts")).expect("read api.ts");
    assert!(
        api_ts.contains("'https://api.example.com'")
            || api_ts.contains("\"https://api.example.com\""),
        "api.ts must set API_BASE to https://api.example.com, got:\n{}",
        &api_ts[..api_ts.len().min(500)]
    );
}

#[test]
fn test_ui_custom_contract_id() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("contract-id-ui");

    tenor()
        .args([
            "ui",
            ESCROW_CONTRACT,
            "--out",
            out_dir.to_str().unwrap(),
            "--contract-id",
            "my-custom-id",
        ])
        .assert()
        .success();

    let api_ts = fs::read_to_string(out_dir.join("src/api.ts")).expect("read api.ts");
    assert!(
        api_ts.contains("'my-custom-id'") || api_ts.contains("\"my-custom-id\""),
        "api.ts must set CONTRACT_ID to my-custom-id"
    );
}

#[test]
fn test_ui_custom_title() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("title-ui");

    tenor()
        .args([
            "ui",
            ESCROW_CONTRACT,
            "--out",
            out_dir.to_str().unwrap(),
            "--title",
            "My Escrow App",
        ])
        .assert()
        .success();

    // Title should appear in package.json or index.html or App.tsx
    let index_html =
        fs::read_to_string(out_dir.join("public/index.html")).expect("read index.html");
    let app_tsx = fs::read_to_string(out_dir.join("src/App.tsx")).expect("read App.tsx");
    let pkg_json = fs::read_to_string(out_dir.join("package.json")).expect("read package.json");

    let title_present = index_html.contains("My Escrow App")
        || app_tsx.contains("My Escrow App")
        || pkg_json.contains("My Escrow App");

    assert!(
        title_present,
        "custom title 'My Escrow App' must appear in index.html, App.tsx, or package.json"
    );
}

#[test]
fn test_ui_json_input() {
    let tmp = TempDir::new().expect("temp dir");
    let json_path = tmp.path().join("escrow.json");
    let out_dir_tenor = tmp.path().join("from-tenor");
    let out_dir_json = tmp.path().join("from-json");

    // 1. Elaborate escrow contract to JSON
    let elaborate_output = tenor()
        .args(["elaborate", ESCROW_CONTRACT])
        .output()
        .expect("elaborate failed");
    assert!(
        elaborate_output.status.success(),
        "elaborate should succeed"
    );
    fs::write(&json_path, &elaborate_output.stdout).expect("write json");

    // 2. Generate from .tenor source
    tenor()
        .args([
            "ui",
            ESCROW_CONTRACT,
            "--out",
            out_dir_tenor.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 3. Generate from .json interchange
    tenor()
        .args([
            "ui",
            json_path.to_str().unwrap(),
            "--out",
            out_dir_json.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 4. Assert both produce the same key files
    let key_files = ["package.json", "src/types.ts", "src/api.ts", "src/theme.ts"];
    for rel in &key_files {
        let tenor_path = out_dir_tenor.join(rel);
        let json_path_out = out_dir_json.join(rel);
        assert!(tenor_path.exists(), "from-tenor must have {rel}");
        assert!(json_path_out.exists(), "from-json must have {rel}");
    }
}

#[test]
fn test_ui_output_dir_created() {
    let tmp = TempDir::new().expect("temp dir");
    // Use a deeply nested non-existent path
    let nested_out = tmp.path().join("some").join("nested").join("path");

    assert!(!nested_out.exists(), "nested path should not exist yet");

    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", nested_out.to_str().unwrap()])
        .assert()
        .success();

    assert!(
        nested_out.exists(),
        "output directory must be created: {}",
        nested_out.display()
    );
    assert!(
        nested_out.join("package.json").exists(),
        "package.json must exist in created directory"
    );
}

#[test]
fn test_ui_nonexistent_contract() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("nonexistent-ui");

    tenor()
        .args([
            "ui",
            "this_contract_does_not_exist.tenor",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_different_contracts_different_themes() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir_1 = tmp.path().join("contract1-ui");
    let out_dir_2 = tmp.path().join("contract2-ui");

    // Contract 1: escrow
    tenor()
        .args(["ui", ESCROW_CONTRACT, "--out", out_dir_1.to_str().unwrap()])
        .assert()
        .success();

    // Contract 2: a different contract
    tenor()
        .args([
            "ui",
            "conformance/positive/fact_basic.tenor",
            "--out",
            out_dir_2.to_str().unwrap(),
        ])
        .assert()
        .success();

    let theme_1 = fs::read_to_string(out_dir_1.join("src/theme.ts")).expect("read theme 1");
    let theme_2 = fs::read_to_string(out_dir_2.join("src/theme.ts")).expect("read theme 2");

    // Extract primary color values — they should differ (different contract ids -> different hues)
    // The themes themselves will be different due to the contract ID comment at minimum
    // but primary colors should also differ for different contract IDs
    assert_ne!(
        theme_1, theme_2,
        "different contracts should produce different theme files"
    );
}

// ──────────────────────────────────────────────
// Failure / error path tests
// ──────────────────────────────────────────────

#[test]
fn test_ui_invalid_tenor_syntax() {
    let tmp = TempDir::new().expect("temp dir");
    let bad_file = tmp.path().join("bad_syntax.tenor");
    let out_dir = tmp.path().join("bad-syntax-ui");

    // Write a file with invalid Tenor syntax
    fs::write(&bad_file, "this is not valid tenor").expect("write bad file");

    tenor()
        .args([
            "ui",
            bad_file.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_ui_incomplete_construct() {
    let tmp = TempDir::new().expect("temp dir");
    let bad_file = tmp.path().join("incomplete.tenor");
    let out_dir = tmp.path().join("incomplete-ui");

    // Write a file with an incomplete construct (opening brace, no closing brace)
    fs::write(&bad_file, "entity Order {\n  states: [draft").expect("write incomplete file");

    tenor()
        .args([
            "ui",
            bad_file.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1);
}
