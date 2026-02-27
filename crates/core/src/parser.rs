/// Raw AST produced by the parser.
/// All constructs carry provenance (file, line of the opening keyword).
/// No type checking or resolution is done here -- that is elaboration's job.
use crate::error::ElabError;
use crate::lexer::{Spanned, Token};
use std::collections::BTreeMap;

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

    // -- Type parsing -------------------------------------------

    fn parse_type(&mut self) -> Result<RawType, ElabError> {
        let name = self.take_word()?;
        match name.as_str() {
            "Bool" => Ok(RawType::Bool),
            "Date" => Ok(RawType::Date),
            "DateTime" => Ok(RawType::DateTime),
            "Int" => {
                if self.peek() == &Token::LParen {
                    self.advance();
                    let (min, max) = self.parse_int_params()?;
                    self.expect_rparen()?;
                    Ok(RawType::Int { min, max })
                } else {
                    Ok(RawType::Int {
                        min: i64::MIN,
                        max: i64::MAX,
                    })
                }
            }
            "Decimal" => {
                self.advance_lparen()?;
                let precision = self.parse_named_or_positional_u32("precision")? as u32;
                if self.peek() == &Token::Comma {
                    self.advance();
                }
                let scale = self.parse_named_or_positional_u32("scale")? as u32;
                self.expect_rparen()?;
                Ok(RawType::Decimal { precision, scale })
            }
            "Text" => {
                if self.peek() == &Token::LParen {
                    self.advance();
                    let max_length = self.parse_named_or_positional_u32("max_length")? as u32;
                    self.expect_rparen()?;
                    Ok(RawType::Text { max_length })
                } else {
                    Ok(RawType::Text { max_length: 0 })
                }
            }
            "Money" => {
                self.advance_lparen()?;
                if self.is_word("currency") {
                    self.advance();
                    self.expect_colon()?;
                }
                let currency = self.take_str()?;
                self.expect_rparen()?;
                Ok(RawType::Money { currency })
            }
            "Duration" => {
                self.advance_lparen()?;
                let mut unit = String::new();
                let mut min = 0i64;
                let mut max = i64::MAX;
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "unit" => {
                            unit = self.take_str()?;
                        }
                        "min" => {
                            min = self.take_int()?;
                        }
                        "max" => {
                            max = self.take_int()?;
                        }
                        _ => return Err(self.err(format!("unknown Duration param '{}'", key))),
                    }
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect_rparen()?;
                Ok(RawType::Duration { unit, min, max })
            }
            "Enum" => {
                self.advance_lparen()?;
                if self.is_word("values") {
                    self.advance();
                    self.expect_colon()?;
                }
                let values = self.parse_string_array()?;
                self.expect_rparen()?;
                Ok(RawType::Enum { values })
            }
            "List" => {
                self.advance_lparen()?;
                let mut element_type: Option<RawType> = None;
                let mut max = 0u32;
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "element_type" => {
                            element_type = Some(self.parse_type()?);
                        }
                        "max" => {
                            max = self.take_int()? as u32;
                        }
                        _ => return Err(self.err(format!("unknown List param '{}'", key))),
                    }
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect_rparen()?;
                let et = element_type.ok_or_else(|| self.err("List missing element_type"))?;
                Ok(RawType::List {
                    element_type: Box::new(et),
                    max,
                })
            }
            "Record" => {
                self.advance_lparen()?;
                if self.is_word("fields") {
                    self.advance();
                    self.expect_colon()?;
                }
                let fields = self.parse_record_fields()?;
                self.expect_rparen()?;
                Ok(RawType::Record { fields })
            }
            other => Ok(RawType::TypeRef(other.to_owned())),
        }
    }

    fn advance_lparen(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::LParen {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '(', got {:?}", self.peek())))
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::RParen {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected ')', got {:?}", self.peek())))
        }
    }

    fn parse_named_or_positional_u32(&mut self, key: &str) -> Result<i64, ElabError> {
        if self.is_word(key) {
            self.advance();
            self.expect_colon()?;
        }
        self.take_int()
    }

    fn parse_int_params(&mut self) -> Result<(i64, i64), ElabError> {
        let first_is_key = self.is_word("min");
        if first_is_key {
            self.advance();
            self.expect_colon()?;
        }
        let min = self.take_int()?;
        if self.peek() == &Token::Comma {
            self.advance();
        }
        if self.is_word("max") {
            self.advance();
            self.expect_colon()?;
        }
        let max = self.take_int()?;
        Ok((min, max))
    }

    fn parse_string_array(&mut self) -> Result<Vec<String>, ElabError> {
        self.expect_lbracket()?;
        let mut values = Vec::new();
        while self.peek() != &Token::RBracket {
            values.push(self.take_str()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(values)
    }

    fn expect_lbracket(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::LBracket {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '[', got {:?}", self.peek())))
        }
    }

    fn expect_rbracket(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::RBracket {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected ']', got {:?}", self.peek())))
        }
    }

    fn parse_record_fields(&mut self) -> Result<BTreeMap<String, RawType>, ElabError> {
        let mut fields = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let name = self.take_word()?;
            self.expect_colon()?;
            let t = self.parse_type()?;
            fields.insert(name, t);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbrace()?;
        Ok(fields)
    }

    // -- Literal parsing ----------------------------------------

    fn parse_literal(&mut self) -> Result<RawLiteral, ElabError> {
        match self.peek().clone() {
            Token::Word(w) if w == "true" => {
                self.advance();
                Ok(RawLiteral::Bool(true))
            }
            Token::Word(w) if w == "false" => {
                self.advance();
                Ok(RawLiteral::Bool(false))
            }
            Token::Int(n) => {
                self.advance();
                Ok(RawLiteral::Int(n))
            }
            Token::Float(f) => {
                let s = f.clone();
                self.advance();
                Ok(RawLiteral::Float(s))
            }
            Token::Str(s) => {
                let v = s.clone();
                self.advance();
                Ok(RawLiteral::Str(v))
            }
            Token::Word(w) if w == "Money" => {
                self.advance();
                self.expect_lbrace()?;
                let mut amount = String::new();
                let mut currency = String::new();
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "amount" => {
                            amount = self.take_str()?;
                        }
                        "currency" => {
                            currency = self.take_str()?;
                        }
                        _ => return Err(self.err(format!("unknown Money key '{}'", key))),
                    }
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect_rbrace()?;
                Ok(RawLiteral::Money { amount, currency })
            }
            _ => Err(self.err(format!("expected literal value, got {:?}", self.peek()))),
        }
    }

    // -- Expression parsing --------------------------------------

    pub fn parse_expr(&mut self) -> Result<RawExpr, ElabError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<RawExpr, ElabError> {
        let mut left = self.parse_and_expr()?;
        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and_expr()?;
            left = RawExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<RawExpr, ElabError> {
        let mut left = self.parse_unary_expr()?;
        while self.peek() == &Token::And || self.is_word("and") {
            self.advance();
            let right = self.parse_unary_expr()?;
            left = RawExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<RawExpr, ElabError> {
        if self.peek() == &Token::Not || self.is_word("not") {
            self.advance();
            let e = self.parse_atom_expr()?;
            return Ok(RawExpr::Not(Box::new(e)));
        }
        self.parse_atom_expr()
    }

    fn parse_atom_expr(&mut self) -> Result<RawExpr, ElabError> {
        if self.peek() == &Token::Forall {
            let line = self.cur_line();
            self.advance();
            let var = self.take_word()?;
            if self.peek() != &Token::In {
                return Err(self.err("expected \u{2208} after quantifier variable"));
            }
            self.advance();
            let domain = self.take_word()?;
            if self.peek() == &Token::Dot {
                self.advance();
            } else {
                return Err(self.err("expected '.' after quantifier domain"));
            }
            let body = self.parse_expr()?;
            return Ok(RawExpr::Forall {
                var,
                domain,
                body: Box::new(body),
                line,
            });
        }

        if self.peek() == &Token::Exists {
            let line = self.cur_line();
            self.advance();
            let var = self.take_word()?;
            if self.peek() != &Token::In {
                return Err(self.err("expected \u{2208} after quantifier variable"));
            }
            self.advance();
            let domain = self.take_word()?;
            if self.peek() == &Token::Dot {
                self.advance();
            } else {
                return Err(self.err("expected '.' after quantifier domain"));
            }
            let body = self.parse_expr()?;
            return Ok(RawExpr::Exists {
                var,
                domain,
                body: Box::new(body),
                line,
            });
        }

        if self.is_word("verdict_present") {
            let line = self.cur_line();
            self.advance();
            self.advance_lparen()?;
            let id = self.take_word()?;
            self.expect_rparen()?;
            return Ok(RawExpr::VerdictPresent { id, line });
        }

        if self.peek() == &Token::LParen {
            self.advance();
            let e = self.parse_expr()?;
            self.expect_rparen()?;
            return Ok(e);
        }

        let line = self.cur_line();
        let left = self.parse_term()?;
        let op = self.parse_compare_op()?;
        let right = self.parse_term()?;
        Ok(RawExpr::Compare {
            op,
            left,
            right,
            line,
        })
    }

    fn parse_compare_op(&mut self) -> Result<String, ElabError> {
        let op = match self.peek() {
            Token::Eq => "=",
            Token::Neq => "!=",
            Token::Lt => "<",
            Token::Lte => "<=",
            Token::Gt => ">",
            Token::Gte => ">=",
            _ => {
                return Err(self.err(format!(
                    "expected comparison operator, got {:?}",
                    self.peek()
                )))
            }
        };
        self.advance();
        Ok(op.to_owned())
    }

    fn parse_base_term(&mut self) -> Result<RawTerm, ElabError> {
        match self.peek().clone() {
            Token::Word(w) if w == "true" => {
                self.advance();
                Ok(RawTerm::Literal(RawLiteral::Bool(true)))
            }
            Token::Word(w) if w == "false" => {
                self.advance();
                Ok(RawTerm::Literal(RawLiteral::Bool(false)))
            }
            Token::Int(n) => {
                self.advance();
                Ok(RawTerm::Literal(RawLiteral::Int(n)))
            }
            Token::Float(f) => {
                let s = f.clone();
                self.advance();
                Ok(RawTerm::Literal(RawLiteral::Float(s)))
            }
            Token::Str(s) => {
                let v = s.clone();
                self.advance();
                Ok(RawTerm::Literal(RawLiteral::Str(v)))
            }
            Token::Word(w) if w == "Money" => {
                let lit = self.parse_literal()?;
                Ok(RawTerm::Literal(lit))
            }
            Token::Word(ref w) => {
                let name = w.clone();
                self.advance();
                if self.peek() == &Token::Dot {
                    self.advance();
                    let field = self.take_word()?;
                    Ok(RawTerm::FieldRef { var: name, field })
                } else {
                    Ok(RawTerm::FactRef(name))
                }
            }
            _ => Err(self.err(format!("expected term, got {:?}", self.peek()))),
        }
    }

    fn parse_term(&mut self) -> Result<RawTerm, ElabError> {
        let left = self.parse_base_term()?;
        if self.peek() == &Token::Star {
            self.advance();
            let right = self.parse_base_term()?;
            return Ok(RawTerm::Mul {
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    // -- Top-level construct parsers ----------------------------

    pub fn parse_file(&mut self) -> Result<Vec<RawConstruct>, ElabError> {
        let mut constructs = Vec::new();
        while self.peek() != &Token::Eof {
            let c = self.parse_construct()?;
            constructs.push(c);
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

    fn parse_typedecl(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        let fields = self.parse_typedecl_body()?;
        Ok(RawConstruct::TypeDecl {
            id,
            fields,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_typedecl_body(&mut self) -> Result<BTreeMap<String, RawType>, ElabError> {
        let mut fields = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let name = self.take_word()?;
            self.expect_colon()?;
            let t = self.parse_type()?;
            fields.insert(name, t);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbrace()?;
        Ok(fields)
    }

    fn parse_fact(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut type_ = None;
        let mut source = None;
        let mut default = None;
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "type" => {
                    type_ = Some(self.parse_type()?);
                }
                "source" => {
                    source = Some(self.parse_fact_source()?);
                }
                "default" => {
                    default = Some(self.parse_literal()?);
                }
                _ => return Err(self.err(format!("unknown Fact field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Fact {
            id,
            type_: type_.ok_or_else(|| self.err("Fact missing 'type'"))?,
            source: source.ok_or_else(|| self.err("Fact missing 'source'"))?,
            default,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_fact_source(&mut self) -> Result<RawSourceDecl, ElabError> {
        // Freetext: source: "some.string"
        // Structured: source: source_id { path: "..." }
        if let Token::Str(_) = self.peek() {
            let s = self.take_str()?;
            return Ok(RawSourceDecl::Freetext(s));
        }
        // Structured form: bare identifier then { path: "..." }
        let source_id = self.take_word()?;
        self.expect_lbrace()?;
        self.expect_word("path")?;
        self.expect_colon()?;
        let path = self.take_str()?;
        self.expect_rbrace()?;
        Ok(RawSourceDecl::Structured { source_id, path })
    }

    fn parse_source(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'source'
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut protocol = None;
        let mut description = None;
        let mut fields = BTreeMap::new();
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "protocol" => {
                    protocol = Some(self.parse_protocol_tag()?);
                }
                "description" => {
                    description = Some(self.take_str()?);
                }
                _ => {
                    // All other fields: accept string or bare word as string value
                    let val = if let Token::Str(_) = self.peek() {
                        self.take_str()?
                    } else {
                        self.take_word()?
                    };
                    fields.insert(key, val);
                }
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Source {
            id,
            protocol: protocol.ok_or_else(|| self.err("Source missing 'protocol'"))?,
            fields,
            description,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_protocol_tag(&mut self) -> Result<String, ElabError> {
        let mut tag = self.take_word()?;
        // Handle dotted extension tags like x_internal.event_bus
        while self.peek() == &Token::Dot {
            self.advance();
            let segment = self.take_word()?;
            tag.push('.');
            tag.push_str(&segment);
        }
        Ok(tag)
    }

    fn parse_entity(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut states = Vec::new();
        let mut initial = String::new();
        let mut initial_line = line;
        let mut transitions = Vec::new();
        let mut parent = None;
        let mut parent_line = None;
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "states" => {
                    states = self.parse_ident_array()?;
                }
                "initial" => {
                    initial_line = field_line;
                    initial = self.take_word()?;
                }
                "transitions" => {
                    transitions = self.parse_transitions()?;
                }
                "parent" => {
                    parent_line = Some(field_line);
                    parent = Some(self.take_word()?);
                }
                _ => return Err(self.err(format!("unknown Entity field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Entity {
            id,
            states,
            initial,
            initial_line,
            transitions,
            parent,
            parent_line,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_ident_array(&mut self) -> Result<Vec<String>, ElabError> {
        self.expect_lbracket()?;
        let mut items = Vec::new();
        while self.peek() != &Token::RBracket {
            items.push(self.take_word()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(items)
    }

    fn parse_transitions(&mut self) -> Result<Vec<(String, String, u32)>, ElabError> {
        self.expect_lbracket()?;
        let mut transitions = Vec::new();
        while self.peek() != &Token::RBracket {
            let t_line = self.cur_line();
            self.advance_lparen()?;
            let from = self.take_word()?;
            self.expect_transition_sep()?;
            let to = self.take_word()?;
            self.expect_rparen()?;
            transitions.push((from, to, t_line));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(transitions)
    }

    fn expect_transition_sep(&mut self) -> Result<(), ElabError> {
        match self.peek() {
            Token::Comma | Token::Gt => {
                self.advance();
                Ok(())
            }
            _ => Err(self.err(format!(
                "expected ',' or '->'/'->''', got {:?}",
                self.peek()
            ))),
        }
    }

    fn expect_comma(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::Comma {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected ',', got {:?}", self.peek())))
        }
    }

    fn parse_rule(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut stratum = None;
        let mut stratum_line = line;
        let mut when = None;
        let mut verdict_type = String::new();
        let mut payload_type = RawType::Bool;
        let mut payload_value = RawTerm::Literal(RawLiteral::Bool(true));
        let mut produce_line = line;
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "stratum" => {
                    stratum_line = field_line;
                    stratum = Some(self.take_int()?);
                }
                "when" => {
                    when = Some(self.parse_expr()?);
                }
                "produce" => {
                    produce_line = field_line;
                    let (vt, pt, pv) = self.parse_produce()?;
                    verdict_type = vt;
                    payload_type = pt;
                    payload_value = pv;
                }
                _ => return Err(self.err(format!("unknown Rule field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Rule {
            id,
            stratum: stratum.ok_or_else(|| self.err("Rule missing 'stratum'"))?,
            stratum_line,
            when: when.ok_or_else(|| self.err("Rule missing 'when'"))?,
            verdict_type,
            payload_type,
            payload_value,
            produce_line,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_produce(&mut self) -> Result<(String, RawType, RawTerm), ElabError> {
        self.expect_word("verdict")?;
        let verdict_type = self.take_word()?;
        self.expect_lbrace()?;
        self.expect_word("payload")?;
        self.expect_colon()?;
        let payload_type = self.parse_type()?;
        self.expect_eq()?;
        let payload_value = self.parse_term()?;
        self.expect_rbrace()?;
        Ok((verdict_type, payload_type, payload_value))
    }

    fn expect_eq(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::Eq {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '=', got {:?}", self.peek())))
        }
    }

    fn parse_operation(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut personas = Vec::new();
        let mut allowed_personas_line = line;
        let mut precondition = None;
        let mut effects = Vec::new();
        let mut error_contract = Vec::new();
        let mut outcomes = Vec::new();
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "allowed_personas" => {
                    allowed_personas_line = field_line;
                    personas = self.parse_ident_array()?;
                }
                "precondition" => {
                    precondition = Some(self.parse_expr()?);
                }
                "effects" => {
                    effects = self.parse_effects()?;
                }
                "error_contract" => {
                    error_contract = self.parse_ident_array()?;
                }
                "outcomes" => {
                    outcomes = self.parse_ident_array()?;
                }
                _ => return Err(self.err(format!("unknown Operation field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Operation {
            id,
            allowed_personas: personas,
            allowed_personas_line,
            precondition: precondition
                .ok_or_else(|| self.err("Operation missing 'precondition'"))?,
            effects,
            error_contract,
            outcomes,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    #[allow(clippy::type_complexity)]
    fn parse_effects(
        &mut self,
    ) -> Result<Vec<(String, String, String, Option<String>, u32)>, ElabError> {
        self.expect_lbracket()?;
        let mut effects = Vec::new();
        while self.peek() != &Token::RBracket {
            let e_line = self.cur_line();
            self.advance_lparen()?;
            let entity = self.take_word()?;
            self.expect_comma()?;
            let from = self.take_word()?;
            self.expect_comma()?;
            let to = self.take_word()?;
            // Optional outcome label: if next token is a comma followed by a word (not RParen)
            let outcome = if self.peek() == &Token::Comma {
                // Peek ahead: save position, check if after comma there's a word before RParen
                let saved_pos = self.pos;
                self.advance(); // consume comma
                if let Token::Word(_) = self.peek() {
                    Some(self.take_word()?)
                } else {
                    // Not a word after comma -- restore position (the comma was a trailing comma)
                    self.pos = saved_pos;
                    None
                }
            } else {
                None
            };
            self.expect_rparen()?;
            effects.push((entity, from, to, outcome, e_line));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(effects)
    }

    fn parse_flow(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance();
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut snapshot = String::new();
        let mut entry = String::new();
        let mut entry_line = line;
        let mut steps = BTreeMap::new();
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "snapshot" => {
                    snapshot = self.take_word()?;
                }
                "entry" => {
                    entry_line = field_line;
                    entry = self.take_word()?;
                }
                "steps" => {
                    steps = self.parse_steps()?;
                }
                _ => return Err(self.err(format!("unknown Flow field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Flow {
            id,
            snapshot,
            entry,
            entry_line,
            steps,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_persona(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'persona'
        let id = self.take_word()?;
        Ok(RawConstruct::Persona {
            id,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    fn parse_system(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'system'
        let id = self.take_word()?;
        self.expect_lbrace()?;

        let mut members = Vec::new();
        let mut shared_personas = Vec::new();
        let mut triggers = Vec::new();
        let mut shared_entities = Vec::new();

        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "members" => {
                    members = self.parse_system_members()?;
                }
                "shared_personas" => {
                    shared_personas = self.parse_system_shared_personas()?;
                }
                "triggers" => {
                    triggers = self.parse_system_triggers()?;
                }
                "shared_entities" => {
                    shared_entities = self.parse_system_shared_entities()?;
                }
                _ => return Err(self.err(format!("unknown System field '{}'", key))),
            }
        }
        self.expect_rbrace()?;

        Ok(RawConstruct::System {
            id,
            members,
            shared_personas,
            triggers,
            shared_entities,
            prov: Provenance {
                file: self.filename.clone(),
                line,
            },
        })
    }

    /// Parse `members: [ member_id: "path", ... ]`
    fn parse_system_members(&mut self) -> Result<Vec<(String, String)>, ElabError> {
        self.expect_lbracket()?;
        let mut members = Vec::new();
        while self.peek() != &Token::RBracket {
            let member_id = self.take_word()?;
            self.expect_colon()?;
            let path = self.take_str()?;
            members.push((member_id, path));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(members)
    }

    /// Parse `shared_personas: [ { persona: id, contracts: [a, b] }, ... ]`
    fn parse_system_shared_personas(&mut self) -> Result<Vec<(String, Vec<String>)>, ElabError> {
        self.expect_lbracket()?;
        let mut entries = Vec::new();
        while self.peek() != &Token::RBracket {
            self.expect_lbrace()?;
            let mut persona = String::new();
            let mut contracts = Vec::new();
            while self.peek() != &Token::RBrace {
                let field = self.take_word()?;
                self.expect_colon()?;
                match field.as_str() {
                    "persona" => {
                        persona = self.take_word()?;
                    }
                    "contracts" => {
                        contracts = self.parse_ident_array()?;
                    }
                    _ => return Err(self.err(format!("unknown shared_personas field '{}'", field))),
                }
                if self.peek() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect_rbrace()?;
            entries.push((persona, contracts));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(entries)
    }

    /// Parse `triggers: [ { source: a.flow, on: success, target: b.flow, persona: p }, ... ]`
    fn parse_system_triggers(&mut self) -> Result<Vec<RawTrigger>, ElabError> {
        self.expect_lbracket()?;
        let mut triggers = Vec::new();
        while self.peek() != &Token::RBracket {
            self.expect_lbrace()?;
            let mut source_contract = String::new();
            let mut source_flow = String::new();
            let mut on = String::new();
            let mut target_contract = String::new();
            let mut target_flow = String::new();
            let mut persona = String::new();
            while self.peek() != &Token::RBrace {
                let field = self.take_word()?;
                self.expect_colon()?;
                match field.as_str() {
                    "source" => {
                        // source: member_id.flow_id
                        source_contract = self.take_word()?;
                        if self.peek() == &Token::Dot {
                            self.advance();
                            source_flow = self.take_word()?;
                        } else {
                            return Err(self.err("expected '.' after source contract in trigger"));
                        }
                    }
                    "on" => {
                        on = self.take_word()?;
                    }
                    "target" => {
                        // target: member_id.flow_id
                        target_contract = self.take_word()?;
                        if self.peek() == &Token::Dot {
                            self.advance();
                            target_flow = self.take_word()?;
                        } else {
                            return Err(self.err("expected '.' after target contract in trigger"));
                        }
                    }
                    "persona" => {
                        persona = self.take_word()?;
                    }
                    _ => return Err(self.err(format!("unknown trigger field '{}'", field))),
                }
                if self.peek() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect_rbrace()?;
            triggers.push(RawTrigger {
                source_contract,
                source_flow,
                on,
                target_contract,
                target_flow,
                persona,
            });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(triggers)
    }

    /// Parse `shared_entities: [ { entity: id, contracts: [a, b] }, ... ]`
    fn parse_system_shared_entities(&mut self) -> Result<Vec<(String, Vec<String>)>, ElabError> {
        self.expect_lbracket()?;
        let mut entries = Vec::new();
        while self.peek() != &Token::RBracket {
            self.expect_lbrace()?;
            let mut entity = String::new();
            let mut contracts = Vec::new();
            while self.peek() != &Token::RBrace {
                let field = self.take_word()?;
                self.expect_colon()?;
                match field.as_str() {
                    "entity" => {
                        entity = self.take_word()?;
                    }
                    "contracts" => {
                        contracts = self.parse_ident_array()?;
                    }
                    _ => return Err(self.err(format!("unknown shared_entities field '{}'", field))),
                }
                if self.peek() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect_rbrace()?;
            entries.push((entity, contracts));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(entries)
    }

    fn parse_steps(&mut self) -> Result<BTreeMap<String, RawStep>, ElabError> {
        let mut steps = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let step_line = self.cur_line();
            let step_id = self.take_word()?;
            self.expect_colon()?;
            let step_kind = self.take_word()?;
            let step = self.parse_step_body(&step_kind, step_line)?;
            steps.insert(step_id, step);
        }
        self.expect_rbrace()?;
        Ok(steps)
    }

    fn parse_step_body(&mut self, kind: &str, step_line: u32) -> Result<RawStep, ElabError> {
        self.expect_lbrace()?;
        let step = match kind {
            "OperationStep" => {
                let mut op = String::new();
                let mut persona = String::new();
                let mut outcomes = BTreeMap::new();
                let mut on_failure = None;
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "op" => {
                            op = self.take_word()?;
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "outcomes" => {
                            outcomes = self.parse_outcomes()?;
                        }
                        "on_failure" => {
                            on_failure = Some(self.parse_failure_handler()?);
                        }
                        _ => return Err(self.err(format!("unknown OperationStep field '{}'", key))),
                    }
                }
                RawStep::OperationStep {
                    op,
                    persona,
                    outcomes,
                    on_failure,
                    line: step_line,
                }
            }
            "BranchStep" => {
                let mut condition = None;
                let mut persona = String::new();
                let mut if_true = None;
                let mut if_false = None;
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "condition" => {
                            condition = Some(self.parse_expr()?);
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "if_true" => {
                            if_true = Some(self.parse_step_target()?);
                        }
                        "if_false" => {
                            if_false = Some(self.parse_step_target()?);
                        }
                        _ => return Err(self.err(format!("unknown BranchStep field '{}'", key))),
                    }
                }
                RawStep::BranchStep {
                    condition: condition.ok_or_else(|| self.err("BranchStep missing condition"))?,
                    persona,
                    if_true: if_true.ok_or_else(|| self.err("BranchStep missing if_true"))?,
                    if_false: if_false.ok_or_else(|| self.err("BranchStep missing if_false"))?,
                    line: step_line,
                }
            }
            "HandoffStep" => {
                let mut from_persona = String::new();
                let mut to_persona = String::new();
                let mut next = String::new();
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "from_persona" => {
                            from_persona = self.take_word()?;
                        }
                        "to_persona" => {
                            to_persona = self.take_word()?;
                        }
                        "next" => {
                            next = self.take_word()?;
                        }
                        _ => return Err(self.err(format!("unknown HandoffStep field '{}'", key))),
                    }
                }
                RawStep::HandoffStep {
                    from_persona,
                    to_persona,
                    next,
                    line: step_line,
                }
            }
            "SubFlowStep" => {
                let mut flow = String::new();
                let mut flow_line = step_line;
                let mut persona = String::new();
                let mut on_success = None;
                let mut on_failure = None;
                while self.peek() != &Token::RBrace {
                    let field_line = self.cur_line();
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "flow" => {
                            flow_line = field_line;
                            flow = self.take_word()?;
                        }
                        "persona" => {
                            persona = self.take_word()?;
                        }
                        "on_success" => {
                            on_success = Some(self.parse_step_target()?);
                        }
                        "on_failure" => {
                            on_failure = Some(self.parse_failure_handler()?);
                        }
                        _ => return Err(self.err(format!("unknown SubFlowStep field '{}'", key))),
                    }
                }
                RawStep::SubFlowStep {
                    flow,
                    flow_line,
                    persona,
                    on_success: on_success
                        .ok_or_else(|| self.err("SubFlowStep missing on_success"))?,
                    on_failure: on_failure
                        .ok_or_else(|| self.err("SubFlowStep missing on_failure"))?,
                    line: step_line,
                }
            }
            "ParallelStep" => {
                let mut branches = Vec::new();
                let mut branches_line = step_line;
                let mut join = None;
                while self.peek() != &Token::RBrace {
                    let field_line = self.cur_line();
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "branches" => {
                            branches_line = field_line;
                            branches = self.parse_branches()?;
                        }
                        "join" => {
                            join = Some(self.parse_join_policy()?);
                        }
                        _ => return Err(self.err(format!("unknown ParallelStep field '{}'", key))),
                    }
                }
                RawStep::ParallelStep {
                    branches,
                    branches_line,
                    join: join.ok_or_else(|| self.err("ParallelStep missing join"))?,
                    line: step_line,
                }
            }
            _ => return Err(self.err(format!("unknown step kind '{}'", kind))),
        };
        self.expect_rbrace()?;
        Ok(step)
    }

    fn parse_outcomes(&mut self) -> Result<BTreeMap<String, RawStepTarget>, ElabError> {
        let mut outcomes = BTreeMap::new();
        self.expect_lbrace()?;
        while self.peek() != &Token::RBrace {
            let label = self.take_word()?;
            self.expect_colon()?;
            let target = self.parse_step_target()?;
            outcomes.insert(label, target);
        }
        self.expect_rbrace()?;
        Ok(outcomes)
    }

    fn parse_step_target(&mut self) -> Result<RawStepTarget, ElabError> {
        if self.is_word("Terminal") {
            self.advance();
            self.advance_lparen()?;
            let outcome = self.take_word()?;
            self.expect_rparen()?;
            return Ok(RawStepTarget::Terminal { outcome });
        }
        let line = self.cur_line();
        let name = self.take_word()?;
        Ok(RawStepTarget::StepRef(name, line))
    }

    fn parse_failure_handler(&mut self) -> Result<RawFailureHandler, ElabError> {
        let kind = self.take_word()?;
        match kind.as_str() {
            "Terminate" => {
                self.advance_lparen()?;
                if self.is_word("outcome") {
                    self.advance();
                    self.expect_colon()?;
                }
                let outcome = self.take_word()?;
                self.expect_rparen()?;
                Ok(RawFailureHandler::Terminate { outcome })
            }
            "Compensate" => {
                self.advance_lparen()?;
                let mut comp_steps = Vec::new();
                let mut then_outcome = String::new();
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "steps" => {
                            comp_steps = self.parse_comp_steps()?;
                        }
                        "then" => {
                            self.expect_word("Terminal")?;
                            self.advance_lparen()?;
                            then_outcome = self.take_word()?;
                            self.expect_rparen()?;
                        }
                        _ => return Err(self.err(format!("unknown Compensate field '{}'", key))),
                    }
                }
                self.expect_rparen()?;
                Ok(RawFailureHandler::Compensate {
                    steps: comp_steps,
                    then: then_outcome,
                })
            }
            "Escalate" => {
                self.advance_lparen()?;
                let mut to_persona = String::new();
                let mut next = String::new();
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "to" | "to_persona" => {
                            to_persona = self.take_word()?;
                        }
                        "next" => {
                            next = self.take_word()?;
                        }
                        _ => return Err(self.err(format!("unknown Escalate field '{}'", key))),
                    }
                }
                self.expect_rparen()?;
                Ok(RawFailureHandler::Escalate { to_persona, next })
            }
            _ => Err(self.err(format!("unknown failure handler kind '{}'", kind))),
        }
    }

    fn parse_branches(&mut self) -> Result<Vec<RawBranch>, ElabError> {
        self.expect_lbracket()?;
        let mut branches = Vec::new();
        while self.peek() != &Token::RBracket {
            branches.push(self.parse_branch()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(branches)
    }

    fn parse_branch(&mut self) -> Result<RawBranch, ElabError> {
        self.expect_word("Branch")?;
        self.expect_lbrace()?;
        let mut id = String::new();
        let mut entry = String::new();
        let mut steps = BTreeMap::new();
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "id" => {
                    id = self.take_word()?;
                }
                "entry" => {
                    entry = self.take_word()?;
                }
                "steps" => {
                    steps = self.parse_steps()?;
                }
                _ => return Err(self.err(format!("unknown Branch field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawBranch { id, entry, steps })
    }

    fn parse_join_policy(&mut self) -> Result<RawJoinPolicy, ElabError> {
        self.expect_word("JoinPolicy")?;
        self.expect_lbrace()?;
        let mut on_all_success = None;
        let mut on_any_failure = None;
        let mut on_all_complete = None;
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "on_all_success" => {
                    on_all_success = Some(self.parse_step_target()?);
                }
                "on_any_failure" => {
                    on_any_failure = Some(self.parse_failure_handler()?);
                }
                "on_all_complete" => {
                    if self.is_word("null") {
                        self.advance();
                    } else {
                        on_all_complete = Some(self.parse_step_target()?);
                    }
                }
                _ => return Err(self.err(format!("unknown JoinPolicy field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawJoinPolicy {
            on_all_success,
            on_any_failure,
            on_all_complete,
        })
    }

    fn parse_comp_steps(&mut self) -> Result<Vec<RawCompStep>, ElabError> {
        let mut steps = Vec::new();
        self.expect_lbracket()?;
        while self.peek() != &Token::RBracket {
            self.expect_lbrace()?;
            let mut op = String::new();
            let mut persona = String::new();
            let mut on_failure = String::new();
            while self.peek() != &Token::RBrace {
                let key = self.take_word()?;
                self.expect_colon()?;
                match key.as_str() {
                    "op" => {
                        op = self.take_word()?;
                    }
                    "persona" => {
                        persona = self.take_word()?;
                    }
                    "on_failure" => {
                        self.expect_word("Terminal")?;
                        self.advance_lparen()?;
                        on_failure = self.take_word()?;
                        self.expect_rparen()?;
                    }
                    _ => return Err(self.err(format!("unknown comp step field '{}'", key))),
                }
            }
            self.expect_rbrace()?;
            steps.push(RawCompStep {
                op,
                persona,
                on_failure,
            });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect_rbracket()?;
        Ok(steps)
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
