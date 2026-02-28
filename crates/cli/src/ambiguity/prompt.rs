//! Prompt construction from spec sections + contract + facts.
//!
//! Builds a layered prompt: system prompt (spec sections + role) and
//! user prompt (contract source + facts + evaluation instructions).

use std::path::Path;

/// Default spec sections to extract for rule evaluation context.
pub const DEFAULT_SECTIONS: &[u32] = &[4, 7, 10, 12];

// ── System prompt ────────────────────────────────────────────────────────────

/// Build the system prompt by reading the spec and extracting relevant sections.
///
/// Reads the spec from `spec_path` (typically `docs/tenor-language-specification.md`) and extracts
/// Sections 4 (BaseType/TypeDecl), 7 (Rule), 10 (PredicateExpression),
/// and 12 (NumericModel).
pub fn build_system_prompt(spec_path: &Path) -> Result<String, String> {
    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|e| format!("Cannot read spec file {}: {}", spec_path.display(), e))?;

    let sections = extract_spec_sections(&spec_text, DEFAULT_SECTIONS);

    Ok(format!(
        "You are a Tenor specification evaluator. Given the spec sections below,\n\
         a Tenor contract, and a set of fact values, determine which verdicts are\n\
         produced by evaluating the contract's rules against the facts.\n\
         Follow the specification text EXACTLY. Do not infer rules beyond what the\n\
         spec states. Show your reasoning step by step.\n\
         \n\
         {}",
        sections
    ))
}

// ── User prompt ──────────────────────────────────────────────────────────────

/// Build the user prompt from contract source and facts.
pub fn build_user_prompt(contract_source: &str, facts: &serde_json::Value) -> String {
    let facts_pretty = serde_json::to_string_pretty(facts).unwrap_or_else(|_| facts.to_string());

    format!(
        "## Contract\n\
         {}\n\
         \n\
         ## Fact Values\n\
         {}\n\
         \n\
         ## Instructions\n\
         Evaluate all rules in this contract against the provided fact values.\n\
         For each rule, evaluate its `when` condition using the spec's PredicateExpression\n\
         evaluation rules. If the condition is satisfied, the rule produces its declared verdict.\n\
         Rules are evaluated in stratum order (stratum 0 first, then stratum 1).\n\
         A stratum 1 rule can reference verdicts produced by stratum 0 rules via verdict_present().\n\
         \n\
         Return your response as JSON with this exact schema:\n\
         {{\n\
           \"verdicts_produced\": [\"verdict_id_1\", \"verdict_id_2\"],\n\
           \"reasoning\": \"step-by-step evaluation trace\",\n\
           \"confidence\": \"high|medium|low\",\n\
           \"ambiguities_noted\": [\"any unclear spec areas encountered\"]\n\
         }}",
        contract_source, facts_pretty
    )
}

// ── Section extraction ───────────────────────────────────────────────────────

/// Extract top-level sections from the spec text by section number.
///
/// Finds lines matching `## N.` or `## N ` (where N is a requested section
/// number) and captures all content until the next `## M` heading at the
/// same level (where M is a different number).
pub fn extract_spec_sections(spec_text: &str, sections: &[u32]) -> String {
    let mut extracted = Vec::new();

    let lines: Vec<&str> = spec_text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if let Some(section_num) = parse_section_heading(lines[i]) {
            if sections.contains(&section_num) {
                // Collect this section's content until the next top-level heading
                let mut section_lines = vec![lines[i]];
                i += 1;
                while i < lines.len() {
                    if parse_section_heading(lines[i]).is_some() {
                        break;
                    }
                    section_lines.push(lines[i]);
                    i += 1;
                }
                extracted.push(section_lines.join("\n"));
                continue; // Don't increment i again
            }
        }
        i += 1;
    }

    extracted.join("\n\n")
}

/// Parse a line as a top-level section heading (`## N.` or `## N `).
/// Returns Some(N) if it matches, None otherwise.
///
/// Only matches numbered top-level sections (not appendices like `## Appendix A`).
fn parse_section_heading(line: &str) -> Option<u32> {
    let trimmed = line.trim();
    if !trimmed.starts_with("## ") {
        return None;
    }

    let after_hash = &trimmed[3..];
    // Extract the number: could be "4. BaseType" or "4 BaseType"
    let num_str: String = after_hash
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if num_str.is_empty() {
        return None;
    }

    num_str.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_section_heading_with_dot() {
        assert_eq!(parse_section_heading("## 4. BaseType"), Some(4));
    }

    #[test]
    fn test_parse_section_heading_with_space() {
        assert_eq!(parse_section_heading("## 7 Rule"), Some(7));
    }

    #[test]
    fn test_parse_section_heading_appendix() {
        assert_eq!(parse_section_heading("## Appendix A"), None);
    }

    #[test]
    fn test_parse_section_heading_not_heading() {
        assert_eq!(parse_section_heading("### 4.1 Subsection"), None);
    }

    #[test]
    fn test_extract_spec_sections() {
        let spec = "## 1. First\nContent 1\n## 2. Second\nContent 2\n## 3. Third\nContent 3\n";
        let result = extract_spec_sections(spec, &[1, 3]);
        assert!(result.contains("Content 1"));
        assert!(!result.contains("Content 2"));
        assert!(result.contains("Content 3"));
    }

    #[test]
    fn test_build_user_prompt_contains_contract() {
        let facts = serde_json::json!({"x": 5});
        let prompt = build_user_prompt("fact x { type: Int }", &facts);
        assert!(prompt.contains("fact x { type: Int }"));
        assert!(prompt.contains("\"x\": 5"));
        assert!(prompt.contains("verdicts_produced"));
    }
}
