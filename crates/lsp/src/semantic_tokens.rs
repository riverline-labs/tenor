//! Semantic token provider for construct-aware highlighting.
//!
//! Uses the lexer token stream to identify positions and the parsed
//! constructs to assign semantic meaning. Best-effort: swallows parse
//! errors so semantic tokens degrade gracefully on incomplete files.

use lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType};
use std::collections::HashSet;
use std::path::Path;
use tenor_core::lexer::{self, Spanned, Token};

/// Index into TOKEN_TYPES for each semantic category.
const TK_KEYWORD: u32 = 0;
const TK_TYPE: u32 = 1;
const TK_VARIABLE: u32 = 2;
const TK_PROPERTY: u32 = 3;
// TK_ENUM_MEMBER (4) reserved in TOKEN_TYPES legend but not yet emitted
const TK_FUNCTION: u32 = 5;
const TK_CLASS: u32 = 6;
const TK_NAMESPACE: u32 = 7;
const TK_STRING: u32 = 8;
const TK_NUMBER: u32 = 9;
// TK_COMMENT (10) reserved in TOKEN_TYPES legend but not yet emitted
const TK_OPERATOR: u32 = 11;

/// Semantic token types registered with the client.
pub static TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,     // 0
    SemanticTokenType::TYPE,        // 1
    SemanticTokenType::VARIABLE,    // 2
    SemanticTokenType::PROPERTY,    // 3
    SemanticTokenType::ENUM_MEMBER, // 4
    SemanticTokenType::FUNCTION,    // 5
    SemanticTokenType::CLASS,       // 6
    SemanticTokenType::NAMESPACE,   // 7
    SemanticTokenType::STRING,      // 8
    SemanticTokenType::NUMBER,      // 9
    SemanticTokenType::COMMENT,     // 10
    SemanticTokenType::OPERATOR,    // 11
];

/// Semantic token modifiers.
pub static TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION, // bit 0
    SemanticTokenModifier::DEFINITION,  // bit 1
    SemanticTokenModifier::READONLY,    // bit 2
];

const MOD_DECLARATION: u32 = 1 << 0;

/// A raw token with absolute position before delta-encoding.
struct RawSemanticToken {
    line: u32,
    col: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

/// Compute semantic tokens for the given file content.
///
/// Uses the lexer for token positions and construct-level knowledge
/// from parsing to assign semantic types. Best-effort: returns None
/// only if lexing fails entirely.
pub fn compute_semantic_tokens(file_path: &Path, content: &str) -> Option<Vec<SemanticToken>> {
    let filename = file_path.to_string_lossy();

    // Lex the content -- if this fails, we can't provide tokens
    let spanned = lexer::lex(content, &filename).ok()?;

    // Build line offset table for column calculation
    let line_offsets = build_line_offsets(content);

    // Collect known names from constructs for better tagging
    let names = collect_construct_names(file_path);

    // Walk the token stream and assign semantic types
    let mut raw_tokens = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut char_pos: usize = 0;

    for (i, sp) in spanned.iter().enumerate() {
        // Advance char_pos to match the token's line
        // We need the column, so we track character positions
        let (col, token_len) = find_token_position(&chars, &mut char_pos, sp, &line_offsets);

        if token_len == 0 {
            continue;
        }

        let (token_type, modifiers) = classify_token(sp, i, &spanned, &names);

        if let Some(tt) = token_type {
            raw_tokens.push(RawSemanticToken {
                line: sp.line.saturating_sub(1), // LSP is 0-indexed
                col,
                length: token_len as u32,
                token_type: tt,
                modifiers,
            });
        }
    }

    // Sort by position and delta-encode
    raw_tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));
    Some(delta_encode(&raw_tokens))
}

/// Build a table of byte offsets for each line start.
fn build_line_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, ch) in content.chars().enumerate() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Find the column and length of a token in the source.
fn find_token_position(
    chars: &[char],
    char_pos: &mut usize,
    sp: &Spanned,
    line_offsets: &[usize],
) -> (u32, usize) {
    // Use line offset to determine column
    let line_idx = (sp.line as usize).saturating_sub(1);
    let line_start = if line_idx < line_offsets.len() {
        line_offsets[line_idx]
    } else {
        0
    };

    let token_text = token_text(&sp.token);
    let token_len = token_text.len();

    if token_len == 0 {
        return (0, 0);
    }

    // Search from line_start for the token text, starting at or after char_pos
    let search_start = (*char_pos).max(line_start);
    if let Some(offset) = find_substr(chars, search_start, &token_text) {
        *char_pos = offset + token_len;
        let col = (offset - line_start) as u32;
        (col, token_len)
    } else if let Some(offset) = find_substr(chars, line_start, &token_text) {
        // Fallback: search from line start
        *char_pos = offset + token_len;
        let col = (offset - line_start) as u32;
        (col, token_len)
    } else {
        (0, token_len)
    }
}

/// Find a substring in a char slice starting at `from`.
fn find_substr(chars: &[char], from: usize, needle: &[char]) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > chars.len() {
        return None;
    }
    for i in from..=(chars.len() - needle.len()) {
        if chars[i..i + needle.len()] == *needle {
            return Some(i);
        }
    }
    None
}

/// Get the text representation of a token for position finding.
fn token_text(tok: &Token) -> Vec<char> {
    match tok {
        Token::Word(w) => w.chars().collect(),
        Token::Str(s) => {
            // Include quotes for position calculation
            let mut v = vec!['"'];
            v.extend(s.chars());
            v.push('"');
            v
        }
        Token::Int(n) => n.to_string().chars().collect(),
        Token::Float(f) => f.chars().collect(),
        Token::LBrace => vec!['{'],
        Token::RBrace => vec!['}'],
        Token::LBracket => vec!['['],
        Token::RBracket => vec![']'],
        Token::LParen => vec!['('],
        Token::RParen => vec![')'],
        Token::Colon => vec![':'],
        Token::Comma => vec![','],
        Token::Dot => vec!['.'],
        Token::Eq => vec!['='],
        Token::Neq => vec!['!', '='],
        Token::Lt => vec!['<'],
        Token::Lte => vec!['<', '='],
        Token::Gt => vec!['>'],
        Token::Gte => vec!['>', '='],
        Token::Star => vec!['*'],
        Token::And => vec!['\u{2227}'],
        Token::Or => vec!['\u{2228}'],
        Token::Not => vec!['\u{00AC}'],
        Token::Forall => vec!['\u{2200}'],
        Token::Exists => vec!['\u{2203}'],
        Token::In => vec!['\u{2208}'],
        Token::Eof => vec![],
    }
}

/// Known construct names gathered from parsing.
struct ConstructNames {
    entities: HashSet<String>,
    operations: HashSet<String>,
    personas: HashSet<String>,
    facts: HashSet<String>,
    type_names: HashSet<String>,
}

/// Try to parse the file and collect construct names for semantic classification.
fn collect_construct_names(file_path: &Path) -> ConstructNames {
    let mut names = ConstructNames {
        entities: HashSet::new(),
        operations: HashSet::new(),
        personas: HashSet::new(),
        facts: HashSet::new(),
        type_names: HashSet::new(),
    };

    // Best-effort: swallow errors
    if let Ok((constructs, _)) = tenor_core::pass1_bundle::load_bundle(file_path) {
        for c in &constructs {
            match c {
                tenor_core::RawConstruct::Entity { id, .. } => {
                    names.entities.insert(id.clone());
                }
                tenor_core::RawConstruct::Operation { id, .. } => {
                    names.operations.insert(id.clone());
                }
                tenor_core::RawConstruct::Persona { id, .. } => {
                    names.personas.insert(id.clone());
                }
                tenor_core::RawConstruct::Fact { id, .. } => {
                    names.facts.insert(id.clone());
                }
                tenor_core::RawConstruct::TypeDecl { id, .. } => {
                    names.type_names.insert(id.clone());
                }
                _ => {}
            }
        }
    }

    names
}

/// Classify a token into a semantic type and modifiers.
fn classify_token(
    sp: &Spanned,
    idx: usize,
    tokens: &[Spanned],
    names: &ConstructNames,
) -> (Option<u32>, u32) {
    match &sp.token {
        Token::Str(_) => (Some(TK_STRING), 0),
        Token::Int(_) | Token::Float(_) => (Some(TK_NUMBER), 0),
        Token::Eq | Token::Neq | Token::Lt | Token::Lte | Token::Gt | Token::Gte | Token::Star => {
            (Some(TK_OPERATOR), 0)
        }
        Token::And | Token::Or | Token::Not | Token::Forall | Token::Exists | Token::In => {
            (Some(TK_KEYWORD), 0)
        }
        Token::Word(w) => classify_word(w, idx, tokens, names),
        _ => (None, 0),
    }
}

/// DSL construct keywords.
static CONSTRUCT_KEYWORDS: &[&str] = &[
    "fact",
    "entity",
    "rule",
    "operation",
    "flow",
    "type",
    "persona",
    "system",
    "import",
];

/// Field-level keywords within construct bodies.
static FIELD_KEYWORDS: &[&str] = &[
    "states",
    "initial",
    "transitions",
    "parent",
    "source",
    "default",
    "stratum",
    "when",
    "produce",
    "type",
    "value",
    "allowed_personas",
    "precondition",
    "effects",
    "error_contract",
    "outcomes",
    "snapshot",
    "entry",
    "steps",
    "condition",
    "if_true",
    "if_false",
    "on_success",
    "on_failure",
    "next",
    "from_persona",
    "to_persona",
    "branches",
    "join",
    "on_all_success",
    "on_any_failure",
    "on_all_complete",
    "members",
    "shared_personas",
    "triggers",
    "shared_entities",
    "verdict_present",
    "verdict_type",
];

/// Built-in type names.
static BUILTIN_TYPES: &[&str] = &[
    "Bool", "Int", "Decimal", "Text", "Date", "DateTime", "Money", "Duration", "Enum", "Record",
    "List",
];

/// Flow step type keywords.
static STEP_TYPES: &[&str] = &[
    "OperationStep",
    "BranchStep",
    "HandoffStep",
    "SubFlowStep",
    "ParallelStep",
    "Terminal",
    "Terminate",
    "Compensate",
    "Escalate",
];

/// Classify a Word token based on context.
fn classify_word(
    word: &str,
    idx: usize,
    tokens: &[Spanned],
    names: &ConstructNames,
) -> (Option<u32>, u32) {
    // Construct declaration keywords
    if CONSTRUCT_KEYWORDS.contains(&word) {
        // Check if this is a declaration (followed by an identifier)
        if idx + 1 < tokens.len() {
            if let Token::Word(_) = &tokens[idx + 1].token {
                return (Some(TK_KEYWORD), MOD_DECLARATION);
            }
        }
        return (Some(TK_KEYWORD), 0);
    }

    // Boolean literals
    if word == "true" || word == "false" {
        return (Some(TK_KEYWORD), 0);
    }

    // Built-in type names
    if BUILTIN_TYPES.contains(&word) {
        return (Some(TK_TYPE), 0);
    }

    // Flow step types
    if STEP_TYPES.contains(&word) {
        return (Some(TK_TYPE), 0);
    }

    // Logical keywords (ASCII forms)
    if word == "and" || word == "or" || word == "not" || word == "forall" || word == "exists" {
        return (Some(TK_KEYWORD), 0);
    }

    // Check if followed by colon -> property/field label
    if idx + 1 < tokens.len() && tokens[idx + 1].token == Token::Colon {
        if FIELD_KEYWORDS.contains(&word) {
            return (Some(TK_PROPERTY), 0);
        }
        // Step IDs and other identifiers followed by colon are properties
        return (Some(TK_PROPERTY), 0);
    }

    // Check if this is a construct name right after a keyword
    if idx > 0 {
        if let Token::Word(prev) = &tokens[idx - 1].token {
            if CONSTRUCT_KEYWORDS.contains(&prev.as_str()) {
                // This is the name of a construct being declared
                return match prev.as_str() {
                    "entity" => (Some(TK_CLASS), MOD_DECLARATION),
                    "operation" => (Some(TK_FUNCTION), MOD_DECLARATION),
                    "persona" => (Some(TK_NAMESPACE), MOD_DECLARATION),
                    "flow" => (Some(TK_FUNCTION), MOD_DECLARATION),
                    "rule" => (Some(TK_VARIABLE), MOD_DECLARATION),
                    "fact" => (Some(TK_VARIABLE), MOD_DECLARATION),
                    "type" => (Some(TK_TYPE), MOD_DECLARATION),
                    "system" => (Some(TK_CLASS), MOD_DECLARATION),
                    _ => (Some(TK_VARIABLE), MOD_DECLARATION),
                };
            }
        }
    }

    // Known construct references
    if names.entities.contains(word) {
        return (Some(TK_CLASS), 0);
    }
    if names.operations.contains(word) {
        return (Some(TK_FUNCTION), 0);
    }
    if names.personas.contains(word) {
        return (Some(TK_NAMESPACE), 0);
    }
    if names.facts.contains(word) {
        return (Some(TK_VARIABLE), 0);
    }
    if names.type_names.contains(word) {
        return (Some(TK_TYPE), 0);
    }

    // Unknown word -- no semantic token
    (None, 0)
}

/// Delta-encode raw tokens into LSP SemanticToken format.
fn delta_encode(raw: &[RawSemanticToken]) -> Vec<SemanticToken> {
    let mut result = Vec::with_capacity(raw.len());
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;

    for tok in raw {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 {
            tok.col - prev_col
        } else {
            tok.col
        };

        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.modifiers,
        });

        prev_line = tok.line;
        prev_col = tok.col;
    }

    result
}
