//! Lexer for Turtle 1.1 / `TriG`.
//!
//! Produces a flat stream of [`Token`] values from a UTF-8 byte slice.
//! The lexer handles:
//! - IRIs: `<…>`
//! - Prefixed names: `prefix:local`
//! - Blank-node labels: `_:label`
//! - Anonymous blank nodes: `[]` (returns `[` and `]` as separate tokens)
//! - String literals: `"…"`, `'…'`, `"""…"""`, `'''…'''`
//! - Numeric literals: integers, decimals, doubles
//! - Booleans: `true`, `false`
//! - Directives: `@prefix`, `@base`
//! - SPARQL-style: `PREFIX`, `BASE`
//! - Punctuation: `.`, `,`, `;`, `(`, `)`, `[`, `]`, `{`, `}`
//! - Comments: `#` through end of line (skipped)
//! - Whitespace (skipped)

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The kind of token.
    pub kind: TokenKind,
    /// Byte offset of the first byte of this token in the source.
    pub offset: usize,
}

/// The variety of a [`Token`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// `<iri>` — raw content between angle brackets (not yet decoded).
    IriRef(String),
    /// `prefix:local` — prefix and local parts already split.
    PrefixedName { prefix: String, local: String },
    /// `_:label` — blank-node label.
    BNodeLabel(String),
    /// `[` — anonymous blank node open bracket.
    BracketOpen,
    /// `]`.
    BracketClose,
    /// `(`.
    ParenOpen,
    /// `)`.
    ParenClose,
    /// `{`.
    BraceOpen,
    /// `}`.
    BraceClose,
    /// `"…"` or `'…'` or `"""…"""` or `'''…'''`.
    /// The raw lexical content (still with escape sequences intact) is stored.
    StringLiteral { raw: String, long_form: bool },
    /// Integer literal (digits, possibly with leading sign).
    IntegerLiteral(String),
    /// Decimal literal (contains `.` but no `e`/`E`).
    DecimalLiteral(String),
    /// Double literal (contains `e` or `E`).
    DoubleLiteral(String),
    /// `true` or `false`.
    BooleanLiteral(bool),
    /// `@prefix`.
    AtPrefix,
    /// `@base`.
    AtBase,
    /// `PREFIX` (SPARQL-style, case-insensitive).
    SparqlPrefix,
    /// `BASE` (SPARQL-style, case-insensitive).
    SparqlBase,
    /// `^^` — datatype tag.
    DataTypeTag,
    /// `@lang` — language tag (the `@` is consumed; `lang` is the tag text).
    LangTag(String),
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `a` keyword (shorthand for `rdf:type`).
    AKeyword,
}

/// Lex errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// Human-readable description.
    pub message: String,
    /// Byte offset.
    pub offset: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at byte {}: {}", self.offset, self.message)
    }
}

/// Lex the entire `src` into a vector of tokens or return the first error.
///
/// # Errors
///
/// Returns a [`LexError`] on encountering invalid syntax.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        lexer.skip_ws_and_comments();
        if lexer.is_at_end() {
            break;
        }
        let tok = lexer.next_token()?;
        tokens.push(tok);
    }
    Ok(tokens)
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        // Skip BOM if present
        let pos = if src.starts_with('\u{FEFF}') { 3 } else { 0 };
        Self { src, pos }
    }

    const fn is_at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.src[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    const fn advance_n_bytes(&mut self, n: usize) {
        self.pos += n;
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            let before = self.pos;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Skip comment
            if self.peek() == Some('#') {
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            if self.pos == before {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let offset = self.pos;
        let ch = self.peek().ok_or_else(|| LexError { message: "unexpected end".into(), offset })?;

        match ch {
            '<' => self.lex_iri_ref(offset),
            '"' | '\'' => self.lex_string(offset),
            '_' => {
                if self.src[self.pos..].starts_with("_:") {
                    self.lex_bnode_label(offset)
                } else {
                    Err(LexError { message: format!("unexpected '_' at byte {offset}"), offset })
                }
            }
            ':' => {
                // Bare colon: empty prefix name  `:local` or just `:`
                self.advance(); // consume ':'
                let local = self.lex_pn_local();
                Ok(Token {
                    kind: TokenKind::PrefixedName { prefix: String::new(), local },
                    offset,
                })
            }
            '@' => self.lex_at_directive(offset),
            '^' => {
                if self.src[self.pos..].starts_with("^^") {
                    self.advance_n_bytes(2);
                    Ok(Token { kind: TokenKind::DataTypeTag, offset })
                } else {
                    Err(LexError { message: "expected '^^'".into(), offset })
                }
            }
            '.' => {
                // Could be start of decimal if followed by digit
                if self.src[self.pos + 1..]
                    .starts_with(|c: char| c.is_ascii_digit())
                {
                    Ok(self.lex_number(offset))
                } else {
                    self.advance();
                    Ok(Token { kind: TokenKind::Dot, offset })
                }
            }
            ',' => {
                self.advance();
                Ok(Token { kind: TokenKind::Comma, offset })
            }
            ';' => {
                self.advance();
                Ok(Token { kind: TokenKind::Semicolon, offset })
            }
            '(' => {
                self.advance();
                Ok(Token { kind: TokenKind::ParenOpen, offset })
            }
            ')' => {
                self.advance();
                Ok(Token { kind: TokenKind::ParenClose, offset })
            }
            '[' => {
                self.advance();
                Ok(Token { kind: TokenKind::BracketOpen, offset })
            }
            ']' => {
                self.advance();
                Ok(Token { kind: TokenKind::BracketClose, offset })
            }
            '{' => {
                self.advance();
                Ok(Token { kind: TokenKind::BraceOpen, offset })
            }
            '}' => {
                self.advance();
                Ok(Token { kind: TokenKind::BraceClose, offset })
            }
            '+' | '-' | '0'..='9' => Ok(self.lex_number(offset)),
            _ => self.lex_word(offset),
        }
    }

    fn lex_iri_ref(&mut self, offset: usize) -> Result<Token, LexError> {
        self.advance(); // consume '<'
        let start = self.pos;
        loop {
            match self.peek() {
                None => {
                    return Err(LexError { message: "unterminated IRI reference".into(), offset });
                }
                Some('>') => {
                    let raw = self.src[start..self.pos].to_owned();
                    self.advance();
                    return Ok(Token { kind: TokenKind::IriRef(raw), offset });
                }
                Some('\\') => {
                    // Allow escape inside IRI (will be decoded later)
                    self.advance();
                    self.advance(); // skip next char
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
    }

    fn lex_string(&mut self, offset: usize) -> Result<Token, LexError> {
        let quote = self.peek().unwrap();
        let long_form =
            self.src[self.pos..].starts_with(if quote == '"' { r#"""""# } else { "'''" });
        if long_form {
            self.advance_n_bytes(3);
            let delim = if quote == '"' { "\"\"\"" } else { "'''" };
            let start = self.pos;
            loop {
                if self.src[self.pos..].starts_with(delim) {
                    let raw = self.src[start..self.pos].to_owned();
                    self.advance_n_bytes(3);
                    return Ok(Token {
                        kind: TokenKind::StringLiteral { raw, long_form: true },
                        offset,
                    });
                }
                if self.is_at_end() {
                    return Err(LexError {
                        message: "unterminated long string literal".into(),
                        offset,
                    });
                }
                if self.peek() == Some('\\') {
                    self.advance();
                }
                self.advance();
            }
        } else {
            self.advance(); // consume opening quote
            let mut raw = String::new();
            loop {
                match self.peek() {
                    None | Some('\n' | '\r') => {
                        return Err(LexError {
                            message: "unterminated string literal".into(),
                            offset,
                        });
                    }
                    Some('\\') => {
                        self.advance();
                        let escaped = self.advance().ok_or_else(|| LexError {
                            message: "trailing backslash".into(),
                            offset,
                        })?;
                        raw.push('\\');
                        raw.push(escaped);
                    }
                    Some(c) if c == quote => {
                        self.advance();
                        return Ok(Token {
                            kind: TokenKind::StringLiteral { raw, long_form: false },
                            offset,
                        });
                    }
                    Some(c) => {
                        self.advance();
                        raw.push(c);
                    }
                }
            }
        }
    }

    fn lex_bnode_label(&mut self, offset: usize) -> Result<Token, LexError> {
        self.advance_n_bytes(2); // skip "_:"
        let start = self.pos;
        // First char must be `PN_CHARS_U` or digit
        match self.peek() {
            Some(c) if is_pn_chars_u(c) || c.is_ascii_digit() => {
                self.advance();
            }
            _ => {
                return Err(LexError { message: "invalid blank node label".into(), offset });
            }
        }
        // Subsequent chars: `PN_CHARS | '.'`
        loop {
            match self.peek() {
                Some(c) if is_pn_chars(c) => {
                    self.advance();
                }
                Some('.') => {
                    // '.' allowed in middle but not at end
                    if self.src[self.pos + 1..].starts_with(|c: char| is_pn_chars(c)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        let label = self.src[start..self.pos].to_owned();
        Ok(Token { kind: TokenKind::BNodeLabel(label), offset })
    }

    fn lex_at_directive(&mut self, offset: usize) -> Result<Token, LexError> {
        self.advance(); // consume '@'
        let word: String = self.src[self.pos..]
            .chars()
            .take_while(|&c| c.is_ascii_alphabetic() || c == '-')
            .collect();
        if word.is_empty() {
            return Err(LexError { message: "expected keyword after '@'".into(), offset });
        }
        self.advance_n_bytes(word.len());
        match word.as_str() {
            "prefix" => Ok(Token { kind: TokenKind::AtPrefix, offset }),
            "base" => Ok(Token { kind: TokenKind::AtBase, offset }),
            _ => Ok(Token { kind: TokenKind::LangTag(word), offset }),
        }
    }

    fn lex_number(&mut self, offset: usize) -> Token {
        let start = self.pos;
        // Optional sign
        if matches!(self.peek(), Some('+' | '-')) {
            self.advance();
        }
        // Integer part
        while matches!(self.peek(), Some('0'..='9')) {
            self.advance();
        }

        if self.peek() == Some('.')
            && matches!(
                self.src[self.pos + 1..].chars().next(),
                Some('0'..='9')
            )
        {
            self.advance(); // consume '.'
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
            // Check for exponent
            if matches!(self.peek(), Some('e' | 'E')) {
                self.advance();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.advance();
                }
                while matches!(self.peek(), Some('0'..='9')) {
                    self.advance();
                }
                let raw = self.src[start..self.pos].to_owned();
                return Token { kind: TokenKind::DoubleLiteral(raw), offset };
            }
            let raw = self.src[start..self.pos].to_owned();
            return Token { kind: TokenKind::DecimalLiteral(raw), offset };
        }

        // Exponent (no decimal point)
        if matches!(self.peek(), Some('e' | 'E')) {
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
            let raw = self.src[start..self.pos].to_owned();
            return Token { kind: TokenKind::DoubleLiteral(raw), offset };
        }

        let raw = self.src[start..self.pos].to_owned();
        Token { kind: TokenKind::IntegerLiteral(raw), offset }
    }

    fn lex_word(&mut self, offset: usize) -> Result<Token, LexError> {
        let start = self.pos;
        let first = self.peek().ok_or_else(|| LexError { message: "unexpected end".into(), offset })?;
        if !is_pn_chars_base(first) && first != '_' {
            return Err(LexError {
                message: format!(
                    "unexpected character '{}' (U+{:04X}) at byte {offset}",
                    first, first as u32
                ),
                offset,
            });
        }
        self.advance();

        loop {
            match self.peek() {
                Some(':') => {
                    let prefix = self.src[start..self.pos].to_owned();
                    self.advance(); // consume ':'
                    let local = self.lex_pn_local();
                    return Ok(Token {
                        kind: TokenKind::PrefixedName { prefix, local },
                        offset,
                    });
                }
                Some(c) if is_pn_chars(c) => {
                    self.advance();
                }
                Some('.') => {
                    let after_dot = self.src[self.pos + 1..].chars().next();
                    if matches!(after_dot, Some(nc) if is_pn_chars(nc) || nc == ':') {
                        self.advance();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        let word = &self.src[start..self.pos];
        match word {
            "a" => Ok(Token { kind: TokenKind::AKeyword, offset }),
            "true" => Ok(Token { kind: TokenKind::BooleanLiteral(true), offset }),
            "false" => Ok(Token { kind: TokenKind::BooleanLiteral(false), offset }),
            "PREFIX" | "prefix" => Ok(Token { kind: TokenKind::SparqlPrefix, offset }),
            "BASE" | "base" => Ok(Token { kind: TokenKind::SparqlBase, offset }),
            other => Ok(Token {
                kind: TokenKind::PrefixedName {
                    prefix: other.to_owned(),
                    local: String::new(),
                },
                offset,
            }),
        }
    }

    /// Lex a `PN_LOCAL` — the local part of a prefixed name after the colon.
    fn lex_pn_local(&mut self) -> String {
        let start = self.pos;
        loop {
            match self.peek() {
                Some('%') => {
                    self.advance();
                    for _ in 0..2 {
                        if matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
                            self.advance();
                        }
                    }
                }
                Some('\\') => {
                    self.advance();
                    self.advance(); // consume the escaped char
                }
                Some(':') => {
                    self.advance();
                }
                Some(c) if is_pn_chars(c) => {
                    self.advance();
                }
                Some('.') => {
                    let after = self.src[self.pos + 1..].chars().next();
                    if matches!(after, Some(c) if is_pn_chars(c) || c == ':' || c == '%' || c == '\\')
                    {
                        self.advance();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        self.src[start..self.pos].to_owned()
    }
}

/// `PN_CHARS_BASE` from W3C Turtle grammar.
const fn is_pn_chars_base(c: char) -> bool {
    matches!(c,
        'A'..='Z'
        | 'a'..='z'
        | '\u{00C0}'..='\u{00D6}'
        | '\u{00D8}'..='\u{00F6}'
        | '\u{00F8}'..='\u{02FF}'
        | '\u{0370}'..='\u{037D}'
        | '\u{037F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}'
    )
}

/// `PN_CHARS_U = PN_CHARS_BASE | '_'`
const fn is_pn_chars_u(c: char) -> bool {
    is_pn_chars_base(c) || c == '_'
}

/// `PN_CHARS = PN_CHARS_U | '-' | [0-9] | U+00B7 | U+0300-U+036F | U+203F-U+2040`
const fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c)
        || c == '-'
        || c.is_ascii_digit()
        || c == '\u{00B7}'
        || matches!(c, '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn iri_ref() {
        let toks = token_kinds("<http://example.org/>");
        assert_eq!(toks, vec![TokenKind::IriRef("http://example.org/".into())]);
    }

    #[test]
    fn prefixed_name() {
        let toks = token_kinds("ex:foo");
        assert_eq!(
            toks,
            vec![TokenKind::PrefixedName {
                prefix: "ex".into(),
                local: "foo".into(),
            }]
        );
    }

    #[test]
    fn bnode_label() {
        let toks = token_kinds("_:abc");
        assert_eq!(toks, vec![TokenKind::BNodeLabel("abc".into())]);
    }

    #[test]
    fn string_double_quote() {
        let toks = token_kinds(r#""hello""#);
        assert_eq!(
            toks,
            vec![TokenKind::StringLiteral {
                raw: "hello".into(),
                long_form: false,
            }]
        );
    }

    #[test]
    fn long_string() {
        let toks = token_kinds(r#""""hello world""""#);
        assert_eq!(
            toks,
            vec![TokenKind::StringLiteral {
                raw: "hello world".into(),
                long_form: true,
            }]
        );
    }

    #[test]
    fn integer_tok() {
        let toks = token_kinds("42");
        assert_eq!(toks, vec![TokenKind::IntegerLiteral("42".into())]);
    }

    #[test]
    fn decimal_tok() {
        let toks = token_kinds("3.14");
        assert_eq!(toks, vec![TokenKind::DecimalLiteral("3.14".into())]);
    }

    #[test]
    fn double_tok() {
        let toks = token_kinds("1.5e10");
        assert_eq!(toks, vec![TokenKind::DoubleLiteral("1.5e10".into())]);
    }

    #[test]
    fn at_prefix() {
        let toks = token_kinds("@prefix");
        assert_eq!(toks, vec![TokenKind::AtPrefix]);
    }

    #[test]
    fn sparql_prefix() {
        let toks = token_kinds("PREFIX");
        assert_eq!(toks, vec![TokenKind::SparqlPrefix]);
    }

    #[test]
    fn a_keyword() {
        let toks = token_kinds("a");
        assert_eq!(toks, vec![TokenKind::AKeyword]);
    }

    #[test]
    fn comment_skipped() {
        let toks = token_kinds("# comment\na");
        assert_eq!(toks, vec![TokenKind::AKeyword]);
    }

    #[test]
    fn bom_skipped() {
        let src = "\u{FEFF}a";
        let toks = token_kinds(src);
        assert_eq!(toks, vec![TokenKind::AKeyword]);
    }

    #[test]
    fn datatype_tag() {
        let toks = token_kinds("^^");
        assert_eq!(toks, vec![TokenKind::DataTypeTag]);
    }

    #[test]
    fn lang_tag() {
        let toks = token_kinds("@en-US");
        assert_eq!(toks, vec![TokenKind::LangTag("en-US".into())]);
    }

    #[test]
    fn boolean_literals() {
        let toks = token_kinds("true false");
        assert_eq!(
            toks,
            vec![TokenKind::BooleanLiteral(true), TokenKind::BooleanLiteral(false)]
        );
    }

    #[test]
    fn punctuation() {
        let toks = token_kinds(". , ; ( ) [ ] { }");
        assert_eq!(
            toks,
            vec![
                TokenKind::Dot,
                TokenKind::Comma,
                TokenKind::Semicolon,
                TokenKind::ParenOpen,
                TokenKind::ParenClose,
                TokenKind::BracketOpen,
                TokenKind::BracketClose,
                TokenKind::BraceOpen,
                TokenKind::BraceClose,
            ]
        );
    }
}
