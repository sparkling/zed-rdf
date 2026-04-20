//! API-surface tests for the `datalog-syntax` crate.
//!
//! These tests work against the stub implementation that lands before the
//! real parser (Phase D, ADR-0023 §2). They verify only that the public
//! types compile and that the frozen `rdf_diff::Parser` trait contract is
//! upheld. They intentionally do NOT assert on the parse outcome — the stub
//! returns `Err(Diagnostics)` for all inputs, which is fine here.
//!
//! ## What is tested
//!
//! - `DatalogParser::new()` and `DatalogParser::default()` both compile and
//!   produce a value (structural: no type errors, no hidden trait bounds).
//! - `Parser::id()` returns the agreed parser identifier `"datalog-syntax"`.
//!   This string is referenced by the diff harness (`rdf_diff::FactProvenance`)
//!   and must not change without a corresponding update to the test catalogue.
//!
//! ## ADR references
//!
//! - ADR-0023 §2 — Phase D tester deliverables.
//! - ADR-0020 §1.4 — frozen `Parser` trait surface.

use datalog_syntax::DatalogParser;
use rdf_diff::Parser as _;

/// Verify `DatalogParser::new()` compiles and satisfies the `Parser` trait.
#[test]
fn datalog_parser_new_compiles() {
    let parser = DatalogParser::new();
    // The `id()` method is the minimum observable from the frozen trait.
    assert_eq!(parser.id(), "datalog-syntax");
}

/// Verify `DatalogParser::default()` compiles and returns the same parser id.
///
/// `Default` is a convenience constructor required by ADR-0023 §2; it must
/// agree with `new()` on the parser identifier.
#[test]
fn datalog_parser_default_compiles() {
    let parser = DatalogParser::default();
    assert_eq!(parser.id(), "datalog-syntax");
}

/// Verify that `new()` and `default()` produce equivalent parsers (same id).
///
/// The stub implementation makes this trivially true. The real implementation
/// must preserve it — both constructors produce a stateless handle.
#[test]
fn datalog_parser_new_and_default_agree_on_id() {
    assert_eq!(DatalogParser::new().id(), DatalogParser::default().id());
}
