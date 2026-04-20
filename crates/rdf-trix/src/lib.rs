//! Main `TriX` parser — Phase B implementation.
//!
//! Agent `pb-rdf-trix` fills this in. The public surface is the
//! [`TriXParser`] type implementing [`rdf_diff::Parser`].
//!
//! # Pinned spec reading
//! `TriX`: Triples in XML (informal spec, HP Labs 2004):
//! <https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html>
//!
//! # Design note (ADR-0021 §Context)
//! `TriX` is an XML wrapper around N-Triples-style content. This crate
//! ships its own minimal streaming XML tokeniser built on `quick-xml`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::too_many_lines, clippy::option_if_let_else)]

use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// The `TriX` namespace URI.
const TRIX_NS: &str = "http://www.w3.org/2004/03/trix/trix-1/";

/// Main `TriX` parser.
///
/// Stateless — construct with [`TriXParser::new`] and reuse across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct TriXParser;

impl TriXParser {
    /// Construct a fresh `TriX` parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Parser for TriXParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parse_trix(input)
    }

    fn id(&self) -> &'static str {
        "rdf-trix"
    }
}

// ---------------------------------------------------------------------------
// Internal parser implementation
// ---------------------------------------------------------------------------

/// Local state machine for parsing `TriX` XML.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    /// Waiting for `<TriX>` root element.
    Root,
    /// Inside `<TriX>` element.
    InTriX,
    /// Inside `<graph>` element; holds optional graph IRI.
    InGraph { graph_iri: Option<String> },
    /// Inside `<triple>` element; holds accumulated terms and the graph.
    InTriple {
        graph_iri: Option<String>,
        terms: Vec<RdfTerm>,
    },
    /// Collecting text content for a `<uri>` element.
    InUri {
        graph_iri: Option<String>,
        terms: Vec<RdfTerm>,
        /// True if this `<uri>` is the graph name declaration in `<graph>`.
        is_graph_name: bool,
        text: String,
    },
    /// Collecting text content for a `<bnode>` element.
    InBNode {
        graph_iri: Option<String>,
        terms: Vec<RdfTerm>,
        text: String,
    },
    /// Collecting text content for a `<plainLiteral>` element.
    InPlainLiteral {
        graph_iri: Option<String>,
        terms: Vec<RdfTerm>,
        lang: Option<String>,
        text: String,
    },
    /// Collecting text content for a `<typedLiteral>` element.
    InTypedLiteral {
        graph_iri: Option<String>,
        terms: Vec<RdfTerm>,
        datatype: String,
        text: String,
    },
}

/// A resolved RDF term with canonical form ready for `Fact`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RdfTerm {
    /// Named node: stored as `<iri>`.
    Iri(String),
    /// Blank node: stored as `_:label`.
    BNode(String),
    /// Literal: stored as `"lex"`, `"lex"@lang`, or `"lex"^^<iri>`.
    Literal(String),
}

impl RdfTerm {
    fn into_canonical(self) -> String {
        match self {
            Self::Iri(s) | Self::BNode(s) | Self::Literal(s) => s,
        }
    }
}

/// Parse `TriX` input bytes into a [`ParseOutcome`].
fn parse_trix(input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);

    let mut state = State::Root;
    let mut facts: Vec<(Fact, FactProvenance)> = Vec::new();
    let mut buf = Vec::new();

    loop {
        let event = reader.read_event_into(&mut buf).map_err(|e| Diagnostics {
            messages: vec![format!("XML parse error: {e}")],
            fatal: true,
        })?;

        match event {
            Event::Eof => break,

            Event::Start(ref e) => {
                let local = local_name_str(e.name().as_ref());
                let pos = reader.buffer_position();

                state = match state {
                    State::Root => {
                        if local == "TriX" {
                            // Validate namespace
                            let ns = get_namespace_attr(e);
                            if ns.as_deref() != Some(TRIX_NS) {
                                return Err(Diagnostics {
                                    messages: vec![format!(
                                        "TriX root element missing required xmlns=\"{TRIX_NS}\""
                                    )],
                                    fatal: true,
                                });
                            }
                            State::InTriX
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "expected root element <TriX>, got <{local}>"
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InTriX => {
                        if local == "graph" {
                            State::InGraph { graph_iri: None }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected element <{local}> inside <TriX>"
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InGraph { ref graph_iri } => {
                        let g = graph_iri.clone();
                        match local.as_str() {
                            "uri" => {
                                // A <uri> directly inside <graph> is the graph name
                                State::InUri {
                                    graph_iri: g,
                                    terms: vec![],
                                    is_graph_name: true,
                                    text: String::new(),
                                }
                            }
                            "triple" => State::InTriple {
                                graph_iri: g,
                                terms: vec![],
                            },
                            _ => {
                                return Err(Diagnostics {
                                    messages: vec![format!(
                                        "unexpected element <{local}> inside <graph>"
                                    )],
                                    fatal: true,
                                });
                            }
                        }
                    }

                    State::InTriple {
                        ref graph_iri,
                        ref terms,
                    } => {
                        let g = graph_iri.clone();
                        let t = terms.clone();
                        match local.as_str() {
                            "uri" => State::InUri {
                                graph_iri: g,
                                terms: t,
                                is_graph_name: false,
                                text: String::new(),
                            },
                            "bnode" => State::InBNode {
                                graph_iri: g,
                                terms: t,
                                text: String::new(),
                            },
                            "plainLiteral" => {
                                let lang = get_xml_lang_attr(e);
                                State::InPlainLiteral {
                                    graph_iri: g,
                                    terms: t,
                                    lang,
                                    text: String::new(),
                                }
                            }
                            "typedLiteral" => {
                                let datatype =
                                    get_attr_value(e, b"datatype", &reader).ok_or_else(|| {
                                        Diagnostics {
                                            messages: vec![
                                                "typedLiteral missing datatype attribute".into(),
                                            ],
                                            fatal: true,
                                        }
                                    })?;
                                State::InTypedLiteral {
                                    graph_iri: g,
                                    terms: t,
                                    datatype,
                                    text: String::new(),
                                }
                            }
                            _ => {
                                return Err(Diagnostics {
                                    messages: vec![format!(
                                        "unexpected element <{local}> inside <triple> at byte {pos}"
                                    )],
                                    fatal: true,
                                });
                            }
                        }
                    }

                    other => {
                        return Err(Diagnostics {
                            messages: vec![format!(
                                "unexpected start element <{local}> in state {other:?}"
                            )],
                            fatal: true,
                        });
                    }
                };
            }

            Event::Empty(ref e) => {
                // Handle self-closing elements: only <uri/>, <bnode/> can be
                // empty in practice (empty IRI or bnode label edge case).
                let local = local_name_str(e.name().as_ref());
                // An empty element is treated as a start+end with empty text.
                // Currently, any self-closing non-TriX element is an error
                // unless it's a zero-content term.
                match state {
                    State::InGraph { graph_iri: _ } => {
                        if local == "triple" {
                            // Empty triple — no terms, skip (invalid but non-fatal)
                        } else if local == "uri" {
                            // Empty graph-name uri — empty IRI string: keep as default
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected empty element <{local}/> inside <graph>"
                                )],
                                fatal: true,
                            });
                        }
                    }
                    _ => {
                        // Other empty elements are unexpected
                        return Err(Diagnostics {
                            messages: vec![format!(
                                "unexpected empty element <{local}/>"
                            )],
                            fatal: true,
                        });
                    }
                }
            }

            Event::End(ref e) => {
                let local = local_name_str(e.name().as_ref());

                state = match state {
                    State::InTriX => {
                        if local == "TriX" {
                            break;
                        }
                        return Err(Diagnostics {
                            messages: vec![format!("unexpected </{}> inside <TriX>", local)],
                            fatal: true,
                        });
                    }

                    State::InGraph { graph_iri: _ } => {
                        if local == "graph" {
                            State::InTriX
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <graph>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InTriple {
                        ref graph_iri,
                        ref terms,
                    } => {
                        if local == "triple" {
                            if terms.len() == 3 {
                                let g = graph_iri.clone();
                                let subject = terms[0].clone().into_canonical();
                                let predicate = terms[1].clone().into_canonical();
                                let object = terms[2].clone().into_canonical();
                                let fact = Fact {
                                    subject,
                                    predicate,
                                    object,
                                    graph: g,
                                };
                                facts.push((
                                    fact,
                                    FactProvenance {
                                        offset: None,
                                        parser: "rdf-trix".to_owned(),
                                    },
                                ));
                                State::InGraph {
                                    graph_iri: graph_iri.clone(),
                                }
                            } else {
                                return Err(Diagnostics {
                                    messages: vec![format!(
                                        "triple must have exactly 3 terms, got {}",
                                        terms.len()
                                    )],
                                    fatal: true,
                                });
                            }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <triple>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InUri {
                        ref graph_iri,
                        ref terms,
                        is_graph_name,
                        ref text,
                    } => {
                        if local == "uri" {
                            let iri = format!("<{}>", text.trim());
                            if is_graph_name {
                                // This <uri> was the graph name declaration
                                State::InGraph {
                                    graph_iri: Some(iri),
                                }
                            } else {
                                let mut new_terms = terms.clone();
                                new_terms.push(RdfTerm::Iri(iri));
                                State::InTriple {
                                    graph_iri: graph_iri.clone(),
                                    terms: new_terms,
                                }
                            }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <uri>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InBNode {
                        ref graph_iri,
                        ref terms,
                        ref text,
                    } => {
                        if local == "bnode" {
                            let label = format!("_:{}", text.trim());
                            let mut new_terms = terms.clone();
                            new_terms.push(RdfTerm::BNode(label));
                            State::InTriple {
                                graph_iri: graph_iri.clone(),
                                terms: new_terms,
                            }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <bnode>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InPlainLiteral {
                        ref graph_iri,
                        ref terms,
                        ref lang,
                        ref text,
                    } => {
                        if local == "plainLiteral" {
                            let lit = if let Some(tag) = lang {
                                format!("\"{}\"@{}", escape_literal(text), tag)
                            } else {
                                format!("\"{}\"", escape_literal(text))
                            };
                            let mut new_terms = terms.clone();
                            new_terms.push(RdfTerm::Literal(lit));
                            State::InTriple {
                                graph_iri: graph_iri.clone(),
                                terms: new_terms,
                            }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <plainLiteral>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    State::InTypedLiteral {
                        ref graph_iri,
                        ref terms,
                        ref datatype,
                        ref text,
                    } => {
                        if local == "typedLiteral" {
                            let lit = format!(
                                "\"{}\"^^<{}>",
                                escape_literal(text),
                                datatype
                            );
                            let mut new_terms = terms.clone();
                            new_terms.push(RdfTerm::Literal(lit));
                            State::InTriple {
                                graph_iri: graph_iri.clone(),
                                terms: new_terms,
                            }
                        } else {
                            return Err(Diagnostics {
                                messages: vec![format!(
                                    "unexpected </{}> inside <typedLiteral>",
                                    local
                                )],
                                fatal: true,
                            });
                        }
                    }

                    other @ State::Root => {
                        return Err(Diagnostics {
                            messages: vec![format!(
                                "unexpected end element </{}> in state {:?}",
                                local, other
                            )],
                            fatal: true,
                        });
                    }
                };
            }

            Event::Text(ref e) => {
                let text = e.unescape().map_err(|e| Diagnostics {
                    messages: vec![format!("XML text decode error: {e}")],
                    fatal: true,
                })?;
                match &mut state {
                    State::InUri { text: buf, .. }
                    | State::InBNode { text: buf, .. }
                    | State::InPlainLiteral { text: buf, .. }
                    | State::InTypedLiteral { text: buf, .. } => buf.push_str(&text),
                    // Whitespace text nodes between elements are OK to ignore
                    _ => {}
                }
            }

            Event::CData(ref e) => {
                let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                match &mut state {
                    State::InUri { text: buf, .. }
                    | State::InBNode { text: buf, .. }
                    | State::InPlainLiteral { text: buf, .. }
                    | State::InTypedLiteral { text: buf, .. } => buf.push_str(&text),
                    _ => {}
                }
            }

            // Processing instructions, comments, declarations are silently skipped.
            Event::PI(_) | Event::Comment(_) | Event::Decl(_) | Event::DocType(_) => {}
        }

        buf.clear();
    }

    // Final state check: we must have consumed the document properly.
    match state {
        State::InTriX | State::Root => {
            // Root is OK if we got an EOF before seeing <TriX>
            if matches!(state, State::Root) {
                return Err(Diagnostics {
                    messages: vec!["no <TriX> root element found".into()],
                    fatal: true,
                });
            }
        }
        other => {
            return Err(Diagnostics {
                messages: vec![format!("unexpected EOF in state {other:?}")],
                fatal: true,
            });
        }
    }

    let canonical = Facts::canonicalise(facts, BTreeMap::new());
    Ok(ParseOutcome {
        facts: canonical,
        warnings: Diagnostics {
            messages: vec![],
            fatal: false,
        },
    })
}

// ---------------------------------------------------------------------------
// XML attribute helpers
// ---------------------------------------------------------------------------

/// Extract the local name (strip namespace prefix if any) from a raw element name byte slice.
fn local_name_str(name: &[u8]) -> String {
    // quick-xml returns names like `TriX`, `graph`, or `ns:triple`
    // We want just the local part after the last `:`.
    let s = std::str::from_utf8(name).unwrap_or("");
    s.rfind(':').map_or_else(|| s.to_owned(), |pos| s[pos + 1..].to_owned())
}

/// Retrieve the `xmlns` attribute value from a start element.
fn get_namespace_attr(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        if key == "xmlns" {
            return attr
                .unescape_value()
                .ok()
                .map(std::borrow::Cow::into_owned);
        }
    }
    None
}

/// Retrieve the `xml:lang` attribute value from a start element.
fn get_xml_lang_attr(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        if key == "xml:lang" || key == "lang" {
            return attr
                .unescape_value()
                .ok()
                .map(std::borrow::Cow::into_owned)
                .filter(|v| !v.is_empty());
        }
    }
    None
}

/// Retrieve an attribute value by name from a start element.
fn get_attr_value(
    e: &quick_xml::events::BytesStart<'_>,
    attr_name: &[u8],
    _reader: &Reader<&[u8]>,
) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == attr_name {
            return attr
                .unescape_value()
                .ok()
                .map(std::borrow::Cow::into_owned);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Literal escaping
// ---------------------------------------------------------------------------

/// Escape a literal lexical form for the canonical `"..."` representation.
/// We need to escape backslash and double-quote characters per the N-Triples
/// literal encoding used by the diff harness's canonical form.
fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
