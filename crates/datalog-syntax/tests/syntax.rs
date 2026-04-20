//! Syntax-level tests for the Datalog parser.
//!
//! Each test exercises a specific part of the grammar defined in ADR-0023.

use datalog_syntax::DatalogParser;
use rdf_diff::Parser as _;

fn parser() -> DatalogParser {
    DatalogParser::new()
}

// ---------------------------------------------------------------------------
// Happy-path tests
// ---------------------------------------------------------------------------

/// An empty program should succeed with zero facts emitted.
#[test]
fn empty_program_succeeds() {
    let outcome = parser().parse(b"").expect("empty program must parse");
    assert!(
        outcome.facts.set.is_empty(),
        "empty program should emit no facts"
    );
}

/// A program containing only whitespace is also empty.
#[test]
fn whitespace_only_program_succeeds() {
    let outcome = parser()
        .parse(b"   \n\t  ")
        .expect("whitespace-only program must parse");
    assert!(outcome.facts.set.is_empty());
}

/// Simple fact: `parent(tom, bob).`
#[test]
fn simple_fact() {
    let outcome = parser()
        .parse(b"parent(tom, bob).")
        .expect("simple fact must parse");
    // At least one fact emitted (the structural encoding).
    assert!(
        !outcome.facts.set.is_empty(),
        "simple fact should emit structural facts"
    );
    // Exactly one `<urn:x-datalog-syntax:fact>` triple from the program subject.
    let fact_pred = "<urn:x-datalog-syntax:fact>";
    let prog_subj = "<urn:x-datalog-syntax:program>";
    let fact_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == fact_pred)
        .count();
    assert_eq!(fact_count, 1, "expected exactly one :fact triple");
}

/// Simple rule: `ancestor(X, Y) :- parent(X, Y).`
#[test]
fn simple_rule() {
    let outcome = parser()
        .parse(b"ancestor(X, Y) :- parent(X, Y).")
        .expect("simple rule must parse");
    let rule_pred = "<urn:x-datalog-syntax:rule>";
    let prog_subj = "<urn:x-datalog-syntax:program>";
    let rule_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == rule_pred)
        .count();
    assert_eq!(rule_count, 1, "expected exactly one :rule triple");
}

/// Rule with two body atoms: `ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).`
#[test]
fn rule_with_two_body_atoms() {
    let src = b"ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).";
    let outcome = parser().parse(src).expect("rule with two body atoms must parse");

    // There should be exactly two :body triples for the rule node.
    let body_pred = "<urn:x-datalog-syntax:body>";
    let body_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.predicate == body_pred)
        .count();
    assert_eq!(body_count, 2, "expected two :body triples");
}

/// Negation: `safe(X) :- person(X), not criminal(X).`
#[test]
fn negation_in_body() {
    let src = b"safe(X) :- person(X), not criminal(X).";
    let outcome = parser().parse(src).expect("negation rule must parse");

    // At least one negated literal should be encoded.
    let neg_pred = "<urn:x-datalog-syntax:negated>";
    let neg_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.predicate == neg_pred)
        .count();
    assert_eq!(neg_count, 1, "expected one :negated triple");
}

/// Comment line: `% this is a comment` — should produce an empty program.
#[test]
fn comment_line() {
    let outcome = parser()
        .parse(b"% this is a comment\n")
        .expect("comment-only input must parse");
    assert!(
        outcome.facts.set.is_empty(),
        "comment-only input should emit no facts"
    );
}

/// Quoted constant: `likes(alice, "ice cream").`
#[test]
fn quoted_constant() {
    let outcome = parser()
        .parse(b"likes(alice, \"ice cream\").")
        .expect("quoted constant must parse");
    // There should be a fact triple and at least one arg encoding.
    let fact_pred = "<urn:x-datalog-syntax:fact>";
    let prog_subj = "<urn:x-datalog-syntax:program>";
    let fact_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == fact_pred)
        .count();
    assert_eq!(fact_count, 1);
}

/// Multiple facts and rules in one program.
#[test]
fn program_with_multiple_statements() {
    let src = b"parent(tom, bob).\nparent(bob, ann).\nancestor(X, Y) :- parent(X, Y).";
    let outcome = parser().parse(src).expect("multi-statement program must parse");

    let prog_subj = "<urn:x-datalog-syntax:program>";
    let fact_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == "<urn:x-datalog-syntax:fact>")
        .count();
    let rule_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == "<urn:x-datalog-syntax:rule>")
        .count();
    assert_eq!(fact_count, 2, "expected two facts");
    assert_eq!(rule_count, 1, "expected one rule");
}

/// Fact with zero-argument atom: `empty().`
#[test]
fn zero_arg_fact() {
    let outcome = parser()
        .parse(b"empty().")
        .expect("zero-arg fact must parse");
    assert!(!outcome.facts.set.is_empty());
}

/// Comments interspersed with real statements.
#[test]
fn comments_interspersed() {
    let src = b"% line one\nparent(a, b). % not a comment position but still tested\n% done";
    // The lexer strips `%`-comments so this should parse cleanly.
    let outcome = parser().parse(src).expect("program with comments must parse");
    let prog_subj = "<urn:x-datalog-syntax:program>";
    let fact_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.subject == prog_subj && f.predicate == "<urn:x-datalog-syntax:fact>")
        .count();
    assert_eq!(fact_count, 1);
}

// ---------------------------------------------------------------------------
// Error-path tests
// ---------------------------------------------------------------------------

/// A fact without a trailing period is a fatal error.
#[test]
fn missing_period_is_fatal() {
    let result = parser().parse(b"parent(tom, bob)");
    assert!(
        result.is_err(),
        "missing period should produce a fatal error"
    );
    let diag = result.unwrap_err();
    assert!(diag.fatal, "diagnostics must be fatal");
}

/// An uppercase relation name is a fatal error.
#[test]
fn uppercase_relation_name_is_fatal() {
    let result = parser().parse(b"Parent(tom, bob).");
    assert!(
        result.is_err(),
        "uppercase relation name should produce a fatal error"
    );
    let diag = result.unwrap_err();
    assert!(diag.fatal, "diagnostics must be fatal");
}
