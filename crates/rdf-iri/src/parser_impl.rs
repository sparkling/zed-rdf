//! Bridge from [`Iri`] to the frozen [`rdf_diff::Parser`] trait surface.
//!
//! Per ADR-0020 §1.4 every parser (main, shadow, oracle) implements this
//! trait so the diff harness can compare them. For `rdf-iri`, the
//! mapping is: one input IRI → one [`rdf_diff::Fact`].
//!
//! The fact shape is:
//!
//! - `subject`   = the parsed IRI wrapped in angle brackets.
//! - `predicate` = the sentinel `<urn:x-rdf-iri:parses-to>`.
//! - `object`    = the *normalised* IRI wrapped in angle brackets.
//! - `graph`     = `None`.
//!
//! This lets the harness surface divergences both at the parse-accept
//! layer (via `Diagnostics`) and at the normalisation layer (via
//! `ObjectMismatch`).

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

use crate::Iri;

/// Zero-sized parser handle implementing [`rdf_diff::Parser`].
#[derive(Debug, Clone, Copy, Default)]
pub struct IriParser;

impl IriParser {
    /// Stable parser id used in diff reports.
    pub const ID: &'static str = "rdf-iri";
}

impl Parser for IriParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let text = match std::str::from_utf8(input) {
            Ok(s) => s,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("IRI-SYNTAX-001: invalid UTF-8: {e}")],
                    fatal: true,
                });
            }
        };

        let iri = Iri::parse(text).map_err(|d| Diagnostics {
            messages: vec![format!("{}: {}", d.code, d.message)],
            fatal: true,
        })?;

        if !iri.is_absolute() {
            return Err(Diagnostics {
                messages: vec![
                    "IRI-SYNTAX-001: input is a relative reference; the diff harness \
                     requires an absolute IRI"
                        .to_owned(),
                ],
                fatal: true,
            });
        }

        let normalised = iri.normalise();

        let fact = Fact {
            subject: format!("<{}>", iri.as_str()),
            predicate: "<urn:x-rdf-iri:parses-to>".to_owned(),
            object: format!("<{}>", normalised.as_str()),
            graph: None,
        };
        let prov = FactProvenance {
            offset: Some(0),
            parser: Self::ID.to_owned(),
        };

        let facts = Facts::canonicalise([(fact, prov)], BTreeMap::new());
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: Vec::new(),
                fatal: false,
            },
        })
    }

    fn id(&self) -> &'static str {
        Self::ID
    }
}
