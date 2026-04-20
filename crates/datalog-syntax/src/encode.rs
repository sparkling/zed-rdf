//! Encode a Datalog [`Program`] AST as a set of [`Fact`] triples.
//!
//! Subject: `<urn:x-datalog-syntax:program>`
//!
//! Predicates used:
//! - `<urn:x-datalog-syntax:rule>`   — links program to a rule blank node
//! - `<urn:x-datalog-syntax:fact>`   — links program to a fact blank node
//! - `<urn:x-datalog-syntax:head>`   — links rule blank node to an atom blank node
//! - `<urn:x-datalog-syntax:body>`   — links rule blank node to a literal blank node
//! - `<urn:x-datalog-syntax:atom>`   — links literal blank node to an atom blank node
//! - `<urn:x-datalog-syntax:arg>`    — links atom blank node to an argument literal

use std::collections::BTreeMap;

use rdf_diff::{Fact, FactProvenance, Facts};

use crate::ast::{Arg, Atom, Literal, Program, Statement};

/// Identifier for this parser, used in [`FactProvenance`].
const PARSER_ID: &str = "datalog-syntax";

/// Well-known IRIs.
const PROG_SUBJ: &str = "<urn:x-datalog-syntax:program>";
const PRED_RULE: &str = "<urn:x-datalog-syntax:rule>";
const PRED_FACT: &str = "<urn:x-datalog-syntax:fact>";
const PRED_HEAD: &str = "<urn:x-datalog-syntax:head>";
const PRED_BODY: &str = "<urn:x-datalog-syntax:body>";
const PRED_ATOM: &str = "<urn:x-datalog-syntax:atom>";
const PRED_ARG: &str = "<urn:x-datalog-syntax:arg>";

/// Allocate the next blank-node label from a shared counter.
///
/// Extracted as a free function so that multiple call sites within
/// [`encode`] can each pass `&mut counter` without triggering Rust's
/// restriction on having two simultaneous closures that both mutably
/// capture the same local variable.
fn fresh(counter: &mut usize) -> String {
    let label = format!("_:b{counter}");
    *counter += 1;
    label
}

/// Encode `program` into a canonicalised [`Facts`] collection.
#[must_use]
pub fn encode(program: &Program) -> Facts {
    let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();
    let mut counter: usize = 0;

    for stmt in &program.statements {
        match stmt {
            Statement::Fact(atom) => {
                let atom_node = fresh(&mut counter);
                // program --fact--> atom_node
                emit(&mut raw, PROG_SUBJ, PRED_FACT, &atom_node);
                encode_atom(&mut raw, &mut counter, &atom_node, atom);
            }
            Statement::Rule(rule) => {
                let rule_node = fresh(&mut counter);
                // program --rule--> rule_node
                emit(&mut raw, PROG_SUBJ, PRED_RULE, &rule_node);

                // rule_node --head--> head_atom_node
                let head_node = fresh(&mut counter);
                emit(&mut raw, &rule_node, PRED_HEAD, &head_node);
                encode_atom(&mut raw, &mut counter, &head_node, &rule.head);

                for lit in &rule.body {
                    let lit_node = fresh(&mut counter);
                    // rule_node --body--> lit_node
                    emit(&mut raw, &rule_node, PRED_BODY, &lit_node);

                    let (inner_atom, neg_marker) = match lit {
                        Literal::Positive(a) => (a, false),
                        Literal::Negative(a) => (a, true),
                    };

                    if neg_marker {
                        // Mark negation by an extra triple
                        emit(
                            &mut raw,
                            &lit_node,
                            "<urn:x-datalog-syntax:negated>",
                            "\"true\"",
                        );
                    }

                    let atom_node = fresh(&mut counter);
                    emit(&mut raw, &lit_node, PRED_ATOM, &atom_node);
                    encode_atom(&mut raw, &mut counter, &atom_node, inner_atom);
                }
            }
        }
    }

    Facts::canonicalise(raw, BTreeMap::new())
}

/// Emit triples that describe one [`Atom`].
fn encode_atom(
    out: &mut Vec<(Fact, FactProvenance)>,
    counter: &mut usize,
    atom_node: &str,
    atom: &Atom,
) {
    // atom_node --atom--> relname_literal
    emit(out, atom_node, PRED_ATOM, &format!("\"{}\"", atom.relname));

    for arg in &atom.args {
        let arg_val = match arg {
            Arg::Variable(v) => format!("\"?{v}\""),
            Arg::Constant(c) => format!("\"{c}\""),
        };
        let arg_node = fresh(counter);
        emit(out, atom_node, PRED_ARG, &arg_node);
        emit(out, &arg_node, PRED_ARG, &arg_val);
    }
}

/// Push a single RDF fact with the default parser provenance.
fn emit(out: &mut Vec<(Fact, FactProvenance)>, s: &str, p: &str, o: &str) {
    out.push((
        Fact {
            subject: s.to_owned(),
            predicate: p.to_owned(),
            object: o.to_owned(),
            graph: None,
        },
        FactProvenance {
            offset: None,
            parser: PARSER_ID.to_owned(),
        },
    ));
}
