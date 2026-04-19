//! Main SPARQL 1.1 Query + Update grammar parser.
//!
//! This crate is the Phase-C main implementation referenced by
//! ADR-0017 §4 and ADR-0018 (scope: "SPARQL 1.1 full + 1.2 behind
//! feature"). Its shadow peer lives at
//! `crates/syntax/sparql-syntax-shadow`; both implement
//! [`rdf_diff::Parser`] and are compared by the diff harness.
//!
//! # Public surface
//!
//! - [`SparqlParser`] — stateless parser handle implementing
//!   [`rdf_diff::Parser`]. Accepts SPARQL 1.1 Query forms (`SELECT`,
//!   `CONSTRUCT`, `ASK`, `DESCRIBE`) and Update operations (`INSERT
//!   DATA`, `DELETE DATA`, `DELETE WHERE`, `DELETE`/`INSERT` Modify,
//!   `LOAD`, `CLEAR`, `CREATE`, `DROP`, `COPY`, `MOVE`, `ADD`).
//! - [`DiagnosticCode`] — structured error-code enum, keyed to the
//!   pins under `docs/spec-readings/sparql/`.
//!
//! # Scope discipline
//!
//! Grammar-only — this crate parses SPARQL and emits a structural
//! AST encoded as `rdf_diff::Fact`s. It does **not** execute queries,
//! evaluate the algebra, or verify semantics beyond the grammar-level
//! static checks required by the nine adversary failure modes (FM1–FM12)
//! in `docs/verification/adversary-findings/sparql.md`.
//!
//! # Adversary-surfaced checks
//!
//! - **FM5 (`SPARQL-PROLOGUE-001`)** — `BASE`/`PREFIX` inside a WHERE
//!   clause is rejected as a parse error.
//! - **FM11b (`SPARQL-BIND-001`)** — `BIND(... AS ?x)` where `?x` is
//!   already in scope in the same group graph pattern is rejected.
//! - **FM8 (`SPARQL-UPDATE-001`)** — `INSERT DATA` forbids variables;
//!   `DELETE DATA` additionally forbids blank nodes.
//! - **FM9 (`SPARQL-PATH-001`)** — inverse-over-negated property path
//!   precedence is encoded so `^!(p)` parses as `^(!(p))` and the
//!   structural encoding reflects that.
//!
//! # AST-as-Facts encoding
//!
//! See `README.md` for the full encoding contract. Briefly: each
//! request produces facts with subject `<urn:x-sparql-syntax:request>`
//! and predicates under `<urn:x-sparql-syntax:*>`. The encoding is
//! **independent** from the shadow's encoding; the diff harness
//! compares the canonical `Facts` set.
//!
//! [`rdf_diff::Parser`]: rdf_diff::Parser

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Pedantic lint carve-outs local to this crate. Keep narrow.
#![allow(
    clippy::doc_markdown,
    clippy::redundant_pub_crate,
    clippy::missing_const_for_fn,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::struct_field_names,
    clippy::needless_pass_by_ref_mut,
    clippy::option_if_let_else,
    clippy::manual_is_ascii_check,
    clippy::manual_strip,
    clippy::unnecessary_map_or,
    clippy::assigning_clones,
    clippy::redundant_closure,
    clippy::map_unwrap_or,
    clippy::needless_continue,
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::large_enum_variant,
    clippy::unnested_or_patterns,
    clippy::uninlined_format_args,
    clippy::must_use_candidate,
    clippy::items_after_statements,
    clippy::cognitive_complexity,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::return_self_not_must_use,
    clippy::pub_underscore_fields,
    clippy::format_push_string,
    clippy::ref_option_ref,
    clippy::branches_sharing_code,
    clippy::or_fun_call,
    clippy::enum_glob_use,
    clippy::cargo_common_metadata,
    clippy::multiple_crate_versions,
    clippy::negative_feature_names,
    clippy::redundant_else,
    clippy::unused_self,
    clippy::manual_let_else,
    clippy::derivable_impls,
    clippy::fallible_impl_from,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::needless_borrow,
    clippy::trivially_copy_pass_by_ref,
    clippy::ignored_unit_patterns,
    clippy::missing_fields_in_debug,
    clippy::suspicious_else_formatting,
    clippy::unreadable_literal,
    clippy::equatable_if_let,
    clippy::semicolon_if_nothing_returned,
    clippy::useless_let_if_seq,
    clippy::explicit_iter_loop,
    clippy::single_char_add_str,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::missing_const_for_thread_local,
    clippy::redundant_type_annotations,
    clippy::unnecessary_wraps,
    clippy::derive_partial_eq_without_eq,
    clippy::pattern_type_mismatch,
    clippy::needless_collect,
    clippy::while_let_loop,
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::needless_lifetimes,
    clippy::collapsible_match,
    clippy::default_trait_access,
    clippy::used_underscore_binding,
    clippy::redundant_field_names,
    clippy::single_match,
    clippy::implicit_clone,
    clippy::range_plus_one,
    clippy::wildcard_in_or_patterns,
    clippy::implicit_hasher,
    clippy::use_self,
    clippy::enum_variant_names,
)]

mod ast;
mod diag;
mod encode;
mod grammar;
mod lexer;

pub use diag::{Diag, DiagnosticCode};

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Facts, ParseOutcome};

use encode::encode_request;
use grammar::Parser as Inner;

/// Stable parser id used in diff reports.
const SPARQL_ID: &str = "sparql-syntax";

/// Main SPARQL 1.1 parser.
///
/// Stateless — construct with [`SparqlParser::new`] (or `default()`) and
/// reuse across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct SparqlParser;

impl SparqlParser {
    /// Construct a fresh SPARQL parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl rdf_diff::Parser for SparqlParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // Early UTF-8 validation keeps downstream paths byte-safe.
        if let Err(e) = std::str::from_utf8(input) {
            return Err(Diagnostics {
                messages: vec![format!(
                    "{}: invalid UTF-8: {e}",
                    DiagnosticCode::InvalidUtf8
                )],
                fatal: true,
            });
        }
        let mut inner = Inner::new(input);
        let req = match inner.parse_request() {
            Ok(r) => r,
            Err(d) => {
                return Err(Diagnostics {
                    messages: vec![d.render()],
                    fatal: true,
                });
            }
        };
        // Capture prologue prefixes for diagnostic context on the Facts.
        let prefixes: BTreeMap<String, String> = match &req {
            crate::ast::Request::Query(q) => q
                .prefixes
                .iter()
                .cloned()
                .map(|(k, v)| (k, format!("<{v}>")))
                .collect(),
            crate::ast::Request::Update(u) => u
                .prefixes
                .iter()
                .cloned()
                .map(|(k, v)| (k, format!("<{v}>")))
                .collect(),
        };
        let raw = encode_request(&req, SPARQL_ID);
        let facts = Facts::canonicalise(raw, prefixes);
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: Vec::new(),
                fatal: false,
            },
        })
    }

    fn id(&self) -> &'static str {
        SPARQL_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdf_diff::Parser;

    fn parse(src: &str) -> Result<ParseOutcome, Diagnostics> {
        SparqlParser::new().parse(src.as_bytes())
    }

    #[test]
    fn parser_id() {
        assert_eq!(SparqlParser::new().id(), "sparql-syntax");
    }

    #[test]
    fn minimal_select_star() {
        let out = parse("SELECT * WHERE { ?s ?p ?o }").expect("parse ok");
        assert!(!out.facts.set.is_empty());
    }

    #[test]
    fn select_projection() {
        parse("SELECT ?s WHERE { ?s <http://ex/p> ?o }").expect("parse ok");
    }

    #[test]
    fn construct_template() {
        parse("CONSTRUCT { ?s <http://ex/p> ?o } WHERE { ?s <http://ex/r> ?o }")
            .expect("parse ok");
    }

    #[test]
    fn ask_form() {
        parse("ASK { ?s ?p ?o }").expect("parse ok");
    }

    #[test]
    fn describe_form() {
        parse("DESCRIBE <http://ex/s>").expect("parse ok");
        parse("DESCRIBE ?x WHERE { ?x <http://ex/p> ?o }").expect("parse ok");
    }

    #[test]
    fn optional_minus() {
        parse("SELECT * WHERE { ?s ?p ?o . OPTIONAL { ?s <http://ex/q> ?x } MINUS { ?s <http://ex/r> ?y } }").expect("parse ok");
    }

    #[test]
    fn filter_expression() {
        parse("SELECT * WHERE { ?s ?p ?o . FILTER(?o > 5) }").expect("parse ok");
    }

    #[test]
    fn bind_expression() {
        parse("SELECT * WHERE { ?s <http://ex/p> ?o . BIND(CONCAT(STR(?o), \"-suffix\") AS ?x) }")
            .expect("parse ok");
    }

    #[test]
    fn bind_scoping_violation_rejected() {
        // FM11b: BIND introduces variable already in scope.
        let err = parse("SELECT * WHERE { ?s <http://ex/q> ?x . BIND(STR(?o) AS ?x) ?s <http://ex/p> ?o }")
            .expect_err("must reject");
        assert!(
            err.messages.iter().any(|m| m.contains("SPARQL-BIND-001")),
            "expected SPARQL-BIND-001 in {:?}",
            err.messages
        );
    }

    #[test]
    fn base_mid_query_rejected() {
        // FM5: BASE inside WHERE.
        let err = parse(
            "BASE <http://example/a/>\nSELECT * WHERE {\n  <foo> <http://example/p> ?o .\n  BASE <http://example/b/>\n  <bar> ?q ?r .\n}",
        )
        .expect_err("must reject");
        assert!(
            err.messages
                .iter()
                .any(|m| m.contains("SPARQL-PROLOGUE-001")),
            "expected SPARQL-PROLOGUE-001 in {:?}",
            err.messages
        );
    }

    #[test]
    fn prefix_declarations() {
        parse("PREFIX ex: <http://ex/>\nSELECT * WHERE { ?s ex:p ?o }").expect("parse ok");
    }

    #[test]
    fn base_prologue() {
        parse("BASE <http://ex/>\nSELECT * WHERE { ?s ?p ?o }").expect("parse ok");
    }

    #[test]
    fn values_clause() {
        parse("SELECT * WHERE { VALUES ?x { 1 2 3 } ?s <http://ex/p> ?x }").expect("parse ok");
    }

    #[test]
    fn service_block() {
        parse(
            "SELECT * WHERE { SERVICE <http://endpoint/e> { ?s ?p ?o } }",
        )
        .expect("parse ok");
    }

    #[test]
    fn nested_service() {
        // FM10
        parse(
            "SELECT * WHERE { SERVICE <http://endpoint.example/outer> { ?s ?p ?o . SERVICE <http://endpoint.example/inner> { ?o ?q ?z } } }",
        )
        .expect("parse ok");
    }

    #[test]
    fn graph_variable() {
        parse("SELECT * WHERE { GRAPH ?g { ?s ?p ?o } }").expect("parse ok");
    }

    #[test]
    fn group_by_having() {
        parse(
            "SELECT (COUNT(?o) AS ?cnt) WHERE { ?s <http://ex/p> ?o } GROUP BY ?s HAVING (?cnt > 2)",
        )
        .expect("parse ok");
    }

    #[test]
    fn order_limit_offset() {
        parse("SELECT * WHERE { ?s ?p ?o } ORDER BY DESC(?o) LIMIT 10 OFFSET 5")
            .expect("parse ok");
    }

    #[test]
    fn property_path_inverse_negated() {
        // FM9
        parse(
            "SELECT * WHERE { ?s ^!(<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>) ?x }",
        )
        .expect("parse ok");
    }

    #[test]
    fn filter_not_exists() {
        // FM7
        parse("SELECT * WHERE { ?s ?p ?o . FILTER NOT EXISTS { ?s ?p ?o } }").expect("parse ok");
    }

    #[test]
    fn insert_data_bnode() {
        // FM8
        parse("INSERT DATA { GRAPH <http://ex/g> { _:b <http://ex/p> <http://ex/o1> . _:b <http://ex/q> <http://ex/o2> } }").expect("parse ok");
    }

    #[test]
    fn insert_data_variable_rejected() {
        let err =
            parse("INSERT DATA { <http://ex/s> <http://ex/p> ?x }").expect_err("must reject");
        assert!(
            err.messages.iter().any(|m| m.contains("SPARQL-UPDATE-001")),
            "expected SPARQL-UPDATE-001 in {:?}",
            err.messages
        );
    }

    #[test]
    fn delete_data_bnode_rejected() {
        let err = parse("DELETE DATA { _:b <http://ex/p> <http://ex/o> }")
            .expect_err("must reject");
        assert!(
            err.messages.iter().any(|m| m.contains("SPARQL-UPDATE-001")),
            "expected SPARQL-UPDATE-001 in {:?}",
            err.messages
        );
    }

    #[test]
    fn delete_insert_modify() {
        parse(
            "DELETE { ?s <http://ex/p> ?o } INSERT { ?s <http://ex/q> ?o } WHERE { ?s <http://ex/p> ?o }",
        )
        .expect("parse ok");
    }

    #[test]
    fn with_modify() {
        parse(
            "WITH <http://ex/g> DELETE { ?s ?p ?o } INSERT { ?s ?p ?o } WHERE { ?s ?p ?o }",
        )
        .expect("parse ok");
    }

    #[test]
    fn load_clear_create_drop() {
        parse("LOAD <http://ex/s>").expect("parse ok");
        parse("LOAD SILENT <http://ex/s> INTO GRAPH <http://ex/g>").expect("parse ok");
        parse("CLEAR ALL").expect("parse ok");
        parse("CLEAR GRAPH <http://ex/g>").expect("parse ok");
        parse("CREATE GRAPH <http://ex/g>").expect("parse ok");
        parse("DROP DEFAULT").expect("parse ok");
    }

    #[test]
    fn copy_move_add() {
        parse("COPY DEFAULT TO GRAPH <http://ex/g>").expect("parse ok");
        parse("MOVE GRAPH <http://ex/a> TO GRAPH <http://ex/b>").expect("parse ok");
        parse("ADD GRAPH <http://ex/a> TO DEFAULT").expect("parse ok");
    }

    #[test]
    fn subquery_projection() {
        // FM12
        parse(
            "SELECT ?s ?o ?internal WHERE { ?s <http://ex/p> ?o . { SELECT ?s WHERE { ?s <http://ex/p> ?o . BIND(42 AS ?internal) } } }",
        )
        .expect("parse ok");
    }

    #[test]
    fn minus_no_shared_variable() {
        // FM2
        parse(
            "SELECT * WHERE { ?s <http://ex/p> ?o . MINUS { ?x <http://ex/q> ?y } }",
        )
        .expect("parse ok");
    }

    #[test]
    fn construct_bnode_per_row() {
        // FM3
        parse(
            "CONSTRUCT { ?s <http://ex/p> _:b . _:b <http://ex/q> ?o } WHERE { ?s <http://ex/r> ?o }",
        )
        .expect("parse ok");
    }

    #[test]
    fn optional_filter_unbound() {
        // FM1
        parse("SELECT * WHERE { ?s <http://ex/p> ?o . OPTIONAL { ?s <http://ex/q> ?x . FILTER(?x > 5) } FILTER(?x > 3) }").expect("parse ok");
    }

    #[test]
    fn having_select_alias() {
        // FM4
        parse("SELECT (COUNT(?o) AS ?cnt) (SUM(?v) AS ?total) WHERE { ?s <http://ex/p> ?o ; <http://ex/q> ?v } GROUP BY ?s HAVING (?cnt > 2)").expect("parse ok");
    }

    #[test]
    fn language_tagged_literal() {
        parse("SELECT * WHERE { ?s ?p \"hello\"@en-US }").expect("parse ok");
    }

    #[test]
    fn typed_literal() {
        parse("SELECT * WHERE { ?s ?p \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> }")
            .expect("parse ok");
    }

    #[test]
    fn numeric_literals() {
        parse("SELECT * WHERE { ?s ?p 42 , 3.14 , 6.022e23 }").expect("parse ok");
    }

    #[test]
    fn boolean_literals() {
        parse("SELECT * WHERE { ?s ?p true , false }").expect("parse ok");
    }
}
