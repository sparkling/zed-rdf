//! Tokeniser for SPARQL 1.1 Query and Update.
//!
//! Hand-rolled lexer covering the lexical productions of SPARQL 1.1
//! §19: IRIREF, PNAME_NS / PNAME_LN, variables (`?var`, `$var`), blank
//! node labels (`_:x`), string literals (all four forms — short and
//! long, single- and double-quoted), numeric literals, language tags,
//! and SPARQL punctuation (`.`, `,`, `;`, `(`, `)`, `{`, `}`, `[`, `]`,
//! `^^`, `|`, `!`, `^`, `?`, `*`, `+`, `/`, `,`, `=`, `!=`, `<`, `<=`,
//! `>`, `>=`, `&&`, `||`).
//!
//! Keywords are returned as raw `Ident` tokens; the grammar layer
//! case-folds and classifies them (SPARQL is case-insensitive on
//! keywords per §4.1).

use crate::diag::{Diag, DiagnosticCode};

/// Numeric literal category (§19.8 NumericLiteral*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumKind {
    /// `xsd:integer` — `[0-9]+`.
    Integer,
    /// `xsd:decimal` — dot present, no exponent.
    Decimal,
    /// `xsd:double` — exponent present.
    Double,
}

/// A SPARQL lexical token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// `<iri>` body (UCHARs decoded).
    IriRef(String),
    /// `prefix:local` or `prefix:` (local empty). Both parts already
    /// percent / `PN_LOCAL_ESC` decoded lexically.
    Pname {
        prefix: String,
        local: String,
    },
    /// `_:label` (blank node label from the source).
    BNodeLabel(String),
    /// `[]` anonymous blank node.
    AnonBNode,
    /// `?x` or `$x` variable.
    Var(String),
    /// Identifier (keyword candidate). SPARQL keywords are case-insensitive.
    Ident(String),
    /// Decoded string literal (ECHAR / UCHAR applied).
    StringLit(String),
    /// Numeric literal — lexeme preserved verbatim.
    NumberLit {
        kind: NumKind,
        lexeme: String,
    },
    /// Language tag body (no leading `@`).
    LangTag(String),
    /// `^^` datatype marker.
    DataTypeMark,
    /// Punctuation tokens.
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Comma,
    Semicolon,
    Star,
    Plus,
    Minus,
    Slash,
    Question,
    Bang,
    Caret,
    Pipe,
    PipePipe,
    Amp,
    AmpAmp,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// NIL — `(` WS* `)` token per §19.8 NIL.
    Nil,
}

/// One token plus its byte-offset span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) tok: Tok,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Hand-rolled SPARQL lexer.
pub(crate) struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) const fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    pub(crate) const fn offset(&self) -> usize {
        self.pos
    }

    /// Consume and return the next token.
    pub(crate) fn next_tok(&mut self) -> Result<Option<Spanned>, Diag> {
        self.skip_ws_and_comments();
        if self.pos >= self.src.len() {
            return Ok(None);
        }
        let start = self.pos;
        let b = self.src[start];
        let tok = match b {
            b'<' => self.lex_iri_or_cmp(start)?,
            b'>' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'=') {
                    self.pos += 1;
                    Tok::GtEq
                } else {
                    Tok::Gt
                }
            }
            b'=' => {
                self.pos += 1;
                Tok::Eq
            }
            b'!' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'=') {
                    self.pos += 1;
                    Tok::NotEq
                } else {
                    Tok::Bang
                }
            }
            b'&' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'&') {
                    self.pos += 1;
                    Tok::AmpAmp
                } else {
                    Tok::Amp
                }
            }
            b'|' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'|') {
                    self.pos += 1;
                    Tok::PipePipe
                } else {
                    Tok::Pipe
                }
            }
            b'{' => {
                self.pos += 1;
                Tok::LBrace
            }
            b'}' => {
                self.pos += 1;
                Tok::RBrace
            }
            b'(' => {
                // Check for NIL — (WS* )
                let save = self.pos;
                self.pos += 1;
                self.skip_ws_and_comments();
                if self.peek_byte() == Some(b')') {
                    self.pos += 1;
                    Tok::Nil
                } else {
                    self.pos = save + 1;
                    Tok::LParen
                }
            }
            b')' => {
                self.pos += 1;
                Tok::RParen
            }
            b'[' => {
                // Check for anon bnode [] — [WS*]
                let save = self.pos;
                self.pos += 1;
                self.skip_ws_and_comments();
                if self.peek_byte() == Some(b']') {
                    self.pos += 1;
                    Tok::AnonBNode
                } else {
                    self.pos = save + 1;
                    Tok::LBracket
                }
            }
            b']' => {
                self.pos += 1;
                Tok::RBracket
            }
            b',' => {
                self.pos += 1;
                Tok::Comma
            }
            b';' => {
                self.pos += 1;
                Tok::Semicolon
            }
            b'*' => {
                self.pos += 1;
                Tok::Star
            }
            b'+' => {
                // Could be a signed number, but the parser handles that
                // when it expects an expression. Here we always return `+`.
                self.pos += 1;
                Tok::Plus
            }
            b'-' => {
                self.pos += 1;
                Tok::Minus
            }
            b'/' => {
                self.pos += 1;
                Tok::Slash
            }
            b'?' | b'$' => self.lex_var(start)?,
            b'^' => {
                self.pos += 1;
                if self.peek_byte() == Some(b'^') {
                    self.pos += 1;
                    Tok::DataTypeMark
                } else {
                    Tok::Caret
                }
            }
            b'.' => {
                // Could be start of a decimal like `.5`.
                if self.peek_byte_at(1).is_some_and(|b| b.is_ascii_digit()) {
                    self.lex_number(start)?
                } else {
                    self.pos += 1;
                    Tok::Dot
                }
            }
            b'0'..=b'9' => self.lex_number(start)?,
            b'"' | b'\'' => self.lex_string(start)?,
            b'@' => self.lex_lang_tag(start)?,
            b'_' => {
                if self.peek_byte_at(1) == Some(b':') {
                    self.lex_bnode_label(start)?
                } else {
                    self.lex_ident_or_pname(start)?
                }
            }
            _ if is_pn_chars_base(b) || b == b':' => self.lex_ident_or_pname(start)?,
            _ => {
                // Non-ASCII identifier (UTF-8) or a truly invalid byte.
                if b >= 0x80 {
                    self.lex_ident_or_pname(start)?
                } else {
                    return Err(Diag::fatal(
                        DiagnosticCode::Syntax,
                        format!("unexpected byte 0x{b:02X}"),
                        start,
                    ));
                }
            }
        };
        Ok(Some(Spanned {
            tok,
            start,
            end: self.pos,
        }))
    }

    fn peek_byte(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_byte_at(&self, n: usize) -> Option<u8> {
        self.src.get(self.pos + n).copied()
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            // Whitespace.
            while let Some(b) = self.peek_byte() {
                if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            // `# ... EOL` comment.
            if self.peek_byte() == Some(b'#') {
                while let Some(b) = self.peek_byte() {
                    self.pos += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn lex_iri_or_cmp(&mut self, start: usize) -> Result<Tok, Diag> {
        // `<` can begin either an IRIREF or a comparison operator. We
        // disambiguate by scanning forward: IRIREF bodies cannot contain
        // whitespace or the characters `<>"{}|^` \``. If we hit one of
        // those before a closing `>`, treat the `<` as a comparison.
        //
        // Additionally `<=` is the le-operator.
        if self.peek_byte_at(1) == Some(b'=') {
            self.pos += 2;
            return Ok(Tok::LtEq);
        }
        // Try to lex an IRIREF body.
        let save = self.pos;
        self.pos += 1; // consume `<`
        let mut buf = String::new();
        while let Some(b) = self.peek_byte() {
            match b {
                b'>' => {
                    self.pos += 1;
                    return Ok(Tok::IriRef(buf));
                }
                b' ' | b'\t' | b'\r' | b'\n' | b'<' | b'"' | b'{' | b'}' | b'|' | b'`' => {
                    // Not an IRIREF body — rewind and emit `Lt`.
                    self.pos = save + 1;
                    return Ok(Tok::Lt);
                }
                b'\\' => {
                    // UCHAR: \uXXXX or \UXXXXXXXX
                    let ch = self.decode_uchar(self.pos)?;
                    buf.push(ch);
                }
                _ => {
                    // Decode UTF-8 codepoint.
                    let (ch, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    buf.push(ch);
                    self.pos += n;
                }
            }
        }
        Err(Diag::fatal(
            DiagnosticCode::Unterminated,
            "unterminated IRIREF",
            start,
        ))
    }

    fn decode_uchar(&mut self, start: usize) -> Result<char, Diag> {
        // self.pos is at `\`
        if self.peek_byte_at(1) == Some(b'u') {
            if self.pos + 6 > self.src.len() {
                return Err(Diag::fatal(
                    DiagnosticCode::LitEsc,
                    "truncated \\u escape",
                    start,
                ));
            }
            let hex = &self.src[self.pos + 2..self.pos + 6];
            let code = parse_hex(hex).map_err(|_| {
                Diag::fatal(DiagnosticCode::LitEsc, "bad \\u escape", start)
            })?;
            self.pos += 6;
            char::from_u32(code).ok_or_else(|| {
                Diag::fatal(
                    DiagnosticCode::LitEsc,
                    "surrogate or out-of-range \\u escape",
                    start,
                )
            })
        } else if self.peek_byte_at(1) == Some(b'U') {
            if self.pos + 10 > self.src.len() {
                return Err(Diag::fatal(
                    DiagnosticCode::LitEsc,
                    "truncated \\U escape",
                    start,
                ));
            }
            let hex = &self.src[self.pos + 2..self.pos + 10];
            let code = parse_hex(hex).map_err(|_| {
                Diag::fatal(DiagnosticCode::LitEsc, "bad \\U escape", start)
            })?;
            self.pos += 10;
            char::from_u32(code).ok_or_else(|| {
                Diag::fatal(
                    DiagnosticCode::LitEsc,
                    "surrogate or out-of-range \\U escape",
                    start,
                )
            })
        } else {
            Err(Diag::fatal(
                DiagnosticCode::LitEsc,
                "backslash in IRIREF must be \\u or \\U",
                start,
            ))
        }
    }

    fn lex_var(&mut self, start: usize) -> Result<Tok, Diag> {
        // `?` or `$` then VARNAME (PN_CHARS_U | [0-9]) (VARCHAR)*.
        // A bare `?` with no following VARCHAR is the property-path
        // zero-or-one modifier.
        let sigil = self.src[self.pos];
        self.pos += 1; // consume sigil
        let body_start = self.pos;
        while let Some(b) = self.peek_byte() {
            if is_var_char(b) || b >= 0x80 {
                if b < 0x80 {
                    self.pos += 1;
                } else {
                    let (_, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    self.pos += n;
                }
            } else {
                break;
            }
        }
        if self.pos == body_start {
            // Bare sigil: `?` is the path `?` modifier; `$` alone is an error.
            if sigil == b'?' {
                return Ok(Tok::Question);
            }
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "empty variable name",
                start,
            ));
        }
        let name = std::str::from_utf8(&self.src[body_start..self.pos])
            .map_err(|_| Diag::fatal(DiagnosticCode::InvalidUtf8, "bad utf-8 in var", start))?;
        Ok(Tok::Var(name.to_owned()))
    }

    fn lex_bnode_label(&mut self, start: usize) -> Result<Tok, Diag> {
        // `_:` followed by PN_LOCAL-ish.
        self.pos += 2; // consume `_:`
        let body_start = self.pos;
        while let Some(b) = self.peek_byte() {
            if is_pn_chars(b) || b == b'.' || b >= 0x80 {
                if b < 0x80 {
                    self.pos += 1;
                } else {
                    let (_, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    self.pos += n;
                }
            } else {
                break;
            }
        }
        // Trailing dot must not be part of the label.
        while self.pos > body_start && self.src[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        if self.pos == body_start {
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "empty blank node label",
                start,
            ));
        }
        let name = std::str::from_utf8(&self.src[body_start..self.pos])
            .map_err(|_| Diag::fatal(DiagnosticCode::InvalidUtf8, "bad utf-8", start))?;
        Ok(Tok::BNodeLabel(name.to_owned()))
    }

    fn lex_ident_or_pname(&mut self, start: usize) -> Result<Tok, Diag> {
        // Lex PN_PREFIX (optional), then optional `:` and PN_LOCAL.
        // An identifier (no colon) is a keyword / `a` / `true` / `false`.
        let p_start = self.pos;
        while let Some(b) = self.peek_byte() {
            if is_pn_chars(b) || b >= 0x80 {
                if b < 0x80 {
                    self.pos += 1;
                } else {
                    let (_, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    self.pos += n;
                }
            } else if b == b'.' {
                // `.` allowed inside PN_PREFIX except trailing; peek ahead.
                if self
                    .peek_byte_at(1)
                    .is_some_and(|c| is_pn_chars(c) || c == b'.' || c >= 0x80)
                {
                    self.pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        // trim trailing dot if any.
        while self.pos > p_start && self.src[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        let prefix_bytes = &self.src[p_start..self.pos];
        let prefix = std::str::from_utf8(prefix_bytes)
            .map_err(|_| Diag::fatal(DiagnosticCode::InvalidUtf8, "bad utf-8 in ident", start))?
            .to_owned();

        if self.peek_byte() == Some(b':') {
            self.pos += 1;
            let local = self.lex_pn_local(start)?;
            return Ok(Tok::Pname { prefix, local });
        }
        if prefix.is_empty() {
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "empty identifier",
                start,
            ));
        }
        Ok(Tok::Ident(prefix))
    }

    fn lex_pn_local(&mut self, start: usize) -> Result<String, Diag> {
        // PN_LOCAL per §19.8. We accept the common shape:
        //   (PN_CHARS_U | ':' | [0-9] | PLX) (PN_CHARS | '.' | ':' | PLX)*
        // PLX = PERCENT | PN_LOCAL_ESC.
        let mut out = String::new();
        let body_start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b == b'%' {
                // PERCENT: %HEX HEX
                if self.pos + 3 > self.src.len()
                    || !self.src[self.pos + 1].is_ascii_hexdigit()
                    || !self.src[self.pos + 2].is_ascii_hexdigit()
                {
                    return Err(Diag::fatal(
                        DiagnosticCode::Syntax,
                        "bad percent escape",
                        self.pos,
                    ));
                }
                out.push('%');
                out.push(self.src[self.pos + 1] as char);
                out.push(self.src[self.pos + 2] as char);
                self.pos += 3;
            } else if b == b'\\' {
                // PN_LOCAL_ESC \\x where x in "_~.-!$&'()*+,;=/?#@%"
                let Some(nxt) = self.peek_byte_at(1) else {
                    return Err(Diag::fatal(
                        DiagnosticCode::Syntax,
                        "truncated PN_LOCAL_ESC",
                        self.pos,
                    ));
                };
                if is_pn_local_esc(nxt) {
                    out.push(nxt as char);
                    self.pos += 2;
                } else {
                    return Err(Diag::fatal(
                        DiagnosticCode::Syntax,
                        "bad PN_LOCAL_ESC",
                        self.pos,
                    ));
                }
            } else if is_pn_chars(b) || b == b':' || b == b'.' || b.is_ascii_digit() || b >= 0x80 {
                if b < 0x80 {
                    out.push(b as char);
                    self.pos += 1;
                } else {
                    let (ch, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    out.push(ch);
                    self.pos += n;
                }
            } else {
                break;
            }
        }
        // Disallow trailing `.`.
        while out.ends_with('.') {
            out.pop();
            self.pos -= 1;
        }
        let _ = body_start;
        let _ = start;
        Ok(out)
    }

    fn lex_number(&mut self, start: usize) -> Result<Tok, Diag> {
        let mut kind = NumKind::Integer;
        let mut lexeme = String::new();
        // Optional leading digit(s).
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_digit() {
                lexeme.push(b as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        // Decimal point.
        if self.peek_byte() == Some(b'.')
            && self.peek_byte_at(1).is_some_and(|c| c.is_ascii_digit())
        {
            kind = NumKind::Decimal;
            lexeme.push('.');
            self.pos += 1;
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_digit() {
                    lexeme.push(b as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
        } else if lexeme.starts_with('.') {
            // `.` was consumed as start? Actually no — lex_number only
            // entered when start byte is a digit or `.`. If `.`, we came
            // via the caller branch `b'.'` which already ensured next is digit.
        }
        // Special case: number started with `.` (caller ensured digit follows)
        if self.src[start] == b'.' && !lexeme.starts_with('.') {
            // Re-scan from start.
            self.pos = start;
            lexeme.clear();
            kind = NumKind::Decimal;
            lexeme.push('.');
            self.pos += 1;
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_digit() {
                    lexeme.push(b as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        // Exponent.
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            kind = NumKind::Double;
            lexeme.push(self.peek_byte().unwrap() as char);
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                lexeme.push(self.peek_byte().unwrap() as char);
                self.pos += 1;
            }
            let exp_start = self.pos;
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_digit() {
                    lexeme.push(b as char);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == exp_start {
                return Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    "missing exponent digits",
                    start,
                ));
            }
        }
        Ok(Tok::NumberLit { kind, lexeme })
    }

    fn lex_string(&mut self, start: usize) -> Result<Tok, Diag> {
        let quote = self.src[self.pos];
        // Check for long string (triple quote).
        let long = self.peek_byte_at(1) == Some(quote) && self.peek_byte_at(2) == Some(quote);
        if long {
            self.pos += 3;
        } else {
            self.pos += 1;
        }
        let mut out = String::new();
        loop {
            let Some(b) = self.peek_byte() else {
                return Err(Diag::fatal(
                    DiagnosticCode::Unterminated,
                    "unterminated string literal",
                    start,
                ));
            };
            match b {
                b'\\' => {
                    let esc = self.peek_byte_at(1).ok_or_else(|| {
                        Diag::fatal(
                            DiagnosticCode::LitEsc,
                            "truncated escape in string",
                            self.pos,
                        )
                    })?;
                    match esc {
                        b't' => {
                            out.push('\t');
                            self.pos += 2;
                        }
                        b'b' => {
                            out.push('\u{0008}');
                            self.pos += 2;
                        }
                        b'n' => {
                            out.push('\n');
                            self.pos += 2;
                        }
                        b'r' => {
                            out.push('\r');
                            self.pos += 2;
                        }
                        b'f' => {
                            out.push('\u{000C}');
                            self.pos += 2;
                        }
                        b'"' => {
                            out.push('"');
                            self.pos += 2;
                        }
                        b'\'' => {
                            out.push('\'');
                            self.pos += 2;
                        }
                        b'\\' => {
                            out.push('\\');
                            self.pos += 2;
                        }
                        b'u' | b'U' => {
                            let ch = self.decode_uchar(self.pos)?;
                            out.push(ch);
                        }
                        _ => {
                            return Err(Diag::fatal(
                                DiagnosticCode::LitEsc,
                                format!("unknown string escape \\{}", esc as char),
                                self.pos,
                            ));
                        }
                    }
                }
                c if c == quote => {
                    if long {
                        if self.peek_byte_at(1) == Some(quote)
                            && self.peek_byte_at(2) == Some(quote)
                        {
                            self.pos += 3;
                            return Ok(Tok::StringLit(out));
                        }
                        out.push(quote as char);
                        self.pos += 1;
                    } else {
                        self.pos += 1;
                        return Ok(Tok::StringLit(out));
                    }
                }
                b'\n' | b'\r' if !long => {
                    return Err(Diag::fatal(
                        DiagnosticCode::LitEsc,
                        "newline in short string literal",
                        start,
                    ));
                }
                _ => {
                    let (ch, n) = utf8_decode(&self.src[self.pos..], self.pos)?;
                    out.push(ch);
                    self.pos += n;
                }
            }
        }
    }

    fn lex_lang_tag(&mut self, start: usize) -> Result<Tok, Diag> {
        self.pos += 1; // consume `@`
        let body_start = self.pos;
        // Primary subtag [a-zA-Z]+
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphabetic() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == body_start {
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "empty language tag",
                start,
            ));
        }
        while self.peek_byte() == Some(b'-') {
            self.pos += 1;
            let sub_start = self.pos;
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_alphanumeric() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == sub_start {
                return Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    "empty language subtag",
                    start,
                ));
            }
        }
        let tag = std::str::from_utf8(&self.src[body_start..self.pos])
            .map_err(|_| Diag::fatal(DiagnosticCode::InvalidUtf8, "bad utf-8 in lang tag", start))?
            .to_owned();
        Ok(Tok::LangTag(tag))
    }
}

fn parse_hex(bytes: &[u8]) -> Result<u32, ()> {
    let mut out: u32 = 0;
    for &b in bytes {
        let d = match b {
            b'0'..=b'9' => u32::from(b - b'0'),
            b'a'..=b'f' => u32::from(b - b'a') + 10,
            b'A'..=b'F' => u32::from(b - b'A') + 10,
            _ => return Err(()),
        };
        out = out * 16 + d;
    }
    Ok(out)
}

fn utf8_decode(bytes: &[u8], offset: usize) -> Result<(char, usize), Diag> {
    match std::str::from_utf8(bytes) {
        Ok(s) => {
            if let Some(ch) = s.chars().next() {
                Ok((ch, ch.len_utf8()))
            } else {
                Err(Diag::fatal(
                    DiagnosticCode::UnexpectedEof,
                    "unexpected end of input",
                    offset,
                ))
            }
        }
        Err(e) => {
            let valid = e.valid_up_to();
            if valid > 0 {
                let s = std::str::from_utf8(&bytes[..valid]).unwrap_or("");
                if let Some(ch) = s.chars().next() {
                    return Ok((ch, ch.len_utf8()));
                }
            }
            Err(Diag::fatal(
                DiagnosticCode::InvalidUtf8,
                "invalid UTF-8 sequence",
                offset,
            ))
        }
    }
}

// -- character class helpers --------------------------------------------

fn is_pn_chars_base(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

fn is_pn_chars_u(b: u8) -> bool {
    is_pn_chars_base(b) || b == b'_'
}

fn is_pn_chars(b: u8) -> bool {
    is_pn_chars_u(b) || b.is_ascii_digit() || b == b'-'
}

fn is_var_char(b: u8) -> bool {
    is_pn_chars_u(b) || b.is_ascii_digit()
}

fn is_pn_local_esc(b: u8) -> bool {
    matches!(
        b,
        b'_' | b'~'
            | b'.'
            | b'-'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'/'
            | b'?'
            | b'#'
            | b'@'
            | b'%'
    )
}
