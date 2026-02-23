//! Hover information provider for construct references.
//!
//! Shows type and summary information when hovering over construct names
//! in `.tenor` files.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::navigation::{get_word_at_position, ProjectIndex};

/// Compute hover information for the word at the given position.
///
/// Checks construct references first (facts, entities, operations, etc.)
/// then falls back to keyword descriptions for DSL keywords and built-in types.
pub fn compute_hover(index: &ProjectIndex, position: Position, content: &str) -> Option<Hover> {
    let word = get_word_at_position(content, position)?;

    // Try each construct kind to find a matching summary
    for kind in &[
        "Fact",
        "Entity",
        "Operation",
        "Flow",
        "Persona",
        "TypeDecl",
        "Rule",
        "System",
    ] {
        let key = (kind.to_string(), word.clone());
        if let Some(summary) = index.summaries.get(&key) {
            let markdown = format!("```tenor\n{}\n```", summary.detail);
            return Some(make_hover(markdown));
        }
    }

    // Fall back to keyword/type hover info
    keyword_hover(&word)
}

/// Create a Hover with markdown content.
fn make_hover(markdown: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    }
}

/// Provide hover info for DSL keywords and built-in types.
fn keyword_hover(word: &str) -> Option<Hover> {
    let desc = match word {
        // Construct keywords
        "fact" => "**fact** -- declares an input variable with a type and data source",
        "entity" => "**entity** -- declares a stateful object with states and transitions",
        "rule" => "**rule** -- declares a verdict-producing rule with a when-condition",
        "operation" => {
            "**operation** -- declares a state-changing action with personas and effects"
        }
        "flow" => "**flow** -- declares a multi-step workflow with branching and failure handling",
        "persona" => "**persona** -- declares an actor role in the contract",
        "system" => {
            "**system** -- declares a multi-contract system with shared entities and triggers"
        }
        // Built-in types
        "Bool" => "**Bool** -- boolean type (`true` / `false`)",
        "Int" => "**Int** -- integer type with optional `(min, max)` bounds",
        "Decimal" => "**Decimal** -- fixed-precision decimal with `(precision, scale)`",
        "Text" => "**Text** -- string type with optional `max_length`",
        "Date" => "**Date** -- calendar date (ISO 8601)",
        "DateTime" => "**DateTime** -- date and time (ISO 8601)",
        "Money" => "**Money** -- currency amount with `(currency: \"XXX\")`",
        "Duration" => "**Duration** -- time interval with `(unit, min, max)`",
        "Enum" => "**Enum** -- enumerated string values",
        "Record" => "**Record** -- structured type with named fields",
        "List" => "**List** -- ordered collection with `(element_type, max)`",
        _ => return None,
    };
    Some(make_hover(desc.to_string()))
}
