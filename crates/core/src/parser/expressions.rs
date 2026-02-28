use super::Parser;
use crate::ast::{RawExpr, RawLiteral, RawTerm};
use crate::error::ElabError;
use crate::lexer::Token;

impl<'a> Parser<'a> {
    // -- Literal parsing ----------------------------------------

    pub(super) fn parse_literal(&mut self) -> Result<RawLiteral, ElabError> {
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

    pub(super) fn parse_term(&mut self) -> Result<RawTerm, ElabError> {
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
}
