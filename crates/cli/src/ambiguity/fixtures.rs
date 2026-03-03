//! Test case loader for ambiguity fixtures.
//!
//! Scans `conformance/ambiguity/` for `.facts.json` files and loads
//! the corresponding `.tenor` contract source and `.verdicts.json`
//! expected verdicts.

use super::AmbiguityTestCase;
use std::path::Path;

/// Map fixture name prefix to its conformance subdirectory.
///
/// Convention: facts files are named `{contract}_{scenario}.facts.json`
/// where `{contract}` matches a `.tenor` file in one of the conformance
/// subdirectories.
fn contract_dir_for(name: &str) -> &str {
    match name {
        "rule_basic" | "integration_escrow" | "rule_mul_valid" => "positive",
        _ => "positive", // default
    }
}

/// Derive the contract name from a fixture filename.
///
/// Given a stem like `rule_basic_all_satisfied`, returns `rule_basic`.
/// Given `escrow_release_approved`, returns `integration_escrow`.
/// Given `mul_valid_satisfied`, returns `rule_mul_valid`.
///
/// The mapping handles the prefix conventions used in fixture naming.
fn derive_contract_name(fixture_stem: &str) -> &str {
    // Check known prefixes longest-first to avoid partial matches
    if fixture_stem.starts_with("escrow_") {
        "integration_escrow"
    } else if fixture_stem.starts_with("mul_valid_") {
        "rule_mul_valid"
    } else if fixture_stem.starts_with("rule_basic_") {
        "rule_basic"
    } else {
        // Fallback: use everything before the last underscore
        fixture_stem
            .rfind('_')
            .map(|pos| &fixture_stem[..pos])
            .unwrap_or(fixture_stem)
    }
}

/// Load all ambiguity test cases from the given directories.
///
/// - `ambiguity_dir`: path to `conformance/ambiguity/`
/// - `conformance_dir`: path to `conformance/` (parent for `positive/`, etc.)
///
/// For each `*.facts.json` file found, loads:
/// - The contract source from `conformance/{subdir}/{contract_name}.tenor`
/// - The fact values from the `.facts.json` file
/// - The expected verdicts from the matching `.verdicts.json` file
pub fn load_test_cases(
    ambiguity_dir: &Path,
    conformance_dir: &Path,
) -> Result<Vec<AmbiguityTestCase>, String> {
    let mut test_cases = Vec::new();

    // Collect and sort .facts.json files for deterministic ordering
    let mut facts_files: Vec<std::path::PathBuf> = Vec::new();
    let entries = std::fs::read_dir(ambiguity_dir).map_err(|e| {
        format!(
            "Cannot read ambiguity dir {}: {}",
            ambiguity_dir.display(),
            e
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading directory entry: {}", e))?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".facts.json") {
                facts_files.push(path);
            }
        }
    }
    facts_files.sort();

    for facts_path in &facts_files {
        let file_name = facts_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Extract fixture stem: "rule_basic_all_satisfied" from "rule_basic_all_satisfied.facts.json"
        let fixture_stem = file_name
            .strip_suffix(".facts.json")
            .ok_or_else(|| format!("Unexpected filename: {}", file_name))?;

        let contract_name = derive_contract_name(fixture_stem);
        let subdir = contract_dir_for(contract_name);

        // Load contract source
        let contract_path = conformance_dir
            .join(subdir)
            .join(format!("{}.tenor", contract_name));
        let contract_source = std::fs::read_to_string(&contract_path)
            .map_err(|e| format!("Cannot read contract {}: {}", contract_path.display(), e))?;

        // Load facts JSON
        let facts_text = std::fs::read_to_string(facts_path)
            .map_err(|e| format!("Cannot read facts {}: {}", facts_path.display(), e))?;
        let facts: serde_json::Value = serde_json::from_str(&facts_text)
            .map_err(|e| format!("Invalid JSON in {}: {}", facts_path.display(), e))?;

        // Load expected verdicts
        let verdicts_path = ambiguity_dir.join(format!("{}.verdicts.json", fixture_stem));
        let verdicts_text = std::fs::read_to_string(&verdicts_path)
            .map_err(|e| format!("Cannot read verdicts {}: {}", verdicts_path.display(), e))?;
        let verdicts_json: serde_json::Value = serde_json::from_str(&verdicts_text)
            .map_err(|e| format!("Invalid JSON in {}: {}", verdicts_path.display(), e))?;
        let expected_verdicts: Vec<String> = verdicts_json
            .get("verdicts")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("Expected \"verdicts\" array in {}", verdicts_path.display()))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        test_cases.push(AmbiguityTestCase {
            name: fixture_stem.to_string(),
            contract_source,
            facts,
            expected_verdicts,
        });
    }

    Ok(test_cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_contract_name() {
        assert_eq!(
            derive_contract_name("rule_basic_all_satisfied"),
            "rule_basic"
        );
        assert_eq!(derive_contract_name("rule_basic_partial"), "rule_basic");
        assert_eq!(derive_contract_name("rule_basic_none"), "rule_basic");
        assert_eq!(
            derive_contract_name("escrow_release_approved"),
            "integration_escrow"
        );
        assert_eq!(
            derive_contract_name("escrow_refund_approved"),
            "integration_escrow"
        );
        assert_eq!(
            derive_contract_name("escrow_compliance_required"),
            "integration_escrow"
        );
        assert_eq!(
            derive_contract_name("mul_valid_satisfied"),
            "rule_mul_valid"
        );
        assert_eq!(
            derive_contract_name("mul_valid_not_satisfied"),
            "rule_mul_valid"
        );
    }

    #[test]
    fn test_contract_dir_for() {
        assert_eq!(contract_dir_for("rule_basic"), "positive");
        assert_eq!(contract_dir_for("integration_escrow"), "positive");
        assert_eq!(contract_dir_for("rule_mul_valid"), "positive");
        assert_eq!(contract_dir_for("unknown"), "positive");
    }
}
