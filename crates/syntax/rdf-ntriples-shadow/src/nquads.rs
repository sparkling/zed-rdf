//! N-Quads parser — implements [`rdf_diff::Parser`].
//!
//! Grammar reference: <https://www.w3.org/TR/n-quads/#n-quads-grammar>
//!
//! N-Quads extends N-Triples with an optional graph name between the object
//! and the statement-terminating `.`:
//!
//! ```text
//! nquadsDoc  ::= statement? (EOL statement?)* EOF
//! statement  ::= subject WS+ predicate WS+ object (WS+ graphLabel)? WS* '.' WS*
//! graphLabel ::= IRIREF | BLANK_NODE_LABEL
//! ```
//!
//! This module re-uses the lexer and unescape infrastructure from
//! [`crate::ntriples`]. The shared tokenisation and subject/predicate/object
//! parsing is delegated to the N-Triples module.

use rdf_diff::{Diagnostics, ParseOutcome, Parser};

use crate::ntriples::parse_input;

/// The shadow N-Quads parser.
///
/// Implements [`Parser`] and is gated by the `shadow` feature.
#[derive(Debug, Default)]
pub struct NQuadsParser {
    _priv: (),
}

impl NQuadsParser {
    /// Create a new parser instance.
    #[must_use]
    pub const fn new() -> Self {
        Self { _priv: () }
    }
}

impl Parser for NQuadsParser {
    fn id(&self) -> &'static str {
        "rdf-nquads-shadow"
    }

    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // Delegate to the shared parser, indicating quad mode so graph names
        // are extracted.
        parse_input(input, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdf_diff::Fact;

    fn parse_nq(input: &str) -> Result<ParseOutcome, Diagnostics> {
        NQuadsParser::new().parse(input.as_bytes())
    }

    fn facts(input: &str) -> Vec<Fact> {
        parse_nq(input)
            .expect("parse should succeed")
            .facts
            .set
            .into_keys()
            .collect()
    }

    #[test]
    fn quad_with_graph() {
        let fs = facts(
            "<http://s> <http://p> <http://o> <http://g> .\n",
        );
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].graph, Some("<http://g>".to_owned()));
    }

    #[test]
    fn quad_without_graph() {
        let fs = facts("<http://s> <http://p> <http://o> .\n");
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].graph, None);
    }

    #[test]
    fn multiple_quads_mixed_graphs() {
        let fs = facts(concat!(
            "<s1> <p> <o1> <g1> .\n",
            "<s2> <p> <o2> .\n",
            "<s3> <p> <o3> <g2> .\n",
        ));
        assert_eq!(fs.len(), 3);
        let graphs: Vec<_> = fs.iter().map(|f| f.graph.clone()).collect();
        assert!(graphs.contains(&Some("<g1>".to_owned())));
        assert!(graphs.contains(&None));
        assert!(graphs.contains(&Some("<g2>".to_owned())));
    }

    #[test]
    fn bom_stripped_in_quads() {
        let with_bom = "\u{FEFF}<s> <p> <o> <g> .\n";
        let fs = facts(with_bom);
        assert_eq!(fs.len(), 1);
    }

    #[test]
    fn crlf_in_quads() {
        let fs = facts("<s1> <p> <o1> <g> .\r\n<s2> <p> <o2> <g> .\r\n");
        assert_eq!(fs.len(), 2);
    }

    #[test]
    fn literal_in_quad() {
        let fs = facts(r#"<s> <p> "hello"@en <g> ."#);
        assert_eq!(fs.len(), 1);
        assert!(fs[0].object.contains("@en"));
        assert_eq!(fs[0].graph, Some("<g>".to_owned()));
    }

    #[test]
    fn unicode_escape_in_graph_iri() {
        // Graph IRI with \u0041 = 'A'
        let fs = facts(r#"<s> <p> <o> <http://ex\u0041mple/g> ."#);
        assert_eq!(fs[0].graph, Some("<http://exAmple/g>".to_owned()));
    }

    #[test]
    fn parser_id() {
        assert_eq!(NQuadsParser::new().id(), "rdf-nquads-shadow");
    }
}
