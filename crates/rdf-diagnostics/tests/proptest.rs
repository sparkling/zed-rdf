//! Property tests for the shared `rdf-diagnostics` crate.
//!
//! Invariants exercised:
//!
//! - **PDG1 `DiagnosticBag::is_fatal` iff any `Severity::Error`.**
//!   The bag's fatality contract (see `src/bag.rs`) is that fatality
//!   tracks any `Error`-level diagnostic.  Proptest shuffles mixes of
//!   `Error` / `Warning` / `Info` / `Hint` into the bag and asserts
//!   the contract holds.
//! - **PDG2 `render` is total.**  `render(diagnostic, source)` must
//!   never panic on arbitrary `Diagnostic`s and arbitrary source
//!   strings — including when `span` is out of bounds, when `source`
//!   contains only newlines, and when the diagnostic's hint / related
//!   notes contain UTF-8 multibyte characters.  Grounded in the
//!   `render` docstring: "rendering never panics".

#![cfg(not(miri))]

use proptest::prelude::*;
use rdf_diagnostics::{Diagnostic, DiagnosticBag, Related, Severity, Span, render};

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

fn severity_strategy() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Error),
        Just(Severity::Warning),
        Just(Severity::Info),
        Just(Severity::Hint),
    ]
}

fn span_strategy() -> impl Strategy<Value = Span> {
    (0usize..=256, 0usize..=256).prop_map(|(a, b)| {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        Span::new(start, end)
    })
}

fn related_strategy() -> impl Strategy<Value = Related> {
    (span_strategy(), ".{0,32}").prop_map(|(span, message)| Related {
        span,
        message: message.to_owned(),
    })
}

/// Codes are `&'static str`; pick from a small set so proptest has
/// something to shrink towards.
fn code_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("X-0001"),
        Just("X-0002"),
        Just("X-0003"),
        Just("NT-LITESC-001"),
        Just("IRI-PCT-001"),
    ]
}

fn diagnostic_strategy() -> impl Strategy<Value = Diagnostic> {
    (
        severity_strategy(),
        code_strategy(),
        ".{0,48}",
        span_strategy(),
        prop::option::of(".{0,32}"),
        prop::collection::vec(related_strategy(), 0..=3),
    )
        .prop_map(|(sev, code, msg, span, hint, related)| {
            let mut d = Diagnostic::new(sev, code, msg, span);
            if let Some(h) = hint {
                d = d.with_hint(h);
            }
            for r in related {
                d = d.with_related(r.span, r.message);
            }
            d
        })
}

/// A source string that may contain multibyte UTF-8 characters and any
/// run of `\n` / `\r\n` so `render`'s line-column math is stress-tested.
fn source_strategy() -> impl Strategy<Value = String> {
    // `.{...}` regex strategy is allowed to yield arbitrary Unicode
    // scalar values except `\n` — which we want — so compose two
    // halves separated by explicit newlines.
    prop::collection::vec(".{0,32}", 0..=4)
        .prop_map(|lines| lines.join("\n"))
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 96,
        .. ProptestConfig::default()
    })]

    /// PDG1 — `DiagnosticBag::is_fatal` is equivalent to the presence
    /// of any `Severity::Error` in the bag.
    #[test]
    fn is_fatal_iff_any_error(diags in prop::collection::vec(diagnostic_strategy(), 0..=10)) {
        let any_error = diags.iter().any(|d| d.severity == Severity::Error);
        let mut bag = DiagnosticBag::new();
        for d in diags {
            bag.push(d);
        }
        prop_assert_eq!(bag.is_fatal(), any_error);
    }

    /// PDG2 — `render` is total (never panics).
    ///
    /// We do not assert anything about the return string's shape — the
    /// snapshot tests in `public_api.rs` cover formatting.  The
    /// property here is purely "does not panic under arbitrary inputs".
    #[test]
    fn render_never_panics(
        diag in diagnostic_strategy(),
        source in source_strategy(),
    ) {
        let _ = render(&diag, &source);
    }
}
