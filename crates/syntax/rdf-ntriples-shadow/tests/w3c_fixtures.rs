//! W3C N-Triples and N-Quads fixture-based integration tests.
//!
//! These tests exercise the shadow parsers against a local manifest of
//! positive and negative test cases derived from the W3C test suites at:
//!
//! - <https://www.w3.org/2013/N-TriplesTests/>
//! - <https://www.w3.org/2013/N-QuadsTests/>
//!
//! Until the `v1-oracle-jvm` agent materialises the full fact corpus the
//! test inputs are embedded inline. The structure mirrors what the oracle
//! will eventually provide so migration is mechanical.

use rdf_diff::Parser;
use rdf_ntriples_shadow::ntriples::NTriplesParser;
use rdf_ntriples_shadow::nquads::NQuadsParser;

// ---------------------------------------------------------------------------
// Positive N-Triples tests — must parse without error
// ---------------------------------------------------------------------------

macro_rules! nt_ok {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            let result = NTriplesParser::new().parse($input.as_bytes());
            assert!(
                result.is_ok(),
                "expected parse success for {}, got: {:?}",
                stringify!($name),
                result.err()
            );
        }
    };
}

// W3C nt-syntax-file-01: empty document
nt_ok!(nt_syntax_file_01, "");

// W3C nt-syntax-file-02: only whitespace
nt_ok!(nt_syntax_file_02, "\n\n\n");

// W3C nt-syntax-file-03: only comments
nt_ok!(nt_syntax_file_03, "# comment\n");

// W3C nt-syntax-uri-01: three IRI triple
nt_ok!(
    nt_syntax_uri_01,
    "<http://example/s> <http://example/p> <http://example/o> .\n"
);

// W3C nt-syntax-uri-02: IRI with fragment
nt_ok!(
    nt_syntax_uri_02,
    "<http://example/s> <http://example/p#q> <http://example/o> .\n"
);

// W3C nt-syntax-uri-03: relative IRI (bare identifier — N-Triples allows any IRI ref)
nt_ok!(
    nt_syntax_uri_03,
    "<http://example/s> <http://example/p> <scheme:path?q#f> .\n"
);

// W3C nt-syntax-string-01: simple string literal
nt_ok!(
    nt_syntax_string_01,
    "<http://example/s> <http://example/p> \"string\" .\n"
);

// W3C nt-syntax-string-02: string literal with language tag
nt_ok!(
    nt_syntax_string_02,
    "<http://example/s> <http://example/p> \"string\"@en .\n"
);

// W3C nt-syntax-string-03: string literal with datatype
nt_ok!(
    nt_syntax_string_03,
    "<http://example/s> <http://example/p> \"string\"^^<http://www.w3.org/2001/XMLSchema#string> .\n"
);

// W3C nt-syntax-bnode-01: subject blank node
nt_ok!(
    nt_syntax_bnode_01,
    "_:b0 <http://example/p> <http://example/o> .\n"
);

// W3C nt-syntax-bnode-02: object blank node
nt_ok!(
    nt_syntax_bnode_02,
    "<http://example/s> <http://example/p> _:b0 .\n"
);

// W3C nt-syntax-bnode-03: both subject and object blank node
nt_ok!(
    nt_syntax_bnode_03,
    "_:s <http://example/p> _:o .\n"
);

// W3C nt-syntax-datatypes-01: integer datatype
nt_ok!(
    nt_syntax_datatypes_01,
    "<http://example/s> <http://example/p> \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
);

// W3C nt-syntax-datatypes-02: decimal datatype
nt_ok!(
    nt_syntax_datatypes_02,
    "<http://example/s> <http://example/p> \"1.0\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n"
);

// \uXXXX escape in IRI
nt_ok!(
    nt_syntax_unicode_escape_iri,
    "<http://example/\\u006F> <http://example/p> <http://example/o> .\n"
);

// \UXXXXXXXX escape in literal
nt_ok!(
    nt_syntax_big_u_escape_literal,
    "<s> <p> \"\\U0001F600\" .\n"
);

// BOM at start of file
nt_ok!(nt_syntax_bom, "\u{FEFF}<s> <p> <o> .\n");

// CRLF line endings
nt_ok!(nt_syntax_crlf, "<s> <p> <o> .\r\n<s2> <p> <o2> .\r\n");

// Bare CR line endings
nt_ok!(nt_syntax_bare_cr, "<s> <p> <o> .\r<s2> <p> <o2> .\r");

// Literal with embedded tab escape
nt_ok!(nt_syntax_literal_tab, "<s> <p> \"col1\\tcol2\" .\n");

// Literal with embedded newline escape
nt_ok!(nt_syntax_literal_newline, "<s> <p> \"line1\\nline2\" .\n");

// Literal lexical form preserved (spaces not trimmed)
nt_ok!(nt_syntax_literal_spaces, "<s> <p> \"  space  \" .\n");

// Multiple triples in a single document
nt_ok!(
    nt_syntax_multi,
    concat!(
        "<http://a> <http://b> <http://c> .\n",
        "<http://d> <http://e> <http://f> .\n",
        "<http://g> <http://h> <http://i> .\n",
    )
);

// ---------------------------------------------------------------------------
// Negative N-Triples tests — must reject
// ---------------------------------------------------------------------------

macro_rules! nt_err {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            let result = NTriplesParser::new().parse($input.as_bytes());
            assert!(
                result.is_err(),
                "expected parse failure for {}, but got Ok",
                stringify!($name)
            );
        }
    };
}

// Missing dot
nt_err!(nt_neg_missing_dot, "<s> <p> <o>");

// Only subject
nt_err!(nt_neg_only_subject, "<s> .");

// Invalid UTF-8
#[test]
fn nt_neg_invalid_utf8() {
    let bad: &[u8] = b"\xFF\xFE <p> <o> .";
    assert!(
        NTriplesParser::new().parse(bad).is_err(),
        "expected parse failure for invalid UTF-8"
    );
}

// Bad unicode escape: \uXYZW where W is non-hex
nt_err!(nt_neg_bad_u_escape, "<s> <p> \"\\u00GG\" .");

// Surrogate in \u escape (U+D800 is not a valid scalar)
nt_err!(nt_neg_surrogate_escape, "<s> <p> \"\\uD800\" .");

// Unterminated IRI
nt_err!(nt_neg_unterminated_iri, "<s> <p> <http://unterm .");

// Unterminated string literal
nt_err!(nt_neg_unterminated_literal, "<s> <p> \"no closing quote .");

// ---------------------------------------------------------------------------
// Positive N-Quads tests
// ---------------------------------------------------------------------------

macro_rules! nq_ok {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            let result = NQuadsParser::new().parse($input.as_bytes());
            assert!(
                result.is_ok(),
                "expected parse success for {}, got: {:?}",
                stringify!($name),
                result.err()
            );
        }
    };
}

// W3C nq-syntax-uri-01
nq_ok!(
    nq_syntax_uri_01,
    "<http://example/s> <http://example/p> <http://example/o> <http://example/g> .\n"
);

// W3C nq-syntax-uri-02: triple without graph (default graph)
nq_ok!(
    nq_syntax_uri_02,
    "<http://example/s> <http://example/p> <http://example/o> .\n"
);

// String literal in quad
nq_ok!(
    nq_syntax_string_01,
    "<s> <p> \"val\"@en <g> .\n"
);

// BNode subject
nq_ok!(
    nq_syntax_bnode_01,
    "_:b <http://p> <o> <g> .\n"
);

// BOM in N-Quads
nq_ok!(nq_syntax_bom, "\u{FEFF}<s> <p> <o> <g> .\n");

// Unicode escape in graph IRI
nq_ok!(
    nq_syntax_unicode_graph,
    "<s> <p> <o> <http://ex\\u0061mple/g> .\n"
);

// Multiple quads
nq_ok!(
    nq_syntax_multi,
    concat!(
        "<s1> <p> <o1> <g1> .\n",
        "<s2> <p> <o2> <g2> .\n",
        "<s3> <p> <o3> .\n",
    )
);

// ---------------------------------------------------------------------------
// Semantic invariant checks
// ---------------------------------------------------------------------------

#[test]
fn literal_lexical_form_preserved() {
    let input = r#"<s> <p> "  leading and trailing spaces  " ."#;
    let outcome = NTriplesParser::new().parse(input.as_bytes()).unwrap();
    let fact = outcome.facts.set.into_keys().next().unwrap();
    assert!(
        fact.object.contains("  leading and trailing spaces  "),
        "lexical form was trimmed; got: {}",
        fact.object
    );
}

#[test]
fn blank_node_relabelling_consistent() {
    // The same blank-node label in subject and object should map to the
    // same canonical label.
    let input = "_:x <http://p> _:x .\n";
    let outcome = NTriplesParser::new().parse(input.as_bytes()).unwrap();
    let fact = outcome.facts.set.into_keys().next().unwrap();
    assert_eq!(
        fact.subject, fact.object,
        "same BNode label in subject and object should canonicalise identically"
    );
}

#[test]
fn two_different_bnodes_get_different_labels() {
    let input = "_:a <http://p> _:b .\n";
    let outcome = NTriplesParser::new().parse(input.as_bytes()).unwrap();
    let fact = outcome.facts.set.into_keys().next().unwrap();
    assert_ne!(
        fact.subject, fact.object,
        "different BNode labels should produce different canonical labels"
    );
}

#[test]
fn nquad_graph_iri_captured() {
    let input = "<http://s> <http://p> <http://o> <http://g> .\n";
    let outcome = NQuadsParser::new().parse(input.as_bytes()).unwrap();
    let fact = outcome.facts.set.into_keys().next().unwrap();
    assert_eq!(fact.graph, Some("<http://g>".to_owned()));
}

#[test]
fn nquad_no_graph_gives_none() {
    let input = "<http://s> <http://p> <http://o> .\n";
    let outcome = NQuadsParser::new().parse(input.as_bytes()).unwrap();
    let fact = outcome.facts.set.into_keys().next().unwrap();
    assert_eq!(fact.graph, None);
}

#[test]
fn diagnostics_fatal_on_error() {
    let result = NTriplesParser::new().parse(b"<s> <p> <o>");
    match result {
        Err(d) => assert!(d.fatal, "diagnostics should be fatal on rejection"),
        Ok(_) => panic!("expected rejection"),
    }
}

#[test]
fn diagnostics_message_populated_on_error() {
    let result = NTriplesParser::new().parse(b"<s> <p> <o>");
    match result {
        Err(d) => assert!(
            !d.messages.is_empty(),
            "diagnostics messages should not be empty on rejection"
        ),
        Ok(_) => panic!("expected rejection"),
    }
}
