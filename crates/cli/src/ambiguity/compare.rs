//! Verdict set comparison with symmetric difference.

use serde::Deserialize;
use std::collections::BTreeSet;

/// The result of comparing expected vs actual verdict sets.
#[allow(dead_code)] // Public API: fields consumed by callers and tests to inspect comparison details
pub struct VerdictComparison {
    /// Whether the sets match exactly.
    pub is_match: bool,
    /// Verdicts in expected but not in actual (the LLM missed these).
    pub missing: BTreeSet<String>,
    /// Verdicts in actual but not in expected (the LLM produced extra).
    pub extra: BTreeSet<String>,
}

/// The expected JSON schema of the LLM's verdict response.
#[derive(Deserialize)]
pub struct LlmVerdictResponse {
    pub verdicts_produced: Vec<String>,
    pub reasoning: String,
    pub confidence: String,
    pub ambiguities_noted: Vec<String>,
}

/// Compare expected verdicts against actual (LLM-produced) verdicts.
///
/// Returns a `VerdictComparison` with the symmetric set difference:
/// - `missing`: in expected but not actual
/// - `extra`: in actual but not expected
pub fn compare_verdicts(expected: &[String], actual: &[String]) -> VerdictComparison {
    let expected_set: BTreeSet<String> = expected.iter().cloned().collect();
    let actual_set: BTreeSet<String> = actual.iter().cloned().collect();

    let missing: BTreeSet<String> = expected_set.difference(&actual_set).cloned().collect();
    let extra: BTreeSet<String> = actual_set.difference(&expected_set).cloned().collect();
    let is_match = missing.is_empty() && extra.is_empty();

    VerdictComparison {
        is_match,
        missing,
        extra,
    }
}

/// Parse the LLM's response text into a structured verdict response.
///
/// Handles the case where the LLM wraps JSON in markdown code fences.
pub fn parse_llm_response(response_text: &str) -> Result<LlmVerdictResponse, String> {
    let cleaned = strip_code_fences(response_text);
    serde_json::from_str::<LlmVerdictResponse>(&cleaned).map_err(|e| {
        format!(
            "Failed to parse LLM response as JSON: {}. Response text: {}",
            e,
            truncate(response_text, 200)
        )
    })
}

/// Strip markdown code fences from text (```json ... ``` or ``` ... ```).
fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();

    // Check for code fence start
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip optional language tag on the opening fence line
        let after_tag = if let Some(newline_pos) = rest.find('\n') {
            &rest[newline_pos + 1..]
        } else {
            rest
        };

        // Strip trailing code fence
        if let Some(content) = after_tag.strip_suffix("```") {
            return content.trim().to_string();
        }
        return after_tag.trim().to_string();
    }

    trimmed.to_string()
}

/// Truncate a string to at most `max_len` characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_exact_match() {
        let expected = vec!["a".to_string(), "b".to_string()];
        let actual = vec!["b".to_string(), "a".to_string()];
        let cmp = compare_verdicts(&expected, &actual);
        assert!(cmp.is_match);
        assert!(cmp.missing.is_empty());
        assert!(cmp.extra.is_empty());
    }

    #[test]
    fn test_compare_missing_and_extra() {
        let expected = vec!["a".to_string(), "b".to_string()];
        let actual = vec!["b".to_string(), "c".to_string()];
        let cmp = compare_verdicts(&expected, &actual);
        assert!(!cmp.is_match);
        assert_eq!(cmp.missing, BTreeSet::from(["a".to_string()]));
        assert_eq!(cmp.extra, BTreeSet::from(["c".to_string()]));
    }

    #[test]
    fn test_compare_both_empty() {
        let cmp = compare_verdicts(&[], &[]);
        assert!(cmp.is_match);
    }

    #[test]
    fn test_parse_llm_response_clean_json() {
        let json = r#"{"verdicts_produced":["a","b"],"reasoning":"step 1","confidence":"high","ambiguities_noted":[]}"#;
        let resp = parse_llm_response(json).unwrap();
        assert_eq!(resp.verdicts_produced, vec!["a", "b"]);
        assert_eq!(resp.confidence, "high");
    }

    #[test]
    fn test_parse_llm_response_code_fences() {
        let text = "```json\n{\"verdicts_produced\":[],\"reasoning\":\"none\",\"confidence\":\"low\",\"ambiguities_noted\":[\"unclear\"]}\n```";
        let resp = parse_llm_response(text).unwrap();
        assert!(resp.verdicts_produced.is_empty());
        assert_eq!(resp.ambiguities_noted, vec!["unclear"]);
    }

    #[test]
    fn test_strip_code_fences_no_fences() {
        assert_eq!(strip_code_fences("hello"), "hello");
    }

    #[test]
    fn test_strip_code_fences_with_lang_tag() {
        assert_eq!(strip_code_fences("```json\n{}\n```"), "{}");
    }

    #[test]
    fn test_strip_code_fences_no_lang_tag() {
        assert_eq!(strip_code_fences("```\n{}\n```"), "{}");
    }
}
