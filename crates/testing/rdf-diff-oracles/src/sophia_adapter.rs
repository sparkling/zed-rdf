//! [`rdf_diff::Parser`] adapter over `sophia_turtle`.
//!
//! Sophia is the **secondary** Turtle-family reference per ADR-0019 §1
//! (oxttl is the primary). Having a second independently-authored
//! Turtle parser in the ensemble is what lets the harness distinguish
//! "oxigraph bug" from "spec ambiguity".
//!
//! ## Scope
//!
//! This adapter wraps `sophia_turtle::parser::turtle::TurtleParser`
//! over the `sophia_api::source::TripleSource` abstraction, collects
//! each yielded triple, and converts it to the sweep's inline
//! canonical form. Only `Turtle` is wired in — `N-Triples` / `N-Quads`
//! / `TriG` can be added as sibling modules without touching this file.
//!
//! Per ADR-0019 §1, `sophia_*` is a **`[dev-dependency]` only** and
//! **optional** in the oracle ensemble. This module is gated behind
//! `#[cfg(all(test, feature = "oracle-sophia"))]`.

use std::collections::BTreeMap;

use sophia_api::parser::TripleParser;
use sophia_api::source::TripleSource;
use sophia_api::term::{Term as SophiaTerm, TermKind};
use sophia_api::triple::Triple as SophiaTriple;
use sophia_turtle::parser::turtle::TurtleParser as SophiaTurtleParser;

use crate::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// Adapter type for `sophia_turtle::parser::turtle::TurtleParser`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Adapter;

impl Adapter {
    /// Construct a fresh Sophia Turtle adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

const PARSER_ID: &str = "sophia-oracle";

impl Parser for Adapter {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let parser = SophiaTurtleParser { base: None };
        let mut source = parser.parse(input);

        let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();

        // `for_each_triple` drains the source. The `Err` branch
        // carries the upstream's error type unchanged; we surface it
        // as fatal [`Diagnostics`] via the crate-local `fatal` helper.
        let drain = source.for_each_triple(|triple| {
            raw.push(triple_to_fact(&triple));
        });

        if let Err(err) = drain {
            return Err(crate::fatal(PARSER_ID, err));
        }

        let facts = Facts::canonicalise(raw, BTreeMap::new());
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: Vec::new(),
                fatal: false,
            },
        })
    }

    fn id(&self) -> &'static str {
        PARSER_ID
    }
}

/// Translate a sophia triple into the sweep's inline `(Fact,
/// Provenance)` pair. Sophia exposes triples through the [`SophiaTriple`]
/// trait (`.s() / .p() / .o()`), and every component implements
/// [`SophiaTerm`]; the dispatch in [`term_to_string`] is therefore
/// uniform.
fn triple_to_fact<T: SophiaTriple>(triple: &T) -> (Fact, FactProvenance) {
    (
        Fact {
            subject: term_to_string(&triple.s()),
            predicate: term_to_string(&triple.p()),
            object: term_to_string(&triple.o()),
            graph: None,
        },
        FactProvenance {
            offset: None,
            parser: PARSER_ID.to_string(),
        },
    )
}

/// Render a sophia term in the sweep's inline pre-canonical form.
/// We format IRIs as `<iri>`, blank nodes as `_:label`, and literals
/// as `"lex"^^<dt>` or `"lex"@lang` — matching oxrdf's `Display` for
/// cross-oracle symmetry under `Facts::canonicalise`.
fn term_to_string<T: SophiaTerm>(term: &T) -> String {
    match term.kind() {
        TermKind::Iri => term
            .iri()
            .map_or_else(|| "<>".to_string(), |iri| format!("<{}>", iri.as_str())),
        TermKind::BlankNode => term
            .bnode_id()
            .map_or_else(|| "_:_".to_string(), |id| format!("_:{}", id.as_str())),
        TermKind::Literal => {
            let lex = term
                .lexical_form()
                .map(|m| m.to_string())
                .unwrap_or_default();
            if let Some(lang) = term.language_tag() {
                format!("\"{lex}\"@{}", lang.as_str())
            } else if let Some(dt) = term.datatype() {
                format!("\"{lex}\"^^<{}>", dt.as_str())
            } else {
                format!("\"{lex}\"")
            }
        }
        // RDF-1.2 / n3-only kinds (triple, variable) are not expected
        // from a Turtle-1.1 parser; if one ever shows up, surface it
        // opaquely so canonicalisation can flag the divergence.
        other => format!("<unsupported-sophia-term:{other:?}>"),
    }
}
