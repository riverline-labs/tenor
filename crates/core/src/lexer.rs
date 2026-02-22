use crate::error::ElabError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Identifiers and keywords — distinguished in the parser
    Word(String),
    /// Quoted string literal (content without quotes, escapes resolved)
    Str(String),
    /// Integer literal
    Int(i64),
    /// Decimal literal — kept as string to preserve exact representation
    Float(String),
    // Punctuation
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Comma,
    Dot,
    // Comparison operators
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    // Arithmetic operators
    Star, // *
    // Logical operators (Unicode)
    And,    // U+2227
    Or,     // U+2228
    Not,    // U+00AC
    Forall, // U+2200
    Exists, // U+2203
    In,     // U+2208
    // End of input
    Eof,
}

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line: u32,
}

pub fn lex(src: &str, filename: &str) -> Result<Vec<Spanned>, ElabError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut pos = 0usize;
    let mut line: u32 = 1;

    while pos < chars.len() {
        let c = chars[pos];

        // Line comment
        if c == '/' && pos + 1 < chars.len() && chars[pos + 1] == '/' {
            while pos < chars.len() && chars[pos] != '\n' {
                pos += 1;
            }
            continue;
        }

        // Block comment
        if c == '/' && pos + 1 < chars.len() && chars[pos + 1] == '*' {
            pos += 2;
            loop {
                if pos >= chars.len() {
                    return Err(ElabError::lex(filename, line, "unterminated block comment"));
                }
                if chars[pos] == '\n' {
                    line += 1;
                }
                if chars[pos] == '*' && pos + 1 < chars.len() && chars[pos + 1] == '/' {
                    pos += 2;
                    break;
                }
                pos += 1;
            }
            continue;
        }

        // Whitespace
        if c.is_whitespace() {
            if c == '\n' {
                line += 1;
            }
            pos += 1;
            continue;
        }

        let tok_line = line;

        // String literal
        if c == '"' {
            pos += 1;
            let mut s = String::new();
            loop {
                if pos >= chars.len() {
                    return Err(ElabError::lex(
                        filename,
                        tok_line,
                        "unterminated string literal",
                    ));
                }
                let sc = chars[pos];
                if sc == '"' {
                    pos += 1;
                    break;
                }
                if sc == '\\' {
                    pos += 1;
                    if pos >= chars.len() {
                        return Err(ElabError::lex(
                            filename,
                            tok_line,
                            "unterminated escape in string",
                        ));
                    }
                    match chars[pos] {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        other => {
                            s.push('\\');
                            s.push(other);
                        }
                    }
                    pos += 1;
                    continue;
                }
                if sc == '\n' {
                    return Err(ElabError::lex(
                        filename,
                        tok_line,
                        "unterminated string literal",
                    ));
                }
                s.push(sc);
                pos += 1;
            }
            tokens.push(Spanned {
                token: Token::Str(s),
                line: tok_line,
            });
            continue;
        }

        // Number
        if c.is_ascii_digit()
            || (c == '-' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit())
        {
            let start = pos;
            if c == '-' {
                pos += 1;
            }
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos < chars.len()
                && chars[pos] == '.'
                && pos + 1 < chars.len()
                && chars[pos + 1].is_ascii_digit()
            {
                pos += 1; // consume '.'
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                let s: String = chars[start..pos].iter().collect();
                tokens.push(Spanned {
                    token: Token::Float(s),
                    line: tok_line,
                });
            } else {
                let s: String = chars[start..pos].iter().collect();
                let n: i64 = s.parse().map_err(|_| {
                    ElabError::lex(filename, tok_line, format!("invalid integer '{}'", s))
                })?;
                tokens.push(Spanned {
                    token: Token::Int(n),
                    line: tok_line,
                });
            }
            continue;
        }

        // Operators
        match c {
            '=' => {
                tokens.push(Spanned {
                    token: Token::Eq,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '<' => {
                if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                    tokens.push(Spanned {
                        token: Token::Lte,
                        line: tok_line,
                    });
                    pos += 2;
                } else {
                    tokens.push(Spanned {
                        token: Token::Lt,
                        line: tok_line,
                    });
                    pos += 1;
                }
                continue;
            }
            '>' => {
                if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                    tokens.push(Spanned {
                        token: Token::Gte,
                        line: tok_line,
                    });
                    pos += 2;
                } else {
                    tokens.push(Spanned {
                        token: Token::Gt,
                        line: tok_line,
                    });
                    pos += 1;
                }
                continue;
            }
            '!' => {
                if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                    tokens.push(Spanned {
                        token: Token::Neq,
                        line: tok_line,
                    });
                    pos += 2;
                } else {
                    return Err(ElabError::lex(
                        filename,
                        tok_line,
                        format!("unexpected character '{}'", c),
                    ));
                }
                continue;
            }
            '*' => {
                tokens.push(Spanned {
                    token: Token::Star,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '{' => {
                tokens.push(Spanned {
                    token: Token::LBrace,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '}' => {
                tokens.push(Spanned {
                    token: Token::RBrace,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '[' => {
                tokens.push(Spanned {
                    token: Token::LBracket,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            ']' => {
                tokens.push(Spanned {
                    token: Token::RBracket,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '(' => {
                tokens.push(Spanned {
                    token: Token::LParen,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            ')' => {
                tokens.push(Spanned {
                    token: Token::RParen,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            ':' => {
                tokens.push(Spanned {
                    token: Token::Colon,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            ',' => {
                tokens.push(Spanned {
                    token: Token::Comma,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '.' => {
                tokens.push(Spanned {
                    token: Token::Dot,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            _ => {}
        }

        // Unicode operators (multi-byte chars)
        match c {
            '\u{2200}' => {
                tokens.push(Spanned {
                    token: Token::Forall,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '\u{2203}' => {
                tokens.push(Spanned {
                    token: Token::Exists,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '\u{2208}' => {
                tokens.push(Spanned {
                    token: Token::In,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '\u{2227}' => {
                tokens.push(Spanned {
                    token: Token::And,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '\u{2228}' => {
                tokens.push(Spanned {
                    token: Token::Or,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            '\u{00AC}' => {
                tokens.push(Spanned {
                    token: Token::Not,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            // U+2192 RIGHTWARDS ARROW — same token as ASCII "->" (where '-' is silently skipped)
            '\u{2192}' => {
                tokens.push(Spanned {
                    token: Token::Gt,
                    line: tok_line,
                });
                pos += 1;
                continue;
            }
            _ => {}
        }

        // Identifier / keyword
        if c.is_alphabetic() || c == '_' {
            let start = pos;
            while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                pos += 1;
            }
            let word: String = chars[start..pos].iter().collect();
            tokens.push(Spanned {
                token: Token::Word(word),
                line: tok_line,
            });
            continue;
        }

        return Err(ElabError::lex(
            filename,
            tok_line,
            format!("unexpected character '{}'", c),
        ));
    }

    tokens.push(Spanned {
        token: Token::Eof,
        line,
    });
    Ok(tokens)
}
