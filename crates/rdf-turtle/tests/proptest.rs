//! Property tests for the main `rdf-turtle` crate.
//!
//! Invariants exercised (spec-level, not shadow-parity):
//!
//! - **PTT1 `@prefix` redefinition does not reset blank-node scope**
//!   (pin `TTL-BNPFX-001`, reading §2: blank-node labels are
//!   document-scope; `@prefix` redefinitions do *not* introduce a new
//!   `_:b` namespace).  The generator emits paired documents that
//!   differ only by additional `@prefix` redefinitions inserted between
//!   triples; both documents must canonicalise to the same `Facts` set.
//! - **PTT2 numeric-literal typing is stable under whitespace variation.**
//!   Inserting SP / TAB / blank lines around numeric tokens does not
//!   change the `xsd:integer` / `xsd:decimal` / `xsd:double` datatype
//!   assignment (Turtle §6.4 — numeric literals).  The generator emits
//!   pairs that differ only by the amount of inter-token whitespace;
//!   both must canonicalise to the same `Facts`.
//!
//! Generators emit only documents the Turtle parser accepts.  They stay
//! small (≤6 triples, ≤3 redefinitions) to keep the 50-case default
//! inside the 30 s per-crate budget.

#![cfg(not(miri))]

use proptest::prelude::*;
use rdf_diff::Parser as _;
use rdf_turtle::TurtleParser;

// ---------------------------------------------------------------------------
// Shared generators
// ---------------------------------------------------------------------------

fn iri_strategy() -> impl Strategy<Value = String> {
    ("[a-z]{1,6}", "[A-Za-z0-9_.-]{1,10}", 0u8..=16)
        .prop_map(|(h, t, n)| format!("<http://{h}.example/{t}/{n}>"))
}

fn bnode_label_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9]{0,4}".prop_map(|s| s.to_owned())
}

// ---------------------------------------------------------------------------
// PTT1 — @prefix redefinition / bnode scoping
// ---------------------------------------------------------------------------

/// A triple using a blank-node subject with a stable label.  Same label
/// appearing in multiple triples must refer to the same blank node
/// (TTL-BNPFX-001).
fn bnode_triple_strategy() -> impl Strategy<Value = (String, String)> {
    (bnode_label_strategy(), iri_strategy(), iri_strategy())
        .prop_map(|(label, pred, obj)| (label.clone(), format!("_:{label} {pred} {obj} .")))
}

/// A prefix redefinition directive.  Uses the same prefix name each time
/// but bound to a different IRI to exercise "redefinition mid-document"
/// — which Turtle 1.1 §2.4 allows and which the pin says does not
/// rescope blank nodes.
fn prefix_redef_strategy() -> impl Strategy<Value = String> {
    iri_strategy().prop_map(|iri| format!("@prefix ex: {iri} ."))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        .. ProptestConfig::default()
    })]

    /// PTT1 — inserting arbitrary `@prefix` redefinitions between triples
    /// does not rescope the `_:label` namespace.  Documents that differ
    /// only by those redefinitions must produce the same canonical
    /// fact set.
    #[test]
    fn prefix_redef_does_not_reset_bnode_namespace(
        triples in prop::collection::vec(bnode_triple_strategy(), 2..=5),
        redefs in prop::collection::vec(prefix_redef_strategy(), 1..=3),
    ) {
        // Reuse a single bnode label across multiple triples so the
        // "same label = same node" property is actually observable.
        let label = &triples[0].0;
        let reused: Vec<String> = triples
            .iter()
            .enumerate()
            .map(|(i, (_, t))| {
                if i == 0 { t.clone() } else {
                    // Replace the bnode label in this triple with the
                    // first triple's label so we assert shared identity.
                    t.replacen(&format!("_:{}", triples[i].0), &format!("_:{label}"), 1)
                }
            })
            .collect();

        // Document A: plain — no prefix directives beyond the seed.
        let doc_a: String = seed_prefix()
            .into_iter()
            .chain(reused.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");

        // Document B: same triples, but with `@prefix ex:` redefined
        // between each pair.  The pin guarantees bnode identity is
        // unaffected.
        let mut lines = seed_prefix();
        for (i, t) in reused.iter().enumerate() {
            lines.push(t.clone());
            if i < redefs.len() {
                lines.push(redefs[i].clone());
            }
        }
        let doc_b = lines.join("\n");

        let a = TurtleParser::new()
            .parse(doc_a.as_bytes())
            .expect("doc A parses")
            .facts;
        let b = TurtleParser::new()
            .parse(doc_b.as_bytes())
            .expect("doc B parses")
            .facts;

        prop_assert_eq!(
            a.set.keys().cloned().collect::<Vec<_>>(),
            b.set.keys().cloned().collect::<Vec<_>>(),
        );
    }
}

/// Seed prefix — present so both documents start from the same parser
/// state.  The `ex:` name is redefined by `prefix_redef_strategy`.
fn seed_prefix() -> Vec<String> {
    vec!["@prefix ex: <http://ex.example/> .".to_owned()]
}

// ---------------------------------------------------------------------------
// PTT2 — numeric-literal typing under whitespace variation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum NumKind {
    Integer,
    Decimal,
    Double,
}

fn numeric_lex_strategy() -> impl Strategy<Value = (String, NumKind)> {
    prop_oneof![
        "(-|\\+)?[0-9]{1,4}".prop_map(|s| (s, NumKind::Integer)),
        "(-|\\+)?[0-9]{1,4}\\.[0-9]{1,4}".prop_map(|s| (s, NumKind::Decimal)),
        "(-|\\+)?[0-9]{1,3}\\.[0-9]{1,3}[eE](-|\\+)?[0-9]{1,2}"
            .prop_map(|s| (s, NumKind::Double)),
    ]
}

fn numeric_triple_strategy() -> impl Strategy<Value = (String, NumKind)> {
    (iri_strategy(), iri_strategy(), numeric_lex_strategy())
        .prop_map(|(s, p, (lex, kind))| (format!("{s} {p} {lex} ."), kind))
}

fn whitespace_strategy() -> impl Strategy<Value = String> {
    // Insert only tabs/spaces/newlines — all are legal inter-token WS.
    prop::collection::vec(prop_oneof![
        Just(" "),
        Just("\t"),
        Just("\n"),
        Just("  "),
    ], 1..=3)
        .prop_map(|v| v.concat())
}

fn perturb_whitespace(triple: &str, extra: &str) -> String {
    // Insert the extra whitespace between every existing token boundary
    // (i.e. wherever a single space appears in the generator's output).
    triple.replace(' ', &format!(" {extra} "))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        .. ProptestConfig::default()
    })]

    /// PTT2 — whitespace-perturbing a numeric literal must not change
    /// its `xsd:integer` / `xsd:decimal` / `xsd:double` typing.
    #[test]
    fn numeric_typing_stable_under_whitespace(
        (triple, _kind) in numeric_triple_strategy(),
        ws in whitespace_strategy(),
    ) {
        let tight = format!("{triple}\n");
        let padded = format!("{}\n", perturb_whitespace(&triple, &ws));

        let a = TurtleParser::new()
            .parse(tight.as_bytes())
            .expect("tight doc parses")
            .facts;
        let b = TurtleParser::new()
            .parse(padded.as_bytes())
            .expect("padded doc parses")
            .facts;

        // Compare the canonical fact sets directly.  The parser emits
        // literals as `"lex"^^<datatype>` (see `grammar.rs`); if the
        // datatype IRI or the lexical form drifted under whitespace the
        // sets would differ.
        prop_assert_eq!(
            a.set.keys().cloned().collect::<Vec<_>>(),
            b.set.keys().cloned().collect::<Vec<_>>(),
        );
    }
}
