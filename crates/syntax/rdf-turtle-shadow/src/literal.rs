//! Typed-literal construction for Turtle 1.1.
//!
//! Turtle numeric literals are assigned XSD datatypes per the grammar:
//!
//! | Turtle production    | XSD datatype                                        |
//! |----------------------|-----------------------------------------------------|
//! | `INTEGER`            | `http://www.w3.org/2001/XMLSchema#integer`          |
//! | `DECIMAL`            | `http://www.w3.org/2001/XMLSchema#decimal`          |
//! | `DOUBLE`             | `http://www.w3.org/2001/XMLSchema#double`           |
//! | `true` / `false`     | `http://www.w3.org/2001/XMLSchema#boolean`          |
//! | `"…"^^<iri>`         | The specified IRI                                   |
//! | `"…"@lang`           | `http://www.w3.org/1999/02/22-rdf-syntax-ns#langString` |
//! | `"…"` (plain)        | `http://www.w3.org/2001/XMLSchema#string`           |
//!
//! Canonical literal form for the diff harness: `"lexical"^^<datatype-iri>` or
//! `"lexical"@lang-tag` (language tag normalised to lower-case per BCP 47).

/// XSD namespace prefix.
pub const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
/// RDF namespace prefix.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Construct the canonical representation for a typed literal.
#[must_use]
pub fn typed_literal(lexical: &str, datatype_iri: &str) -> String {
    format!("\"{}\"^^<{}>", escape_lexical(lexical), datatype_iri)
}

/// Construct the canonical representation for a language-tagged literal.
/// Language tag is normalised to lower-case.
#[must_use]
pub fn lang_literal(lexical: &str, lang: &str) -> String {
    format!("\"{}\"@{}", escape_lexical(lexical), lang.to_lowercase())
}

/// Construct a plain string literal (xsd:string).
#[must_use]
pub fn string_literal(lexical: &str) -> String {
    typed_literal(lexical, &format!("{XSD}string"))
}

/// Construct an xsd:integer literal from its lexical form (as written in
/// the Turtle document). No normalisation — preserve lexical form.
#[must_use]
pub fn integer_literal(lexical: &str) -> String {
    typed_literal(lexical, &format!("{XSD}integer"))
}

/// Construct an xsd:decimal literal from its lexical form.
#[must_use]
pub fn decimal_literal(lexical: &str) -> String {
    typed_literal(lexical, &format!("{XSD}decimal"))
}

/// Construct an xsd:double literal from its lexical form.
#[must_use]
pub fn double_literal(lexical: &str) -> String {
    typed_literal(lexical, &format!("{XSD}double"))
}

/// Construct an xsd:boolean literal.
#[must_use]
pub fn boolean_literal(value: bool) -> String {
    typed_literal(if value { "true" } else { "false" }, &format!("{XSD}boolean"))
}

/// Escape the lexical form for embedding in a `"…"` string.
/// We escape `"` and `\` only; the canonical form uses double-quoted strings.
fn escape_lexical(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer() {
        let s = integer_literal("42");
        assert_eq!(s, "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>");
    }

    #[test]
    fn decimal() {
        let s = decimal_literal("3.14");
        assert_eq!(s, "\"3.14\"^^<http://www.w3.org/2001/XMLSchema#decimal>");
    }

    #[test]
    fn boolean_true() {
        let s = boolean_literal(true);
        assert_eq!(s, "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>");
    }

    #[test]
    fn lang_lowercased() {
        let s = lang_literal("hello", "EN");
        assert_eq!(s, "\"hello\"@en");
    }

    #[test]
    fn escape_quotes() {
        let s = string_literal("say \"hi\"");
        assert!(s.contains("\\\""));
    }
}
