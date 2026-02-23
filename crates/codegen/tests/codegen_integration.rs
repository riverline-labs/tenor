//! Integration tests for the TypeScript code generation pipeline.
//!
//! These tests verify the complete generation flow from interchange JSON
//! to TypeScript output files (types.ts, schemas.ts, client.ts, index.ts).

use std::fs;
use std::path::Path;
use tenor_codegen::{generate_typescript, TypeScriptConfig};

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> &'static Path {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/codegen -> workspace root is two levels up
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
}

/// Read and parse a conformance fixture JSON file.
fn read_fixture(name: &str) -> serde_json::Value {
    let path = workspace_root()
        .join("conformance/positive")
        .join(format!("{}.expected.json", name));
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture '{}': {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse fixture '{}': {}", path.display(), e))
}

#[test]
fn test_generate_typescript_from_json() {
    let fixture = read_fixture("operation_basic");
    let dir = tempfile::tempdir().expect("temp dir");

    let config = TypeScriptConfig {
        out_dir: dir.path().to_path_buf(),
        sdk_import: "@tenor/sdk".to_string(),
    };

    let output_dir = generate_typescript(&fixture, &config).expect("generation failed");

    // Assert output directory exists
    assert!(output_dir.exists(), "output directory should exist");
    assert_eq!(
        output_dir.file_name().unwrap().to_str().unwrap(),
        "operation-basic"
    );

    // Assert all 4 files exist
    let types_path = output_dir.join("types.ts");
    let schemas_path = output_dir.join("schemas.ts");
    let client_path = output_dir.join("client.ts");
    let index_path = output_dir.join("index.ts");
    assert!(types_path.exists(), "types.ts should exist");
    assert!(schemas_path.exists(), "schemas.ts should exist");
    assert!(client_path.exists(), "client.ts should exist");
    assert!(index_path.exists(), "index.ts should exist");

    // Verify types.ts content
    let types = fs::read_to_string(&types_path).unwrap();
    assert!(
        types.contains("export type TenorMoney = string &"),
        "types.ts should contain branded TenorMoney type"
    );
    assert!(
        types.contains("export type OrderState = 'draft' | 'submitted' | 'approved' | 'rejected'"),
        "types.ts should contain OrderState union"
    );
    assert!(
        types.contains("export interface OperationBasicFacts"),
        "types.ts should contain facts interface"
    );
    assert!(
        types.contains("isActive: boolean"),
        "types.ts should contain boolean fact mapping"
    );

    // Verify client.ts content
    let client = fs::read_to_string(&client_path).unwrap();
    assert!(
        client.contains("export class OperationBasicClient"),
        "client.ts should contain wrapper class"
    );
    assert!(
        client.contains("submitOrder("),
        "client.ts should contain submitOrder method"
    );
    assert!(
        client.contains("approveOrder("),
        "client.ts should contain approveOrder method"
    );
    assert!(
        client.contains("rejectOrder("),
        "client.ts should contain rejectOrder method"
    );
    assert!(
        client.contains("private readonly client: TenorClient"),
        "client.ts should use composition (not inheritance)"
    );

    // Verify index.ts content
    let index = fs::read_to_string(&index_path).unwrap();
    assert!(
        index.contains("export * from './types.ts'"),
        "index.ts should re-export types"
    );
    assert!(
        index.contains("export * from './schemas.ts'"),
        "index.ts should re-export schemas"
    );
    assert!(
        index.contains("export { OperationBasicClient } from './client.ts'"),
        "index.ts should re-export client class"
    );
}

#[test]
fn test_generate_typescript_integration_escrow() {
    let fixture = read_fixture("integration_escrow");
    let dir = tempfile::tempdir().expect("temp dir");

    let config = TypeScriptConfig {
        out_dir: dir.path().to_path_buf(),
        sdk_import: "@tenor/sdk".to_string(),
    };

    let output_dir = generate_typescript(&fixture, &config).expect("generation failed");

    // Verify types.ts content for complex contract
    let types = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(
        types.contains(
            "export type EscrowAccountState = 'held' | 'released' | 'refunded' | 'disputed'"
        ),
        "types.ts should contain EscrowAccountState union"
    );
    assert!(
        types.contains("export type DeliveryRecordState = 'pending' | 'confirmed' | 'failed'"),
        "types.ts should contain DeliveryRecordState union"
    );
    assert!(
        types.contains("TenorMoney"),
        "types.ts should reference TenorMoney branded type"
    );
    // List/Record types for line_items
    assert!(
        types.contains("Array<"),
        "types.ts should contain Array type for list facts"
    );

    // Verify schemas.ts content
    let schemas = fs::read_to_string(output_dir.join("schemas.ts")).unwrap();
    assert!(
        schemas.contains("z.boolean()"),
        "schemas.ts should contain z.boolean() for bool facts"
    );
    assert!(
        schemas.contains("moneySchema"),
        "schemas.ts should contain money schema for money facts"
    );
    assert!(
        schemas.contains("z.array("),
        "schemas.ts should contain z.array for list facts"
    );

    // Verify client.ts content for escrow contract
    let client = fs::read_to_string(output_dir.join("client.ts")).unwrap();
    assert!(
        client.contains("export class IntegrationEscrowClient"),
        "client.ts should contain IntegrationEscrowClient class"
    );
    assert!(
        client.contains("confirmDelivery("),
        "client.ts should contain confirmDelivery method"
    );
    assert!(
        client.contains("releaseEscrow("),
        "client.ts should contain releaseEscrow method"
    );
    assert!(
        client.contains("refundEscrow("),
        "client.ts should contain refundEscrow method"
    );
    assert!(
        client.contains("flagDispute("),
        "client.ts should contain flagDispute method"
    );
}

#[test]
fn test_generate_typescript_overwrites_existing() {
    let fixture = read_fixture("operation_basic");
    let dir = tempfile::tempdir().expect("temp dir");

    let config = TypeScriptConfig {
        out_dir: dir.path().to_path_buf(),
        sdk_import: "@tenor/sdk".to_string(),
    };

    // Generate once
    let output_dir = generate_typescript(&fixture, &config).expect("first generation failed");
    let types_first = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    let client_first = fs::read_to_string(output_dir.join("client.ts")).unwrap();

    // Generate again into the same directory
    let output_dir2 = generate_typescript(&fixture, &config).expect("second generation failed");
    let types_second = fs::read_to_string(output_dir2.join("types.ts")).unwrap();
    let client_second = fs::read_to_string(output_dir2.join("client.ts")).unwrap();

    // Content should be identical both times (deterministic output)
    assert_eq!(
        types_first, types_second,
        "types.ts should be identical on re-generation"
    );
    assert_eq!(
        client_first, client_second,
        "client.ts should be identical on re-generation"
    );
}

#[test]
fn test_generate_multiple_contracts() {
    let fixture_basic = read_fixture("operation_basic");
    let fixture_escrow = read_fixture("integration_escrow");
    let dir = tempfile::tempdir().expect("temp dir");

    let config = TypeScriptConfig {
        out_dir: dir.path().to_path_buf(),
        sdk_import: "@tenor/sdk".to_string(),
    };

    // Generate both contracts into the same output root
    let dir_basic = generate_typescript(&fixture_basic, &config).expect("basic generation failed");
    let dir_escrow =
        generate_typescript(&fixture_escrow, &config).expect("escrow generation failed");

    // Verify separate directories
    assert_ne!(dir_basic, dir_escrow, "contracts should have separate dirs");
    assert!(
        dir_basic
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("operation-basic"),
        "basic contract directory name"
    );
    assert!(
        dir_escrow
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("integration-escrow"),
        "escrow contract directory name"
    );

    // Verify each has all 4 files with no namespace collisions
    for contract_dir in [&dir_basic, &dir_escrow] {
        assert!(contract_dir.join("types.ts").exists());
        assert!(contract_dir.join("schemas.ts").exists());
        assert!(contract_dir.join("client.ts").exists());
        assert!(contract_dir.join("index.ts").exists());
    }

    // Verify the content is contract-specific (no collisions)
    let basic_client = fs::read_to_string(dir_basic.join("client.ts")).unwrap();
    let escrow_client = fs::read_to_string(dir_escrow.join("client.ts")).unwrap();
    assert!(basic_client.contains("OperationBasicClient"));
    assert!(escrow_client.contains("IntegrationEscrowClient"));
    assert!(!basic_client.contains("IntegrationEscrowClient"));
    assert!(!escrow_client.contains("OperationBasicClient"));
}
