//! API-surface tests for the `shex-syntax` crate.
//!
//! These tests work against the stub implementation that lands before the
//! real parser (Phase D, ADR-0023 §2). They verify only that the public
//! types compile and that the frozen `rdf_diff::Parser` trait contract is
//! upheld. They intentionally do NOT assert on the parse outcome — the stub
//! returns `Err(Diagnostics)` for all inputs, which is fine here.
//!
//! ## What is tested
//!
//! - `ShExParser::new()` and `ShExParser::default()` both compile and
//!   produce a value (structural: no type errors, no hidden trait bounds).
//! - `Parser::id()` returns the agreed parser identifier `"shex-syntax"`.
//!   This string is referenced by the diff harness (`rdf_diff::FactProvenance`)
//!   and must not change without a corresponding update to the test catalogue.
//!
//! ## ADR references
//!
//! - ADR-0023 §2 — Phase D tester deliverables.
//! - ADR-0020 §1.4 — frozen `Parser` trait surface.

use rdf_diff::Parser as _;
use shex_syntax::ShExParser;

/// Verify `ShExParser::new()` compiles and satisfies the `Parser` trait.
#[test]
fn shex_parser_new_compiles() {
    let parser = ShExParser::new();
    // The `id()` method is the minimum observable from the frozen trait.
    assert_eq!(parser.id(), "shex-syntax");
}

/// Verify `ShExParser::default()` compiles and returns the same parser id.
///
/// `Default` is a convenience constructor required by ADR-0023 §2; it must
/// agree with `new()` on the parser identifier.
#[test]
fn shex_parser_default_compiles() {
    let parser = ShExParser::default();
    assert_eq!(parser.id(), "shex-syntax");
}

/// Verify that `new()` and `default()` produce equivalent parsers (same id).
///
/// The stub implementation makes this trivially true. The real implementation
/// must preserve it — both constructors produce a stateless handle.
#[test]
fn shex_parser_new_and_default_agree_on_id() {
    assert_eq!(ShExParser::new().id(), ShExParser::default().id());
}
