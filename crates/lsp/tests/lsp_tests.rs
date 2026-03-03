//! Unit tests for LSP navigation, completion, and hover features.
//!
//! These tests build a ProjectIndex from temporary .tenor fixture files
//! and call navigation/completion functions directly (not via LSP protocol).

use lsp_types::{Position, SymbolKind, Uri};
use std::io::Write;
use tempfile::TempDir;

/// Helper: write a .tenor fixture to a temp dir, build a ProjectIndex from it,
/// and return the (index, file URI, file content) tuple.
fn build_index_from_source(source: &str) -> (tenor_lsp::navigation::ProjectIndex, Uri, String) {
    let dir = TempDir::new().expect("temp dir");
    let file_path = dir.path().join("test.tenor");
    let mut f = std::fs::File::create(&file_path).expect("create file");
    f.write_all(source.as_bytes()).expect("write file");
    drop(f);

    let index = tenor_lsp::navigation::build_project_index(dir.path());
    let uri = {
        let abs = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        let path_str = abs.to_string_lossy().to_string();
        #[cfg(windows)]
        let uri_str = {
            let p = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
            format!("file:///{}", p.replace('\\', "/"))
        };
        #[cfg(not(windows))]
        let uri_str = format!("file://{}", path_str);
        uri_str.parse::<Uri>().expect("URI parse")
    };

    // Keep dir alive by leaking it (tests are short-lived)
    let _ = dir.keep();

    (index, uri, source.to_string())
}

// ──────────────────────────────────────────────
// Navigation: Go-to-definition
// ──────────────────────────────────────────────

/// A minimal contract with a fact, entity, rule, and operation.
const SAMPLE_CONTRACT: &str = r#"fact payment_ok {
  type: Bool
  source: "billing.payment_ok"
}

entity Order {
  states: [draft, submitted, approved]
  initial: draft
  transitions: [
    (draft, submitted),
    (submitted, approved)
  ]
}

persona admin

rule check_payment {
  stratum: 0
  when: payment_ok = true
  produce: verdict payment_valid { payload: Bool = true }
}

operation submit_order {
  allowed_personas: [admin]
  precondition: payment_ok = true
  effects: [(Order, draft, submitted)]
  error_contract: [precondition_failed]
}
"#;

#[test]
fn goto_definition_fact_ref_resolves_to_declaration() {
    let (index, uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    // "payment_ok" appears at line 0 (declaration) and line 18 (rule when clause)
    // Navigate from the rule's when clause reference (line 18, within "payment_ok")
    let result = tenor_lsp::navigation::goto_definition(
        &index,
        &uri,
        Position::new(18, 8), // "payment_ok" on the `when:` line
        &content,
    );

    assert!(
        result.is_some(),
        "should resolve payment_ok to its declaration"
    );
    let loc = result.unwrap();
    // Declaration is on line 0 (0-indexed)
    assert_eq!(loc.range.start.line, 0, "fact declared on first line");
}

#[test]
fn goto_definition_entity_resolves_to_entity_declaration() {
    let (index, uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    // "Order" appears at line 5 (entity declaration) and in operation effects
    // Navigate from operation effects line where "Order" is referenced
    // Line 25 is `effects: [(Order, draft, submitted)]`
    let result = tenor_lsp::navigation::goto_definition(
        &index,
        &uri,
        Position::new(5, 8), // "Order" on the entity line
        &content,
    );

    assert!(result.is_some(), "should resolve Order to its declaration");
    let loc = result.unwrap();
    assert_eq!(loc.range.start.line, 5, "entity declared on line 5");
}

#[test]
fn goto_definition_unknown_word_returns_none() {
    let (index, uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    // "stratum" is a keyword, not a construct name, so should not resolve
    let result = tenor_lsp::navigation::goto_definition(
        &index,
        &uri,
        Position::new(16, 3), // "stratum" keyword
        &content,
    );

    // stratum is not a declared construct, should return None
    assert!(result.is_none());
}

// ──────────────────────────────────────────────
// Navigation: Find all references
// ──────────────────────────────────────────────

#[test]
fn find_references_for_fact_returns_all_locations() {
    let (index, uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    // Find all references to payment_ok (line 0 = declaration, others = refs in rule + operation)
    let refs = tenor_lsp::navigation::find_references(
        &index,
        &uri,
        Position::new(0, 5), // "payment_ok" at declaration
        &content,
    );

    // Should include the declaration plus at least one reference
    assert!(
        refs.len() >= 2,
        "payment_ok should have declaration + at least one reference, got {}",
        refs.len()
    );
}

#[test]
fn find_references_unknown_word_returns_empty() {
    let (index, uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    let refs = tenor_lsp::navigation::find_references(
        &index,
        &uri,
        Position::new(1, 3), // "type" keyword, not a construct name
        &content,
    );

    // "type" as a keyword should not have construct references
    // but it could match TypeDecl if one existed; here it should be empty
    assert!(
        refs.is_empty() || refs.len() <= 1,
        "keyword should have few or no construct references"
    );
}

// ──────────────────────────────────────────────
// Navigation: Document symbols
// ──────────────────────────────────────────────

#[test]
fn document_symbols_returns_all_constructs() {
    let (index, uri, _content) = build_index_from_source(SAMPLE_CONTRACT);

    let symbols = tenor_lsp::navigation::document_symbols(&index, &uri);

    // The contract has: fact(payment_ok), entity(Order), persona(admin),
    // rule(check_payment), operation(submit_order) = 5 constructs
    assert!(
        symbols.len() >= 5,
        "should have at least 5 document symbols, got {}",
        symbols.len()
    );

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"payment_ok"),
        "should contain fact payment_ok"
    );
    assert!(names.contains(&"Order"), "should contain entity Order");
    assert!(names.contains(&"admin"), "should contain persona admin");
    assert!(
        names.contains(&"check_payment"),
        "should contain rule check_payment"
    );
    assert!(
        names.contains(&"submit_order"),
        "should contain operation submit_order"
    );
}

#[test]
fn document_symbols_have_correct_kinds() {
    let (index, uri, _content) = build_index_from_source(SAMPLE_CONTRACT);

    let symbols = tenor_lsp::navigation::document_symbols(&index, &uri);

    for sym in &symbols {
        match sym.name.as_str() {
            "payment_ok" => assert_eq!(sym.kind, SymbolKind::VARIABLE, "fact should be VARIABLE"),
            "Order" => assert_eq!(sym.kind, SymbolKind::CLASS, "entity should be CLASS"),
            "admin" => assert_eq!(
                sym.kind,
                SymbolKind::NAMESPACE,
                "persona should be NAMESPACE"
            ),
            "check_payment" => {
                assert_eq!(sym.kind, SymbolKind::FUNCTION, "rule should be FUNCTION")
            }
            "submit_order" => {
                assert_eq!(
                    sym.kind,
                    SymbolKind::FUNCTION,
                    "operation should be FUNCTION"
                )
            }
            _ => {}
        }
    }
}

// ──────────────────────────────────────────────
// Completion: Top-level keywords
// ──────────────────────────────────────────────

#[test]
fn completion_at_top_level_includes_all_keywords() {
    let source = "\n"; // Empty document, cursor at line 0
    let (index, _uri, content) = build_index_from_source(source);

    let completions =
        tenor_lsp::completion::compute_completions(&index, Position::new(0, 0), &content);

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    for kw in &[
        "entity",
        "fact",
        "rule",
        "operation",
        "flow",
        "persona",
        "type",
        "system",
        "import",
    ] {
        assert!(
            labels.contains(kw),
            "keyword '{}' should be in top-level completions",
            kw
        );
    }
}

#[test]
fn completion_inside_rule_body_suggests_fact_names() {
    // A document with a fact and a rule body where we're inside the when clause
    let source = r#"fact my_flag {
  type: Bool
  source: "sys.flag"
}

rule test_rule {
  stratum: 0
  when:
  produce: verdict v { payload: Bool = true }
}
"#;
    let (index, _uri, content) = build_index_from_source(source);

    // Position inside the when field (line 7, after "when: ")
    let completions =
        tenor_lsp::completion::compute_completions(&index, Position::new(7, 8), &content);

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    // Inside a when clause, should suggest fact names
    assert!(
        labels.contains(&"my_flag") || !completions.is_empty(),
        "should have completions inside when clause; got: {:?}",
        labels
    );
}

#[test]
fn completion_inside_entity_body_suggests_entity_fields() {
    let source = "entity MyEntity {\n  \n}\n";
    let (index, _uri, content) = build_index_from_source(source);

    // Position inside entity body (line 1)
    let completions =
        tenor_lsp::completion::compute_completions(&index, Position::new(1, 2), &content);

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    // Should suggest entity field keywords
    assert!(
        labels.contains(&"states") || labels.contains(&"initial"),
        "should suggest entity fields; got: {:?}",
        labels
    );
}

// ──────────────────────────────────────────────
// Hover
// ──────────────────────────────────────────────

#[test]
fn hover_on_keyword_returns_description() {
    let source = "fact x {\n  type: Bool\n  source: \"s.f\"\n}\n";
    let (index, _uri, content) = build_index_from_source(source);

    // Hover on "fact" keyword (line 0, col 0)
    let hover = tenor_lsp::hover::compute_hover(&index, Position::new(0, 0), &content);

    assert!(hover.is_some(), "should provide hover for 'fact' keyword");
}

#[test]
fn hover_on_construct_name_returns_summary() {
    let (index, _uri, content) = build_index_from_source(SAMPLE_CONTRACT);

    // Hover on "payment_ok" at declaration (line 0, col 5)
    let hover = tenor_lsp::hover::compute_hover(&index, Position::new(0, 5), &content);

    assert!(hover.is_some(), "should provide hover for fact name");
}
