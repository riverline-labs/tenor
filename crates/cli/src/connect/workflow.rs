//! Interactive review, batch export, and apply workflows for `tenor connect`.
//!
//! Provides three modes:
//! - **Interactive**: prompts the user to accept/reject/edit each proposed mapping
//! - **Batch**: writes proposals to a TOML review file for offline editing
//! - **Apply**: reads an edited review file and returns accepted mappings

use std::io::{self, BufRead, Write};
use std::path::Path;

use super::matching::Confidence;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A proposal from the matching pipeline, ready for user review.
///
/// This is a self-contained snapshot — it carries all fields needed for
/// display, serialisation to TOML, and downstream adapter-config generation.
#[derive(Debug, Clone)]
pub struct MappingProposal {
    pub fact_id: String,
    pub fact_type: String,
    pub source_id: String,
    pub endpoint: String,
    pub field_path: String,
    pub confidence: Confidence,
    pub explanation: String,
}

/// User disposition for a single mapping proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStatus {
    Accepted,
    Rejected,
    Skipped,
}

/// A mapping that has been reviewed by a human (interactive or batch).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReviewedMapping {
    pub fact_id: String,
    pub source_id: String,
    pub endpoint: String,
    pub field_path: String,
    pub confidence: String,
    pub explanation: String,
    pub status: MappingStatus,
}

// ---------------------------------------------------------------------------
// 4A  Interactive mode
// ---------------------------------------------------------------------------

/// Run an interactive review session over `proposals`, reading from `stdin`.
///
/// Each proposal is printed with an index and the user is prompted for a
/// disposition. Returns the full list of reviewed mappings.
pub fn run_interactive(proposals: &[MappingProposal], quiet: bool) -> Vec<ReviewedMapping> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    run_interactive_with_reader(proposals, quiet, &mut reader)
}

/// Testable inner implementation that accepts any `BufRead`.
fn run_interactive_with_reader<R: BufRead>(
    proposals: &[MappingProposal],
    quiet: bool,
    reader: &mut R,
) -> Vec<ReviewedMapping> {
    if !quiet {
        println!(
            "\nProposed mappings ({} facts, {} proposals):\n",
            count_unique_facts(proposals),
            proposals.len(),
        );
    }

    let mut reviewed: Vec<ReviewedMapping> = Vec::new();

    for (idx, p) in proposals.iter().enumerate() {
        if !quiet {
            println!("  {}. {} ({})", idx + 1, p.fact_id, p.fact_type);
            println!("     -> {} -> {}", p.endpoint, p.field_path);
            println!("     Confidence: {}", confidence_label(p.confidence));
            println!("     Reason: {}", p.explanation);
            print!("     [a]ccept / [r]eject / [e]dit / [s]kip? ");
        }
        let _ = io::stdout().flush();

        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            line = String::new(); // treat read error as empty (accept)
        }
        let choice = line.trim().to_lowercase();

        let (status, endpoint, field_path) = match choice.as_str() {
            "r" | "reject" => (
                MappingStatus::Rejected,
                p.endpoint.clone(),
                p.field_path.clone(),
            ),
            "s" | "skip" => (
                MappingStatus::Skipped,
                p.endpoint.clone(),
                p.field_path.clone(),
            ),
            "e" | "edit" => {
                let (ep, fp) = prompt_edit(reader, &p.endpoint, &p.field_path, quiet);
                (MappingStatus::Accepted, ep, fp)
            }
            _ => {
                // "a", "accept", empty, or anything else defaults to accept
                (
                    MappingStatus::Accepted,
                    p.endpoint.clone(),
                    p.field_path.clone(),
                )
            }
        };

        reviewed.push(ReviewedMapping {
            fact_id: p.fact_id.clone(),
            source_id: p.source_id.clone(),
            endpoint,
            field_path,
            confidence: confidence_label(p.confidence).to_string(),
            explanation: p.explanation.clone(),
            status,
        });
    }

    if !quiet {
        let accepted = reviewed
            .iter()
            .filter(|r| r.status == MappingStatus::Accepted)
            .count();
        let rejected = reviewed
            .iter()
            .filter(|r| r.status == MappingStatus::Rejected)
            .count();
        let skipped = reviewed
            .iter()
            .filter(|r| r.status == MappingStatus::Skipped)
            .count();
        println!(
            "\nSummary: {} accepted, {} rejected, {} skipped",
            accepted, rejected, skipped
        );
    }

    reviewed
}

/// Prompt the user for corrected endpoint and field path.
fn prompt_edit<R: BufRead>(
    reader: &mut R,
    default_endpoint: &str,
    default_field: &str,
    quiet: bool,
) -> (String, String) {
    if !quiet {
        print!("     Endpoint [{}]: ", default_endpoint);
    }
    let _ = io::stdout().flush();
    let mut ep_line = String::new();
    let _ = reader.read_line(&mut ep_line);
    let ep = ep_line.trim();
    let endpoint = if ep.is_empty() {
        default_endpoint.to_string()
    } else {
        ep.to_string()
    };

    if !quiet {
        print!("     Field path [{}]: ", default_field);
    }
    let _ = io::stdout().flush();
    let mut fp_line = String::new();
    let _ = reader.read_line(&mut fp_line);
    let fp = fp_line.trim();
    let field_path = if fp.is_empty() {
        default_field.to_string()
    } else {
        fp.to_string()
    };

    (endpoint, field_path)
}

// ---------------------------------------------------------------------------
// 4B  Batch mode — write review file
// ---------------------------------------------------------------------------

/// Write all proposals to a TOML review file at `output_path`.
///
/// Each mapping entry has `status = "proposed"` — the user edits this to
/// `"accepted"` or `"rejected"` before running `tenor connect --apply`.
pub fn write_review_file(proposals: &[MappingProposal], output_path: &Path) -> Result<(), String> {
    let mut buf = String::new();
    buf.push_str(
        "# Generated by tenor connect \u{2014} review and edit before applying\n\
         # To apply: tenor connect --apply <this-file>\n\n",
    );

    for p in proposals {
        buf.push_str("[[mapping]]\n");
        buf.push_str(&format!("fact_id = {}\n", toml_quote(&p.fact_id)));
        buf.push_str(&format!("fact_type = {}\n", toml_quote(&p.fact_type)));
        buf.push_str(&format!("source_id = {}\n", toml_quote(&p.source_id)));
        buf.push_str(&format!("endpoint = {}\n", toml_quote(&p.endpoint)));
        buf.push_str(&format!("field_path = {}\n", toml_quote(&p.field_path)));
        buf.push_str(&format!(
            "confidence = {}\n",
            toml_quote(confidence_label(p.confidence))
        ));
        buf.push_str(&format!("explanation = {}\n", toml_quote(&p.explanation)));
        buf.push_str("status = \"proposed\"  # Change to \"accepted\" or \"rejected\"\n");
        buf.push('\n');
    }

    std::fs::write(output_path, buf).map_err(|e| {
        format!(
            "failed to write review file '{}': {}",
            output_path.display(),
            e
        )
    })
}

// ---------------------------------------------------------------------------
// 4C  Apply mode — read review file
// ---------------------------------------------------------------------------

/// Read a TOML review file, returning only mappings whose status is
/// `"accepted"`.
pub fn read_review_file(path: &Path) -> Result<Vec<ReviewedMapping>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read review file '{}': {}", path.display(), e))?;

    let table: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("invalid TOML in '{}': {}", path.display(), e))?;

    let mappings = table
        .get("mapping")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("no [[mapping]] entries found in '{}'", path.display()))?;

    let mut result = Vec::new();

    for entry in mappings {
        let status_str = entry
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("proposed");

        let status = match status_str {
            "accepted" => MappingStatus::Accepted,
            "rejected" => MappingStatus::Rejected,
            "skipped" => MappingStatus::Skipped,
            _ => continue, // "proposed" or unknown — skip
        };

        if status != MappingStatus::Accepted {
            continue;
        }

        let get_str = |key: &str| -> String {
            entry
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        result.push(ReviewedMapping {
            fact_id: get_str("fact_id"),
            source_id: get_str("source_id"),
            endpoint: get_str("endpoint"),
            field_path: get_str("field_path"),
            confidence: get_str("confidence"),
            explanation: get_str("explanation"),
            status,
        });
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn confidence_label(c: Confidence) -> &'static str {
    match c {
        Confidence::High => "HIGH",
        Confidence::Medium => "MEDIUM",
        Confidence::Low => "LOW",
    }
}

fn count_unique_facts(proposals: &[MappingProposal]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for p in proposals {
        seen.insert(&p.fact_id);
    }
    seen.len()
}

/// Escape and quote a string for TOML output.
fn toml_quote(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample_proposals() -> Vec<MappingProposal> {
        vec![
            MappingProposal {
                fact_id: "escrow_amount".into(),
                fact_type: "Money(USD)".into(),
                source_id: "escrow_service".into(),
                endpoint: "GET /accounts/{id}/balance".into(),
                field_path: "balance.amount".into(),
                confidence: Confidence::High,
                explanation: "Source path directly maps to balance amount".into(),
            },
            MappingProposal {
                fact_id: "loan_status".into(),
                fact_type: "Text".into(),
                source_id: "loan_service".into(),
                endpoint: "GET /loans/{id}".into(),
                field_path: "status".into(),
                confidence: Confidence::Medium,
                explanation: "Partial path match on status field".into(),
            },
            MappingProposal {
                fact_id: "interest_rate".into(),
                fact_type: "Decimal(10,4)".into(),
                source_id: "rate_service".into(),
                endpoint: "GET /rates/current".into(),
                field_path: "rates.interest".into(),
                confidence: Confidence::Low,
                explanation: "Inferred from path, no schema available".into(),
            },
        ]
    }

    // --- 4D.1: write_review_file produces valid TOML with correct structure ---

    #[test]
    fn test_write_review_file_produces_valid_toml() {
        let proposals = sample_proposals();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review.toml");

        write_review_file(&proposals, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();

        // Must parse as valid TOML
        let table: toml::Value = toml::from_str(&content).expect("output must be valid TOML");
        let mappings = table["mapping"].as_array().unwrap();
        assert_eq!(mappings.len(), 3);

        // First entry spot-checks
        assert_eq!(mappings[0]["fact_id"].as_str().unwrap(), "escrow_amount");
        assert_eq!(mappings[0]["fact_type"].as_str().unwrap(), "Money(USD)");
        assert_eq!(mappings[0]["source_id"].as_str().unwrap(), "escrow_service");
        assert_eq!(
            mappings[0]["endpoint"].as_str().unwrap(),
            "GET /accounts/{id}/balance"
        );
        assert_eq!(
            mappings[0]["field_path"].as_str().unwrap(),
            "balance.amount"
        );
        assert_eq!(mappings[0]["confidence"].as_str().unwrap(), "HIGH");
        assert_eq!(mappings[0]["status"].as_str().unwrap(), "proposed");
    }

    // --- 4D.2: read_review_file parses and filters by status ---

    #[test]
    fn test_read_review_file_filters_by_status() {
        let toml_content = r#"
# Review file
[[mapping]]
fact_id = "escrow_amount"
source_id = "escrow_service"
endpoint = "GET /accounts/{id}/balance"
field_path = "balance.amount"
confidence = "HIGH"
explanation = "Direct match"
status = "accepted"

[[mapping]]
fact_id = "loan_status"
source_id = "loan_service"
endpoint = "GET /loans/{id}"
field_path = "status"
confidence = "MEDIUM"
explanation = "Partial match"
status = "rejected"

[[mapping]]
fact_id = "interest_rate"
source_id = "rate_service"
endpoint = "GET /rates/current"
field_path = "rates.interest"
confidence = "LOW"
explanation = "Inferred"
status = "proposed"
"#;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review.toml");
        std::fs::write(&path, toml_content).unwrap();

        let result = read_review_file(&path).unwrap();

        // Only "accepted" entries returned
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fact_id, "escrow_amount");
        assert_eq!(result[0].status, MappingStatus::Accepted);
        assert_eq!(result[0].confidence, "HIGH");
        assert_eq!(result[0].endpoint, "GET /accounts/{id}/balance");
    }

    // --- 4D.3: roundtrip write -> modify -> read ---

    #[test]
    fn test_roundtrip_write_modify_read() {
        let proposals = sample_proposals();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("review.toml");

        // Write
        write_review_file(&proposals, &path).unwrap();

        // Modify: accept first, reject second, leave third as proposed
        let content = std::fs::read_to_string(&path).unwrap();
        let modified = content
            .replacen("status = \"proposed\"", "status = \"accepted\"", 1)
            .replacen("status = \"proposed\"", "status = \"rejected\"", 1);
        std::fs::write(&path, modified).unwrap();

        // Read
        let result = read_review_file(&path).unwrap();

        // Only the accepted one
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fact_id, "escrow_amount");
        assert_eq!(result[0].source_id, "escrow_service");
        assert_eq!(result[0].endpoint, "GET /accounts/{id}/balance");
        assert_eq!(result[0].field_path, "balance.amount");
        assert_eq!(result[0].confidence, "HIGH");
        assert_eq!(result[0].status, MappingStatus::Accepted);
    }

    // --- Interactive mode tests (using BufRead reader) ---

    #[test]
    fn test_interactive_accept_default() {
        let proposals = vec![sample_proposals().remove(0)];
        // Empty line = accept by default
        let input = b"\n";
        let mut reader = Cursor::new(input.as_slice());

        let result = run_interactive_with_reader(&proposals, true, &mut reader);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, MappingStatus::Accepted);
        assert_eq!(result[0].fact_id, "escrow_amount");
    }

    #[test]
    fn test_interactive_reject() {
        let proposals = vec![sample_proposals().remove(0)];
        let input = b"r\n";
        let mut reader = Cursor::new(input.as_slice());

        let result = run_interactive_with_reader(&proposals, true, &mut reader);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, MappingStatus::Rejected);
    }

    #[test]
    fn test_interactive_skip() {
        let proposals = vec![sample_proposals().remove(0)];
        let input = b"s\n";
        let mut reader = Cursor::new(input.as_slice());

        let result = run_interactive_with_reader(&proposals, true, &mut reader);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, MappingStatus::Skipped);
    }

    #[test]
    fn test_interactive_edit() {
        let proposals = vec![sample_proposals().remove(0)];
        // "e" then new endpoint, new field_path
        let input = b"e\nPOST /v2/balance\namount.total\n";
        let mut reader = Cursor::new(input.as_slice());

        let result = run_interactive_with_reader(&proposals, true, &mut reader);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, MappingStatus::Accepted);
        assert_eq!(result[0].endpoint, "POST /v2/balance");
        assert_eq!(result[0].field_path, "amount.total");
    }

    #[test]
    fn test_interactive_multiple_proposals() {
        let proposals = sample_proposals();
        // accept, reject, skip
        let input = b"a\nr\ns\n";
        let mut reader = Cursor::new(input.as_slice());

        let result = run_interactive_with_reader(&proposals, true, &mut reader);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].status, MappingStatus::Accepted);
        assert_eq!(result[1].status, MappingStatus::Rejected);
        assert_eq!(result[2].status, MappingStatus::Skipped);
    }
}
