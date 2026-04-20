//! Tokeniser for N3 (Notation3) — Turtle + N3 extensions.
//!
//! This is a standalone lexer that covers the full Turtle 1.1 token set
//! plus the N3-specific tokens: `=>` (implication), `@keywords`,
//! `@forAll`, `@forSome`, and the bare-keyword mode enabled by `@keywords`.
//!
//! The N3 Team Submission grammar is at:
//! <https://www.w3.org/TeamSubmission/n3/#grammar-def>

/// Numeric literal category (Turtle §2.5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumKind {
    /// `xsd:integer`.
    Integer,
    /// `xsd:decimal`.
    Decimal,
    /// `xsd:double`.
    Double,
}

/// A single lexical token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    // ---- IRIs / blank nodes / literals ----
    /// `<iri>` body (decoded UCHARs applied).
    IriRef(String),
    /// `prefix:local` — both parts kept separate. `local` may be empty.
    Pname {
        /// Prefix name, possibly empty (`:local`).
        prefix: String,
        /// Local part, possibly empty (`ex:`).
        local: String,
    },
    /// `_:label`.
    BNodeLabel(String),
    /// A variable `?name` (N3 extension, treated as blank node).
    Variable(String),
    /// String literal — decoded USV contents.
    StringLit(String),
    /// Numeric literal — lexeme preserved verbatim.
    NumberLit {
        /// Numeric category.
        kind: NumKind,
        /// The raw lexeme as written.
        lexeme: String,
    },
    /// Language tag body (no leading `@`).
    LangTag(String),
    /// `^^` datatype marker.
    DataTypeMark,

    // ---- Punctuation ----
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `.` (statement terminator)
    Dot,
    /// `=>` (N3 logical implication)
    Implies,

    // ---- Keywords ----
    /// `a` keyword (rdf:type) — also bare `a` in @keywords mode.
    KwA,
    /// `true` keyword.
    KwTrue,
    /// `false` keyword.
    KwFalse,
    /// `is` keyword (N3 reverse property path).
    KwIs,
    /// `of` keyword (N3 reverse property path, follows `is P`).
    KwOf,
    /// `has` keyword (N3 forward property path shorthand).
    KwHas,
    /// `@prefix`.
    DirPrefix,
    /// `@base`.
    DirBase,
    /// `@keywords` directive.
    DirKeywords,
    /// `@forAll` directive.
    DirForAll,
    /// `@forSome` directive.
    DirForSome,
    /// SPARQL-style `PREFIX`.
    SparqlPrefix,
    /// SPARQL-style `BASE`.
    SparqlBase,
    /// TriG / N3 `GRAPH` keyword.
    KwGraph,
    /// Bare identifier that is NOT a recognized keyword or pname — error.
    BareIdent(String),
}

/// One token plus its byte-offset span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    pub(crate) tok: Tok,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// N3 lexer state.
pub(crate) struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    /// When `@keywords` was encountered, bare words listed in that directive
    /// act as keywords (e.g. `a` → `rdf:type`, `is` / `of` for reverse path).
    /// This flag enables the bare-keyword recognition in `lex_pname`.
    pub(crate) keywords_mode: bool,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            pos: 0,
            keywords_mode: false,
        }
    }

    pub(crate) const fn offset(&self) -> usize {
        self.pos
    }

    pub(crate) const fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Skip whitespace and `# …\n` comments.
    fn skip_trivia(&mut self) {
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                self.pos += 1;
            } else if b == b'#' {
                while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    pub(crate) fn peek(&mut self) -> Result<Option<Spanned>, String> {
        let save = self.pos;
        let out = self.next()?;
        self.pos = save;
        Ok(out)
    }

    /// Produce the next token, or `None` at EOF.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn next(&mut self) -> Result<Option<Spanned>, String> {
        self.skip_trivia();
        if self.pos >= self.src.len() {
            return Ok(None);
        }
        let start = self.pos;
        let b = self.src[self.pos];

        // Punctuation.
        let one = |this: &mut Self, tok: Tok| -> Result<Option<Spanned>, String> {
            this.pos += 1;
            Ok(Some(Spanned {
                tok,
                start: this.pos - 1,
                end: this.pos,
            }))
        };
        match b {
            b'(' => return one(self, Tok::LParen),
            b')' => return one(self, Tok::RParen),
            b'[' => return one(self, Tok::LBracket),
            b']' => return one(self, Tok::RBracket),
            b'{' => return one(self, Tok::LBrace),
            b'}' => return one(self, Tok::RBrace),
            b',' => return one(self, Tok::Comma),
            b';' => return one(self, Tok::Semicolon),
            _ => {}
        }

        // `=>` implication operator.
        if b == b'=' && self.src.get(self.pos + 1).copied() == Some(b'>') {
            self.pos += 2;
            return Ok(Some(Spanned {
                tok: Tok::Implies,
                start,
                end: self.pos,
            }));
        }

        // IRIREF.
        if b == b'<' {
            let (iri, end) = self.lex_iriref(start)?;
            return Ok(Some(Spanned {
                tok: Tok::IriRef(iri),
                start,
                end,
            }));
        }

        // String literal.
        if b == b'"' || b == b'\'' {
            let (s, end) = self.lex_string(start, b)?;
            return Ok(Some(Spanned {
                tok: Tok::StringLit(s),
                start,
                end,
            }));
        }

        // `@` — directives or language tag.
        if b == b'@' {
            return self.lex_at(start);
        }

        // `^^` datatype marker.
        if b == b'^' {
            if self.src.get(self.pos + 1).copied() == Some(b'^') {
                self.pos += 2;
                return Ok(Some(Spanned {
                    tok: Tok::DataTypeMark,
                    start,
                    end: self.pos,
                }));
            }
            return Err(format!(
                "N3-SYNTAX-001: lone '^' at byte {start} — expected '^^'"
            ));
        }

        // N3 variable `?name`.
        if b == b'?' {
            let (name, end) = self.lex_variable(start)?;
            return Ok(Some(Spanned {
                tok: Tok::Variable(name),
                start,
                end,
            }));
        }

        // Numeric literal or `.` statement terminator.
        if b == b'+' || b == b'-' || b == b'.' || b.is_ascii_digit() {
            if b == b'.' {
                let next = self.src.get(self.pos + 1).copied();
                if !next.is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                    return Ok(Some(Spanned {
                        tok: Tok::Dot,
                        start,
                        end: self.pos,
                    }));
                }
            }
            if let Some(tok) = self.lex_number(start)? {
                return Ok(Some(tok));
            }
        }

        // Blank node label `_:label`.
        if b == b'_' && self.src.get(self.pos + 1).copied() == Some(b':') {
            let (label, end) = self.lex_bnode(start)?;
            return Ok(Some(Spanned {
                tok: Tok::BNodeLabel(label),
                start,
                end,
            }));
        }

        // Prefixed name or keyword.
        if is_pn_chars_base(b) || b == b':' {
            let (prefix, local, end, had_colon) = self.lex_pname(start)?;
            let tok = if had_colon {
                Tok::Pname { prefix, local }
            } else {
                // Bare identifier — map to keyword.
                match prefix.as_str() {
                    "a" => Tok::KwA,
                    "true" => Tok::KwTrue,
                    "false" => Tok::KwFalse,
                    // N3 bare keywords (active both in @keywords mode and generally
                    // recognized for is/of/has path syntax).
                    "is" => Tok::KwIs,
                    "of" => Tok::KwOf,
                    "has" => Tok::KwHas,
                    p if p.eq_ignore_ascii_case("prefix") => Tok::SparqlPrefix,
                    p if p.eq_ignore_ascii_case("base") => Tok::SparqlBase,
                    p if p.eq_ignore_ascii_case("graph") => Tok::KwGraph,
                    other => Tok::BareIdent(other.to_owned()),
                }
            };
            return Ok(Some(Spanned { tok, start, end }));
        }

        Err(format!(
            "N3-SYNTAX-001: unexpected byte 0x{b:02X} at byte offset {start}"
        ))
    }

    // -- sub-lexers -------------------------------------------------------

    fn lex_variable(&mut self, start: usize) -> Result<(String, usize), String> {
        debug_assert_eq!(self.src[self.pos], b'?');
        self.pos += 1;
        let name_start = self.pos;
        while self.pos < self.src.len()
            && (self.src[self.pos].is_ascii_alphanumeric() || self.src[self.pos] == b'_')
        {
            self.pos += 1;
        }
        let name = std::str::from_utf8(&self.src[name_start..self.pos])
            .map_err(|_| format!("N3-SYNTAX-001: non-UTF-8 variable name at byte {start}"))?
            .to_owned();
        Ok((name, self.pos))
    }

    fn lex_iriref(&mut self, start: usize) -> Result<(String, usize), String> {
        debug_assert_eq!(self.src[self.pos], b'<');
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b'>' {
                self.pos += 1;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            if matches!(b, 0x00..=0x20) {
                return Err(format!(
                    "N3-SYNTAX-001: control byte 0x{b:02X} inside <IRI> at byte {start}"
                ));
            }
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos)
                .ok_or_else(|| format!("N3-SYNTAX-001: invalid UTF-8 in IRI at byte {ch_start}"))?;
            // Reject characters forbidden in IRIREF per Turtle §6.3.
            if matches!(ch, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\') {
                return Err(format!(
                    "N3-SYNTAX-001: invalid character {ch:?} in IRI at byte {ch_start}"
                ));
            }
            out.push(ch);
        }
        Err(format!(
            "N3-SYNTAX-001: unterminated <IRI> starting at byte {start}"
        ))
    }

    fn lex_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), String> {
        let triple = self.src.get(self.pos + 1).copied() == Some(quote)
            && self.src.get(self.pos + 2).copied() == Some(quote);
        if triple {
            self.pos += 3;
            return self.lex_long_string(start, quote);
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == quote {
                self.pos += 1;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_echar_or_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            if matches!(b, b'\n' | b'\r') {
                return Err(format!(
                    "TTL-LITESC-001: unescaped {} (U+{:04X}) in short string at byte {}",
                    if b == b'\n' { "LF" } else { "CR" },
                    u32::from(b),
                    self.pos,
                ));
            }
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos).ok_or_else(|| {
                format!("N3-SYNTAX-001: invalid UTF-8 in string at byte {ch_start}")
            })?;
            if matches!(ch, '\u{2028}' | '\u{0085}') {
                return Err(format!(
                    "TTL-LITESC-001: unescaped {} (U+{:04X}) in short string at byte {}",
                    if ch == '\u{2028}' { "LS" } else { "NEL" },
                    u32::from(ch),
                    ch_start,
                ));
            }
            out.push(ch);
        }
        Err(format!(
            "N3-SYNTAX-001: unterminated short string at byte {start}"
        ))
    }

    fn lex_long_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), String> {
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == quote
                && self.src.get(self.pos + 1).copied() == Some(quote)
                && self.src.get(self.pos + 2).copied() == Some(quote)
            {
                self.pos += 3;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_echar_or_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos).ok_or_else(|| {
                format!("N3-SYNTAX-001: invalid UTF-8 in long string at byte {ch_start}")
            })?;
            out.push(ch);
        }
        Err(format!(
            "N3-SYNTAX-001: unterminated \"\"\"…\"\"\" string at byte {start}"
        ))
    }

    fn lex_echar_or_uchar(&mut self, at: usize) -> Result<char, String> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        if self.pos + 1 >= self.src.len() {
            return Err(format!(
                "TTL-LITESC-001: escape at end of input at byte {at}"
            ));
        }
        let esc = self.src[self.pos + 1];
        match esc {
            b't' => {
                self.pos += 2;
                Ok('\t')
            }
            b'b' => {
                self.pos += 2;
                Ok('\u{0008}')
            }
            b'n' => {
                self.pos += 2;
                Ok('\n')
            }
            b'r' => {
                self.pos += 2;
                Ok('\r')
            }
            b'f' => {
                self.pos += 2;
                Ok('\u{000C}')
            }
            b'"' => {
                self.pos += 2;
                Ok('"')
            }
            b'\'' => {
                self.pos += 2;
                Ok('\'')
            }
            b'\\' => {
                self.pos += 2;
                Ok('\\')
            }
            b'u' | b'U' => self.lex_uchar(at),
            other => Err(format!(
                "TTL-LITESC-001: unknown ECHAR escape '\\{}' at byte {at}",
                other as char
            )),
        }
    }

    fn lex_uchar(&mut self, at: usize) -> Result<char, String> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        let marker = self.src.get(self.pos + 1).copied();
        let (width, step) = match marker {
            Some(b'u') => (4usize, 2usize),
            Some(b'U') => (8usize, 2usize),
            _ => {
                return Err(format!(
                    "TTL-LITESC-001: expected \\uXXXX or \\UXXXXXXXX at byte {at}"
                ));
            }
        };
        let hex_start = self.pos + step;
        let hex_end = hex_start + width;
        if hex_end > self.src.len() {
            return Err(format!(
                "TTL-LITESC-001: truncated UCHAR escape at byte {at}"
            ));
        }
        let mut cp: u32 = 0;
        for &b in &self.src[hex_start..hex_end] {
            let d = match b {
                b'0'..=b'9' => u32::from(b - b'0'),
                b'a'..=b'f' => u32::from(b - b'a' + 10),
                b'A'..=b'F' => u32::from(b - b'A' + 10),
                _ => {
                    return Err(format!(
                        "TTL-LITESC-001: non-hex digit in UCHAR at byte {at}"
                    ));
                }
            };
            cp = (cp << 4) | d;
        }
        self.pos = hex_end;
        if (0xD800..=0xDFFF).contains(&cp) {
            return Err(format!(
                "TTL-LITESC-001: surrogate UCHAR decode U+{cp:04X} at byte {at}"
            ));
        }
        char::from_u32(cp).ok_or_else(|| {
            format!("TTL-LITESC-001: invalid UCHAR code point U+{cp:04X} at byte {at}")
        })
    }

    fn lex_at(&mut self, start: usize) -> Result<Option<Spanned>, String> {
        debug_assert_eq!(self.src[self.pos], b'@');
        self.pos += 1;
        let rest = &self.src[self.pos..];
        // Check N3 directives before Turtle ones (they're longer).
        if rest.starts_with(b"keywords")
            && !matches!(rest.get(8), Some(c) if is_pn_chars(*c))
        {
            self.pos += 8;
            return Ok(Some(Spanned {
                tok: Tok::DirKeywords,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"forAll")
            && !matches!(rest.get(6), Some(c) if is_pn_chars(*c))
        {
            self.pos += 6;
            return Ok(Some(Spanned {
                tok: Tok::DirForAll,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"forSome")
            && !matches!(rest.get(7), Some(c) if is_pn_chars(*c))
        {
            self.pos += 7;
            return Ok(Some(Spanned {
                tok: Tok::DirForSome,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"prefix")
            && !matches!(rest.get(6), Some(c) if is_pn_chars(*c))
        {
            self.pos += 6;
            return Ok(Some(Spanned {
                tok: Tok::DirPrefix,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"base")
            && !matches!(rest.get(4), Some(c) if is_pn_chars(*c))
        {
            self.pos += 4;
            return Ok(Some(Spanned {
                tok: Tok::DirBase,
                start,
                end: self.pos,
            }));
        }
        // Language tag.
        let tag_start = self.pos;
        if !self
            .src
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphabetic())
        {
            return Err(format!(
                "N3-SYNTAX-001: expected @prefix, @base, @keywords, @forAll, @forSome, or language tag at byte {start}"
            ));
        }
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        while self.pos < self.src.len() && self.src[self.pos] == b'-' {
            self.pos += 1;
            let subtag_start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphanumeric() {
                self.pos += 1;
            }
            if self.pos == subtag_start {
                return Err(format!(
                    "N3-SYNTAX-001: empty subtag in language tag at byte {}", self.pos
                ));
            }
        }
        let tag = std::str::from_utf8(&self.src[tag_start..self.pos])
            .map_err(|_| {
                format!("N3-SYNTAX-001: non-UTF-8 language tag at byte {tag_start}")
            })?
            .to_owned();
        Ok(Some(Spanned {
            tok: Tok::LangTag(tag),
            start,
            end: self.pos,
        }))
    }

    fn lex_number(&mut self, start: usize) -> Result<Option<Spanned>, String> {
        let save = self.pos;
        let mut has_digit_before_dot = false;
        let mut has_dot = false;
        let mut has_exp = false;
        if matches!(self.src[self.pos], b'+' | b'-') {
            self.pos += 1;
        }
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            self.pos += 1;
            has_digit_before_dot = true;
        }
        if self.pos < self.src.len() && self.src[self.pos] == b'.' {
            let after_dot = self.src.get(self.pos + 1).copied();
            let consume_dot = match after_dot {
                Some(c) if c.is_ascii_digit() => true,
                Some(b'e' | b'E') if has_digit_before_dot => true,
                _ => false,
            };
            if consume_dot {
                has_dot = true;
                self.pos += 1;
                while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
        }
        if self.pos < self.src.len() && matches!(self.src[self.pos], b'e' | b'E') {
            has_exp = true;
            self.pos += 1;
            if matches!(self.src.get(self.pos).copied(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(format!(
                    "N3-SYNTAX-001: missing exponent digits at byte {}", self.pos
                ));
            }
        }
        if !has_digit_before_dot && !has_dot {
            self.pos = save;
            return Ok(None);
        }
        let lexeme = std::str::from_utf8(&self.src[save..self.pos])
            .expect("ASCII-only numeric tokens")
            .to_owned();
        let kind = if has_exp {
            NumKind::Double
        } else if has_dot {
            NumKind::Decimal
        } else {
            NumKind::Integer
        };
        Ok(Some(Spanned {
            tok: Tok::NumberLit { kind, lexeme },
            start,
            end: self.pos,
        }))
    }

    fn lex_bnode(&mut self, start: usize) -> Result<(String, usize), String> {
        debug_assert_eq!(self.src[self.pos], b'_');
        self.pos += 2; // skip "_:"
        let label_start = self.pos;
        if self.pos >= self.src.len() {
            return Err(format!(
                "N3-SYNTAX-001: empty blank-node label at byte {start}"
            ));
        }
        let first = self.src[self.pos];
        if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
            return Err(format!(
                "N3-SYNTAX-001: invalid blank-node label start at byte {}", self.pos
            ));
        }
        self.pos += 1;
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if is_pn_chars(b) || b == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        while self.pos > label_start + 1 && self.src[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        let label = std::str::from_utf8(&self.src[label_start..self.pos])
            .map_err(|_| {
                format!("N3-SYNTAX-001: non-UTF-8 blank-node label at byte {label_start}")
            })?
            .to_owned();
        Ok((label, self.pos))
    }

    fn lex_pname(&mut self, start: usize) -> Result<(String, String, usize, bool), String> {
        let prefix_start = self.pos;
        if self.src[self.pos] != b':' {
            if !is_pn_chars_base(self.src[self.pos]) {
                return Err(format!(
                    "N3-SYNTAX-001: expected PN_PREFIX start at byte {start}"
                ));
            }
            self.pos += 1;
            while self.pos < self.src.len() {
                let b = self.src[self.pos];
                if is_pn_chars(b) || b == b'.' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            while self.pos > prefix_start + 1 && self.src[self.pos - 1] == b'.' {
                self.pos -= 1;
            }
        }
        let prefix_end = self.pos;
        if self.pos >= self.src.len() || self.src[self.pos] != b':' {
            let name = std::str::from_utf8(&self.src[prefix_start..prefix_end])
                .map_err(|_| {
                    format!("N3-SYNTAX-001: non-UTF-8 identifier at byte {prefix_start}")
                })?
                .to_owned();
            return Ok((name, String::new(), self.pos, false));
        }
        let prefix = std::str::from_utf8(&self.src[prefix_start..prefix_end])
            .map_err(|_| {
                format!("N3-SYNTAX-001: non-UTF-8 prefix at byte {prefix_start}")
            })?
            .to_owned();
        self.pos += 1; // consume ':'

        // PN_LOCAL body.
        let mut local = String::new();
        let local_start = self.pos;
        let mut first_char = true;
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b'%' {
                if self.pos + 2 >= self.src.len()
                    || !is_hex(self.src[self.pos + 1])
                    || !is_hex(self.src[self.pos + 2])
                {
                    return Err(format!(
                        "N3-SYNTAX-001: malformed PERCENT escape in PN_LOCAL at byte {}", self.pos
                    ));
                }
                local.push('%');
                local.push(self.src[self.pos + 1] as char);
                local.push(self.src[self.pos + 2] as char);
                self.pos += 3;
                first_char = false;
                continue;
            }
            if b == b'\\' {
                let next = self.src.get(self.pos + 1).copied();
                if next.is_some_and(is_pn_local_esc) {
                    local.push(next.unwrap() as char);
                    self.pos += 2;
                    first_char = false;
                    continue;
                }
                return Err(format!(
                    "N3-SYNTAX-001: invalid PN_LOCAL_ESC at byte {}", self.pos
                ));
            }
            let ok_start = is_pn_chars_u(b) || b == b':' || b.is_ascii_digit();
            let ok_cont = is_pn_chars(b) || b == b':' || b == b'.';
            if (first_char && ok_start) || (!first_char && ok_cont) {
                let ch_start = self.pos;
                let ch = next_utf8_char(self.src, &mut self.pos).ok_or_else(|| {
                    format!("N3-SYNTAX-001: invalid UTF-8 in local part at byte {ch_start}")
                })?;
                local.push(ch);
                first_char = false;
                continue;
            }
            break;
        }
        while local.ends_with('.') {
            local.pop();
            self.pos -= 1;
        }
        let _ = local_start;
        Ok((prefix, local, self.pos, true))
    }
}

// -- Character class helpers --------------------------------------------------

fn is_hex(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

pub(crate) fn is_pn_chars_base(b: u8) -> bool {
    b.is_ascii_alphabetic() || b >= 0x80
}

pub(crate) fn is_pn_chars_u(b: u8) -> bool {
    is_pn_chars_base(b) || b == b'_'
}

pub(crate) fn is_pn_chars(b: u8) -> bool {
    is_pn_chars_u(b) || b.is_ascii_digit() || b == b'-' || b == 0xB7
}

fn is_pn_local_esc(b: u8) -> bool {
    matches!(
        b,
        b'_' | b'~'
            | b'.'
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
            | b'-'
    )
}

pub(crate) fn next_utf8_char(src: &[u8], pos: &mut usize) -> Option<char> {
    let remaining = src.get(*pos..)?;
    let s = std::str::from_utf8(remaining).ok()?;
    let ch = s.chars().next()?;
    *pos += ch.len_utf8();
    Some(ch)
}
