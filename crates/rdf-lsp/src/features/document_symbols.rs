//! Document symbols handler — extracts named symbols from RDF/query documents.
//!
//! Phase F uses heuristic line-based scanning rather than a full re-parse.
//! Each language extracts the subset of named symbols that appear at the
//! top-level scope of the document.

use lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

use crate::Language;

/// Extract document symbols from `text` for the given language.
///
/// Returns a flat list of `DocumentSymbol` items.  Returns an empty list for
/// languages where symbol extraction is not specified (NT, NQ, RDF/XML,
/// JSON-LD, `TriX`).
#[must_use]
#[allow(deprecated)] // DocumentSymbol::deprecated field is itself deprecated
pub fn handle_document_symbols(text: &str, lang: Language) -> Vec<DocumentSymbol> {
    match lang {
        // N3 uses the same Turtle-like subject syntax heuristic.
        Language::Turtle | Language::TriG | Language::N3 => turtle_symbols(text),
        Language::Sparql => sparql_symbols(text),
        Language::ShEx => shex_symbols(text),
        Language::Datalog => datalog_symbols(text),
        Language::NTriples
        | Language::NQuads
        | Language::RdfXml
        | Language::JsonLd
        | Language::TriX => vec![],
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `DocumentSymbol` spanning the given line.
#[allow(deprecated)]
fn symbol(name: &str, kind: SymbolKind, line: u32) -> DocumentSymbol {
    let range = Range {
        start: Position { line, character: 0 },
        end: Position { line, character: u32::MAX },
    };
    DocumentSymbol {
        name: name.to_owned(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    }
}

/// Convert a line index (`usize`) to a `u32`, clamping at `u32::MAX` for
/// documents that exceed 4 billion lines.
fn line_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Turtle / TriG / N3
// ---------------------------------------------------------------------------

/// Extract subject IRIs from lines that begin with `<` (full IRI) or a
/// prefixed name (e.g. `ex:Foo`).
///
/// Lines that start with whitespace (continuation lines), `@prefix`, `@base`,
/// `#` (comments), or `}` (`TriG` graph close) are skipped.
fn turtle_symbols(text: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('@')
            || trimmed.starts_with('}')
            || line.starts_with(' ')
            || line.starts_with('\t')
        {
            continue;
        }

        // Extract the subject token (up to the first whitespace).
        let subject = trimmed.split_whitespace().next().unwrap_or("");
        if subject.starts_with('<') || subject.contains(':') {
            let name = subject.trim_matches(|c| c == '<' || c == '>');
            if !name.is_empty() {
                symbols.push(symbol(name, SymbolKind::MODULE, line_u32(line_no)));
            }
        }
    }
    symbols
}

// ---------------------------------------------------------------------------
// SPARQL
// ---------------------------------------------------------------------------

/// Extract variable names (`?varname`) from the document.
fn sparql_symbols(text: &str) -> Vec<DocumentSymbol> {
    let mut seen = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        for token in line.split_whitespace() {
            if let Some(var) = token.strip_prefix('?') {
                // Strip trailing punctuation that may be attached.
                let var = var.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
                if !var.is_empty() && seen.insert(var.to_owned()) {
                    symbols.push(symbol(
                        &format!("?{var}"),
                        SymbolKind::VARIABLE,
                        line_u32(line_no),
                    ));
                }
            }
        }
    }
    symbols
}

// ---------------------------------------------------------------------------
// ShEx
// ---------------------------------------------------------------------------

/// Extract shape labels: lines starting with `<` followed by content and `{`.
fn shex_symbols(text: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('<') && trimmed.contains('{') {
            // Grab everything between `<` and `>`.
            if let Some(close) = trimmed.find('>') {
                let label = &trimmed[1..close];
                if !label.is_empty() {
                    symbols.push(symbol(label, SymbolKind::CLASS, line_u32(line_no)));
                }
            }
        }
    }
    symbols
}

// ---------------------------------------------------------------------------
// Datalog
// ---------------------------------------------------------------------------

/// Extract relation names from rule heads.
///
/// Heuristic: a token before `(` that appears on the left-hand side of `:-`
/// or at the end of a line ending with `.` is a head predicate.
fn datalog_symbols(text: &str) -> Vec<DocumentSymbol> {
    let mut seen = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') || trimmed.starts_with('#') {
            continue;
        }

        // Head is left of `:-` or the whole line (facts end with `.`).
        let head_part = trimmed.split(":-").next().unwrap_or(trimmed).trim();

        // Relation name is the token before the first `(`.
        if let Some(paren_pos) = head_part.find('(') {
            let rel = head_part[..paren_pos].trim();
            if !rel.is_empty()
                && rel.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && seen.insert(rel.to_owned())
            {
                symbols.push(symbol(rel, SymbolKind::FUNCTION, line_u32(line_no)));
            }
        }
    }
    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_subject_iri_extracted() {
        let text =
            "@prefix ex: <http://example.org/> .\n<http://example.org/Alice> a ex:Person .\n";
        let syms = handle_document_symbols(text, Language::Turtle);
        assert!(
            syms.iter().any(|s| s.name.contains("Alice")),
            "expected Alice IRI in symbols; got {syms:?}"
        );
    }

    #[test]
    fn turtle_prefix_lines_excluded() {
        let text = "@prefix ex: <http://example.org/> .\n";
        let syms = handle_document_symbols(text, Language::Turtle);
        assert!(
            !syms.iter().any(|s| s.name.contains("prefix")),
            "@prefix lines should not be symbols"
        );
    }

    #[test]
    fn sparql_variables_extracted() {
        let text = "SELECT ?s ?p ?o WHERE { ?s ?p ?o . }";
        let syms = handle_document_symbols(text, Language::Sparql);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"?s"), "expected ?s variable");
        assert!(names.contains(&"?p"), "expected ?p variable");
    }

    #[test]
    fn shex_shape_labels_extracted() {
        let text = "<PersonShape> {\n  sh:property [ sh:path foaf:name ] ;\n}\n";
        let syms = handle_document_symbols(text, Language::ShEx);
        assert!(
            syms.iter().any(|s| s.name == "PersonShape"),
            "expected PersonShape; got {syms:?}"
        );
    }

    #[test]
    fn datalog_relation_names_extracted() {
        let text =
            "ancestor(X, Y) :- parent(X, Y).\nancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).\n";
        let syms = handle_document_symbols(text, Language::Datalog);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"ancestor"), "expected 'ancestor' relation");
    }

    #[test]
    fn ntriples_returns_empty() {
        let text =
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
        let syms = handle_document_symbols(text, Language::NTriples);
        assert!(syms.is_empty(), "N-Triples should have no symbols");
    }
}
