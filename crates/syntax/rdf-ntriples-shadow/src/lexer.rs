//! Low-level byte-oriented lexer for N-Triples and N-Quads.
//!
//! The lexer splits the input into *lines* (handling LF, CRLF, and bare CR)
//! and then tokenises each line into the token types that N-Triples/N-Quads
//! need: IRIs, blank-node labels, string literals, language tags, and the
//! `^^` datatype separator.
//!
//! All tokens are returned as `&str` slices into the original UTF-8 input.
//! The caller is responsible for escape decoding (see [`crate::unescape`]).

/// The tokens emitted by the lexer for a single statement line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    /// `<…>` — the raw content between the angle brackets (escapes not yet decoded).
    Iri(&'a str),
    /// `_:label` — the label after `_:`.
    BlankNode(&'a str),
    /// The content between the outer `"…"` (escapes not yet decoded).
    StringLiteral(&'a str),
    /// `@tag` — the language tag after `@`.
    LangTag(&'a str),
    /// `^^` — datatype separator.
    DatatypeSep,
    /// `.` — statement terminator.
    Dot,
}

/// Split `input` into logical lines, handling LF, CRLF, and bare CR.
///
/// A leading UTF-8 BOM (`\u{FEFF}`) on the very first line is stripped.
pub fn lines(input: &str) -> impl Iterator<Item = (usize, &str)> {
    LineIter {
        remaining: input,
        offset: 0,
        first: true,
    }
}

struct LineIter<'a> {
    remaining: &'a str,
    offset: usize,
    first: bool,
}

impl<'a> Iterator for LineIter<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        // Strip BOM from the very first line.
        if self.first {
            self.first = false;
            if let Some(stripped) = self.remaining.strip_prefix('\u{FEFF}') {
                self.remaining = stripped;
                self.offset += 3; // BOM is 3 bytes in UTF-8
            }
        }

        // Find the next line-ending: LF, CRLF, or bare CR.
        let (line_end, terminator_len) = find_line_end(self.remaining);
        let line = &self.remaining[..line_end];
        let line_offset = self.offset;
        self.offset += line_end + terminator_len;
        self.remaining = &self.remaining[line_end + terminator_len..];
        Some((line_offset, line))
    }
}

/// Returns `(content_end, terminator_bytes)` for the current line.
fn find_line_end(s: &str) -> (usize, usize) {
    for (i, ch) in s.char_indices() {
        match ch {
            '\n' => return (i, 1),
            '\r' => {
                // Check for CRLF.
                let term = if s[i + 1..].starts_with('\n') { 2 } else { 1 };
                return (i, term);
            }
            _ => {}
        }
    }
    (s.len(), 0)
}

/// Tokenise a single non-comment, non-empty line.
///
/// Returns `Err(message)` on any lexical error.
pub fn tokenise(line: &str) -> Result<Vec<Token<'_>>, String> {
    let mut tokens = Vec::new();
    let mut rest = line.trim_start_matches(is_ws);

    while !rest.is_empty() {
        // Skip inline whitespace.
        rest = rest.trim_start_matches(is_ws);
        if rest.is_empty() {
            break;
        }

        if rest.starts_with('#') {
            // Inline comment — stop tokenising this line.
            break;
        }

        if let Some(after) = rest.strip_prefix('<') {
            // IRI reference: scan for matching `>` that is not escaped.
            let (iri_content, remainder) = scan_iri(after)?;
            tokens.push(Token::Iri(iri_content));
            rest = remainder;
            continue;
        }

        if let Some(after) = rest.strip_prefix("_:") {
            // Blank node label.
            let (label, remainder) = scan_bnode_label(after)?;
            tokens.push(Token::BlankNode(label));
            rest = remainder;
            continue;
        }

        if rest.starts_with('"') {
            // String literal.
            let (content, remainder) = scan_string_literal(rest)?;
            tokens.push(Token::StringLiteral(content));
            rest = remainder;
            // Check for language tag or datatype.
            rest = rest.trim_start_matches(is_ws);
            if let Some(after) = rest.strip_prefix('@') {
                let (tag, rem) = scan_lang_tag(after)?;
                tokens.push(Token::LangTag(tag));
                rest = rem;
            } else if rest.starts_with("^^") {
                tokens.push(Token::DatatypeSep);
                rest = &rest[2..];
            }
            continue;
        }

        if rest.starts_with("^^") {
            tokens.push(Token::DatatypeSep);
            rest = &rest[2..];
            continue;
        }

        if let Some(after) = rest.strip_prefix('.') {
            tokens.push(Token::Dot);
            rest = after;
            continue;
        }

        return Err(format!(
            "unexpected character '{}'",
            rest.chars().next().unwrap_or('?')
        ));
    }

    Ok(tokens)
}

const fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Scan from just after the opening `<` until the closing `>`.
/// Returns `(iri_content, rest_after_closing_angle)`.
fn scan_iri(after_open: &str) -> Result<(&str, &str), String> {
    // IRIs in N-Triples: no bare control chars, no unescaped `<`, `>`,
    // `{`, `}`, `|`, `^`, `` ` ``, `\` (unless it's a \u or \U escape).
    let mut char_iter = after_open.char_indices();
    loop {
        match char_iter.next() {
            None => return Err("unterminated IRI: missing '>'".into()),
            Some((i, '>')) => {
                let content = &after_open[..i];
                let rest = &after_open[i + 1..];
                return Ok((content, rest));
            }
            Some((_, '\\')) => {
                // Consume the next char (escape validation happens in unescape).
                if char_iter.next().is_none() {
                    return Err("unterminated escape in IRI".into());
                }
            }
            Some((_, ch)) if (ch as u32) <= 0x20 => {
                return Err(format!(
                    "control character U+{:04X} in IRI",
                    ch as u32
                ));
            }
            Some(_) => {}
        }
    }
}

/// Scan a blank-node label (after `_:`).
fn scan_bnode_label(s: &str) -> Result<(&str, &str), String> {
    // PN_CHARS_U range + digits; the label must not be empty.
    if s.is_empty() {
        return Err("empty blank-node label after '_:'".into());
    }
    // First character: PN_CHARS_BASE | '_' | [0-9]
    let mut iter = s.char_indices();
    match iter.next() {
        None => return Err("empty blank-node label after '_:'".into()),
        Some((_, c)) if !is_pn_chars_u(c) && !c.is_ascii_digit() => {
            return Err(format!("invalid first character '{c}' in blank-node label"));
        }
        _ => {}
    }
    // Remaining characters: PN_CHARS | '.'
    // Labels must not end in '.'.
    let mut last_was_dot = false;
    let mut end = s.len();
    for (i, c) in iter {
        if c == '.' {
            last_was_dot = true;
        } else if is_pn_chars(c) {
            last_was_dot = false;
        } else {
            end = i;
            break;
        }
    }
    if last_was_dot {
        // Find the last non-dot position.
        end = s[..end]
            .trim_end_matches('.')
            .len();
    }
    if end == 0 {
        return Err("empty blank-node label after '_:'".into());
    }
    Ok((&s[..end], &s[end..]))
}

const fn is_pn_chars_u(c: char) -> bool {
    is_pn_chars_base(c) || c == '_'
}

const fn is_pn_chars_base(c: char) -> bool {
    matches!(c,
        'A'..='Z' | 'a'..='z'
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

const fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c)
        || c == '-'
        || c.is_ascii_digit()
        || c == '\u{00B7}'
        || matches!(c, '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

/// Scan a double-quoted string literal. Returns `(content, rest)` where
/// `content` is the raw (un-decoded) text between the quotes.
fn scan_string_literal(s: &str) -> Result<(&str, &str), String> {
    debug_assert!(s.starts_with('"'));
    let after_open = &s[1..];
    let mut iter = after_open.char_indices();
    loop {
        match iter.next() {
            None => return Err("unterminated string literal".into()),
            Some((i, '"')) => {
                let content = &after_open[..i];
                let rest = &after_open[i + 1..];
                return Ok((content, rest));
            }
            Some((_, '\\')) => {
                // Skip one character of escape content.
                if iter.next().is_none() {
                    return Err("unterminated escape at end of string literal".into());
                }
            }
            Some(_) => {}
        }
    }
}

/// Scan a language tag after `@`. Returns `(tag, rest)`.
fn scan_lang_tag(s: &str) -> Result<(&str, &str), String> {
    // BCP 47 language tag: [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*
    if s.is_empty() {
        return Err("empty language tag after '@'".into());
    }
    let mut iter = s.char_indices().peekable();
    // Primary subtag: one or more letters.
    let first_char = iter.peek().map_or('\0', |(_, c)| *c);
    if !first_char.is_ascii_alphabetic() {
        return Err(format!(
            "language tag must start with a letter, found '{first_char}'"
        ));
    }
    let mut end = s.len();
    for (i, c) in iter {
        if c == '-' || c.is_ascii_alphanumeric() {
            // ok
        } else {
            end = i;
            break;
        }
    }
    // Tags must not end in '-'.
    let tag = s[..end].trim_end_matches('-');
    if tag.is_empty() {
        return Err("language tag is empty after trimming trailing hyphens".into());
    }
    Ok((tag, &s[tag.len()..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_lines(s: &str) -> Vec<&str> {
        lines(s).map(|(_, l)| l).collect()
    }

    #[test]
    fn line_splitting_lf() {
        assert_eq!(collect_lines("a\nb\nc"), vec!["a", "b", "c"]);
    }

    #[test]
    fn line_splitting_crlf() {
        assert_eq!(collect_lines("a\r\nb\r\nc"), vec!["a", "b", "c"]);
    }

    #[test]
    fn line_splitting_bare_cr() {
        assert_eq!(collect_lines("a\rb\rc"), vec!["a", "b", "c"]);
    }

    #[test]
    fn bom_stripped() {
        let with_bom = "\u{FEFF}hello\nworld";
        let ls: Vec<_> = lines(with_bom).map(|(_, l)| l).collect();
        assert_eq!(ls, vec!["hello", "world"]);
    }

    #[test]
    fn tokenise_simple_triple() {
        let toks = tokenise("<http://a> <http://b> <http://c> .").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Iri("http://a"),
                Token::Iri("http://b"),
                Token::Iri("http://c"),
                Token::Dot,
            ]
        );
    }

    #[test]
    fn tokenise_literal_with_lang() {
        let toks = tokenise(r#"<s> <p> "hello"@en ."#).unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Iri("s"),
                Token::Iri("p"),
                Token::StringLiteral("hello"),
                Token::LangTag("en"),
                Token::Dot,
            ]
        );
    }

    #[test]
    fn tokenise_literal_with_datatype() {
        let toks = tokenise(r#"<s> <p> "42"^^<http://www.w3.org/2001/XMLSchema#integer> ."#)
            .unwrap();
        assert_eq!(
            toks[2],
            Token::StringLiteral("42"),
        );
        assert_eq!(toks[3], Token::DatatypeSep);
    }

    #[test]
    fn tokenise_blank_node() {
        let toks = tokenise("_:foo <p> _:bar .").unwrap();
        assert_eq!(toks[0], Token::BlankNode("foo"));
        assert_eq!(toks[2], Token::BlankNode("bar"));
    }
}
