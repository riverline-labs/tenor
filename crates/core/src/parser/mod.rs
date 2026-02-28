/// Raw AST produced by the parser.
/// All constructs carry provenance (file, line of the opening keyword).
/// No type checking or resolution is done here -- that is elaboration's job.
use crate::error::ElabError;
use crate::lexer::{Spanned, Token};

mod constructs;
mod expressions;
mod flow;
mod system;
mod types;

// Re-export AST types so existing callers that use `parser::Foo` still work.
pub use crate::ast::{
    Provenance, RawBranch, RawCompStep, RawConstruct, RawExpr, RawFailureHandler, RawJoinPolicy,
    RawLiteral, RawSourceDecl, RawStep, RawStepTarget, RawTerm, RawTrigger, RawType,
};

// ──────────────────────────────────────────────
// Parser
// ──────────────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [Spanned],
    pos: usize,
    filename: String,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Spanned], filename: &str) -> Self {
        Parser {
            tokens,
            pos: 0,
            filename: filename.to_owned(),
        }
    }

    fn cur(&self) -> &Spanned {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek(&self) -> &Token {
        &self.cur().token
    }

    fn cur_line(&self) -> u32 {
        self.cur().line
    }

    fn advance(&mut self) -> &Spanned {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect_word(&mut self, expected: &str) -> Result<u32, ElabError> {
        let s = self.cur();
        let line = s.line;
        if let Token::Word(w) = &s.token {
            if w == expected {
                self.advance();
                return Ok(line);
            }
        }
        Err(self.err(format!("expected '{}', got {:?}", expected, self.peek())))
    }

    fn expect_colon(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::Colon {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected ':', got {:?}", self.peek())))
        }
    }

    fn expect_lbrace(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::LBrace {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '{{', got {:?}", self.peek())))
        }
    }

    fn expect_rbrace(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::RBrace {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '}}', got {:?}", self.peek())))
        }
    }

    fn err(&self, msg: impl Into<String>) -> ElabError {
        ElabError::parse(&self.filename, self.cur_line(), msg)
    }

    fn is_word(&self, w: &str) -> bool {
        matches!(self.peek(), Token::Word(x) if x == w)
    }

    fn take_word(&mut self) -> Result<String, ElabError> {
        if let Token::Word(w) = self.peek().clone() {
            self.advance();
            Ok(w)
        } else {
            Err(self.err(format!("expected identifier, got {:?}", self.peek())))
        }
    }

    fn take_str(&mut self) -> Result<String, ElabError> {
        if let Token::Str(s) = self.peek().clone() {
            self.advance();
            Ok(s)
        } else {
            Err(self.err(format!("expected string literal, got {:?}", self.peek())))
        }
    }

    fn take_int(&mut self) -> Result<i64, ElabError> {
        match self.peek().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(n)
            }
            Token::Word(w) if w.parse::<i64>().is_ok() => {
                let n = w
                    .parse::<i64>()
                    .map_err(|_| self.err(format!("invalid integer literal: {}", w)))?;
                self.advance();
                Ok(n)
            }
            _ => Err(self.err(format!("expected integer, got {:?}", self.peek()))),
        }
    }

    // -- Top-level construct parsers ----------------------------

    pub fn parse_file(&mut self) -> Result<Vec<RawConstruct>, ElabError> {
        let mut constructs = Vec::new();
        while self.peek() != &Token::Eof {
            let c = self.parse_construct()?;
            constructs.push(c);
        }

        // C-SYS-05: at most one system declaration per file
        let systems: Vec<_> = constructs
            .iter()
            .filter_map(|c| match c {
                RawConstruct::System { id, prov, .. } => Some((id, prov)),
                _ => None,
            })
            .collect();
        if systems.len() > 1 {
            let (_, prov) = &systems[1];
            return Err(ElabError::parse(
                &prov.file,
                prov.line,
                "multiple System declarations in a single file",
            ));
        }

        // C-SYS-04: a file with a system declaration may not contain contract constructs
        if let Some((_, sys_prov)) = systems.first() {
            for c in &constructs {
                match c {
                    RawConstruct::System { .. } | RawConstruct::Import { .. } => {}
                    other => {
                        let prov = match other {
                            RawConstruct::Fact { prov, .. }
                            | RawConstruct::Entity { prov, .. }
                            | RawConstruct::Rule { prov, .. }
                            | RawConstruct::Operation { prov, .. }
                            | RawConstruct::Flow { prov, .. }
                            | RawConstruct::Persona { prov, .. }
                            | RawConstruct::Source { prov, .. }
                            | RawConstruct::TypeDecl { prov, .. } => prov,
                            _ => sys_prov,
                        };
                        return Err(ElabError::parse(
                            &prov.file,
                            prov.line,
                            "System files may not contain contract constructs",
                        ));
                    }
                }
            }
        }

        Ok(constructs)
    }

    fn parse_construct(&mut self) -> Result<RawConstruct, ElabError> {
        let line = self.cur_line();
        match self.peek().clone() {
            Token::Word(w) => match w.as_str() {
                "import" => self.parse_import(line),
                "type" => self.parse_typedecl(line),
                "fact" => self.parse_fact(line),
                "entity" => self.parse_entity(line),
                "rule" => self.parse_rule(line),
                "operation" => self.parse_operation(line),
                "flow" => self.parse_flow(line),
                "persona" => self.parse_persona(line),
                "system" => self.parse_system(line),
                "source" => self.parse_source(line),
                _ => Err(self.err(format!("unexpected token '{}'", w))),
            },
            other => Err(self.err(format!("expected construct keyword, got {:?}", other))),
        }
    }

    fn parse_import(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let path = self.take_str()?;
        Ok(RawConstruct::Import {
            path,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }
}

/// Default maximum number of errors collected in multi-error mode before aborting.
pub const DEFAULT_MAX_ERRORS: usize = 10;

pub fn parse(tokens: &[Spanned], filename: &str) -> Result<Vec<RawConstruct>, ElabError> {
    let mut p = Parser::new(tokens, filename);
    p.parse_file()
}

/// Parse in multi-error recovery mode.
///
/// Returns successfully-parsed constructs plus accumulated errors.
/// If a truly fatal error occurs (e.g., a lexer-level issue that prevents
/// any recovery), returns `Err` with that single error.
///
/// The parser recovers at construct boundaries: when an error occurs inside
/// a construct body, it skips tokens until it reaches a closing `}` at the
/// matching nesting level or a top-level keyword, then resumes parsing.
pub fn parse_recovering(
    tokens: &[Spanned],
    filename: &str,
    max_errors: usize,
) -> Result<(Vec<RawConstruct>, Vec<ElabError>), ElabError> {
    let mut p = Parser::new(tokens, filename);
    p.parse_file_recovering(max_errors)
}

impl<'a> Parser<'a> {
    /// Check whether the current token is a top-level construct keyword.
    fn is_construct_keyword(&self) -> bool {
        matches!(
            self.peek(),
            Token::Word(w) if matches!(
                w.as_str(),
                "fact" | "entity" | "rule" | "operation" | "flow"
                    | "type" | "persona" | "system" | "import" | "source"
            )
        )
    }

    /// Skip tokens until we find a closing `}` at the original nesting level,
    /// or a top-level construct keyword at nesting level 0.
    fn recover_to_next_construct(&mut self) {
        let mut depth: i32 = 0;
        loop {
            match self.peek() {
                Token::Eof => break,
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    if depth <= 0 {
                        // Consume the closing brace that ends the broken construct
                        self.advance();
                        break;
                    }
                    depth -= 1;
                    self.advance();
                }
                _ => {
                    if depth == 0 && self.is_construct_keyword() {
                        // Found a new top-level keyword; stop here (don't consume it).
                        break;
                    }
                    self.advance();
                }
            }
        }
    }

    /// Parse the file with error recovery at construct boundaries.
    fn parse_file_recovering(
        &mut self,
        max_errors: usize,
    ) -> Result<(Vec<RawConstruct>, Vec<ElabError>), ElabError> {
        let mut constructs = Vec::new();
        let mut errors = Vec::new();

        while self.peek() != &Token::Eof {
            match self.parse_construct() {
                Ok(c) => {
                    constructs.push(c);
                }
                Err(e) => {
                    errors.push(e);
                    if errors.len() >= max_errors {
                        break;
                    }
                    // Recover: skip to next construct boundary
                    self.recover_to_next_construct();
                }
            }
        }

        Ok((constructs, errors))
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    /// Helper: lex + parse_recovering for a source string.
    fn lex_and_parse_recovering(
        src: &str,
        filename: &str,
    ) -> Result<(Vec<RawConstruct>, Vec<ElabError>), ElabError> {
        let tokens = lexer::lex(src, filename)?;
        parse_recovering(&tokens, filename, DEFAULT_MAX_ERRORS)
    }

    #[test]
    fn multi_error_reports_both_construct_errors() {
        // Two facts that lex fine but each is missing a required field.
        // The parser should recover after the first broken fact
        // and report an error for the second broken fact too.
        let src = r#"
fact bad_fact_1 {
    source: "s1"
}

fact bad_fact_2 {
    type: Int
}
"#;
        let result = lex_and_parse_recovering(src, "test.tenor");

        match result {
            Ok((constructs, errors)) => {
                // Both constructs should produce errors (missing 'type' and missing 'source')
                assert!(
                    errors.len() >= 2,
                    "Expected at least 2 errors, got {}: {:?}",
                    errors.len(),
                    errors
                );
                assert!(
                    errors[0].message.contains("missing"),
                    "First error: {}",
                    errors[0].message
                );
                assert!(
                    errors[1].message.contains("missing"),
                    "Second error: {}",
                    errors[1].message
                );
                // No valid constructs should have been produced
                assert_eq!(constructs.len(), 0, "No valid constructs expected");
            }
            Err(e) => {
                panic!("Expected Ok with errors, got fatal Err: {:?}", e);
            }
        }
    }

    #[test]
    fn fatal_lexer_error_aborts_immediately() {
        // An unterminated string is a fatal lexer error -- parse_recovering
        // never gets called because lex() itself returns Err.
        let src = "fact foo {\n    source: \"unterminated\n}\n";
        let tokens = lexer::lex(src, "fatal.tenor");
        assert!(tokens.is_err(), "Lexer should reject unterminated string");
    }

    #[test]
    fn one_valid_one_invalid_parses_the_valid_one() {
        let src = r#"
fact valid_fact {
    type: Bool
    source: "input"
}

fact invalid_fact {
    type: Bool
}
"#;
        let (constructs, errors) =
            lex_and_parse_recovering(src, "mixed.tenor").expect("should not be fatal");
        assert_eq!(constructs.len(), 1, "One valid construct expected");
        assert!(!errors.is_empty(), "At least one error expected");
        // The valid fact should be the one that parsed correctly
        match &constructs[0] {
            RawConstruct::Fact { id, .. } => {
                assert_eq!(id, "valid_fact");
            }
            other => panic!("Expected Fact, got {:?}", other),
        }
    }

    #[test]
    fn no_errors_returns_empty_error_vec() {
        let src = r#"
fact good_fact {
    type: Bool
    source: "input"
}
"#;
        let (constructs, errors) =
            lex_and_parse_recovering(src, "good.tenor").expect("should not be fatal");
        assert_eq!(constructs.len(), 1);
        assert!(errors.is_empty(), "No errors expected for valid input");
    }

    #[test]
    fn max_errors_limit_stops_collection() {
        // Create a source with many broken constructs to test the limit.
        let mut src = String::new();
        for i in 0..20 {
            // Each fact is missing 'source' field -- will error out.
            src.push_str(&format!("fact broken_{} {{ type: Bool }}\n", i));
        }
        let tokens = lexer::lex(&src, "limit.tenor").expect("lex should succeed");
        let (_, errors) = parse_recovering(&tokens, "limit.tenor", 5).expect("should not be fatal");
        assert_eq!(
            errors.len(),
            5,
            "Should stop at max_errors=5, got {}",
            errors.len()
        );
    }
}
