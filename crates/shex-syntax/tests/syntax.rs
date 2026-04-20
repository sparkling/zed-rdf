//! Integration tests for the shex-syntax parser.
//!
//! Tests cover:
//! - Empty schema (no facts emitted).
//! - Simple shape declaration.
//! - Shape with cardinality wildcard.
//! - PREFIX and BASE directives.
//! - Node constraint keywords (IRI, LITERAL, NONLITERAL, BNODE).
//! - Multiple cardinality forms (`*`, `+`, `?`, `{n}`, `{n,m}`).
//! - Shape references with `@`.
//! - AND / OR / NOT operators.
//! - Invalid input: missing `}` → fatal error.
//! - Invalid input: unknown token → fatal error.

use rdf_diff::Parser as _;
use shex_syntax::ShExParser;

fn parse_ok(input: &str) -> rdf_diff::ParseOutcome {
    ShExParser::new()
        .parse(input.as_bytes())
        .expect("expected successful parse")
}

fn parse_err(input: &str) -> rdf_diff::Diagnostics {
    ShExParser::new()
        .parse(input.as_bytes())
        .expect_err("expected parse failure")
}

// -----------------------------------------------------------------------
// Fixture helpers
// -----------------------------------------------------------------------

fn load_fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[test]
fn empty_schema_produces_no_facts() {
    let fixture = load_fixture("empty.shex");
    let outcome = ShExParser::new()
        .parse(&fixture)
        .expect("empty schema must parse cleanly");
    assert!(
        outcome.facts.set.is_empty(),
        "empty schema must produce no facts, got {:?}",
        outcome.facts.set.keys().collect::<Vec<_>>()
    );
}

#[test]
fn empty_string_produces_no_facts() {
    let outcome = parse_ok("");
    assert!(
        outcome.facts.set.is_empty(),
        "empty string must produce no facts"
    );
}

#[test]
fn simple_shape_with_iri_predicate() {
    // `<S> { <p> xsd:string }` — loads from fixture.
    let fixture = load_fixture("simple_shape.shex");
    let outcome = ShExParser::new()
        .parse(&fixture)
        .expect("simple shape must parse");

    // Must have some facts.
    assert!(
        !outcome.facts.set.is_empty(),
        "simple shape must produce facts"
    );

    // The schema subject must be referenced.
    let schema_subject = "<urn:x-shex-syntax:schema>";
    let has_shape_fact = outcome.facts.set.keys().any(|f| {
        f.subject == schema_subject && f.predicate == "<urn:x-shex-syntax:shape>"
    });
    assert!(
        has_shape_fact,
        "expected a shex-syntax:shape fact on the schema subject"
    );
}

#[test]
fn shape_with_cardinality_star() {
    // `<S> { <p> . * }` from fixture.
    let fixture = load_fixture("cardinality.shex");
    let outcome = ShExParser::new()
        .parse(&fixture)
        .expect("cardinality shape must parse");

    // Must have a triple constraint fact.
    let has_tc = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/tripleConstraint>"
    });
    assert!(has_tc, "expected a tripleConstraint fact");

    // Must have a cardinality fact with `0,*`.
    let has_star_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"0,*\""
    });
    assert!(has_star_card, "expected cardinality '0,*' for * operator");
}

#[test]
fn prefix_and_base_directives() {
    let fixture = load_fixture("prefix_base.shex");
    let outcome = ShExParser::new()
        .parse(&fixture)
        .expect("prefix+base fixture must parse");

    // BASE fact emitted.
    let has_base = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:base>"
    });
    assert!(has_base, "expected a base fact");

    // PREFIX fact emitted.
    let has_prefix = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:prefix>"
    });
    assert!(has_prefix, "expected a prefix fact");
}

#[test]
fn inline_prefix_base() {
    let input = "PREFIX ex: <http://example.org/>\nBASE <http://example.org/>\n";
    let outcome = parse_ok(input);
    let has_base = outcome.facts.set.keys().any(|f| f.predicate == "<urn:x-shex-syntax:base>");
    let has_prefix = outcome.facts.set.keys().any(|f| f.predicate == "<urn:x-shex-syntax:prefix>");
    assert!(has_base, "BASE directive must produce a base fact");
    assert!(has_prefix, "PREFIX directive must produce a prefix fact");
}

#[test]
fn node_constraint_iri_keyword() {
    let input = "<S> IRI";
    let outcome = parse_ok(input);
    let has_nc = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:nodeConstraint>" && f.object == "\"IRI\""
    });
    assert!(has_nc, "IRI node constraint must emit nodeConstraint: IRI");
}

#[test]
fn node_constraint_literal_keyword() {
    let input = "<S> LITERAL";
    let outcome = parse_ok(input);
    let has_nc = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:nodeConstraint>" && f.object == "\"LITERAL\""
    });
    assert!(has_nc, "LITERAL node constraint must emit nodeConstraint: LITERAL");
}

#[test]
fn node_constraint_nonliteral_keyword() {
    let input = "<S> NONLITERAL";
    let outcome = parse_ok(input);
    let has_nc = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:nodeConstraint>" && f.object == "\"NONLITERAL\""
    });
    assert!(has_nc, "NONLITERAL node constraint must be emitted");
}

#[test]
fn node_constraint_bnode_keyword() {
    let input = "<S> BNODE";
    let outcome = parse_ok(input);
    let has_nc = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:nodeConstraint>" && f.object == "\"BNODE\""
    });
    assert!(has_nc, "BNODE node constraint must be emitted");
}

#[test]
fn cardinality_optional() {
    let input = "<S> { <p> . ? }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"0,1\""
    });
    assert!(has_card, "? cardinality must produce 0,1");
}

#[test]
fn cardinality_plus() {
    let input = "<S> { <p> . + }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"1,*\""
    });
    assert!(has_card, "+ cardinality must produce 1,*");
}

#[test]
fn cardinality_exact() {
    let input = "<S> { <p> . {3} }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"3,3\""
    });
    assert!(has_card, "{{3}} cardinality must produce 3,3");
}

#[test]
fn cardinality_range() {
    let input = "<S> { <p> . {2,5} }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"2,5\""
    });
    assert!(has_card, "{{2,5}} cardinality must produce 2,5");
}

#[test]
fn cardinality_at_least() {
    let input = "<S> { <p> . {1,} }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"1,*\""
    });
    assert!(has_card, "{{1,}} cardinality must produce 1,*");
}

#[test]
fn default_cardinality_is_one() {
    let input = "<S> { <p> . }";
    let outcome = parse_ok(input);
    let has_card = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/cardinality>" && f.object == "\"1,1\""
    });
    assert!(has_card, "default cardinality must be 1,1");
}

#[test]
fn shape_reference_at_notation() {
    let input = "PREFIX ex: <http://example.org/>\n\
                 <T> {}\n\
                 <S> @ex:Other";
    let outcome = parse_ok(input);
    let has_ref = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/ref>"
    });
    assert!(has_ref, "@ shape reference must emit shape/ref fact");
}

#[test]
fn and_operator() {
    let input = "<S> IRI AND LITERAL";
    let outcome = parse_ok(input);
    let has_and = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/and>"
    });
    assert!(has_and, "AND operator must emit shape/and fact");
}

#[test]
fn or_operator() {
    let input = "<S> IRI OR LITERAL";
    let outcome = parse_ok(input);
    let has_or = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/or>"
    });
    assert!(has_or, "OR operator must emit shape/or fact");
}

#[test]
fn not_operator() {
    let input = "<S> NOT LITERAL";
    let outcome = parse_ok(input);
    let has_not = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/not>"
    });
    assert!(has_not, "NOT operator must emit shape/not fact");
}

#[test]
fn comment_lines_are_skipped() {
    let input = "# This is a comment\n<S> { <p> . }\n# Another comment";
    let outcome = parse_ok(input);
    assert!(!outcome.facts.set.is_empty(), "comments must not prevent parsing");
}

#[test]
fn multiple_triple_constraints_with_semicolon() {
    let input = "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
                 <S> {\n\
                     <name> xsd:string ;\n\
                     <age>  xsd:integer ?\n\
                 }";
    let outcome = parse_ok(input);
    // Should have 2 triple constraint facts.
    let tc_count = outcome
        .facts
        .set
        .keys()
        .filter(|f| f.predicate == "<urn:x-shex-syntax:shape/tripleConstraint>")
        .count();
    assert_eq!(tc_count, 2, "two triple constraints must each emit a fact");
}

#[test]
fn rdf_type_predicate() {
    let input = "<S> { a . }";
    let outcome = parse_ok(input);
    let has_rdf_type = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/predicate>"
            && f.object == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
    });
    assert!(has_rdf_type, "'a' predicate must expand to rdf:type");
}

#[test]
fn turtle_style_prefix_directive() {
    let input = "@prefix ex: <http://example.org/>\n<S> {}";
    let outcome = parse_ok(input);
    let has_prefix = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:prefix>"
    });
    assert!(has_prefix, "@prefix directive must produce a prefix fact");
}

#[test]
fn turtle_style_base_directive() {
    let input = "@base <http://example.org/>\n<S> {}";
    let outcome = parse_ok(input);
    let has_base = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:base>"
    });
    assert!(has_base, "@base directive must produce a base fact");
}

#[test]
fn closed_shape() {
    let input = "<S> CLOSED { <p> . }";
    let outcome = parse_ok(input);
    let has_closed = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/closed>"
    });
    assert!(has_closed, "CLOSED modifier must emit shape/closed fact");
}

#[test]
fn extends_shape() {
    let input = "<T> {}\n<S> EXTENDS @<T> { <p> . }";
    let outcome = parse_ok(input);
    let has_extends = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:shape/extends>"
    });
    assert!(has_extends, "EXTENDS modifier must emit shape/extends fact");
}

#[test]
fn inverse_predicate_with_keyword() {
    let input = "<S> { INVERSE <p> . }";
    let outcome = parse_ok(input);
    let has_inv = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/inverse>"
    });
    assert!(has_inv, "INVERSE keyword must emit tc/inverse fact");
}

#[test]
fn inverse_predicate_with_caret() {
    let input = "<S> { ^<p> . }";
    let outcome = parse_ok(input);
    let has_inv = outcome.facts.set.keys().any(|f| {
        f.predicate == "<urn:x-shex-syntax:tc/inverse>"
    });
    assert!(has_inv, "^ inverse marker must emit tc/inverse fact");
}

// -----------------------------------------------------------------------
// Error cases
// -----------------------------------------------------------------------

#[test]
fn missing_closing_brace_is_fatal() {
    let input = "<S> { <p> .";
    let diag = parse_err(input);
    assert!(diag.fatal, "missing '}}' must be a fatal error");
    assert!(
        !diag.messages.is_empty(),
        "fatal error must include a message"
    );
}

#[test]
fn unknown_token_is_fatal() {
    // `~~~` is not valid ShExC syntax.
    let input = "~~~";
    let diag = parse_err(input);
    assert!(diag.fatal, "unknown token must be a fatal error");
}

#[test]
fn undefined_prefix_in_predicate_is_fatal() {
    // `undeclared:foo` used as predicate without PREFIX declaration.
    let input = "<S> { undeclared:foo . }";
    let diag = parse_err(input);
    assert!(diag.fatal, "undefined prefix must be a fatal error");
}

#[test]
fn unterminated_iri_is_fatal() {
    let input = "<S> { <unclosed .";
    let diag = parse_err(input);
    assert!(diag.fatal, "unterminated IRI must be a fatal error");
}

// -----------------------------------------------------------------------
// Parser id
// -----------------------------------------------------------------------

#[test]
fn parser_id_is_shex_syntax() {
    assert_eq!(ShExParser::new().id(), "shex-syntax");
}

// -----------------------------------------------------------------------
// Fixture-based tests
// -----------------------------------------------------------------------

#[test]
fn fixture_empty_parses() {
    let fixture = load_fixture("empty.shex");
    let outcome = ShExParser::new()
        .parse(&fixture)
        .expect("empty.shex must parse");
    assert!(outcome.facts.set.is_empty(), "empty fixture must produce no facts");
}

#[test]
fn fixture_simple_shape_parses() {
    let fixture = load_fixture("simple_shape.shex");
    ShExParser::new()
        .parse(&fixture)
        .expect("simple_shape.shex must parse");
}

#[test]
fn fixture_cardinality_parses() {
    let fixture = load_fixture("cardinality.shex");
    ShExParser::new()
        .parse(&fixture)
        .expect("cardinality.shex must parse");
}

#[test]
fn fixture_prefix_base_parses() {
    let fixture = load_fixture("prefix_base.shex");
    ShExParser::new()
        .parse(&fixture)
        .expect("prefix_base.shex must parse");
}
