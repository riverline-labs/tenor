//! Validates all positive conformance test expected JSONs against the formal
//! interchange schema at schema/interchange-schema.json.

use std::path::Path;

fn validate_file(
    validator: &jsonschema::Validator,
    path: &Path,
    failures: &mut Vec<String>,
    tested: &mut usize,
) {
    let json_src = std::fs::read_to_string(path).unwrap();
    let instance: serde_json::Value = serde_json::from_str(&json_src).unwrap();
    if let Err(error) = validator.validate(&instance) {
        failures.push(format!("{}: {}", path.display(), error));
    }
    *tested += 1;
}

fn collect_expected_json_files(dir: &Path) -> Vec<std::path::PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map_or(false, |e| e == "json")
                && p.to_string_lossy().contains("expected.json")
                && !p.to_string_lossy().contains("expected-error.json")
        })
        .collect();
    paths.sort();
    paths
}

#[test]
fn validate_all_positive_conformance_outputs_against_schema() {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/interchange-schema.json");
    let schema_src = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to read schema at {}: {}", schema_path.display(), e));
    let schema_value: serde_json::Value = serde_json::from_str(&schema_src).unwrap();
    let validator = jsonschema::validator_for(&schema_value)
        .unwrap_or_else(|e| panic!("Failed to compile schema: {}", e));

    let conformance_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");

    let mut tested = 0usize;
    let mut failures = Vec::new();

    // Check all directories with expected.json files
    for dir_name in &["positive", "numeric", "promotion", "shorthand"] {
        let dir = conformance_root.join(dir_name);
        for path in collect_expected_json_files(&dir) {
            validate_file(&validator, &path, &mut failures, &mut tested);
        }
    }

    // Also check cross_file expected JSONs
    let cross_dir = conformance_root.join("cross_file");
    for path in collect_expected_json_files(&cross_dir) {
        validate_file(&validator, &path, &mut failures, &mut tested);
    }

    assert!(
        tested > 0,
        "No conformance expected.json files found -- check paths"
    );
    assert!(
        failures.is_empty(),
        "Schema validation failed for {} of {} files:\n{}",
        failures.len(),
        tested,
        failures.join("\n")
    );

    eprintln!(
        "Schema validation passed for {} expected.json files",
        tested
    );
}
