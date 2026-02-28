use super::Parser;
use crate::ast::{Provenance, RawConstruct, RawLiteral, RawSourceDecl, RawTerm, RawType};
use crate::error::ElabError;
use crate::lexer::Token;
use std::collections::BTreeMap;

impl<'a> Parser<'a> {
    pub(super) fn parse_typedecl(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_fact(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_source(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_entity(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_ident_array(&mut self) -> Result<Vec<String>, ElabError> {
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

    pub(super) fn parse_rule(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_operation(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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

    pub(super) fn parse_persona(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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
}
