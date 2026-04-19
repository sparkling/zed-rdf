//! Property tests for the `rdf-diff` frozen trait surface.
//!
//! Tracked in `docs/verification/tests/catalogue.md` under invariants
//! `P1`, `P2`, `P3`. These tests integrate against the frozen API defined
//! in `crates/testing/rdf-diff/src/lib.rs`; per ADR-0020 §1.4 the
//! signatures are immutable for the duration of the verification-v1 sweep.
//!
//! ## Why `#[ignore]` by default
//!
//! `Facts::canonicalise`, `diff`, and `diff_many` are stubbed with
//! `todo!()` until the `v1-diff-core` agent fills them. Running the
//! properties before that would panic. Tests are therefore gated with
//! `#[ignore]` and run via `cargo test --workspace -- --include-ignored`
//! once the stubs land (wired into the `cargo-llvm-cov` target by
//! `v1-ci-wiring`). This keeps `cargo test --workspace` green across the
//! sweep without sacrificing the tests themselves.
//!
//! ## Why no `proptest` dependency
//!
//! `rdf-diff`'s `Cargo.toml` is claimed by `v1-diff-core`; per ADR-0020
//! §6.5 this agent does not edit it. The property harness below is a
//! minimal deterministic LCG-driven generator, sufficient for the three
//! invariants under test. Once `v1-diff-core` lands, a follow-up handoff
//! can replace it with `proptest` without changing the invariants.

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
#[ignore = "unignore once v1-diff-core fills Facts::canonicalise (see properties.rs header)"]
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
#[ignore = "unignore once v1-diff-core fills Facts::canonicalise + diff"]
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
#[ignore = "unignore once v1-diff-core fills Facts::canonicalise + diff"]
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

/// Sanity: the property harness itself compiles and exercises the API
/// shape even before the stubs are filled. This is **not** a stub for a
/// real invariant — it lets `cargo build --tests` tell us fast when the
/// frozen surface drifts.
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
