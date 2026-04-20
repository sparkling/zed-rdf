//! Snapshot tests for `TurtleWriter`.
//!
//! Each test feeds one `Fact` through `TurtleWriter` with no registered
//! prefixes and asserts byte-exact output.  The expected strings were derived
//! by reading the `write_term_ttl` / `write_literal_ttl` code paths in
//! `src/lib.rs`.
//!
//! When no prefixes are registered the Turtle output is identical to
//! N-Triples body format with a ` .\n` terminator — there is no `@prefix`
//! header because `ensure_header` only emits prefix lines when the map is
//! non-empty.

use rdf_diff::Fact;
use rdf_format::TurtleWriter;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Write a single fact through `TurtleWriter` with no prefix map and return
/// the UTF-8 string.
fn ttl_render_single(fact: &Fact) -> String {
    let mut buf = Vec::new();
    let mut w = TurtleWriter::new(&mut buf);
    w.write_fact(fact).expect("write_fact should not fail");
    w.finish().expect("finish should not fail");
    String::from_utf8(buf).expect("output must be valid UTF-8")
}

// ---------------------------------------------------------------------------
// Snapshot 1: single triple — IRI subject, predicate, plain string object
// ---------------------------------------------------------------------------

/// `<http://ex.org/s> <http://ex.org/p> "hello" .`
///
/// The canonical literal `"hello"` contains no control characters, no
/// embedded quotes, and no datatype suffix, so `write_literal_ttl` falls
/// through to the short-string path and emits `"hello"`.
#[test]
fn turtle_snapshot_single_plain_literal() {
    let fact = Fact {
        subject: "<http://ex.org/s>".to_owned(),
        predicate: "<http://ex.org/p>".to_owned(),
        object: "\"hello\"".to_owned(),
        graph: None,
    };

    let got = ttl_render_single(&fact);
    let expected = "<http://ex.org/s> <http://ex.org/p> \"hello\" .\n";

    assert_eq!(
        got, expected,
        "Turtle single-triple snapshot mismatch.\nGot:      {got:?}\nExpected: {expected:?}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 2: blank-node subject
// ---------------------------------------------------------------------------

/// `_:b0 <http://ex.org/p> <http://ex.org/o> .`
///
/// Blank-node labels are passed through verbatim by `write_term_ttl`
/// (neither the `<…>` nor the `"…"` branch matches, so the raw bytes are
/// written directly).  The label `_:b0` is a valid blank-node label in
/// every RDF text format.
#[test]
fn turtle_snapshot_blank_node_subject() {
    let fact = Fact {
        subject: "_:b0".to_owned(),
        predicate: "<http://ex.org/p>".to_owned(),
        object: "<http://ex.org/o>".to_owned(),
        graph: None,
    };

    let got = ttl_render_single(&fact);
    let expected = "_:b0 <http://ex.org/p> <http://ex.org/o> .\n";

    assert_eq!(
        got, expected,
        "Turtle blank-node snapshot mismatch.\nGot:      {got:?}\nExpected: {expected:?}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 3: typed literal with xsd:integer datatype
// ---------------------------------------------------------------------------

/// `<http://ex.org/s> <http://ex.org/p> "42"^^<http://www.w3.org/2001/XMLSchema#integer> .`
///
/// The canonical object is `"42"^^<http://www.w3.org/2001/XMLSchema#integer>`.
/// `split_literal` extracts `"42"` as the lex part and
/// `^^<http://www.w3.org/2001/XMLSchema#integer>` as the suffix.
/// No embedded quotes, so the short literal path is taken; the suffix is
/// appended verbatim by `write_literal_ttl`.
#[test]
fn turtle_snapshot_datatype_literal_xsd_integer() {
    let fact = Fact {
        subject: "<http://ex.org/s>".to_owned(),
        predicate: "<http://ex.org/p>".to_owned(),
        object: "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
        graph: None,
    };

    let got = ttl_render_single(&fact);
    let expected = "<http://ex.org/s> <http://ex.org/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n";

    assert_eq!(
        got, expected,
        "Turtle datatype-literal snapshot mismatch.\nGot:      {got:?}\nExpected: {expected:?}"
    );
}
