//! TAP-format ambiguity reporting.
//!
//! Follows the same TAP v14 style as `tap.rs`, extended with YAML
//! diagnostic blocks for mismatch details.

use super::AmbiguityResult;

/// Collects ambiguity test results and produces TAP-format output.
pub struct AmbiguityReport {
    results: Vec<AmbiguityResult>,
}

impl AmbiguityReport {
    pub fn new() -> Self {
        AmbiguityReport {
            results: Vec::new(),
        }
    }

    /// Add a test result to the report.
    pub fn add_result(&mut self, result: AmbiguityResult) {
        self.results.push(result);
    }

    /// Print TAP v14 output to stdout.
    ///
    /// Matches are "ok", mismatches are "not ok" with YAML diagnostic
    /// blocks showing missing/extra verdicts, LLM confidence, and
    /// ambiguities noted.
    pub fn print_tap(&self) {
        println!("TAP version 14");
        println!("1..{}", self.results.len());

        for (i, result) in self.results.iter().enumerate() {
            let n = i + 1;
            let is_match = result.expected_verdicts == result.llm_verdicts;

            if is_match {
                println!("ok {} - {}", n, result.test_name);
            } else {
                println!("not ok {} - {}", n, result.test_name);
                // YAML diagnostic block
                println!("  ---");
                println!("  missing_verdicts:");
                let missing: Vec<&String> = result
                    .expected_verdicts
                    .difference(&result.llm_verdicts)
                    .collect();
                if missing.is_empty() {
                    println!("    []");
                } else {
                    for v in &missing {
                        println!("    - {}", v);
                    }
                }
                println!("  extra_verdicts:");
                let extra: Vec<&String> = result
                    .llm_verdicts
                    .difference(&result.expected_verdicts)
                    .collect();
                if extra.is_empty() {
                    println!("    []");
                } else {
                    for v in &extra {
                        println!("    - {}", v);
                    }
                }
                println!("  llm_confidence: {}", result.confidence);
                println!("  ambiguities_noted:");
                if result.ambiguities_noted.is_empty() {
                    println!("    []");
                } else {
                    for a in &result.ambiguities_noted {
                        println!("    - {}", a);
                    }
                }
                println!(
                    "  reasoning_excerpt: {}",
                    truncate_reasoning(&result.llm_reasoning, 200)
                );
                println!("  ...");
            }
        }

        let (total, matches, mismatches) = self.summary();
        println!("# tests      {}", total);
        println!("# matches    {}", matches);
        println!("# mismatches {}", mismatches);
    }

    /// Returns (total, matches, mismatches) counts.
    pub fn summary(&self) -> (usize, usize, usize) {
        let total = self.results.len();
        let matches = self
            .results
            .iter()
            .filter(|r| r.expected_verdicts == r.llm_verdicts)
            .count();
        let mismatches = total - matches;
        (total, matches, mismatches)
    }

    /// Returns only mismatched results for investigation.
    #[allow(dead_code)] // Public API: available for callers to filter mismatches for analysis
    pub fn ambiguity_signals(&self) -> Vec<&AmbiguityResult> {
        self.results
            .iter()
            .filter(|r| r.expected_verdicts != r.llm_verdicts)
            .collect()
    }
}

impl Default for AmbiguityReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate reasoning text for diagnostic output.
fn truncate_reasoning(s: &str, max_len: usize) -> String {
    // Replace newlines with spaces for single-line YAML output
    let oneline: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if oneline.len() <= max_len {
        format!("\"{}\"", oneline)
    } else {
        format!("\"{}...\"", &oneline[..max_len])
    }
}
