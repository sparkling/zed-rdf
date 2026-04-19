//! End-to-end round-trip tests: for each Phase-A format the pipeline
//!
//! ```text
//! source bytes → parse → Facts → serialise → bytes → parse → Facts
//! ```
//!
//! must produce a `Facts` set that equals the first-pass `Facts` set
//! exactly (canonical form). This is the Phase-A exit-gate contract from
//! ADR-0017 §4 / ADR-0018 §4.

use std::collections::BTreeMap;

use rdf_diff::{Facts, ParseOutcome, Parser};
use rdf_format::{NQuadsWriter, NTriplesWriter, TriGWriter, TurtleWriter};
use rdf_ntriples::{NQuadsParser, NTriplesParser};
use rdf_turtle::{TriGParser, TurtleParser};

fn parse_nt(input: &[u8]) -> Facts {
    let ParseOutcome { facts, .. } = NTriplesParser.parse(input).expect("parse");
    facts
}

fn parse_nq(input: &[u8]) -> Facts {
    let ParseOutcome { facts, .. } = NQuadsParser.parse(input).expect("parse");
    facts
}

fn parse_ttl(input: &[u8]) -> Facts {
    let ParseOutcome { facts, .. } = TurtleParser.parse(input).expect("parse");
    facts
}

fn parse_trig(input: &[u8]) -> Facts {
    let ParseOutcome { facts, .. } = TriGParser.parse(input).expect("parse");
    facts
}

/// `Facts` equality ignoring per-fact provenance (which is diagnostic)
/// and parser-reported prefixes (also diagnostic). We compare exactly on
/// the canonical fact set — the integration contract from `rdf-diff`.
fn facts_eq(a: &Facts, b: &Facts) -> bool {
    let ka: Vec<_> = a.set.keys().collect();
    let kb: Vec<_> = b.set.keys().collect();
    ka == kb
}

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

// -----------------------------------------------------------------------
// N-Triples round-trips
// -----------------------------------------------------------------------

const NT_FIXTURES: &[&str] = &[
    "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
    "<http://ex/s> <http://ex/p> \"hello\" .\n",
    "<http://ex/s> <http://ex/p> \"hello\"@en-US .\n",
    "<http://ex/s> <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "<http://ex/s> <http://ex/p> \"line1\\nline2\" .\n",
    "<http://ex/s> <http://ex/p> \"tab\\there\" .\n",
    "<http://ex/s> <http://ex/p> \"quote \\\"inside\\\"\" .\n",
    "<http://ex/s> <http://ex/p> \"back\\\\slash\" .\n",
    "_:a <http://ex/p> _:b .\n",
    "_:a <http://ex/p> <http://ex/o> .\n",
    "<http://ex/s1> <http://ex/p> <http://ex/o1> .\n\
     <http://ex/s2> <http://ex/p> <http://ex/o2> .\n",
    "<http://ex/s> <http://ex/p> \"snowman: \\u2603\" .\n",
];

#[test]
fn nt_round_trip_all_fixtures() {
    for (i, src) in NT_FIXTURES.iter().enumerate() {
        let first = parse_nt(src.as_bytes());
        let rendered = render_nt(&first);
        let second = parse_nt(&rendered);
        assert!(
            facts_eq(&first, &second),
            "NT round-trip mismatch at fixture {i}\nsrc:\n{src}\nrendered:\n{}\nfirst: {:?}\nsecond: {:?}",
            String::from_utf8_lossy(&rendered),
            first.set.keys().collect::<Vec<_>>(),
            second.set.keys().collect::<Vec<_>>(),
        );
    }
}

// -----------------------------------------------------------------------
// N-Quads round-trips
// -----------------------------------------------------------------------

const NQ_FIXTURES: &[&str] = &[
    "<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n",
    "<http://ex/s> <http://ex/p> \"hello\" <http://ex/g> .\n",
    "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
    "<http://ex/s> <http://ex/p> <http://ex/o> _:g .\n",
    "<http://ex/s1> <http://ex/p> <http://ex/o1> <http://ex/g1> .\n\
     <http://ex/s2> <http://ex/p> <http://ex/o2> <http://ex/g2> .\n",
];

#[test]
fn nq_round_trip_all_fixtures() {
    for (i, src) in NQ_FIXTURES.iter().enumerate() {
        let first = parse_nq(src.as_bytes());
        let rendered = render_nq(&first);
        let second = parse_nq(&rendered);
        assert!(
            facts_eq(&first, &second),
            "NQ round-trip mismatch at fixture {i}\nsrc:\n{src}\nrendered:\n{}",
            String::from_utf8_lossy(&rendered),
        );
    }
}

// -----------------------------------------------------------------------
// Turtle round-trips
// -----------------------------------------------------------------------

const TTL_FIXTURES: &[&str] = &[
    "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
    "<http://ex/s> <http://ex/p> \"hello\" .\n",
    "<http://ex/s> <http://ex/p> \"hello\"@en .\n",
    "<http://ex/s> <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "<http://ex/s> <http://ex/p> \"line1\\nline2\" .\n",
    "<http://ex/s> <http://ex/p> \"quoted \\\"bit\\\" ok\" .\n",
    "_:a <http://ex/p> _:b .\n",
];

#[test]
fn ttl_round_trip_all_fixtures_no_prefixes() {
    for (i, src) in TTL_FIXTURES.iter().enumerate() {
        let first = parse_ttl(src.as_bytes());
        let rendered = render_ttl(&first, &[]);
        let second = parse_ttl(&rendered);
        assert!(
            facts_eq(&first, &second),
            "TTL round-trip mismatch at fixture {i}\nsrc:\n{src}\nrendered:\n{}",
            String::from_utf8_lossy(&rendered),
        );
    }
}

#[test]
fn ttl_round_trip_with_prefix_compaction() {
    let src = "@prefix ex: <http://ex/> . ex:s ex:p ex:o .\n";
    let first = parse_ttl(src.as_bytes());
    let rendered = render_ttl(&first, &[("ex", "http://ex/")]);
    // Sanity: the rendered output should actually use the prefix.
    let rendered_str = String::from_utf8_lossy(&rendered);
    assert!(
        rendered_str.contains("ex:s"),
        "expected compaction: {rendered_str}"
    );
    let second = parse_ttl(&rendered);
    assert!(
        facts_eq(&first, &second),
        "TTL prefix round-trip failed: {rendered_str}"
    );
}

// -----------------------------------------------------------------------
// TriG round-trips
// -----------------------------------------------------------------------

const TRIG_FIXTURES: &[&str] = &[
    "<http://ex/g> { <http://ex/s> <http://ex/p> <http://ex/o> . }\n",
    "<http://ex/s> <http://ex/p> <http://ex/o> .\n", // default graph
    "<http://ex/g1> { <http://ex/s1> <http://ex/p> <http://ex/o1> . }\n\
     <http://ex/g2> { <http://ex/s2> <http://ex/p> <http://ex/o2> . }\n",
    "<http://ex/g> { <http://ex/s> <http://ex/p> \"literal\"@en . }\n",
];

#[test]
fn trig_round_trip_all_fixtures() {
    for (i, src) in TRIG_FIXTURES.iter().enumerate() {
        let first = parse_trig(src.as_bytes());
        let rendered = render_trig(&first, &[]);
        let second = parse_trig(&rendered);
        assert!(
            facts_eq(&first, &second),
            "TriG round-trip mismatch at fixture {i}\nsrc:\n{src}\nrendered:\n{}",
            String::from_utf8_lossy(&rendered),
        );
    }
}

// -----------------------------------------------------------------------
// Diagnostic round-trip: empty input
// -----------------------------------------------------------------------

#[test]
fn empty_fact_set_produces_empty_output_for_all_formats() {
    let empty = Facts {
        set: BTreeMap::new(),
        prefixes: BTreeMap::new(),
    };
    assert!(render_nt(&empty).is_empty());
    assert!(render_nq(&empty).is_empty());
    // Turtle + TriG emit no prefix header when no prefixes are registered
    // and no facts are passed.
    assert!(render_ttl(&empty, &[]).is_empty());
    assert!(render_trig(&empty, &[]).is_empty());
}
