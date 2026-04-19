//! Smoke-tests for the public surface.  Downstream parsers read these
//! as the reference for how to construct diagnostics.

use rdf_diagnostics::{Diagnostic, DiagnosticBag, Severity, Span, render};

#[test]
fn severity_ordering_and_fatality() {
    assert!(Severity::Error.is_fatal());
    assert!(!Severity::Warning.is_fatal());
    assert!(!Severity::Info.is_fatal());
    assert!(!Severity::Hint.is_fatal());
    assert!(Severity::Error < Severity::Warning);
    assert!(Severity::Warning < Severity::Info);
    assert!(Severity::Info < Severity::Hint);
}

#[test]
fn span_basics() {
    let s = Span::new(3, 7);
    assert_eq!(s.len(), 4);
    assert!(!s.is_empty());
    assert!(Span::point(5).is_empty());
    let cover = Span::new(2, 4).cover(Span::new(6, 9));
    assert_eq!(cover, Span::new(2, 9));
}

#[test]
fn diagnostic_builder_shape() {
    let d = Diagnostic::error("NT-LITESC-001", "invalid UCHAR escape", Span::new(5, 11))
        .with_hint("use a non-surrogate scalar value")
        .with_related(Span::new(0, 1), "literal opens here");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.code, "NT-LITESC-001");
    assert_eq!(d.span, Span::new(5, 11));
    assert_eq!(d.hint.as_deref(), Some("use a non-surrogate scalar value"));
    assert_eq!(d.related.len(), 1);
    assert!(d.is_fatal());
}

#[test]
fn bag_accumulates_and_reports_fatality() {
    let mut bag = DiagnosticBag::new();
    assert!(bag.is_empty());
    assert!(!bag.is_fatal());
    bag.push(Diagnostic::warning("W-001", "unused prefix", Span::new(0, 7)));
    assert!(!bag.is_fatal());
    bag.push(Diagnostic::error("E-001", "boom", Span::new(8, 9)));
    assert!(bag.is_fatal());
    assert_eq!(bag.len(), 2);
    let v = bag.into_vec();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].severity, Severity::Warning);
    assert_eq!(v[1].severity, Severity::Error);
}

#[test]
fn bag_collects_and_iterates() {
    let diags = vec![
        Diagnostic::warning("W-1", "a", Span::new(0, 1)),
        Diagnostic::error("E-1", "b", Span::new(1, 2)),
    ];
    let bag: DiagnosticBag = diags.into_iter().collect();
    assert_eq!(bag.len(), 2);
    let codes: Vec<_> = bag.as_slice().iter().map(|d| d.code).collect();
    assert_eq!(codes, vec!["W-1", "E-1"]);
    let owned: Vec<_> = bag.into_iter().map(|d| d.code).collect();
    assert_eq!(owned, vec!["W-1", "E-1"]);
}

#[test]
fn render_includes_header_location_and_caret() {
    let source = "a b c\n<s> <p> <o> .\nnext";
    //             0 1 2 3 4 5 6 7 8 ...      '<' at byte 6 → line 2 col 1
    let d = Diagnostic::error("NT-001", "bad subject", Span::new(6, 9))
        .with_hint("subject must be an IRI or blank node");
    let out = render(&d, source);
    assert!(out.starts_with("error[NT-001]: bad subject\n"), "header\n{out}");
    assert!(out.contains("--> line 2, column 1"), "location\n{out}");
    assert!(out.contains("<s> <p> <o> ."), "source line\n{out}");
    assert!(out.contains("^^^"), "caret\n{out}");
    assert!(out.contains("= hint: subject must be an IRI or blank node"), "hint\n{out}");
}

#[test]
fn render_point_span_emits_single_caret() {
    let source = "abc";
    let d = Diagnostic::error("X", "expected token", Span::point(2));
    let out = render(&d, source);
    // Exactly one caret when the span is zero-width.
    let caret_line = out
        .lines()
        .find(|l| l.contains('^'))
        .expect("caret line present");
    assert_eq!(caret_line.matches('^').count(), 1, "output was\n{out}");
}

#[test]
fn render_out_of_bounds_span_does_not_panic() {
    let source = "short";
    let d = Diagnostic::error("X", "past EOF", Span::new(100, 200));
    let out = render(&d, source);
    // Header still emitted; no caret frame.
    assert!(out.starts_with("error[X]: past EOF\n"));
    assert!(!out.contains("-->"));
}

#[test]
fn render_related_notes_carry_location() {
    let source = "line1\nline2 here\nline3";
    let d = Diagnostic::error("X", "msg", Span::new(0, 1))
        .with_related(Span::new(6, 11), "see here"); // start of "line2 here"
    let out = render(&d, source);
    assert!(out.contains("= note: see here (line 2, column 1)"), "out\n{out}");
}
