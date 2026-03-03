//! Context-aware completion provider for Tenor DSL.
//!
//! Offers completions based on the cursor position within a construct body:
//! construct keywords at top level, field keywords within bodies, and
//! construct/type references in appropriate contexts.

use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind, Position,
};

use crate::navigation::{get_construct_context, get_field_context, ProjectIndex};

/// Built-in type names offered as completions.
static BUILTIN_TYPES: &[&str] = &[
    "Bool", "Int", "Decimal", "Text", "Date", "DateTime", "Money", "Duration", "Enum", "Record",
    "List",
];

/// Top-level construct keywords.
static CONSTRUCT_KEYWORDS: &[&str] = &[
    "entity",
    "fact",
    "rule",
    "operation",
    "flow",
    "persona",
    "type",
    "system",
    "import",
];

/// Compute completions for the given position in the document.
pub fn compute_completions(
    index: &ProjectIndex,
    position: Position,
    content: &str,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let construct_ctx = get_construct_context(content, position);
    let field_ctx = get_field_context(content, position);

    match (construct_ctx.as_deref(), field_ctx.as_deref()) {
        // Top level: offer construct keywords
        (None, _) => {
            for kw in CONSTRUCT_KEYWORDS {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some("construct keyword".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside entity body: offer entity field keywords
        (Some("entity"), None) => {
            for kw in &["states", "initial", "transitions", "parent"] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some("entity field".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside operation body: offer operation field keywords
        (Some("operation"), None) => {
            for kw in &[
                "allowed_personas",
                "precondition",
                "effects",
                "outcomes",
                "error_contract",
            ] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some("operation field".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside rule body: offer rule field keywords
        (Some("rule"), None) => {
            for kw in &["stratum", "when", "produce"] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some("rule field".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside flow body: offer flow field keywords
        (Some("flow"), None) => {
            for kw in &["snapshot", "entry", "steps"] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some("flow field".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside fact body: offer fact field keywords
        (Some("fact"), None) => {
            for kw in &["type", "source", "default"] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some("fact field".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside when/precondition: offer fact names and verdict_present
        (_, Some("when")) | (_, Some("precondition")) => {
            add_fact_completions(index, &mut items);
            items.push(CompletionItem {
                label: "verdict_present".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("check if verdict exists".to_string()),
                ..Default::default()
            });
        }
        // Inside effects: offer entity names
        (_, Some("effects")) => {
            add_entity_completions(index, &mut items);
        }
        // Inside allowed_personas: offer persona names
        (_, Some("allowed_personas")) => {
            add_persona_completions(index, &mut items);
        }
        // Inside steps: offer operation names and step type keywords
        (_, Some("steps")) => {
            add_operation_completions(index, &mut items);
            for kw in &[
                "OperationStep",
                "BranchStep",
                "HandoffStep",
                "SubFlowStep",
                "ParallelStep",
            ] {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("flow step type".to_string()),
                    ..Default::default()
                });
            }
        }
        // Inside entry: offer step names (not available from index, skip)
        // Inside produce/outcomes: offer type names
        (_, Some("produce")) | (_, Some("outcomes")) => {
            add_type_completions(index, &mut items);
        }
        // Default for any construct body without specific field context:
        // offer a mix of relevant references
        (Some(_), _) => {
            add_fact_completions(index, &mut items);
            add_entity_completions(index, &mut items);
            add_type_completions(index, &mut items);
        }
    }

    items
}

/// Add fact names from the project index as completions.
fn add_fact_completions(index: &ProjectIndex, items: &mut Vec<CompletionItem>) {
    for ((kind, id), summary) in &index.summaries {
        if kind == "Fact" {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("fact".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tenor\n{}\n```", summary.detail),
                })),
                ..Default::default()
            });
        }
    }
}

/// Add entity names from the project index as completions.
fn add_entity_completions(index: &ProjectIndex, items: &mut Vec<CompletionItem>) {
    for ((kind, id), summary) in &index.summaries {
        if kind == "Entity" {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("entity".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tenor\n{}\n```", summary.detail),
                })),
                ..Default::default()
            });
        }
    }
}

/// Add persona names from the project index as completions.
fn add_persona_completions(index: &ProjectIndex, items: &mut Vec<CompletionItem>) {
    for ((kind, id), summary) in &index.summaries {
        if kind == "Persona" {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("persona".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tenor\n{}\n```", summary.detail),
                })),
                ..Default::default()
            });
        }
    }
}

/// Add operation names from the project index as completions.
fn add_operation_completions(index: &ProjectIndex, items: &mut Vec<CompletionItem>) {
    for ((kind, id), summary) in &index.summaries {
        if kind == "Operation" {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("operation".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tenor\n{}\n```", summary.detail),
                })),
                ..Default::default()
            });
        }
    }
}

/// Add type completions: built-in types + declared types from the project index.
fn add_type_completions(index: &ProjectIndex, items: &mut Vec<CompletionItem>) {
    // Built-in types
    for ty in BUILTIN_TYPES {
        items.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("built-in type".to_string()),
            ..Default::default()
        });
    }
    // Declared types
    for ((kind, id), summary) in &index.summaries {
        if kind == "TypeDecl" {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("declared type".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```tenor\n{}\n```", summary.detail),
                })),
                ..Default::default()
            });
        }
    }
}
