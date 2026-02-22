//! AI ambiguity testing harness for Tenor specifications.
//!
//! This module provides infrastructure for testing whether an LLM can
//! unambiguously evaluate Tenor contracts against fact sets, producing
//! the same verdicts as the reference elaborator.

pub mod api;
pub mod compare;
pub mod fixtures;
pub mod prompt;
pub mod report;

use std::collections::BTreeSet;
use std::path::Path;

/// A single ambiguity test case: contract + facts + expected verdicts.
pub struct AmbiguityTestCase {
    pub name: String,
    /// The `.tenor` contract source text.
    pub contract_source: String,
    /// Fact values as a JSON object.
    pub facts: serde_json::Value,
    /// Sorted list of expected verdict IDs.
    pub expected_verdicts: Vec<String>,
    /// Relevant spec section text extracted from TENOR.md.
    /// Populated during fixture loading for future spec-targeted prompting.
    #[allow(dead_code)] // Loaded for future use in spec-section-targeted prompt construction
    pub spec_sections: Vec<String>,
}

/// The result of running one ambiguity test case through the LLM.
pub struct AmbiguityResult {
    pub test_name: String,
    pub expected_verdicts: BTreeSet<String>,
    pub llm_verdicts: BTreeSet<String>,
    pub llm_reasoning: String,
    pub ambiguities_noted: Vec<String>,
    pub confidence: String,
}

/// Result of running the full ambiguity suite.
pub struct AmbiguityRunResult {
    #[allow(dead_code)] // Public API: available for callers to inspect suite totals
    pub total: usize,
    #[allow(dead_code)] // Public API: available for callers to inspect match count
    pub matches: usize,
    #[allow(dead_code)] // Public API: available for callers to inspect mismatch count
    pub mismatches: usize,
    /// Hard errors: API failures after all retries, missing files, parse errors.
    /// LLM verdict mismatches are NOT hard errors.
    pub hard_errors: usize,
}

/// Run the ambiguity test suite end-to-end.
///
/// 1. Checks for ANTHROPIC_API_KEY (skips gracefully if absent).
/// 2. Loads test cases from `suite_dir/ambiguity/`.
/// 3. Builds prompts from the spec at `spec_path`.
/// 4. Calls the Anthropic API for each test case.
/// 5. Compares LLM verdicts against expected ground truth.
/// 6. Prints TAP report to stdout.
///
/// Returns an `AmbiguityRunResult` with match/mismatch/error counts.
pub fn run_ambiguity_suite(
    suite_dir: &Path,
    spec_path: &Path,
    model: Option<&str>,
) -> AmbiguityRunResult {
    // 1. Check for API key
    let api_key = match api::get_api_key() {
        Ok(key) => key,
        Err(_) => {
            eprintln!("# Skipping ambiguity tests: ANTHROPIC_API_KEY not set");
            return AmbiguityRunResult {
                total: 0,
                matches: 0,
                mismatches: 0,
                hard_errors: 0,
            };
        }
    };

    // 2. Load test cases
    let ambiguity_dir = suite_dir.join("ambiguity");
    if !ambiguity_dir.exists() {
        eprintln!(
            "# No ambiguity test cases found in {}/ambiguity",
            suite_dir.display()
        );
        return AmbiguityRunResult {
            total: 0,
            matches: 0,
            mismatches: 0,
            hard_errors: 0,
        };
    }

    let test_cases = match fixtures::load_test_cases(&ambiguity_dir, suite_dir) {
        Ok(cases) => cases,
        Err(e) => {
            eprintln!("# Error loading test cases: {}", e);
            return AmbiguityRunResult {
                total: 0,
                matches: 0,
                mismatches: 0,
                hard_errors: 1,
            };
        }
    };

    if test_cases.is_empty() {
        eprintln!(
            "# No ambiguity test cases found in {}/ambiguity",
            suite_dir.display()
        );
        return AmbiguityRunResult {
            total: 0,
            matches: 0,
            mismatches: 0,
            hard_errors: 0,
        };
    }

    // 3. Build system prompt from spec
    let system_prompt = match prompt::build_system_prompt(spec_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("# Error reading spec file: {}", e);
            return AmbiguityRunResult {
                total: test_cases.len(),
                matches: 0,
                mismatches: 0,
                hard_errors: 1,
            };
        }
    };

    let model_name = model.unwrap_or_else(|| api::default_model());

    // 4. Initialize report
    let mut report = report::AmbiguityReport::new();
    let mut hard_errors: usize = 0;
    let total = test_cases.len();

    // 5. Process each test case
    for (i, test) in test_cases.iter().enumerate() {
        // a. Build user prompt
        let user_prompt = prompt::build_user_prompt(&test.contract_source, &test.facts);

        // b. Call API
        let response_text =
            match api::call_anthropic(&api_key, &system_prompt, &user_prompt, model_name) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("# [{}/{}] {} -- API ERROR: {}", i + 1, total, test.name, e);
                    hard_errors += 1;
                    continue;
                }
            };

        // c. Parse LLM response
        let llm_response = match compare::parse_llm_response(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!(
                    "# [{}/{}] {} -- PARSE ERROR: {}",
                    i + 1,
                    total,
                    test.name,
                    e
                );
                hard_errors += 1;
                continue;
            }
        };

        // d. Compare verdicts
        let comparison =
            compare::compare_verdicts(&test.expected_verdicts, &llm_response.verdicts_produced);

        let status = if comparison.is_match {
            "match"
        } else {
            "MISMATCH"
        };

        eprintln!("# [{}/{}] {} -- {}", i + 1, total, test.name, status);

        // e. Build AmbiguityResult
        let expected_set: BTreeSet<String> = test.expected_verdicts.iter().cloned().collect();
        let llm_set: BTreeSet<String> = llm_response.verdicts_produced.iter().cloned().collect();

        let result = AmbiguityResult {
            test_name: test.name.clone(),
            expected_verdicts: expected_set,
            llm_verdicts: llm_set,
            llm_reasoning: llm_response.reasoning,
            ambiguities_noted: llm_response.ambiguities_noted,
            confidence: llm_response.confidence,
        };

        // f. Add to report
        report.add_result(result);
    }

    // 6. Print TAP report
    report.print_tap();

    // 7. Return result
    let (_, matches, mismatches) = report.summary();
    AmbiguityRunResult {
        total,
        matches,
        mismatches,
        hard_errors,
    }
}
