//! Property tests for the main `rdf-iri` crate.
//!
//! Invariants exercised (spec-level, not shadow-parity):
//!
//! - **PI1 round-trip.** For every absolute IRI the generator emits,
//!   `Iri::parse(iri.as_str()) == iri` — i.e. `parse ∘ to_string` is the
//!   identity on parse-accepted inputs. Grounded in RFC 3986 §5.3's
//!   component round-trip and the `IRI-PCT-001` pin forbidding
//!   equality-changing normalisations at parse time.
//! - **PI2 idempotent normalisation.** `normalise(normalise(x)) ==
//!   normalise(x)` for every accepted absolute IRI. Grounded in
//!   RFC 3986 §6.2.2 (syntax-based normalisation is a closure).
//! - **PI3 resolve absoluteness.** If `base` is absolute then
//!   `resolve(r, base)` is absolute for every parse-accepted reference
//!   `r`. RFC 3986 §5.3 step `T.scheme := B.scheme` when `R.scheme` is
//!   empty — the resolved form always carries a scheme.
//!
//! Generators are intentionally small (ASCII-only host/path/query/
//! fragment) so each case parses in microseconds and the full suite
//! comfortably fits inside the 30 s per-crate budget.

#![cfg(not(miri))]

use proptest::prelude::*;
use rdf_iri::Iri;

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// A scheme: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
fn scheme_strategy() -> impl Strategy<Value = String> {
    // Keep it short; real schemes are 2–10 chars.
    ("[a-z][a-z0-9+.-]{0,7}").prop_map(|s| s.to_owned())
}

/// `reg-name` label: iunreserved-only (no pct-encoded, no sub-delims).
/// Keeps the generator's acceptance set aligned with the parser's
/// `ireg-name` validator.
fn host_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,15}(\\.[a-z][a-z0-9-]{0,15}){0,2}".prop_map(|s| s.to_owned())
}

fn port_strategy() -> impl Strategy<Value = Option<u16>> {
    prop::option::of(any::<u16>())
}

/// A path segment made only of iunreserved + `:` + `@`. The RFC 3986
/// `pchar` production also allows pct-encoded triplets and sub-delims;
/// we omit them to keep the generator's shrinker small.
fn path_segment_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_~.-]{0,8}".prop_map(|s| s.to_owned())
}

fn path_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(path_segment_strategy(), 0..=4)
        .prop_map(|segs| if segs.is_empty() { String::new() } else { format!("/{}", segs.join("/")) })
}

fn query_strategy() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[A-Za-z0-9_~.=&-]{0,16}".prop_map(|s| s.to_owned()))
}

fn fragment_strategy() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[A-Za-z0-9_~.-]{0,16}".prop_map(|s| s.to_owned()))
}

/// Assemble a valid absolute IRI from generator-produced parts.
/// Emits only IRIs the parser accepts, so the properties never need to
/// paper over "bad input" branches.
fn absolute_iri_strategy() -> impl Strategy<Value = String> {
    (
        scheme_strategy(),
        host_strategy(),
        port_strategy(),
        path_strategy(),
        query_strategy(),
        fragment_strategy(),
    )
        .prop_map(|(scheme, host, port, path, query, fragment)| {
            let mut s = String::new();
            s.push_str(&scheme);
            s.push_str("://");
            s.push_str(&host);
            if let Some(p) = port {
                s.push(':');
                s.push_str(&p.to_string());
            }
            s.push_str(&path);
            if let Some(q) = query {
                s.push('?');
                s.push_str(&q);
            }
            if let Some(f) = fragment {
                s.push('#');
                s.push_str(&f);
            }
            s
        })
}

/// A reference IRI: absolute, or a path-only/query-only/fragment-only
/// relative reference. Everything here is an input that
/// `Iri::parse` accepts.
fn ref_iri_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        absolute_iri_strategy(),
        path_strategy().prop_filter(
            "relative-path first segment must not contain ':'",
            |p| !p.is_empty(),
        ),
        fragment_strategy().prop_filter_map("fragment", |f| f.map(|s| format!("#{s}"))),
        query_strategy().prop_filter_map("query", |q| q.map(|s| format!("?{s}"))),
    ]
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    /// PI1 — parse(Iri::to_string(parse(s))) == parse(s). The raw byte
    /// sequence is preserved verbatim by `Iri::parse` (no normalisation
    /// at parse time — `IRI-PCT-001`).
    #[test]
    fn parse_round_trips_on_absolute_iris(s in absolute_iri_strategy()) {
        let iri = Iri::parse(&s).expect("generator emits valid absolute IRIs");
        // The stored byte sequence is exactly what we handed in.
        prop_assert_eq!(iri.as_str(), s.as_str());
        // Re-parse the string form and compare for structural equality.
        let reparsed = Iri::parse(iri.as_str()).expect("re-parse must succeed");
        prop_assert_eq!(reparsed, iri);
    }

    /// PI2 — normalisation is idempotent: `normalise ∘ normalise == normalise`.
    #[test]
    fn normalisation_is_idempotent(s in absolute_iri_strategy()) {
        let iri = Iri::parse(&s).expect("valid absolute IRI");
        let once = iri.normalise();
        let twice = once.normalise();
        prop_assert_eq!(once, twice);
    }

    /// PI3 — resolving any reference against an absolute base is itself
    /// absolute (RFC 3986 §5.3 `T.scheme` is always set).
    #[test]
    fn resolve_against_absolute_base_is_absolute(
        base in absolute_iri_strategy(),
        reference in ref_iri_strategy(),
    ) {
        let base_iri = Iri::parse(&base).expect("valid absolute base");
        let Ok(ref_iri) = Iri::parse(&reference) else {
            // The reference generator is not a parser-perfect subset; skip
            // any input the parser rejects so the property stays pure.
            return Ok(());
        };
        let resolved = ref_iri.resolve(&base_iri);
        prop_assert!(
            resolved.is_absolute(),
            "resolve produced a relative IRI: base={base:?} ref={reference:?} result={}",
            resolved.as_str()
        );
    }
}
