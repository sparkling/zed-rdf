//! Idempotency tests for all four format writers.
//!
//! Property tested: `render(render(facts)) == render(facts)` as a byte-exact
//! string comparison. This is stronger than round-trip (which only asserts
//! `Facts` equality) and directly verifies that writers produce stable,
//! re-entrant output.
//!
//! Design (per arch-memo §3):
//!
//! 1. Parse a source string with the real Phase-A parser → `Facts`.
//! 2. Render `Facts` → `rendered1` (bytes).
//! 3. Parse `rendered1` with the same parser → `Facts2`.
//! 4. Render `Facts2` with the **same** prefix list → `rendered2` (bytes).
//! 5. Assert `rendered1 == rendered2` (byte-exact).
//!
//! The same prefix set is used on both render passes so that Turtle / TriG
//! compaction is deterministic and does not diverge between passes.

use std::collections::BTreeMap;

use rdf_diff::{Facts, ParseOutcome, Parser};
use rdf_format::{NQuadsWriter, NTriplesWriter, TriGWriter, TurtleWriter};
use rdf_ntriples::{NQuadsParser, NTriplesParser};
use rdf_turtle::{TriGParser, TurtleParser};

// ---------------------------------------------------------------------------
// Render helpers — mirrors round_trip.rs exactly
// ---------------------------------------------------------------------------

fn render_nt(facts: &Facts) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut w = NTriplesWriter::new(&mut buf);
    for fact in facts.set.keys() {
        w.write_fact(fact).expect("write");
    }
    w.finish().expect("finish");
    buf
}

fn render_nq(facts: &Facts) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut w = NQuadsWriter::new(&mut buf);
    for fact in facts.set.keys() {
        w.write_fact(fact).expect("write");
    }
    w.finish().expect("finish");
    buf
}

fn render_ttl(facts: &Facts, prefixes: &[(&str, &str)]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut w = TurtleWriter::new(&mut buf);
    for (p, i) in prefixes {
        w.with_prefix(p, i);
    }
    for fact in facts.set.keys() {
        w.write_fact(fact).expect("write");
    }
    w.finish().expect("finish");
    buf
}

fn render_trig(facts: &Facts, prefixes: &[(&str, &str)]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut w = TriGWriter::new(&mut buf);
    for (p, i) in prefixes {
        w.with_prefix(p, i);
    }
    for fact in facts.set.keys() {
        w.write_fact(fact).expect("write");
    }
    w.finish().expect("finish");
    buf
}

// ---------------------------------------------------------------------------
// Core idempotency helpers
// ---------------------------------------------------------------------------

/// Assert idempotency for N-Triples: render(parse(render(parse(src)))) == render(parse(src)).
fn assert_nt_idempotent(src: &str) {
    let facts1 = {
        let ParseOutcome { facts, .. } = NTriplesParser.parse(src.as_bytes()).expect("parse pass 1");
        facts
    };
    let rendered1 = render_nt(&facts1);

    let facts2 = {
        let ParseOutcome { facts, .. } = NTriplesParser.parse(&rendered1).expect("parse pass 2");
        facts
    };
    let rendered2 = render_nt(&facts2);

    assert_eq!(
        String::from_utf8_lossy(&rendered1),
        String::from_utf8_lossy(&rendered2),
        "NT idempotency violated for:\n{src}",
    );
}

/// Assert idempotency for N-Quads.
fn assert_nq_idempotent(src: &str) {
    let facts1 = {
        let ParseOutcome { facts, .. } = NQuadsParser.parse(src.as_bytes()).expect("parse pass 1");
        facts
    };
    let rendered1 = render_nq(&facts1);

    let facts2 = {
        let ParseOutcome { facts, .. } = NQuadsParser.parse(&rendered1).expect("parse pass 2");
        facts
    };
    let rendered2 = render_nq(&facts2);

    assert_eq!(
        String::from_utf8_lossy(&rendered1),
        String::from_utf8_lossy(&rendered2),
        "NQ idempotency violated for:\n{src}",
    );
}

/// Assert idempotency for Turtle. The same `prefixes` slice is used on both
/// render passes so compaction behaviour is identical.
fn assert_ttl_idempotent(src: &str, prefixes: &[(&str, &str)]) {
    let facts1 = {
        let ParseOutcome { facts, .. } = TurtleParser.parse(src.as_bytes()).expect("parse pass 1");
        facts
    };
    let rendered1 = render_ttl(&facts1, prefixes);

    let facts2 = {
        let ParseOutcome { facts, .. } = TurtleParser.parse(&rendered1).expect("parse pass 2");
        facts
    };
    let rendered2 = render_ttl(&facts2, prefixes);

    assert_eq!(
        String::from_utf8_lossy(&rendered1),
        String::from_utf8_lossy(&rendered2),
        "TTL idempotency violated for:\n{src}",
    );
}

/// Assert idempotency for TriG.
fn assert_trig_idempotent(src: &str, prefixes: &[(&str, &str)]) {
    let facts1 = {
        let ParseOutcome { facts, .. } = TriGParser.parse(src.as_bytes()).expect("parse pass 1");
        facts
    };
    let rendered1 = render_trig(&facts1, prefixes);

    let facts2 = {
        let ParseOutcome { facts, .. } = TriGParser.parse(&rendered1).expect("parse pass 2");
        facts
    };
    let rendered2 = render_trig(&facts2, prefixes);

    assert_eq!(
        String::from_utf8_lossy(&rendered1),
        String::from_utf8_lossy(&rendered2),
        "TriG idempotency violated for:\n{src}",
    );
}

// ---------------------------------------------------------------------------
// N-Triples idempotency tests  (8 tests)
// ---------------------------------------------------------------------------

#[test]
fn nt_idempotent_single_triple() {
    assert_nt_idempotent("<http://ex/s> <http://ex/p> <http://ex/o> .\n");
}

#[test]
fn nt_idempotent_multiple_triples() {
    assert_nt_idempotent(
        "<http://ex/s> <http://ex/p1> <http://ex/o1> .\n\
         <http://ex/s> <http://ex/p2> <http://ex/o2> .\n\
         <http://ex/s> <http://ex/p3> <http://ex/o3> .\n",
    );
}

#[test]
fn nt_idempotent_literal_with_lang_tag() {
    assert_nt_idempotent("<http://ex/s> <http://ex/p> \"bonjour\"@fr .\n");
}

#[test]
fn nt_idempotent_literal_with_datatype() {
    assert_nt_idempotent(
        "<http://ex/s> <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    );
}

#[test]
fn nt_idempotent_blank_node_subject() {
    assert_nt_idempotent("_:a <http://ex/p> <http://ex/o> .\n");
}

#[test]
fn nt_idempotent_blank_node_object() {
    assert_nt_idempotent("<http://ex/s> <http://ex/p> _:b .\n");
}

#[test]
fn nt_idempotent_escaped_literal() {
    // literal with \n and \t — the writer must escape these; the second pass
    // must produce the same bytes.
    assert_nt_idempotent("<http://ex/s> <http://ex/p> \"line1\\nline2\" .\n");
}

#[test]
fn nt_idempotent_empty_input() {
    // Empty input → empty render1 → empty render2.
    let empty = Facts {
        set: BTreeMap::new(),
        prefixes: BTreeMap::new(),
    };
    let rendered1 = render_nt(&empty);
    let rendered2 = render_nt(&empty);
    assert_eq!(rendered1, rendered2);
    assert!(rendered1.is_empty());
}

// ---------------------------------------------------------------------------
// N-Quads idempotency tests  (8 tests)
// ---------------------------------------------------------------------------

#[test]
fn nq_idempotent_single_quad_with_graph() {
    assert_nq_idempotent(
        "<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n",
    );
}

#[test]
fn nq_idempotent_multiple_quads_same_graph() {
    assert_nq_idempotent(
        "<http://ex/s> <http://ex/p1> <http://ex/o1> <http://ex/g> .\n\
         <http://ex/s> <http://ex/p2> <http://ex/o2> <http://ex/g> .\n",
    );
}

#[test]
fn nq_idempotent_literal_with_lang_tag() {
    assert_nq_idempotent(
        "<http://ex/s> <http://ex/p> \"hola\"@es <http://ex/g> .\n",
    );
}

#[test]
fn nq_idempotent_literal_with_datatype() {
    assert_nq_idempotent(
        "<http://ex/s> <http://ex/p> \"3.14\"^^<http://www.w3.org/2001/XMLSchema#decimal> <http://ex/g> .\n",
    );
}

#[test]
fn nq_idempotent_blank_node_subject() {
    assert_nq_idempotent("_:a <http://ex/p> <http://ex/o> <http://ex/g> .\n");
}

#[test]
fn nq_idempotent_named_graph() {
    assert_nq_idempotent(
        "<http://ex/s1> <http://ex/p> <http://ex/o1> <http://ex/g1> .\n\
         <http://ex/s2> <http://ex/p> <http://ex/o2> <http://ex/g2> .\n",
    );
}

#[test]
fn nq_idempotent_default_graph_triple() {
    // No graph slot — default graph.
    assert_nq_idempotent("<http://ex/s> <http://ex/p> <http://ex/o> .\n");
}

#[test]
fn nq_idempotent_empty_input() {
    let empty = Facts {
        set: BTreeMap::new(),
        prefixes: BTreeMap::new(),
    };
    let rendered1 = render_nq(&empty);
    let rendered2 = render_nq(&empty);
    assert_eq!(rendered1, rendered2);
    assert!(rendered1.is_empty());
}

// ---------------------------------------------------------------------------
// Turtle idempotency tests  (8 tests)
// ---------------------------------------------------------------------------

#[test]
fn ttl_idempotent_single_triple_no_prefixes() {
    assert_ttl_idempotent(
        "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
        &[],
    );
}

#[test]
fn ttl_idempotent_multiple_triples_same_subject() {
    // Multiple triples sharing a subject — tests subject-grouping stability.
    assert_ttl_idempotent(
        "<http://ex/s> <http://ex/p1> <http://ex/o1> .\n\
         <http://ex/s> <http://ex/p2> <http://ex/o2> .\n",
        &[("ex", "http://ex/")],
    );
}

#[test]
fn ttl_idempotent_literal_with_lang_tag() {
    assert_ttl_idempotent(
        "<http://ex/s> <http://ex/p> \"guten Tag\"@de .\n",
        &[],
    );
}

#[test]
fn ttl_idempotent_literal_with_datatype() {
    assert_ttl_idempotent(
        "<http://ex/s> <http://ex/p> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n",
        &[("xsd", "http://www.w3.org/2001/XMLSchema#")],
    );
}

#[test]
fn ttl_idempotent_blank_node_subject() {
    assert_ttl_idempotent("_:a <http://ex/p> <http://ex/o> .\n", &[]);
}

#[test]
fn ttl_idempotent_with_prefix_compaction() {
    // Verify that prefix-compacted output is itself idempotent.
    assert_ttl_idempotent(
        "@prefix ex: <http://ex/> .\nex:s ex:p ex:o .\n",
        &[("ex", "http://ex/")],
    );
}

#[test]
fn ttl_idempotent_escaped_literal() {
    assert_ttl_idempotent(
        "<http://ex/s> <http://ex/p> \"a\\tb\" .\n",
        &[],
    );
}

#[test]
fn ttl_idempotent_empty_input() {
    let empty = Facts {
        set: BTreeMap::new(),
        prefixes: BTreeMap::new(),
    };
    let rendered1 = render_ttl(&empty, &[]);
    let rendered2 = render_ttl(&empty, &[]);
    assert_eq!(rendered1, rendered2);
    assert!(rendered1.is_empty());
}

// ---------------------------------------------------------------------------
// TriG idempotency tests  (8 tests)
// ---------------------------------------------------------------------------

#[test]
fn trig_idempotent_single_triple_named_graph() {
    assert_trig_idempotent(
        "<http://ex/g> { <http://ex/s> <http://ex/p> <http://ex/o> . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_multiple_triples_same_subject() {
    // Multiple triples in the same named graph with same subject.
    assert_trig_idempotent(
        "<http://ex/g> { <http://ex/s> <http://ex/p1> <http://ex/o1> . }\n\
         <http://ex/g> { <http://ex/s> <http://ex/p2> <http://ex/o2> . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_literal_with_lang_tag() {
    assert_trig_idempotent(
        "<http://ex/g> { <http://ex/s> <http://ex/p> \"ciao\"@it . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_literal_with_datatype() {
    assert_trig_idempotent(
        "<http://ex/g> { <http://ex/s> <http://ex/p> \"2026-04-20\"^^<http://www.w3.org/2001/XMLSchema#date> . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_blank_node_subject() {
    assert_trig_idempotent(
        "<http://ex/g> { _:a <http://ex/p> <http://ex/o> . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_named_graph_with_prefix() {
    assert_trig_idempotent(
        "<http://ex/g> { <http://ex/s> <http://ex/p> <http://ex/o> . }\n",
        &[("ex", "http://ex/")],
    );
}

#[test]
fn trig_idempotent_multiple_named_graphs() {
    assert_trig_idempotent(
        "<http://ex/g1> { <http://ex/s1> <http://ex/p> <http://ex/o1> . }\n\
         <http://ex/g2> { <http://ex/s2> <http://ex/p> <http://ex/o2> . }\n",
        &[],
    );
}

#[test]
fn trig_idempotent_empty_input() {
    let empty = Facts {
        set: BTreeMap::new(),
        prefixes: BTreeMap::new(),
    };
    let rendered1 = render_trig(&empty, &[]);
    let rendered2 = render_trig(&empty, &[]);
    assert_eq!(rendered1, rendered2);
    assert!(rendered1.is_empty());
}
