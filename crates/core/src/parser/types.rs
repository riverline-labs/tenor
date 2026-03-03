use super::Parser;
use crate::ast::RawType;
use crate::error::ElabError;
use crate::lexer::Token;
use std::collections::BTreeMap;

impl<'a> Parser<'a> {
    // -- Type parsing -------------------------------------------

    pub(super) fn parse_type(&mut self) -> Result<RawType, ElabError> {
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
            "TaggedUnion" => {
                let variants = self.parse_record_fields()?;
                Ok(RawType::TaggedUnion { variants })
            }
            other => Ok(RawType::TypeRef(other.to_owned())),
        }
    }

    pub(super) fn advance_lparen(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::LParen {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '(', got {:?}", self.peek())))
        }
    }

    pub(super) fn expect_rparen(&mut self) -> Result<(), ElabError> {
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

    pub(super) fn parse_string_array(&mut self) -> Result<Vec<String>, ElabError> {
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

    pub(super) fn expect_lbracket(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::LBracket {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected '[', got {:?}", self.peek())))
        }
    }

    pub(super) fn expect_rbracket(&mut self) -> Result<(), ElabError> {
        if self.peek() == &Token::RBracket {
            self.advance();
            Ok(())
        } else {
            Err(self.err(format!("expected ']', got {:?}", self.peek())))
        }
    }

    pub(super) fn parse_record_fields(&mut self) -> Result<BTreeMap<String, RawType>, ElabError> {
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
}
