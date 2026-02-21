/// Raw AST produced by the parser.
/// All constructs carry provenance (file, line of the opening keyword).
/// No type checking or resolution is done here — that is elaboration's job.
use crate::error::ElabError;
use crate::lexer::{Spanned, Token};
use std::collections::BTreeMap;

// ──────────────────────────────────────────────
// Raw types (pre-elaboration)
// ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Provenance {
    pub file: String,
    pub line: u32,
}

/// A raw BaseType as it appears in the DSL, before TypeRef resolution.
#[derive(Debug, Clone)]
pub enum RawType {
    Bool,
    Int { min: i64, max: i64 },
    Decimal { precision: u32, scale: u32 },
    Text { max_length: u32 },
    Date,
    DateTime,
    Money { currency: String },
    Duration { unit: String, min: i64, max: i64 },
    Enum { values: Vec<String> },
    Record { fields: BTreeMap<String, RawType> },
    List { element_type: Box<RawType>, max: u32 },
    /// Named type reference — resolved during Pass 3/4
    TypeRef(String),
}

/// A raw literal value
#[derive(Debug, Clone)]
pub enum RawLiteral {
    Bool(bool),
    Int(i64),
    Float(String),
    Str(String),
    Money { amount: String, currency: String },
}

/// A raw predicate expression
#[derive(Debug, Clone)]
pub enum RawExpr {
    /// fact_ref op literal — line is the line of the left operand
    Compare { op: String, left: RawTerm, right: RawTerm, line: u32 },
    /// verdict_present(id) — line is the line of the verdict_present token
    VerdictPresent { id: String, line: u32 },
    /// e1 ∧ e2
    And(Box<RawExpr>, Box<RawExpr>),
    /// e1 ∨ e2
    Or(Box<RawExpr>, Box<RawExpr>),
    /// ¬ e
    Not(Box<RawExpr>),
    /// ∀ var ∈ list_ref . body — line is the line of the ∀ token
    Forall {
        var: String,
        domain: String,
        body: Box<RawExpr>,
        line: u32,
    },
}

#[derive(Debug, Clone)]
pub enum RawTerm {
    FactRef(String),
    FieldRef { var: String, field: String },
    Literal(RawLiteral),
    /// Arithmetic multiplication: left * right
    Mul { left: Box<RawTerm>, right: Box<RawTerm> },
}

/// Raw construct from the parser
#[derive(Debug, Clone)]
pub enum RawConstruct {
    Import {
        path: String,
        prov: Provenance,
    },
    TypeDecl {
        id: String,
        fields: BTreeMap<String, RawType>,
        prov: Provenance,
    },
    Fact {
        id: String,
        type_: RawType,
        source: String,
        default: Option<RawLiteral>,
        prov: Provenance,
    },
    Entity {
        id: String,
        states: Vec<String>,
        initial: String,
        /// Line of the `initial:` field keyword
        initial_line: u32,
        /// (from, to, line_of_lparen) — line is the line of the `(` opening each tuple
        transitions: Vec<(String, String, u32)>,
        parent: Option<String>,
        /// Line of the `parent:` field keyword, when present
        parent_line: Option<u32>,
        prov: Provenance,
    },
    Rule {
        id: String,
        stratum: i64,
        /// Line of the `stratum:` field keyword
        stratum_line: u32,
        when: RawExpr,
        verdict_type: String,
        payload_type: RawType,
        /// Payload value expression (literal or multiplication)
        payload_value: RawTerm,
        /// Line of the `produce:` field keyword
        produce_line: u32,
        prov: Provenance,
    },
    Operation {
        id: String,
        allowed_personas: Vec<String>,
        /// Line of the `allowed_personas:` field keyword
        allowed_personas_line: u32,
        precondition: RawExpr,
        /// (entity, from, to, line_of_lparen) — line is the line of the `(` opening each tuple
        effects: Vec<(String, String, String, u32)>,
        error_contract: Vec<String>,
        prov: Provenance,
    },
    Flow {
        id: String,
        snapshot: String,
        entry: String,
        /// Line of the `entry:` field keyword
        entry_line: u32,
        steps: BTreeMap<String, RawStep>,
        prov: Provenance,
    },
}

#[derive(Debug, Clone)]
pub enum RawStep {
    OperationStep {
        op: String,
        persona: String,
        outcomes: BTreeMap<String, RawStepTarget>,
        /// Optional at parse time; absence is a Pass 5 error (not a parse error)
        on_failure: Option<RawFailureHandler>,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    BranchStep {
        condition: RawExpr,
        persona: String,
        if_true: RawStepTarget,
        if_false: RawStepTarget,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    HandoffStep {
        from_persona: String,
        to_persona: String,
        next: String,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    SubFlowStep {
        /// Id of the referenced Flow construct
        flow: String,
        /// Line of the `flow:` field keyword (used in cycle error reporting)
        flow_line: u32,
        persona: String,
        on_success: RawStepTarget,
        on_failure: RawFailureHandler,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
    ParallelStep {
        branches: Vec<RawBranch>,
        /// Line of the `branches:` field keyword
        branches_line: u32,
        join: RawJoinPolicy,
        /// Line of the `step_id:` token in the steps map
        line: u32,
    },
}

#[derive(Debug, Clone)]
pub enum RawStepTarget {
    /// (step_id, line_of_step_id_token)
    StepRef(String, u32),
    Terminal { outcome: String },
}

#[derive(Debug, Clone)]
pub enum RawFailureHandler {
    Terminate { outcome: String },
    Compensate {
        steps: Vec<RawCompStep>,
        then: String,
    },
}

#[derive(Debug, Clone)]
pub struct RawCompStep {
    pub op: String,
    pub persona: String,
    pub on_failure: String,
}

#[derive(Debug, Clone)]
pub struct RawBranch {
    pub id: String,
    pub entry: String,
    pub steps: BTreeMap<String, RawStep>,
}

#[derive(Debug, Clone)]
pub struct RawJoinPolicy {
    pub on_all_success: Option<RawStepTarget>,
    pub on_any_failure: Option<RawFailureHandler>,
    pub on_all_complete: Option<RawStepTarget>,
}

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
        Parser { tokens, pos: 0, filename: filename.to_owned() }
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
            Token::Int(n) => { self.advance(); Ok(n) }
            Token::Word(w) if w.parse::<i64>().is_ok() => {
                let n = w.parse().unwrap();
                self.advance();
                Ok(n)
            }
            _ => Err(self.err(format!("expected integer, got {:?}", self.peek()))),
        }
    }

    // ── Type parsing ───────────────────────────────

    fn parse_type(&mut self) -> Result<RawType, ElabError> {
        let name = self.take_word()?;
        match name.as_str() {
            "Bool" => Ok(RawType::Bool),
            "Date" => Ok(RawType::Date),
            "DateTime" => Ok(RawType::DateTime),
            "Int" => {
                if self.peek() == &Token::LParen {
                    self.advance(); // consume (
                    let (min, max) = self.parse_int_params()?;
                    self.expect_rparen()?;
                    Ok(RawType::Int { min, max })
                } else {
                    Ok(RawType::Int { min: i64::MIN, max: i64::MAX })
                }
            }
            "Decimal" => {
                self.advance_lparen()?;
                let precision = self.parse_named_or_positional_u32("precision")? as u32;
                if self.peek() == &Token::Comma { self.advance(); }
                let scale = self.parse_named_or_positional_u32("scale")? as u32;
                self.expect_rparen()?;
                Ok(RawType::Decimal { precision, scale })
            }
            "Text" => {
                if self.peek() == &Token::LParen {
                    self.advance(); // consume (
                    let max_length = self.parse_named_or_positional_u32("max_length")? as u32;
                    self.expect_rparen()?;
                    Ok(RawType::Text { max_length })
                } else {
                    Ok(RawType::Text { max_length: 0 }) // inferred from literal value in elaboration
                }
            }
            "Money" => {
                self.advance_lparen()?;
                // currency: "USD"
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
                // parse named params in any order
                while self.peek() != &Token::RParen {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "unit" => { unit = self.take_str()?; }
                        "min" => { min = self.take_int()?; }
                        "max" => { max = self.take_int()?; }
                        _ => return Err(self.err(format!("unknown Duration param '{}'", key))),
                    }
                    if self.peek() == &Token::Comma { self.advance(); }
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
                        "element_type" => { element_type = Some(self.parse_type()?); }
                        "max" => { max = self.take_int()? as u32; }
                        _ => return Err(self.err(format!("unknown List param '{}'", key))),
                    }
                    if self.peek() == &Token::Comma { self.advance(); }
                }
                self.expect_rparen()?;
                let et = element_type.ok_or_else(|| self.err("List missing element_type"))?;
                Ok(RawType::List { element_type: Box::new(et), max })
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
            // Named type reference
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

    /// Parse named param `key: value` or just `value` (positional)
    fn parse_named_or_positional_u32(&mut self, key: &str) -> Result<i64, ElabError> {
        if self.is_word(key) {
            self.advance();
            self.expect_colon()?;
        }
        self.take_int()
    }

    fn parse_int_params(&mut self) -> Result<(i64, i64), ElabError> {
        // Int(min: 0, max: 1000) or Int(0, 1000)
        let first_is_key = self.is_word("min");
        if first_is_key {
            self.advance();
            self.expect_colon()?;
        }
        let min = self.take_int()?;
        if self.peek() == &Token::Comma { self.advance(); }
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
            if self.peek() == &Token::Comma { self.advance(); }
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
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect_rbrace()?;
        Ok(fields)
    }

    // ── Literal parsing ────────────────────────────

    fn parse_literal(&mut self) -> Result<RawLiteral, ElabError> {
        match self.peek().clone() {
            Token::Word(w) if w == "true" => { self.advance(); Ok(RawLiteral::Bool(true)) }
            Token::Word(w) if w == "false" => { self.advance(); Ok(RawLiteral::Bool(false)) }
            Token::Int(n) => { self.advance(); Ok(RawLiteral::Int(n)) }
            Token::Float(f) => { let s = f.clone(); self.advance(); Ok(RawLiteral::Float(s)) }
            Token::Str(s) => { let v = s.clone(); self.advance(); Ok(RawLiteral::Str(v)) }
            Token::Word(w) if w == "Money" => {
                self.advance();
                self.expect_lbrace()?;
                let mut amount = String::new();
                let mut currency = String::new();
                while self.peek() != &Token::RBrace {
                    let key = self.take_word()?;
                    self.expect_colon()?;
                    match key.as_str() {
                        "amount" => { amount = self.take_str()?; }
                        "currency" => { currency = self.take_str()?; }
                        _ => return Err(self.err(format!("unknown Money key '{}'", key))),
                    }
                    if self.peek() == &Token::Comma { self.advance(); }
                }
                self.expect_rbrace()?;
                Ok(RawLiteral::Money { amount, currency })
            }
            _ => Err(self.err(format!("expected literal value, got {:?}", self.peek()))),
        }
    }

    // ── Expression parsing ──────────────────────────

    /// Parse a predicate expression (top-level: handles ∧ and ∨)
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
        // ∀ var ∈ domain . body
        if self.peek() == &Token::Forall {
            let line = self.cur_line();
            self.advance();
            let var = self.take_word()?;
            if self.peek() != &Token::In {
                return Err(self.err("expected ∈ after quantifier variable"));
            }
            self.advance(); // consume ∈
            let domain = self.take_word()?;
            // consume the ". " separator
            if self.peek() == &Token::Dot {
                self.advance();
            } else {
                return Err(self.err("expected '.' after quantifier domain"));
            }
            let body = self.parse_expr()?;
            return Ok(RawExpr::Forall { var, domain, body: Box::new(body), line });
        }

        // verdict_present(id)
        if self.is_word("verdict_present") {
            let line = self.cur_line();
            self.advance();
            self.advance_lparen()?;
            let id = self.take_word()?;
            self.expect_rparen()?;
            return Ok(RawExpr::VerdictPresent { id, line });
        }

        // Parenthesized expression
        if self.peek() == &Token::LParen {
            self.advance();
            let e = self.parse_expr()?;
            self.expect_rparen()?;
            return Ok(e);
        }

        // Comparison: term op term
        let line = self.cur_line();
        let left = self.parse_term()?;
        let op = self.parse_compare_op()?;
        let right = self.parse_term()?;
        Ok(RawExpr::Compare { op, left, right, line })
    }

    fn parse_compare_op(&mut self) -> Result<String, ElabError> {
        let op = match self.peek() {
            Token::Eq => "=",
            Token::Neq => "!=",
            Token::Lt => "<",
            Token::Lte => "<=",
            Token::Gt => ">",
            Token::Gte => ">=",
            _ => return Err(self.err(format!("expected comparison operator, got {:?}", self.peek()))),
        };
        self.advance();
        Ok(op.to_owned())
    }

    /// Parse a single base term (no multiplication).
    fn parse_base_term(&mut self) -> Result<RawTerm, ElabError> {
        match self.peek().clone() {
            Token::Word(w) if w == "true" => { self.advance(); Ok(RawTerm::Literal(RawLiteral::Bool(true))) }
            Token::Word(w) if w == "false" => { self.advance(); Ok(RawTerm::Literal(RawLiteral::Bool(false))) }
            Token::Int(n) => { self.advance(); Ok(RawTerm::Literal(RawLiteral::Int(n))) }
            Token::Float(f) => { let s = f.clone(); self.advance(); Ok(RawTerm::Literal(RawLiteral::Float(s))) }
            Token::Str(s) => { let v = s.clone(); self.advance(); Ok(RawTerm::Literal(RawLiteral::Str(v))) }
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

    /// Parse a term, handling optional `*` multiplication.
    fn parse_term(&mut self) -> Result<RawTerm, ElabError> {
        let left = self.parse_base_term()?;
        if self.peek() == &Token::Star {
            self.advance();
            let right = self.parse_base_term()?;
            return Ok(RawTerm::Mul { left: Box::new(left), right: Box::new(right) });
        }
        Ok(left)
    }

    // ── Top-level construct parsers ─────────────────

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
                _ => Err(self.err(format!("unexpected token '{}'", w))),
            },
            other => Err(self.err(format!("expected construct keyword, got {:?}", other))),
        }
    }

    fn parse_import(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'import'
        let path = self.take_str()?;
        Ok(RawConstruct::Import {
            path,
            prov: Provenance { file: self.filename.clone(), line },
        })
    }

    fn parse_typedecl(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'type'
        let id = self.take_word()?;
        let fields = self.parse_typedecl_body()?;
        Ok(RawConstruct::TypeDecl {
            id,
            fields,
            prov: Provenance { file: self.filename.clone(), line },
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
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect_rbrace()?;
        Ok(fields)
    }

    fn parse_fact(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'fact'
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut type_ = None;
        let mut source = None;
        let mut default = None;
        while self.peek() != &Token::RBrace {
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "type" => { type_ = Some(self.parse_type()?); }
                "source" => { source = Some(self.take_str()?); }
                "default" => { default = Some(self.parse_literal()?); }
                _ => return Err(self.err(format!("unknown Fact field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Fact {
            id,
            type_: type_.ok_or_else(|| self.err("Fact missing 'type'"))?,
            source: source.ok_or_else(|| self.err("Fact missing 'source'"))?,
            default,
            prov: Provenance { file: self.filename.clone(), line },
        })
    }

    fn parse_entity(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'entity'
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
                "states" => { states = self.parse_ident_array()?; }
                "initial" => { initial_line = field_line; initial = self.take_word()?; }
                "transitions" => { transitions = self.parse_transitions()?; }
                "parent" => { parent_line = Some(field_line); parent = Some(self.take_word()?); }
                _ => return Err(self.err(format!("unknown Entity field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Entity {
            id, states, initial, initial_line, transitions, parent, parent_line,
            prov: Provenance { file: self.filename.clone(), line },
        })
    }

    fn parse_ident_array(&mut self) -> Result<Vec<String>, ElabError> {
        self.expect_lbracket()?;
        let mut items = Vec::new();
        while self.peek() != &Token::RBracket {
            items.push(self.take_word()?);
            if self.peek() == &Token::Comma { self.advance(); }
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
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect_rbracket()?;
        Ok(transitions)
    }

    /// Accept either ',' or '→'/'->' (both lex as Gt since '-' is silently consumed)
    fn expect_transition_sep(&mut self) -> Result<(), ElabError> {
        match self.peek() {
            Token::Comma | Token::Gt => { self.advance(); Ok(()) }
            _ => Err(self.err(format!("expected ',' or '->'/'→', got {:?}", self.peek())))
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
        self.advance(); // consume 'rule'
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
                "stratum" => { stratum_line = field_line; stratum = Some(self.take_int()?); }
                "when" => { when = Some(self.parse_expr()?); }
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
            prov: Provenance { file: self.filename.clone(), line },
        })
    }

    /// Parse `verdict <id> { payload: <type> = <value_or_expr> }`
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
        self.advance(); // consume 'operation'
        let id = self.take_word()?;
        self.expect_lbrace()?;
        let mut personas = Vec::new();
        let mut allowed_personas_line = line;
        let mut precondition = None;
        let mut effects = Vec::new();
        let mut error_contract = Vec::new();
        while self.peek() != &Token::RBrace {
            let field_line = self.cur_line();
            let key = self.take_word()?;
            self.expect_colon()?;
            match key.as_str() {
                "allowed_personas" => {
                    allowed_personas_line = field_line;
                    personas = self.parse_ident_array()?;
                }
                "precondition" => { precondition = Some(self.parse_expr()?); }
                "effects" => { effects = self.parse_effects()?; }
                "error_contract" => { error_contract = self.parse_ident_array()?; }
                _ => return Err(self.err(format!("unknown Operation field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Operation {
            id, allowed_personas: personas, allowed_personas_line,
            precondition: precondition.ok_or_else(|| self.err("Operation missing 'precondition'"))?,
            effects, error_contract,
            prov: Provenance { file: self.filename.clone(), line },
        })
    }

    fn parse_effects(&mut self) -> Result<Vec<(String, String, String, u32)>, ElabError> {
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
            self.expect_rparen()?;
            effects.push((entity, from, to, e_line));
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect_rbracket()?;
        Ok(effects)
    }

    fn parse_flow(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
        self.advance(); // consume 'flow'
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
                "snapshot" => { snapshot = self.take_word()?; }
                "entry" => { entry_line = field_line; entry = self.take_word()?; }
                "steps" => { steps = self.parse_steps()?; }
                _ => return Err(self.err(format!("unknown Flow field '{}'", key))),
            }
        }
        self.expect_rbrace()?;
        Ok(RawConstruct::Flow {
            id, snapshot, entry, entry_line, steps,
            prov: Provenance { file: self.filename.clone(), line },
        })
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
                        "op" => { op = self.take_word()?; }
                        "persona" => { persona = self.take_word()?; }
                        "outcomes" => { outcomes = self.parse_outcomes()?; }
                        "on_failure" => { on_failure = Some(self.parse_failure_handler()?); }
                        _ => return Err(self.err(format!("unknown OperationStep field '{}'", key))),
                    }
                }
                // on_failure absence is a Pass 5 error, not a parse error
                RawStep::OperationStep { op, persona, outcomes, on_failure, line: step_line }
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
                        "condition" => { condition = Some(self.parse_expr()?); }
                        "persona" => { persona = self.take_word()?; }
                        "if_true" => { if_true = Some(self.parse_step_target()?); }
                        "if_false" => { if_false = Some(self.parse_step_target()?); }
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
                        "from_persona" => { from_persona = self.take_word()?; }
                        "to_persona" => { to_persona = self.take_word()?; }
                        "next" => { next = self.take_word()?; }
                        _ => return Err(self.err(format!("unknown HandoffStep field '{}'", key))),
                    }
                }
                RawStep::HandoffStep { from_persona, to_persona, next, line: step_line }
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
                        "flow"       => { flow_line = field_line; flow = self.take_word()?; }
                        "persona"    => { persona = self.take_word()?; }
                        "on_success" => { on_success = Some(self.parse_step_target()?); }
                        "on_failure" => { on_failure = Some(self.parse_failure_handler()?); }
                        _ => return Err(self.err(format!("unknown SubFlowStep field '{}'", key))),
                    }
                }
                RawStep::SubFlowStep {
                    flow,
                    flow_line,
                    persona,
                    on_success: on_success.ok_or_else(|| self.err("SubFlowStep missing on_success"))?,
                    on_failure: on_failure.ok_or_else(|| self.err("SubFlowStep missing on_failure"))?,
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
                        "branches" => { branches_line = field_line; branches = self.parse_branches()?; }
                        "join"     => { join = Some(self.parse_join_policy()?); }
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
        // Terminal(success) or Terminal(failure) or step_id
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
                        "steps" => { comp_steps = self.parse_comp_steps()?; }
                        "then" => {
                            // Terminal(failure)
                            self.expect_word("Terminal")?;
                            self.advance_lparen()?;
                            then_outcome = self.take_word()?;
                            self.expect_rparen()?;
                        }
                        _ => return Err(self.err(format!("unknown Compensate field '{}'", key))),
                    }
                }
                self.expect_rparen()?;
                Ok(RawFailureHandler::Compensate { steps: comp_steps, then: then_outcome })
            }
            _ => Err(self.err(format!("unknown failure handler kind '{}'", kind))),
        }
    }

    fn parse_branches(&mut self) -> Result<Vec<RawBranch>, ElabError> {
        self.expect_lbracket()?;
        let mut branches = Vec::new();
        while self.peek() != &Token::RBracket {
            branches.push(self.parse_branch()?);
            if self.peek() == &Token::Comma { self.advance(); }
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
                "id"    => { id = self.take_word()?; }
                "entry" => { entry = self.take_word()?; }
                "steps" => { steps = self.parse_steps()?; }
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
                "on_all_success" => { on_all_success = Some(self.parse_step_target()?); }
                "on_any_failure" => { on_any_failure = Some(self.parse_failure_handler()?); }
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
        Ok(RawJoinPolicy { on_all_success, on_any_failure, on_all_complete })
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
                    "op" => { op = self.take_word()?; }
                    "persona" => { persona = self.take_word()?; }
                    "on_failure" => {
                        // Terminal(failure) or Terminal(outcome)
                        self.expect_word("Terminal")?;
                        self.advance_lparen()?;
                        on_failure = self.take_word()?;
                        self.expect_rparen()?;
                    }
                    _ => return Err(self.err(format!("unknown comp step field '{}'", key))),
                }
            }
            self.expect_rbrace()?;
            steps.push(RawCompStep { op, persona, on_failure });
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect_rbracket()?;
        Ok(steps)
    }
}

pub fn parse(tokens: &[Spanned], filename: &str) -> Result<Vec<RawConstruct>, ElabError> {
    let mut p = Parser::new(tokens, filename);
    p.parse_file()
}
