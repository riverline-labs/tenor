//! Navigation features: go-to-definition, find-all-references, document symbols.
//!
//! Builds a `ProjectIndex` from all `.tenor` files under the workspace root,
//! mapping construct declarations and references to file:line locations.

use lsp_types::{DocumentSymbol, Location, Position, Range, SymbolKind, Uri};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tenor_core::ast::{RawConstruct, RawExpr, RawStep, RawTerm};
use tenor_core::lexer;
use tenor_core::parser;

/// Cached construct locations across the workspace.
#[derive(Default)]
pub struct ProjectIndex {
    /// Maps (construct_kind, id) to the declaration location.
    pub declarations: HashMap<(String, String), Location>,
    /// Maps (construct_kind, id) to all reference locations.
    pub references: HashMap<(String, String), Vec<Location>>,
    /// Per-file document symbols.
    pub symbols: HashMap<String, Vec<DocumentSymbol>>,
    /// Maps (construct_kind, id) to a brief summary for hover.
    pub summaries: HashMap<(String, String), ConstructSummary>,
}

/// Summary information about a construct, used for hover tooltips.
#[derive(Clone, Debug)]
pub struct ConstructSummary {
    pub kind: String,
    pub id: String,
    pub detail: String,
}

impl ProjectIndex {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Build a project index from all `.tenor` files under `root_path`.
pub fn build_project_index(root_path: &Path) -> ProjectIndex {
    let mut index = ProjectIndex::new();
    let files = find_tenor_files(root_path);

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let filename = file_path.to_string_lossy().to_string();
        let tokens = match lexer::lex(&content, &filename) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let constructs = match parser::parse(&tokens, &filename) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let uri = path_to_uri(file_path);
        let mut file_symbols = Vec::new();

        for construct in &constructs {
            index_construct(construct, &uri, &content, &mut index, &mut file_symbols);
        }

        index.symbols.insert(uri.as_str().to_string(), file_symbols);
    }

    index
}

/// Go-to-definition: find the declaration location for the word at position.
pub fn goto_definition(
    index: &ProjectIndex,
    _uri: &Uri,
    position: Position,
    content: &str,
) -> Option<Location> {
    let word = word_at_position(content, position)?;
    // Try each construct kind
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
        if let Some(loc) = index.declarations.get(&key) {
            return Some(loc.clone());
        }
    }
    None
}

/// Find all references to the construct at position (including the declaration).
pub fn find_references(
    index: &ProjectIndex,
    uri: &Uri,
    position: Position,
    content: &str,
) -> Vec<Location> {
    let word = match word_at_position(content, position) {
        Some(w) => w,
        None => return Vec::new(),
    };

    // Determine which construct kind this word refers to
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
        if index.declarations.contains_key(&key) {
            let mut result = Vec::new();
            // Include the declaration itself
            if let Some(decl) = index.declarations.get(&key) {
                result.push(decl.clone());
            }
            // Include all references
            if let Some(refs) = index.references.get(&key) {
                result.extend(refs.iter().cloned());
            }
            return result;
        }
    }

    // Also check if word matches any reference even without declaration in scope
    let _ = uri; // used for context if needed in future
    Vec::new()
}

/// Return cached document symbols for the file.
pub fn document_symbols(index: &ProjectIndex, uri: &Uri) -> Vec<DocumentSymbol> {
    index.symbols.get(uri.as_str()).cloned().unwrap_or_default()
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Recursively find all `.tenor` files under a directory.
fn find_tenor_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if root.is_file() && root.extension().is_some_and(|e| e == "tenor") {
        files.push(root.to_path_buf());
        return files;
    }
    if root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(find_tenor_files(&path));
                } else if path.extension().is_some_and(|e| e == "tenor") {
                    files.push(path);
                }
            }
        }
    }
    files
}

/// Convert a file path to a `file://` URI.
fn path_to_uri(path: &Path) -> Uri {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = format!("file://{}", abs.display());
    s.parse().unwrap_or_else(|_| {
        // Fallback
        format!("file://{}", path.display())
            .parse()
            .expect("fallback URI must parse")
    })
}

/// Extract the word (identifier) at the given LSP position.
fn word_at_position(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = position.line as usize;
    if line_idx >= lines.len() {
        return None;
    }
    let line = lines[line_idx];
    let chars: Vec<char> = line.chars().collect();
    let col = position.character as usize;
    if col >= chars.len() {
        return None;
    }

    // Find word boundaries around the cursor position
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_';

    if !is_ident_char(chars[col]) {
        return None;
    }

    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    let word: String = chars[start..end].iter().collect();
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}

/// Create an LSP Range for a construct declaration at the given line.
/// `line` is 1-indexed from the parser; LSP positions are 0-indexed.
fn make_range(content: &str, line_1indexed: u32) -> Range {
    let line = line_1indexed.saturating_sub(1);
    let lines: Vec<&str> = content.lines().collect();
    let end_char = if (line as usize) < lines.len() {
        lines[line as usize].len() as u32
    } else {
        0
    };
    Range::new(Position::new(line, 0), Position::new(line, end_char))
}

/// Create a Location from a URI and 1-indexed line number.
fn make_location(uri: &Uri, content: &str, line_1indexed: u32) -> Location {
    Location::new(uri.clone(), make_range(content, line_1indexed))
}

/// Index a single parsed construct: record declaration, references, symbols, summary.
#[allow(deprecated)]
fn index_construct(
    construct: &RawConstruct,
    uri: &Uri,
    content: &str,
    index: &mut ProjectIndex,
    symbols: &mut Vec<DocumentSymbol>,
) {
    match construct {
        RawConstruct::Fact {
            id,
            prov,
            type_,
            default,
            ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Fact".to_string(), id.clone()), loc);

            let type_str = format_raw_type(type_);
            let default_str = if default.is_some() {
                " (has default)"
            } else {
                ""
            };
            index.summaries.insert(
                ("Fact".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Fact".to_string(),
                    id: id.clone(),
                    detail: format!("fact {} : {}{}", id, type_str, default_str),
                },
            );

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("fact : {}", type_str)),
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Entity {
            id, prov, states, ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Entity".to_string(), id.clone()), loc);

            index.summaries.insert(
                ("Entity".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Entity".to_string(),
                    id: id.clone(),
                    detail: format!("entity {}\n  states: [{}]", id, states.join(", ")),
                },
            );

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("entity ({})", states.join(", "))),
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Rule {
            id,
            prov,
            verdict_type,
            when,
            ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Rule".to_string(), id.clone()), loc);

            index.summaries.insert(
                ("Rule".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Rule".to_string(),
                    id: id.clone(),
                    detail: format!("rule {}\n  verdict: {}", id, verdict_type),
                },
            );

            // Index fact references in the when expression
            index_expr_refs(when, uri, content, index);

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("rule -> {}", verdict_type)),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Operation {
            id,
            prov,
            allowed_personas,
            precondition,
            effects,
            outcomes,
            ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Operation".to_string(), id.clone()), loc);

            let personas_str = allowed_personas.join(", ");
            let effects_summary: Vec<String> = effects
                .iter()
                .map(|(entity, from, to, _, _)| format!("{}: {} -> {}", entity, from, to))
                .collect();
            let outcomes_str = if outcomes.is_empty() {
                String::new()
            } else {
                format!("\n  outcomes: [{}]", outcomes.join(", "))
            };
            index.summaries.insert(
                ("Operation".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Operation".to_string(),
                    id: id.clone(),
                    detail: format!(
                        "operation {}\n  personas: [{}]\n  effects: [{}]{}",
                        id,
                        personas_str,
                        effects_summary.join("; "),
                        outcomes_str,
                    ),
                },
            );

            // Index persona references
            for persona in allowed_personas {
                add_reference(index, "Persona", persona, uri, content, prov.line);
            }

            // Index entity references in effects
            for (entity, _, _, _, effect_line) in effects {
                add_reference(index, "Entity", entity, uri, content, *effect_line);
            }

            // Index fact references in precondition
            index_expr_refs(precondition, uri, content, index);

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("operation [{}]", personas_str)),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Flow {
            id,
            prov,
            entry,
            steps,
            ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Flow".to_string(), id.clone()), loc);

            index.summaries.insert(
                ("Flow".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Flow".to_string(),
                    id: id.clone(),
                    detail: format!("flow {}\n  entry: {}\n  steps: {}", id, entry, steps.len(),),
                },
            );

            // Index operation/flow/persona references in steps
            for step in steps.values() {
                index_step_refs(step, uri, content, index);
            }

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("flow ({} steps)", steps.len())),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Persona { id, prov } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("Persona".to_string(), id.clone()), loc);

            index.summaries.insert(
                ("Persona".to_string(), id.clone()),
                ConstructSummary {
                    kind: "Persona".to_string(),
                    id: id.clone(),
                    detail: format!("persona {}", id),
                },
            );

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some("persona".to_string()),
                kind: SymbolKind::NAMESPACE,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::TypeDecl { id, prov, fields } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("TypeDecl".to_string(), id.clone()), loc);

            let fields_str: Vec<String> = fields
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, format_raw_type(ty)))
                .collect();
            index.summaries.insert(
                ("TypeDecl".to_string(), id.clone()),
                ConstructSummary {
                    kind: "TypeDecl".to_string(),
                    id: id.clone(),
                    detail: format!("type {}\n  {}", id, fields_str.join("\n  ")),
                },
            );

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("type ({} fields)", fields.len())),
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::System {
            id, prov, members, ..
        } => {
            let loc = make_location(uri, content, prov.line);
            index
                .declarations
                .insert(("System".to_string(), id.clone()), loc);

            index.summaries.insert(
                ("System".to_string(), id.clone()),
                ConstructSummary {
                    kind: "System".to_string(),
                    id: id.clone(),
                    detail: format!("system {}\n  members: {}", id, members.len(),),
                },
            );

            symbols.push(DocumentSymbol {
                name: id.clone(),
                detail: Some(format!("system ({} members)", members.len())),
                kind: SymbolKind::NAMESPACE,
                tags: None,
                deprecated: None,
                range: make_range(content, prov.line),
                selection_range: make_range(content, prov.line),
                children: None,
            });
        }
        RawConstruct::Import { .. } => {}
    }
}

/// Add a reference entry to the index.
fn add_reference(
    index: &mut ProjectIndex,
    kind: &str,
    id: &str,
    uri: &Uri,
    content: &str,
    line_1indexed: u32,
) {
    let key = (kind.to_string(), id.to_string());
    let loc = make_location(uri, content, line_1indexed);
    index.references.entry(key).or_default().push(loc);
}

/// Index fact references from expressions.
fn index_expr_refs(expr: &RawExpr, uri: &Uri, content: &str, index: &mut ProjectIndex) {
    match expr {
        RawExpr::Compare {
            left, right, line, ..
        } => {
            index_term_refs(left, uri, content, index, *line);
            index_term_refs(right, uri, content, index, *line);
        }
        RawExpr::VerdictPresent { .. } => {}
        RawExpr::And(a, b) | RawExpr::Or(a, b) => {
            index_expr_refs(a, uri, content, index);
            index_expr_refs(b, uri, content, index);
        }
        RawExpr::Not(e) => {
            index_expr_refs(e, uri, content, index);
        }
        RawExpr::Forall {
            domain, body, line, ..
        } => {
            add_reference(index, "Fact", domain, uri, content, *line);
            index_expr_refs(body, uri, content, index);
        }
        RawExpr::Exists {
            domain, body, line, ..
        } => {
            add_reference(index, "Fact", domain, uri, content, *line);
            index_expr_refs(body, uri, content, index);
        }
    }
}

/// Index fact references from terms.
fn index_term_refs(term: &RawTerm, uri: &Uri, content: &str, index: &mut ProjectIndex, line: u32) {
    match term {
        RawTerm::FactRef(name) => {
            add_reference(index, "Fact", name, uri, content, line);
        }
        RawTerm::FieldRef { .. } => {}
        RawTerm::Literal(_) => {}
        RawTerm::Mul { left, right } => {
            index_term_refs(left, uri, content, index, line);
            index_term_refs(right, uri, content, index, line);
        }
    }
}

/// Index operation, flow, and persona references from flow steps.
fn index_step_refs(step: &RawStep, uri: &Uri, content: &str, index: &mut ProjectIndex) {
    match step {
        RawStep::OperationStep {
            op, persona, line, ..
        } => {
            add_reference(index, "Operation", op, uri, content, *line);
            add_reference(index, "Persona", persona, uri, content, *line);
        }
        RawStep::BranchStep {
            persona,
            condition,
            line,
            ..
        } => {
            add_reference(index, "Persona", persona, uri, content, *line);
            index_expr_refs(condition, uri, content, index);
        }
        RawStep::HandoffStep {
            from_persona,
            to_persona,
            line,
            ..
        } => {
            add_reference(index, "Persona", from_persona, uri, content, *line);
            add_reference(index, "Persona", to_persona, uri, content, *line);
        }
        RawStep::SubFlowStep {
            flow,
            persona,
            flow_line,
            ..
        } => {
            add_reference(index, "Flow", flow, uri, content, *flow_line);
            add_reference(index, "Persona", persona, uri, content, *flow_line);
        }
        RawStep::ParallelStep { branches, .. } => {
            for branch in branches {
                for s in branch.steps.values() {
                    index_step_refs(s, uri, content, index);
                }
            }
        }
    }
}

/// Format a RawType for display in summaries.
fn format_raw_type(ty: &tenor_core::ast::RawType) -> String {
    use tenor_core::ast::RawType;
    match ty {
        RawType::Bool => "Bool".to_string(),
        RawType::Int { min, max } => {
            if *min == i64::MIN && *max == i64::MAX {
                "Int".to_string()
            } else {
                format!("Int({}, {})", min, max)
            }
        }
        RawType::Decimal { precision, scale } => format!("Decimal({}, {})", precision, scale),
        RawType::Text { max_length } => {
            if *max_length == 0 {
                "Text".to_string()
            } else {
                format!("Text({})", max_length)
            }
        }
        RawType::Date => "Date".to_string(),
        RawType::DateTime => "DateTime".to_string(),
        RawType::Money { currency } => format!("Money(\"{}\")", currency),
        RawType::Duration { unit, .. } => format!("Duration(\"{}\")", unit),
        RawType::Enum { values } => {
            let vals: Vec<String> = values.iter().map(|v| format!("\"{}\"", v)).collect();
            format!("Enum([{}])", vals.join(", "))
        }
        RawType::Record { fields } => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_raw_type(v)))
                .collect();
            format!("Record({{ {} }})", fs.join(", "))
        }
        RawType::List { element_type, max } => {
            format!("List({}, max: {})", format_raw_type(element_type), max)
        }
        RawType::TypeRef(name) => name.clone(),
    }
}

/// Index type references from RawType.
fn _index_type_refs(
    ty: &tenor_core::ast::RawType,
    uri: &Uri,
    content: &str,
    index: &mut ProjectIndex,
    line: u32,
) {
    use tenor_core::ast::RawType;
    match ty {
        RawType::TypeRef(name) => {
            add_reference(index, "TypeDecl", name, uri, content, line);
        }
        RawType::Record { fields } => {
            for field_ty in fields.values() {
                _index_type_refs(field_ty, uri, content, index, line);
            }
        }
        RawType::List { element_type, .. } => {
            _index_type_refs(element_type, uri, content, index, line);
        }
        _ => {}
    }
}

/// Get the word at the given position (for use by hover and completion modules).
#[allow(dead_code)]
pub(crate) fn get_word_at_position(content: &str, position: Position) -> Option<String> {
    word_at_position(content, position)
}

/// Get the construct context at the given position (what kind of construct body are we in?).
/// Returns the construct keyword (e.g., "rule", "operation", "entity") or None if at top level.
#[allow(dead_code)]
pub(crate) fn get_construct_context(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let target_line = position.line as usize;

    // Scan backwards from the current line to find the nearest construct keyword
    let construct_keywords = [
        "fact",
        "entity",
        "rule",
        "operation",
        "flow",
        "persona",
        "type",
        "system",
    ];

    let mut brace_depth: i32 = 0;
    for line_idx in (0..=target_line).rev() {
        if line_idx >= lines.len() {
            continue;
        }
        let line = lines[line_idx];
        // Count braces to track nesting
        for ch in line.chars().rev() {
            match ch {
                '}' => brace_depth += 1,
                '{' => brace_depth -= 1,
                _ => {}
            }
        }
        // If we're at or inside a construct (brace_depth <= 0), check for keyword
        if brace_depth <= 0 {
            let trimmed = line.trim();
            for kw in &construct_keywords {
                if trimmed.starts_with(kw) {
                    return Some(kw.to_string());
                }
            }
        }
    }
    None
}

/// Get the field context within a construct body at the given position.
/// Returns the field keyword (e.g., "when", "precondition", "effects") or None.
#[allow(dead_code)]
pub(crate) fn get_field_context(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let target_line = position.line as usize;

    let field_keywords = [
        "when",
        "precondition",
        "effects",
        "allowed_personas",
        "states",
        "transitions",
        "steps",
        "produce",
        "outcomes",
        "entry",
    ];

    // Scan backwards from current line to find the nearest field keyword
    let mut brace_depth: i32 = 0;
    for line_idx in (0..=target_line).rev() {
        if line_idx >= lines.len() {
            continue;
        }
        let line = lines[line_idx];
        for ch in line.chars().rev() {
            match ch {
                '}' | ']' => brace_depth += 1,
                '{' | '[' => brace_depth -= 1,
                _ => {}
            }
        }
        if brace_depth <= 0 {
            let trimmed = line.trim();
            for kw in &field_keywords {
                if trimmed.starts_with(kw) && trimmed.contains(':') {
                    return Some(kw.to_string());
                }
            }
        }
    }
    None
}
