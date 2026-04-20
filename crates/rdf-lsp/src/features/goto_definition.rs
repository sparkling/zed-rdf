//! Goto-definition handler — resolves prefix declarations in Turtle/TriG.
//!
//! Phase F scope: if the cursor is on a prefixed name like `ex:foo`, find the
//! `@prefix ex: <...>` declaration in the document and return its location.
//! Non-Turtle/TriG languages always return `None`.

use lsp_types::{Location, Position, Range, Url};

use crate::Language;

/// Attempt to resolve the prefix declaration for the prefixed name under
/// the cursor.
///
/// Returns `Some(Location)` pointing to the `@prefix` directive line when
/// a match is found, otherwise `None`.
///
/// The `uri` is required to construct the `Location`; callers (the dispatch
/// layer) supply the document URI.  For Phase F we synthesise a file URI
/// placeholder so the signature matches the frozen interface — callers that
/// have the real URI should wrap this and replace the returned URL.
#[must_use]
pub fn handle_goto_definition(
    text: &str,
    lang: Language,
    pos: Position,
) -> Option<Location> {
    match lang {
        Language::Turtle | Language::TriG | Language::N3 => {
            goto_prefix_decl(text, pos)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Prefix-declaration resolution
// ---------------------------------------------------------------------------

/// Extract the prefix name from a prefixed-name token at `pos`.
///
/// E.g. `ex:Foo` yields `"ex"`, `rdf:type` yields `"rdf"`.
fn prefix_at_cursor(text: &str, pos: Position) -> Option<String> {
    let line_idx = pos.line as usize;
    let char_idx = pos.character as usize;

    let line = text.lines().nth(line_idx)?;

    // Find the byte offset of the cursor within the line (UTF-8, not UTF-16;
    // Phase F approximation — close enough for ASCII-heavy RDF documents).
    let cursor_byte: usize = line
        .char_indices()
        .take(char_idx)
        .map(|(_, c)| c.len_utf8())
        .sum();

    let bytes = line.as_bytes();
    if cursor_byte > bytes.len() {
        return None;
    }

    // Expand left to find the start of the token.
    let mut start = cursor_byte;
    while start > 0 && is_pname_byte(bytes[start - 1]) {
        start -= 1;
    }

    // Expand right to find the end.
    let mut end = cursor_byte;
    while end < bytes.len() && is_pname_byte(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let token = &line[start..end];
    // A prefixed name contains exactly one `:` and neither part is empty.
    let colon_pos = token.find(':')?;
    if colon_pos == 0 {
        return None; // no prefix part
    }
    Some(token[..colon_pos].to_owned())
}

/// Returns `true` for bytes that can appear in a prefixed name.
const fn is_pname_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' || b == b':'
}

/// Scan the document for `@prefix <prefix>: <iri>` or
/// `PREFIX <prefix>: <iri>` (SPARQL style in Turtle 1.1 `sparqlPrefix`).
/// Returns the zero-based line number of the declaration if found.
fn find_prefix_decl_line(text: &str, prefix: &str) -> Option<usize> {
    let needle_at = format!("@prefix {prefix}:");
    let needle_prefix = format!("PREFIX {prefix}:");

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(needle_at.as_str())
            || trimmed
                .to_uppercase()
                .starts_with(needle_prefix.to_uppercase().as_str())
        {
            return Some(idx);
        }
    }
    None
}

/// Build a zero-width `Location` pointing to the start of the `@prefix` line.
fn goto_prefix_decl(text: &str, pos: Position) -> Option<Location> {
    let prefix = prefix_at_cursor(text, pos)?;
    let decl_line = find_prefix_decl_line(text, &prefix)?;

    // Phase F: we do not have the document URI available in the feature
    // signature, so we return a synthetic file:///unknown URI.  The dispatch
    // layer (pf-lsp-protocol) is expected to replace this with the real URI
    // before sending the response.
    let uri = Url::parse("file:///unknown").ok()?;
    let decl_line_u32 = u32::try_from(decl_line).unwrap_or(u32::MAX);
    let range = Range {
        start: Position {
            line: decl_line_u32,
            character: 0,
        },
        end: Position {
            line: decl_line_u32,
            character: 0,
        },
    };
    Some(Location { uri, range })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TURTLE_DOC: &str = "\
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:Alice rdf:type ex:Person .
";

    #[test]
    fn goto_def_resolves_ex_prefix() {
        // Cursor on `ex` in `ex:Alice` (line 3, char 0)
        let pos = Position { line: 3, character: 0 };
        let result = handle_goto_definition(TURTLE_DOC, Language::Turtle, pos);
        assert!(result.is_some(), "expected a location for ex: prefix");
        let loc = result.unwrap();
        assert_eq!(loc.range.start.line, 0, "expected line 0 for @prefix ex:");
    }

    #[test]
    fn goto_def_resolves_rdf_prefix() {
        // Cursor on `rdf` in `rdf:type` (line 3, char 9)
        let pos = Position { line: 3, character: 9 };
        let result = handle_goto_definition(TURTLE_DOC, Language::Turtle, pos);
        assert!(result.is_some(), "expected a location for rdf: prefix");
        let loc = result.unwrap();
        assert_eq!(loc.range.start.line, 1, "expected line 1 for @prefix rdf:");
    }

    #[test]
    fn goto_def_returns_none_for_sparql() {
        let result = handle_goto_definition(
            "SELECT ?s WHERE { ?s ?p ?o }",
            Language::Sparql,
            Position { line: 0, character: 0 },
        );
        assert!(result.is_none(), "SPARQL goto-def is not supported in Phase F");
    }

    #[test]
    fn goto_def_returns_none_for_unknown_prefix() {
        // `unk:` is not declared; cursor on line 1 at char 0
        let doc = "@prefix ex: <http://example.org/> .\nunk:Alice a ex:Person .\n";
        let result = handle_goto_definition(
            doc,
            Language::Turtle,
            Position { line: 1, character: 0 },
        );
        assert!(result.is_none());
    }
}
