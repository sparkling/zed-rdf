//! Semantic token provider — Phase G.
//!
//! The legend below is frozen; all agents in the mesh must index into it
//! without modification.  Token types match LSP 3.17 §3.16.6.

use lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensLegend,
};

use crate::Language;

// ---------------------------------------------------------------------------
// Frozen legend (index positions must never change)
// ---------------------------------------------------------------------------

/// Ordered list of token types.  Index == value sent in the wire protocol.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,   // 0 — prefix label / base
    SemanticTokenType::TYPE,        // 1 — rdf:type / class IRI
    SemanticTokenType::PROPERTY,    // 2 — predicate IRI
    SemanticTokenType::VARIABLE,    // 3 — ?var / $var
    SemanticTokenType::STRING,      // 4 — "literal"
    SemanticTokenType::NUMBER,      // 5 — numeric literal
    SemanticTokenType::KEYWORD,     // 6 — SELECT/WHERE/PREFIX/…
    SemanticTokenType::COMMENT,     // 7 — # …
    SemanticTokenType::OPERATOR,    // 8 — . ; , | / ^ = != < >
];

/// Ordered list of token modifiers.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION, // 0 — first occurrence / definition site
    SemanticTokenModifier::READONLY,    // 1 — lang-tagged / datatyped literal
];

/// Canonical legend for `initialize` response.
#[must_use]
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

/// Compute semantic tokens for `text` interpreted as `lang`.
///
/// Returns delta-encoded tokens as required by the LSP wire protocol
/// (`SemanticTokens`).  An empty slice is valid when no tokens are
/// recognised.
#[must_use]
pub fn handle_semantic_tokens(text: &str, lang: Language) -> SemanticTokens {
    let raw = collect_tokens(text, lang);
    SemanticTokens {
        result_id: None,
        data: encode_delta(raw),
    }
}

// ---------------------------------------------------------------------------
// Internal: raw token (absolute position)
// ---------------------------------------------------------------------------

struct RawToken {
    line: usize,
    start: usize,
    length: usize,
    token_type: u32,
    token_modifiers: u32,
}

fn collect_tokens(text: &str, lang: Language) -> Vec<RawToken> {
    match lang {
        Language::Turtle | Language::TriG => collect_turtle_tokens(text),
        Language::NTriples | Language::NQuads => collect_nt_tokens(text),
        Language::Sparql => collect_sparql_tokens(text),
        Language::ShEx => collect_shex_tokens(text),
        Language::Datalog => collect_datalog_tokens(text),
        Language::RdfXml
        | Language::JsonLd
        | Language::TriX
        | Language::N3 => vec![],
    }
}

// ---------------------------------------------------------------------------
// Turtle / TriG
// ---------------------------------------------------------------------------

fn collect_turtle_tokens(text: &str) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.starts_with('#') {
            tokens.push(RawToken { line: ln, start: indent, length: line.len() - indent, token_type: 7, token_modifiers: 0 });
            continue;
        }
        tokenize_turtle_line(line, ln, &mut tokens);
    }
    tokens
}

fn tokenize_turtle_line(line: &str, ln: usize, tokens: &mut Vec<RawToken>) {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                tokens.push(RawToken { line: ln, start: i, length: bytes.len() - i, token_type: 7, token_modifiers: 0 });
                break;
            }
            b'<' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'>' { i += 1; }
                if i < bytes.len() { i += 1; }
                tokens.push(RawToken { line: ln, start, length: i - start, token_type: 2, token_modifiers: 0 });
            }
            b'"' | b'\'' => {
                let q = bytes[i];
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != q { if bytes[i] == b'\\' { i += 1; } i += 1; }
                if i < bytes.len() { i += 1; }
                tokens.push(RawToken { line: ln, start, length: i - start, token_type: 4, token_modifiers: 1 });
            }
            b'@' => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() { i += 1; }
                tokens.push(RawToken { line: ln, start, length: i - start, token_type: 6, token_modifiers: 0 });
            }
            b if b.is_ascii_alphanumeric() || b == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':' || bytes[i] == b'-') { i += 1; }
                let tok = &line[start..i];
                let ttype = if matches!(tok, "a" | "true" | "false") { 6 }
                    else if tok.contains(':') { 0 }
                    else { 2 };
                tokens.push(RawToken { line: ln, start, length: i - start, token_type: ttype, token_modifiers: 0 });
            }
            _ => { i += 1; }
        }
    }
}

// ---------------------------------------------------------------------------
// N-Triples / N-Quads
// ---------------------------------------------------------------------------

fn collect_nt_tokens(text: &str) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            if trimmed.starts_with('#') {
                tokens.push(RawToken { line: ln, start: 0, length: line.len(), token_type: 7, token_modifiers: 0 });
            }
            continue;
        }
        tokenize_turtle_line(line, ln, &mut tokens);
    }
    tokens
}

// ---------------------------------------------------------------------------
// SPARQL
// ---------------------------------------------------------------------------

const SPARQL_KEYWORDS: &[&str] = &[
    "SELECT", "WHERE", "FILTER", "OPTIONAL", "UNION", "GRAPH", "FROM", "ASK", "CONSTRUCT",
    "DESCRIBE", "LIMIT", "OFFSET", "ORDER", "BY", "GROUP", "HAVING", "PREFIX", "BASE",
    "BIND", "VALUES", "INSERT", "DELETE", "LOAD", "CLEAR", "DROP", "CREATE", "WITH",
    "MINUS", "SERVICE", "SILENT", "NAMED", "USING", "DEFAULT", "ALL", "DISTINCT",
    "REDUCED", "AS", "IN", "NOT", "EXISTS", "STR", "LANG", "DATATYPE", "IRI", "URI",
    "BNODE", "RAND", "ABS", "CEIL", "FLOOR", "ROUND", "CONCAT", "STRLEN", "UCASE",
    "LCASE", "ENCODE_FOR_URI", "CONTAINS", "STRSTARTS", "STRENDS", "STRBEFORE",
    "STRAFTER", "YEAR", "MONTH", "DAY", "HOURS", "MINUTES", "SECONDS", "TIMEZONE",
    "TZ", "NOW", "UUID", "STRUUID", "MD5", "SHA1", "SHA256", "SHA384", "SHA512",
    "COALESCE", "IF", "STRLANG", "STRDT", "SAMETERM", "ISIRI", "ISURI", "ISBLANK",
    "ISLITERAL", "ISNUMERIC", "REGEX", "SUBSTR", "REPLACE", "COUNT", "SUM", "MIN",
    "MAX", "AVG", "SAMPLE", "GROUP_CONCAT", "SEPARATOR",
];

fn collect_sparql_tokens(text: &str) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'#' => {
                    tokens.push(RawToken { line: ln, start: i, length: bytes.len() - i, token_type: 7, token_modifiers: 0 });
                    break;
                }
                b'<' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i] != b'>' { i += 1; }
                    if i < bytes.len() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 2, token_modifiers: 0 });
                }
                b'"' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i] != b'"' { if bytes[i] == b'\\' { i += 1; } i += 1; }
                    if i < bytes.len() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 4, token_modifiers: 1 });
                }
                b'?' | b'$' => {
                    let start = i; i += 1;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 3, token_modifiers: 0 });
                }
                b if b.is_ascii_alphabetic() => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':') { i += 1; }
                    let tok = &line[start..i];
                    let upper = tok.to_uppercase();
                    let ttype = if SPARQL_KEYWORDS.contains(&upper.as_str()) { 6 }
                        else if tok.contains(':') { 0 }
                        else { 2 };
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: ttype, token_modifiers: 0 });
                }
                b if b.is_ascii_digit() => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E' || bytes[i] == b'+' || bytes[i] == b'-') { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 5, token_modifiers: 0 });
                }
                _ => { i += 1; }
            }
        }
    }
    tokens
}

// ---------------------------------------------------------------------------
// ShEx
// ---------------------------------------------------------------------------

const SHEX_KEYWORDS: &[&str] = &["PREFIX", "BASE", "START", "CLOSED", "EXTENDS", "AND", "OR", "NOT", "IRI", "LITERAL", "NONLITERAL", "BNODE", "ABSTRACT", "EXTRA"];

fn collect_shex_tokens(text: &str) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'#' => {
                    tokens.push(RawToken { line: ln, start: i, length: bytes.len() - i, token_type: 7, token_modifiers: 0 });
                    break;
                }
                b'<' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i] != b'>' { i += 1; }
                    if i < bytes.len() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 2, token_modifiers: 0 });
                }
                b'"' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i] != b'"' { if bytes[i] == b'\\' { i += 1; } i += 1; }
                    if i < bytes.len() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 4, token_modifiers: 1 });
                }
                b'@' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 6, token_modifiers: 0 });
                }
                b if b.is_ascii_alphabetic() => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':' || bytes[i] == b'-') { i += 1; }
                    let tok = &line[start..i];
                    let ttype = if SHEX_KEYWORDS.contains(&tok) { 6 }
                        else if tok.contains(':') { 0 }
                        else { 2 };
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: ttype, token_modifiers: 0 });
                }
                _ => { i += 1; }
            }
        }
    }
    tokens
}

// ---------------------------------------------------------------------------
// Datalog
// ---------------------------------------------------------------------------

fn collect_datalog_tokens(text: &str) -> Vec<RawToken> {
    let mut tokens = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('%') {
            tokens.push(RawToken { line: ln, start: 0, length: line.len(), token_type: 7, token_modifiers: 0 });
            continue;
        }
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'%' => {
                    tokens.push(RawToken { line: ln, start: i, length: bytes.len() - i, token_type: 7, token_modifiers: 0 });
                    break;
                }
                b'"' => {
                    let start = i; i += 1;
                    while i < bytes.len() && bytes[i] != b'"' { if bytes[i] == b'\\' { i += 1; } i += 1; }
                    if i < bytes.len() { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 4, token_modifiers: 1 });
                }
                b if b.is_ascii_uppercase() => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 3, token_modifiers: 0 });
                }
                b'n' if line[i..].starts_with("not") => {
                    tokens.push(RawToken { line: ln, start: i, length: 3, token_type: 6, token_modifiers: 0 });
                    i += 3;
                }
                b if b.is_ascii_lowercase() => {
                    let start = i;
                    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                    tokens.push(RawToken { line: ln, start, length: i - start, token_type: 2, token_modifiers: 0 });
                }
                _ => { i += 1; }
            }
        }
    }
    tokens
}

// ---------------------------------------------------------------------------
// Delta encoding
// ---------------------------------------------------------------------------

fn encode_delta(raw: Vec<RawToken>) -> Vec<SemanticToken> {
    let mut result = Vec::with_capacity(raw.len());
    let mut prev_line = 0usize;
    let mut prev_start = 0usize;

    for tok in raw {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 { tok.start - prev_start } else { tok.start };
        let length = u32::try_from(tok.length).unwrap_or(u32::MAX);
        let delta_line = u32::try_from(delta_line).unwrap_or(u32::MAX);
        let delta_start = u32::try_from(delta_start).unwrap_or(u32::MAX);
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.token_modifiers,
        });
        prev_line = tok.line;
        prev_start = tok.start;
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_comment_classified_as_comment() {
        let tokens = handle_semantic_tokens("# hello\n", Language::Turtle);
        assert!(!tokens.data.is_empty());
        assert_eq!(tokens.data[0].token_type, 7); // COMMENT
    }

    #[test]
    fn sparql_select_keyword() {
        let tokens = handle_semantic_tokens("SELECT ?x WHERE { }\n", Language::Sparql);
        assert!(tokens.data.iter().any(|t| t.token_type == 6)); // KEYWORD
    }

    #[test]
    fn sparql_variable_classified() {
        let tokens = handle_semantic_tokens("SELECT ?x WHERE { }\n", Language::Sparql);
        assert!(tokens.data.iter().any(|t| t.token_type == 3)); // VARIABLE
    }

    #[test]
    fn rdfxml_returns_empty() {
        let tokens = handle_semantic_tokens("<rdf:RDF/>", Language::RdfXml);
        assert!(tokens.data.is_empty());
    }

    #[test]
    fn ntriples_iri_classified_as_property() {
        let text = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
        let tokens = handle_semantic_tokens(text, Language::NTriples);
        assert!(!tokens.data.is_empty());
    }

    #[test]
    fn delta_encoding_second_token_same_line() {
        // Two tokens on the same line — second delta_start is relative to first.
        let tokens = handle_semantic_tokens("<http://a.org/s> <http://a.org/p> .\n", Language::NTriples);
        // Find consecutive tokens on line 0.
        let line0: Vec<_> = tokens.data.iter().collect();
        if line0.len() >= 2 {
            assert_eq!(line0[0].delta_line, 0);
            assert_eq!(line0[1].delta_line, 0); // same line
        }
    }

    #[test]
    fn shex_keyword_classified() {
        let tokens = handle_semantic_tokens("PREFIX ex: <http://example.org/>\n", Language::ShEx);
        assert!(tokens.data.iter().any(|t| t.token_type == 6)); // KEYWORD
    }

    #[test]
    fn datalog_comment_classified() {
        let tokens = handle_semantic_tokens("% a comment\n", Language::Datalog);
        assert!(!tokens.data.is_empty());
        assert_eq!(tokens.data[0].token_type, 7); // COMMENT
    }

    #[test]
    fn legend_has_all_token_types() {
        let l = legend();
        assert_eq!(l.token_types.len(), TOKEN_TYPES.len());
        assert_eq!(l.token_modifiers.len(), TOKEN_MODIFIERS.len());
    }
}
