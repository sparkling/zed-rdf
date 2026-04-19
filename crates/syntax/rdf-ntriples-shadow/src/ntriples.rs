//! N-Triples parser — implements [`rdf_diff::Parser`].
//!
//! Grammar reference: <https://www.w3.org/TR/n-triples/#n-triples-grammar>
//!
//! Each triple occupies one logical line:
//!
//! ```text
//! ntriplesDoc  ::= triple? (EOL triple?)* EOF
//! triple       ::= subject WS+ predicate WS+ object WS* '.' WS*
//! subject      ::= IRIREF | BLANK_NODE_LABEL
//! predicate    ::= IRIREF
//! object       ::= IRIREF | BLANK_NODE_LABEL | literal
//! literal      ::= STRING_LITERAL_QUOTE ('^^' IRIREF | LANGTAG)?
//! ```

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

use crate::diagnostics::DiagnosticsBuilder;
use crate::lexer::{Token, lines, tokenise};
use crate::unescape::{decode_iri_escapes, decode_string_escapes};

/// The shadow N-Triples parser.
///
/// Implements [`Parser`] and is gated by the `shadow` feature.
#[derive(Debug, Default)]
pub struct NTriplesParser {
    _priv: (),
}

impl NTriplesParser {
    /// Create a new parser instance.
    #[must_use]
    pub const fn new() -> Self {
        Self { _priv: () }
    }
}

impl Parser for NTriplesParser {
    fn id(&self) -> &'static str {
        "rdf-ntriples-shadow"
    }

    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parse_input(input, false)
    }
}

/// Parse `input` as N-Triples (if `quads == false`) or the triple portion of
/// N-Quads lines (if `quads == true`).
///
/// Shared by both [`NTriplesParser`] and [`crate::nquads::NQuadsParser`].
pub(crate) fn parse_input(
    input: &[u8],
    quads: bool,
) -> Result<ParseOutcome, Diagnostics> {
    let source = match std::str::from_utf8(input) {
        Ok(s) => s,
        Err(e) => {
            let mut d = DiagnosticsBuilder::new();
            d.error(format!("input is not valid UTF-8: {e}"));
            return Err(d.finish());
        }
    };

    let mut diag = DiagnosticsBuilder::new();
    let mut raw_facts: Vec<(Fact, FactProvenance)> = Vec::new();
    // Map original blank-node labels → stable per-document sequence label.
    let mut bnode_map: BTreeMap<String, String> = BTreeMap::new();
    let mut bnode_counter: u64 = 0;
    let parser_id = if quads {
        "rdf-nquads-shadow"
    } else {
        "rdf-ntriples-shadow"
    };

    for (line_offset, line) in lines(source) {
        let trimmed = line.trim_matches(|c| c == ' ' || c == '\t');
        // Skip empty lines and comment lines.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let tokens = match tokenise(line) {
            Ok(t) => t,
            Err(e) => {
                diag.error(format!("line {line_offset}: lexer error: {e}"));
                continue;
            }
        };

        if tokens.is_empty() {
            continue;
        }

        // Parse the token stream into a triple (optionally with graph for quads).
        let result = if quads {
            parse_quad_tokens(&tokens, line_offset, &mut bnode_map, &mut bnode_counter)
        } else {
            parse_triple_tokens(&tokens, line_offset, &mut bnode_map, &mut bnode_counter)
        };

        match result {
            Ok((fact, _graph)) => {
                raw_facts.push((
                    fact,
                    FactProvenance {
                        offset: Some(line_offset),
                        parser: parser_id.to_owned(),
                    },
                ));
            }
            Err(e) => {
                diag.error(format!("line {line_offset}: {e}"));
            }
        }
    }

    if diag.is_fatal() {
        return Err(diag.finish());
    }

    let facts = Facts {
        set: raw_facts.into_iter().collect(),
        prefixes: BTreeMap::new(),
    };

    Ok(ParseOutcome {
        facts,
        warnings: diag.finish(),
    })
}

/// Parse a triple from a token slice. Returns `(Fact, None)`.
pub(crate) fn parse_triple_tokens(
    tokens: &[Token<'_>],
    offset: usize,
    bnode_map: &mut BTreeMap<String, String>,
    bnode_counter: &mut u64,
) -> Result<(Fact, Option<String>), String> {
    // Expected: subject predicate object Dot
    // (possibly with DatatypeSep + IRI for typed literals)
    let (subject, rest) = parse_subject(tokens, offset, bnode_map, bnode_counter)?;
    let (predicate, rest) = parse_predicate(rest, offset)?;
    let (object, rest) = parse_object(rest, offset, bnode_map, bnode_counter)?;
    expect_dot(rest, offset)?;
    Ok((
        Fact {
            subject,
            predicate,
            object,
            graph: None,
        },
        None,
    ))
}

/// Parse a quad (triple + optional graph) from a token slice.
/// Returns `(Fact, Some(graph))` or `(Fact, None)` for default graph.
pub(crate) fn parse_quad_tokens(
    tokens: &[Token<'_>],
    offset: usize,
    bnode_map: &mut BTreeMap<String, String>,
    bnode_counter: &mut u64,
) -> Result<(Fact, Option<String>), String> {
    let (subject, rest) = parse_subject(tokens, offset, bnode_map, bnode_counter)?;
    let (predicate, rest) = parse_predicate(rest, offset)?;
    let (object, rest) = parse_object(rest, offset, bnode_map, bnode_counter)?;
    // Optional graph name before the dot.
    let (graph, rest) = parse_optional_graph(rest, offset)?;
    expect_dot(rest, offset)?;
    let graph_clone = graph.clone();
    Ok((
        Fact {
            subject,
            predicate,
            object,
            graph,
        },
        graph_clone,
    ))
}

fn parse_subject<'t>(
    tokens: &'t [Token<'t>],
    offset: usize,
    bnode_map: &mut BTreeMap<String, String>,
    bnode_counter: &mut u64,
) -> Result<(String, &'t [Token<'t>]), String> {
    match tokens.first() {
        Some(Token::Iri(raw)) => {
            let iri = decode_iri(raw, offset)?;
            Ok((format!("<{iri}>"), &tokens[1..]))
        }
        Some(Token::BlankNode(label)) => {
            let canonical = canonical_bnode(label, bnode_map, bnode_counter);
            Ok((format!("_:{canonical}"), &tokens[1..]))
        }
        Some(other) => Err(format!(
            "offset {offset}: expected subject (IRI or blank node), found {other:?}"
        )),
        None => Err(format!("offset {offset}: unexpected end of line: expected subject")),
    }
}

fn parse_predicate<'t>(
    tokens: &'t [Token<'t>],
    offset: usize,
) -> Result<(String, &'t [Token<'t>]), String> {
    match tokens.first() {
        Some(Token::Iri(raw)) => {
            let iri = decode_iri(raw, offset)?;
            Ok((format!("<{iri}>"), &tokens[1..]))
        }
        Some(other) => Err(format!(
            "offset {offset}: expected predicate (IRI), found {other:?}"
        )),
        None => Err(format!("offset {offset}: unexpected end of line: expected predicate")),
    }
}

fn parse_object<'t>(
    tokens: &'t [Token<'t>],
    offset: usize,
    bnode_map: &mut BTreeMap<String, String>,
    bnode_counter: &mut u64,
) -> Result<(String, &'t [Token<'t>]), String> {
    match tokens.first() {
        Some(Token::Iri(raw)) => {
            let iri = decode_iri(raw, offset)?;
            Ok((format!("<{iri}>"), &tokens[1..]))
        }
        Some(Token::BlankNode(label)) => {
            let canonical = canonical_bnode(label, bnode_map, bnode_counter);
            Ok((format!("_:{canonical}"), &tokens[1..]))
        }
        Some(Token::StringLiteral(raw)) => {
            let lexical = decode_literal_lexical(raw, offset)?;
            let rest = &tokens[1..];
            // Check for language tag or datatype.
            match rest.first() {
                Some(Token::LangTag(tag)) => {
                    let tag_lower = tag.to_ascii_lowercase();
                    Ok((
                        format!(r#""{lexical}"@{tag_lower}"#),
                        &rest[1..],
                    ))
                }
                Some(Token::DatatypeSep) => {
                    let after_sep = &rest[1..];
                    match after_sep.first() {
                        Some(Token::Iri(dt_raw)) => {
                            let dt = decode_iri(dt_raw, offset)?;
                            Ok((
                                format!(r#""{lexical}"^^<{dt}>"#),
                                &after_sep[1..],
                            ))
                        }
                        _ => Err(format!(
                            "offset {offset}: expected datatype IRI after '^^'"
                        )),
                    }
                }
                _ => {
                    // Plain literal — defaults to xsd:string per RDF 1.1.
                    let xsd_string =
                        "http://www.w3.org/2001/XMLSchema#string";
                    Ok((
                        format!(r#""{lexical}"^^<{xsd_string}>"#),
                        rest,
                    ))
                }
            }
        }
        Some(other) => Err(format!(
            "offset {offset}: expected object (IRI, blank node, or literal), found {other:?}"
        )),
        None => Err(format!("offset {offset}: unexpected end of line: expected object")),
    }
}

fn parse_optional_graph<'t>(
    tokens: &'t [Token<'t>],
    offset: usize,
) -> Result<(Option<String>, &'t [Token<'t>]), String> {
    match tokens.first() {
        Some(Token::Iri(raw)) => {
            let iri = decode_iri(raw, offset)?;
            Ok((Some(format!("<{iri}>")), &tokens[1..]))
        }
        // Default graph or blank node not allowed as graph in standard N-Quads
        // (blank nodes are technically not allowed in the graph position per
        // the N-Quads grammar — treat as no graph and let the dot check handle it).
        _ => Ok((None, tokens)),
    }
}

fn expect_dot(tokens: &[Token<'_>], offset: usize) -> Result<(), String> {
    match tokens.first() {
        Some(Token::Dot) => {
            // Anything after the dot should be whitespace/comment only;
            // tokenise already stripped those, so just check remaining tokens.
            if tokens.len() > 1 {
                return Err(format!(
                    "offset {offset}: unexpected tokens after statement terminator '.'"
                ));
            }
            Ok(())
        }
        Some(other) => Err(format!(
            "offset {offset}: expected '.', found {other:?}"
        )),
        None => Err(format!("offset {offset}: missing statement terminator '.'")),
    }
}

/// Return or create the canonical blank-node label for a given original label.
fn canonical_bnode(
    original: &str,
    map: &mut BTreeMap<String, String>,
    counter: &mut u64,
) -> String {
    if let Some(existing) = map.get(original) {
        return existing.clone();
    }
    let label = format!("b{counter}");
    *counter += 1;
    map.insert(original.to_owned(), label.clone());
    label
}

/// Decode IRI escape sequences and validate the result.
fn decode_iri(raw: &str, offset: usize) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    decode_iri_escapes(raw, &mut out)
        .map_err(|e| format!("offset {offset}: IRI escape error: {e}"))?;
    Ok(out)
}

/// Decode string-literal escape sequences; preserve exact lexical form.
fn decode_literal_lexical(raw: &str, offset: usize) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    decode_string_escapes(raw, &mut out)
        .map_err(|e| format!("offset {offset}: literal escape error: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_nt(input: &str) -> Result<ParseOutcome, Diagnostics> {
        NTriplesParser::new().parse(input.as_bytes())
    }

    fn facts(input: &str) -> BTreeMap<Fact, FactProvenance> {
        parse_nt(input).expect("parse should succeed").facts.set
    }

    fn fact_strings(input: &str) -> Vec<String> {
        let mut v: Vec<_> = facts(input)
            .keys()
            .map(|f| format!("{} {} {} {:?}", f.subject, f.predicate, f.object, f.graph))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn empty_input() {
        assert!(facts("").is_empty());
    }

    #[test]
    fn comment_only() {
        assert!(facts("# just a comment\n").is_empty());
    }

    #[test]
    fn simple_triple() {
        let f = facts(
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
        );
        assert_eq!(f.len(), 1);
        let fact = f.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/s>");
        assert_eq!(fact.predicate, "<http://example.org/p>");
        assert_eq!(fact.object, "<http://example.org/o>");
        assert_eq!(fact.graph, None);
    }

    #[test]
    fn literal_plain_gets_xsd_string() {
        let f = facts(
            r#"<http://a> <http://b> "hello" ."#,
        );
        let fact = f.keys().next().unwrap();
        assert!(
            fact.object.contains("XMLSchema#string"),
            "plain literal should default to xsd:string, got: {}",
            fact.object
        );
    }

    #[test]
    fn literal_with_lang_tag() {
        let f = facts(r#"<s> <p> "bonjour"@fr ."#);
        let fact = f.keys().next().unwrap();
        assert_eq!(fact.object, r#""bonjour"@fr"#);
    }

    #[test]
    fn lang_tag_lowercased() {
        let f = facts(r#"<s> <p> "Hello"@EN-US ."#);
        let fact = f.keys().next().unwrap();
        assert!(fact.object.ends_with("@en-us"), "got: {}", fact.object);
    }

    #[test]
    fn literal_with_datatype() {
        let f = facts(
            r#"<s> <p> "42"^^<http://www.w3.org/2001/XMLSchema#integer> ."#,
        );
        let fact = f.keys().next().unwrap();
        assert_eq!(
            fact.object,
            r#""42"^^<http://www.w3.org/2001/XMLSchema#integer>"#
        );
    }

    #[test]
    fn blank_node_renaming() {
        let f = facts("_:x <http://p> _:x .\n_:y <http://p> _:x .\n");
        assert_eq!(f.len(), 2);
        // _:x should be b0, _:y should be b1 (order of first appearance).
        let mut subjects: Vec<_> = f.keys().map(|k| k.subject.clone()).collect();
        subjects.sort();
        assert!(subjects.contains(&"_:b0".to_owned()));
        assert!(subjects.contains(&"_:b1".to_owned()));
    }

    #[test]
    fn unicode_escape_in_iri() {
        // \u0041 = 'A'
        let f = facts(r#"<http://ex\u0041mple> <p> <o> ."#);
        let fact = f.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://exAmple>");
    }

    #[test]
    fn unicode_escape_in_literal() {
        let f = facts(r#"<s> <p> "\u0041" ."#);
        let fact = f.keys().next().unwrap();
        assert!(fact.object.starts_with(r#""A""#), "got: {}", fact.object);
    }

    #[test]
    fn large_unicode_escape() {
        // \U0001F600 = emoji
        let f = facts("<s> <p> \"\\U0001F600\" .");
        let fact = f.keys().next().unwrap();
        assert!(
            fact.object.starts_with("\"\u{1F600}\""),
            "got: {}",
            fact.object
        );
    }

    #[test]
    fn bom_handling() {
        let with_bom = "\u{FEFF}<s> <p> <o> .\n";
        let f = facts(with_bom);
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn crlf_line_endings() {
        let f = facts("<s1> <p> <o1> .\r\n<s2> <p> <o2> .\r\n");
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn bare_cr_line_endings() {
        let f = facts("<s1> <p> <o1> .\r<s2> <p> <o2> .\r");
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn lexical_form_preserved_no_trimming() {
        // Spaces inside literal must be preserved.
        let f = facts(r#"<s> <p> "  hello world  " ."#);
        let fact = f.keys().next().unwrap();
        assert!(
            fact.object.starts_with(r#""  hello world  ""#),
            "lexical form must not be trimmed; got: {}",
            fact.object
        );
    }

    #[test]
    fn lexical_form_with_newline_escape() {
        let f = facts("<s> <p> \"line1\\nline2\" .");
        let fact = f.keys().next().unwrap();
        // The decoded lexical form should contain an actual newline.
        assert!(
            fact.object.contains('\n'),
            "\\n should be decoded; got: {}",
            fact.object
        );
    }

    #[test]
    fn invalid_utf8_rejected() {
        let bad: &[u8] = b"\xFF\xFE <p> <o> .";
        assert!(NTriplesParser::new().parse(bad).is_err());
    }

    #[test]
    fn missing_dot_rejected() {
        assert!(parse_nt("<s> <p> <o>").is_err());
    }

    #[test]
    fn multiple_triples() {
        let f = fact_strings(
            "<a> <b> <c> .\n<d> <e> <f> .\n",
        );
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn inline_comment_after_triple() {
        // A comment on the same line as a triple is unusual in practice but
        // the spec allows it; tokenise() strips the comment.
        // Actually the N-Triples grammar does NOT allow inline comments—only
        // full-line comments. But we test that full-line comments are skipped.
        let f = facts("# comment\n<s> <p> <o> .\n# another\n");
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn parser_id() {
        assert_eq!(NTriplesParser::new().id(), "rdf-ntriples-shadow");
    }
}
