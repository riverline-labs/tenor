use super::Parser;
use crate::ast::{Provenance, RawConstruct, RawTrigger};
use crate::error::ElabError;
use crate::lexer::Token;

impl<'a> Parser<'a> {
    pub(super) fn parse_system(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
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
}
