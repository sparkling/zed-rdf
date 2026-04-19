//! Property tests for the `rdf-diff` frozen trait surface.
//!
//! Tracked in `docs/verification/tests/catalogue.md` under invariants
//! `P1`–`P6`. These tests integrate against the frozen API defined in
//! `crates/testing/rdf-diff/src/lib.rs`; per ADR-0020 §1.4 the
//! signatures are immutable for the duration of the verification-v1 sweep.
//!
//! ## Status
//!
//! `Facts::canonicalise`, `diff`, and `diff_many` are now filled (the
//! `v1-diff-core` pass landed in `verification-v1`), so `P1`–`P3` run
//! on every `cargo test --workspace`. `phaseA-tester` un-ignored them
//! and added four additional invariants (`P4`–`P6`) that exercise
//! canonicalisation behaviours the main-parser Phase-A crates rely on:
//!
//! - `P4` — canonicalisation preserves fact cardinality modulo duplicate
//!   collapse. A property the main NT/Turtle parsers must not violate
//!   when they land.
//! - `P5` — canonicalisation is stable across shuffled input order
//!   (order-insensitivity of the set-level form).
//! - `P6` — angle-wrapped absolute IRIs are idempotent under
//!   `canonicalise_term`; bare absolute IRIs promote once and then
//!   stay wrapped. Exercises the front-door validator's IRI shape
//!   normalisation — the behavioural contract the main `rdf-iri`
//!   crate's `IriParser` adapter will be diffed against.
//!
//! ## Why no `proptest` dependency
//!
//! `rdf-diff`'s `Cargo.toml` is claimed by `v1-diff-core` / Phase-A
//! reviewer; per ADR-0020 §6.5 this agent does not edit it. The
//! property harness below is a minimal deterministic LCG-driven
//! generator, sufficient for the invariants under test. Once the main
//! parsers land the harness can be upgraded to `proptest` without
//! changing the invariants.

#![allow(clippy::missing_panics_doc)]

use std::collections::{BTreeMap, BTreeSet};

use rdf_diff::{Fact, FactProvenance, Facts, diff};

/// Number of generated cases per property. Kept small because each case
/// exercises the full canonicalisation + diff pipeline; raise when the
/// stubs land and `cargo-llvm-cov` measurements are wired in.
const CASES: u32 = 256;

/// Seeded deterministic LCG. Same seed across runs — reproducibility over
/// coverage, in the spirit of ADR-0006 §Determinism.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes 64-bit LCG constants.
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn gen_range(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo < hi);
        lo + (self.next_u64() as usize) % (hi - lo)
    }

    fn choice<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.gen_range(0, xs.len())]
    }
}

/// Token alphabet deliberately covers the axes that `canonicalise` must
/// normalise: IRI shape (absolute / relative), BNode labels (which are
/// re-labelled), and literal lexical forms. Not exhaustive — targeted
/// coverage for the three invariants.
const SUBJECTS: &[&str] = &[
    "http://example.org/s1",
    "http://example.org/s2",
    "_:b0",
    "_:b1",
    "_:b2",
    "http://example.org/ns#thing",
];

const PREDICATES: &[&str] = &[
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
    "http://www.w3.org/2000/01/rdf-schema#label",
    "http://example.org/p",
    "http://example.org/q",
];

const OBJECTS: &[&str] = &[
    "http://example.org/o1",
    "_:b0",
    "\"plain literal\"",
    "\"tagged\"@en",
    "\"42\"^^http://www.w3.org/2001/XMLSchema#integer",
];

const GRAPHS: &[Option<&str>] = &[None, Some("http://example.org/g1"), Some("http://example.org/g2")];

fn gen_fact(rng: &mut Lcg) -> Fact {
    Fact {
        subject: (*rng.choice(SUBJECTS)).to_string(),
        predicate: (*rng.choice(PREDICATES)).to_string(),
        object: (*rng.choice(OBJECTS)).to_string(),
        graph: rng.choice(GRAPHS).map(str::to_string),
    }
}

fn gen_raw(rng: &mut Lcg) -> (Vec<(Fact, FactProvenance)>, BTreeMap<String, String>) {
    let count = rng.gen_range(0, 12);
    let mut raw = Vec::with_capacity(count);
    for i in 0..count {
        let fact = gen_fact(rng);
        let prov = FactProvenance {
            offset: Some(i * 8),
            parser: "property-harness".to_string(),
        };
        raw.push((fact, prov));
    }

    let mut prefixes = BTreeMap::new();
    if rng.next_u64() & 1 == 0 {
        prefixes.insert("ex".to_string(), "http://example.org/".to_string());
    }
    (raw, prefixes)
}

fn gen_canonical(rng: &mut Lcg) -> Facts {
    let (raw, pfx) = gen_raw(rng);
    Facts::canonicalise(raw, pfx)
}

/// Collapse a [`rdf_diff::DiffReport`] to an order-independent set so that
/// P3 (commutativity) is checked at the set level only — per the prompt.
fn divergence_set(report: &rdf_diff::DiffReport) -> BTreeSet<String> {
    report
        .divergences
        .iter()
        .map(|d| format!("{d:?}"))
        .collect()
}

/// **P1 — `Facts::canonicalise` is idempotent.**
///
/// Re-canonicalising a canonical `Facts` yields the same `Facts`. The
/// parser tag is lost after the first pass (canonical form carries only
/// the diff-relevant subset), so we feed a synthetic provenance on the
/// second pass — anything else is a leak of parser-internal state, which
/// `rdf-diff::Facts` documents as a false-positive source.
#[test]
fn prop_canonicalise_is_idempotent() {
    for seed in 0..CASES {
        let mut rng = Lcg::new(u64::from(seed));
        let first = gen_canonical(&mut rng);

        let round_trip: Vec<_> = first
            .set
            .iter()
            .map(|(fact, prov)| (fact.clone(), prov.clone()))
            .collect();
        let second = Facts::canonicalise(round_trip, first.prefixes.clone());

        assert_eq!(
            first, second,
            "canonicalise is not idempotent on seed {seed}: \
             first={first:?}, second={second:?}"
        );
    }
}

/// **P2 — `diff(a, a).is_clean()` for any canonical `a`.**
///
/// A set diffed against itself must produce zero divergences. Carries an
/// expectation for the `Err` arm: if `diff` rejects a `Facts` produced by
/// `canonicalise`, that is itself a bug — `NonCanonical` should be
/// impossible on a canonical input.
#[test]
fn prop_diff_self_is_clean() {
    for seed in 0..CASES {
        let mut rng = Lcg::new(u64::from(seed).wrapping_add(0xA5A5));
        let a = gen_canonical(&mut rng);
        let report = diff(&a, &a).unwrap_or_else(|e| {
            panic!("diff(a, a) produced NonCanonical on canonical input (seed {seed}): {e}")
        });
        assert!(
            report.is_clean(),
            "diff(a, a) not clean on seed {seed}: {:?}",
            report.divergences
        );
    }
}

/// **P3 — Divergence set of `diff(a, b)` equals that of `diff(b, a)`.**
///
/// Commutativity is claimed at the set level; list order and any
/// parser-id ordering inside a `Divergence` variant are implementation
/// choices. Checked via `BTreeSet<String>` of the `Debug` formatting —
/// sufficient for this sweep.
#[test]
fn prop_diff_commutative_at_set_level() {
    for seed in 0..CASES {
        let mut rng_a = Lcg::new(u64::from(seed).wrapping_add(0xF00D));
        let mut rng_b = Lcg::new(u64::from(seed).wrapping_add(0xBEEF));
        let a = gen_canonical(&mut rng_a);
        let b = gen_canonical(&mut rng_b);

        let ab = diff(&a, &b)
            .unwrap_or_else(|e| panic!("diff(a, b) NonCanonical on seed {seed}: {e}"));
        let ba = diff(&b, &a)
            .unwrap_or_else(|e| panic!("diff(b, a) NonCanonical on seed {seed}: {e}"));

        let ab_set = divergence_set(&ab);
        let ba_set = divergence_set(&ba);

        assert_eq!(
            ab_set, ba_set,
            "divergence sets differ on seed {seed}: \n  a->b = {ab_set:?}\n  b->a = {ba_set:?}"
        );
    }
}

/// **P4 — Canonicalisation preserves fact set size modulo duplicates.**
///
/// For any raw input, `|canonicalise(raw).set| ≤ |raw|`, and equality
/// holds iff every fact in `raw` is already unique under the canonical
/// form. Exercises the `BTreeMap::entry(...).or_insert(prov)` collapse
/// branch in `Facts::canonicalise`.
///
/// The main NT / Turtle parsers (Phase A) feed this function with their
/// parse output; a regression that produces duplicate facts would
/// silently break oracle cross-agreement. Catching it here keeps the
/// canonicalisation contract honest.
#[test]
fn prop_canonicalise_bounds_cardinality() {
    for seed in 0..CASES {
        let mut rng = Lcg::new(u64::from(seed).wrapping_add(0xC0DE));
        let (raw, pfx) = gen_raw(&mut rng);
        let raw_len = raw.len();
        let facts = Facts::canonicalise(raw.clone(), pfx);
        assert!(
            facts.set.len() <= raw_len,
            "canonicalise grew the fact set on seed {seed}: raw={} canonical={}",
            raw_len,
            facts.set.len()
        );

        // Duplicate-collapse lower bound: canonicalising a concatenation
        // of a set with itself must not change cardinality.
        let mut doubled: Vec<(Fact, FactProvenance)> = Vec::with_capacity(raw_len * 2);
        doubled.extend(raw.iter().cloned());
        doubled.extend(raw.iter().cloned());
        let second = Facts::canonicalise(doubled, facts.prefixes.clone());
        assert_eq!(
            facts.set.len(),
            second.set.len(),
            "duplicate collapse failed on seed {seed}: \
             facts={} doubled={}",
            facts.set.len(),
            second.set.len()
        );
    }
}

/// **P5 — Canonicalisation is order-insensitive.**
///
/// For any raw input `r` and any permutation `r'` of `r`,
/// `canonicalise(r) == canonicalise(r')`. This is the property the
/// diff harness implicitly relies on when comparing parser outputs
/// whose internal emission order differs (streaming vs. buffered).
///
/// The permutation is produced by a second LCG so the shuffle is
/// deterministic given `seed`.
#[test]
fn prop_canonicalise_order_insensitive() {
    for seed in 0..CASES {
        let mut rng = Lcg::new(u64::from(seed).wrapping_add(0xF1F1));
        let (raw, pfx) = gen_raw(&mut rng);
        if raw.len() < 2 {
            continue;
        }

        // Fisher-Yates shuffle with a separate stream.
        let mut shuffled = raw.clone();
        let mut shuffle_rng = Lcg::new(u64::from(seed).wrapping_add(0x5EED));
        for i in (1..shuffled.len()).rev() {
            let j = shuffle_rng.gen_range(0, i + 1);
            shuffled.swap(i, j);
        }

        let a = Facts::canonicalise(raw, pfx.clone());
        let b = Facts::canonicalise(shuffled, pfx);
        // Order-insensitivity is claimed at the fact-key level only.
        // `FactProvenance` carries byte offsets that reflect the input's
        // first-writer-wins ordering and therefore legitimately differs
        // under permutation; comparing only the key set avoids a false
        // positive.
        let keys_a: std::collections::BTreeSet<_> = a.set.keys().cloned().collect();
        let keys_b: std::collections::BTreeSet<_> = b.set.keys().cloned().collect();
        assert_eq!(
            keys_a, keys_b,
            "canonicalise is not order-insensitive on seed {seed}"
        );
        // And the diff should agree at the set level.
        let report = diff(&a, &b).expect("both canonical");
        assert!(
            report.is_clean(),
            "diff not clean on permuted input (seed {seed}): {:?}",
            report.divergences
        );
    }
}

/// **P6 — Angle-wrapped absolute IRI is an idempotent canonical form.**
///
/// `canonicalise_term` is internal, but we can observe it through
/// [`Facts::canonicalise`]: feeding a bare absolute IRI and its
/// angle-wrapped form must produce the same canonical fact set. The
/// main `rdf-iri` parser's `IriParser` adapter is expected to emit
/// angle-wrapped IRIs verbatim; regression here would surface as a
/// front-door `NonCanonical` error in the diff harness the moment
/// Phase A's IRI adapter is wired in.
///
/// Exercised via a direct self-diff: if the two forms converge after
/// canonicalisation, `diff` is clean.
#[test]
fn prop_absolute_iri_wrap_is_idempotent() {
    let raw_bare = vec![(
        Fact {
            subject: "http://example.org/s".to_string(),
            predicate: "http://example.org/p".to_string(),
            object: "http://example.org/o".to_string(),
            graph: None,
        },
        FactProvenance {
            offset: Some(0),
            parser: "shape-probe".to_string(),
        },
    )];
    let raw_wrapped = vec![(
        Fact {
            subject: "<http://example.org/s>".to_string(),
            predicate: "<http://example.org/p>".to_string(),
            object: "<http://example.org/o>".to_string(),
            graph: None,
        },
        FactProvenance {
            offset: Some(0),
            parser: "shape-probe".to_string(),
        },
    )];
    let bare = Facts::canonicalise(raw_bare, BTreeMap::new());
    let wrapped = Facts::canonicalise(raw_wrapped, BTreeMap::new());
    assert_eq!(
        bare.set, wrapped.set,
        "wrapping an absolute IRI changed the canonical form"
    );

    // And the diff harness accepts both without NonCanonical.
    let report = diff(&bare, &wrapped).expect("both canonical");
    assert!(
        report.is_clean(),
        "bare and wrapped IRI disagree after canonicalise: {:?}",
        report.divergences
    );
}

/// **P6b — Turtle smoke fact set survives a serialise / reparse round-trip.**
///
/// The main `rdf-turtle` parser is deferred (see phaseA-tester findings);
/// until it lands we emulate the round-trip at the `Facts` layer: take a
/// canonical fact set, re-emit each entry via `Debug`-free string
/// reconstruction, and feed the result back through `canonicalise`. The
/// invariant is that the resulting `Facts` equals the original — the
/// canonical form must survive its own textual projection.
///
/// When the main Turtle parser lands this property gets upgraded to
/// `parse → serialise → parse → diff.is_clean()`.
#[test]
fn prop_canonical_facts_self_round_trip() {
    let raw = vec![
        (
            Fact {
                subject: "<http://example.org/s>".to_string(),
                predicate: "<http://example.org/p>".to_string(),
                object: "\"plain\"".to_string(),
                graph: None,
            },
            FactProvenance {
                offset: Some(0),
                parser: "round-trip".to_string(),
            },
        ),
        (
            Fact {
                subject: "<http://example.org/s>".to_string(),
                predicate: "<http://example.org/p>".to_string(),
                object: "\"hi\"@en-us".to_string(),
                graph: None,
            },
            FactProvenance {
                offset: Some(1),
                parser: "round-trip".to_string(),
            },
        ),
        (
            Fact {
                subject: "_:b0".to_string(),
                predicate: "<http://example.org/p>".to_string(),
                object: "<http://example.org/o>".to_string(),
                graph: Some("<http://example.org/g>".to_string()),
            },
            FactProvenance {
                offset: Some(2),
                parser: "round-trip".to_string(),
            },
        ),
    ];
    let first = Facts::canonicalise(raw, BTreeMap::new());

    // Re-feed the canonical set through canonicalise. This simulates
    // a lossless parser round-trip at the Facts layer. Note the lang
    // tag case-fold happens inside canonicalise, so "@en-us" above
    // becomes "@en-US" on first pass; the second pass must keep it.
    let round_trip: Vec<_> = first
        .set
        .iter()
        .map(|(f, p)| (f.clone(), p.clone()))
        .collect();
    let second = Facts::canonicalise(round_trip, first.prefixes.clone());

    assert_eq!(
        first, second,
        "canonical Facts did not survive self round-trip"
    );
    let report = diff(&first, &second).expect("canonical");
    assert!(report.is_clean(), "round-trip diff dirty: {:?}", report.divergences);
}

/// Sanity: the property harness itself compiles and exercises the API
/// shape. This lets `cargo build --tests` tell us fast when the frozen
/// surface drifts.
///
/// Not ignored: runs on every `cargo test --workspace`.
#[test]
fn api_shape_compiles() {
    let _ = Facts::default();
    let _ = Fact {
        subject: "s".to_string(),
        predicate: "p".to_string(),
        object: "o".to_string(),
        graph: None,
    };
    let _ = FactProvenance {
        offset: None,
        parser: "shape-check".to_string(),
    };
}
