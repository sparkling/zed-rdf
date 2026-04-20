//! AST → `(Fact, FactProvenance)` encoder for the ShEx compact syntax parser.
//!
//! The encoding uses a namespace under `urn:x-shex-syntax:` for all
//! predicates. The schema top-level subject is `<urn:x-shex-syntax:schema>`.
//!
//! # Fact vocabulary
//!
//! | Predicate | Domain | Object |
//! |---|---|---|
//! | `<urn:x-shex-syntax:base>` | schema | IRI string literal |
//! | `<urn:x-shex-syntax:prefix>` | schema | `"name: <iri>"` literal |
//! | `<urn:x-shex-syntax:shape>` | schema | shape label IRI |
//! | `<urn:x-shex-syntax:shape/closed>` | shape | `"true"` |
//! | `<urn:x-shex-syntax:shape/extends>` | shape | extended shape label IRI |
//! | `<urn:x-shex-syntax:shape/tripleConstraint>` | shape | blank node |
//! | `<urn:x-shex-syntax:tc/predicate>` | triple constraint | predicate IRI |
//! | `<urn:x-shex-syntax:tc/inverse>` | triple constraint | `"true"` |
//! | `<urn:x-shex-syntax:tc/cardinality>` | triple constraint | `"min,max"` literal |
//! | `<urn:x-shex-syntax:tc/valueConstraint>` | triple constraint | literal description |
//! | `<urn:x-shex-syntax:nodeConstraint>` | shape | node constraint kind literal |

use std::collections::BTreeMap;

use rdf_diff::{Fact, FactProvenance};

use crate::ast::{
    Cardinality, DatatypeRef, NodeConstraint, Predicate, Schema, ShapeExpr, TripleExpr,
};
use crate::parser::{expand_label, ParseError};

const SCHEMA_SUBJECT: &str = "<urn:x-shex-syntax:schema>";
const NS: &str = "urn:x-shex-syntax:";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Encode a `Schema` into a flat list of canonical facts.
///
/// The `parser_id` is used to fill `FactProvenance::parser`.
pub(crate) fn encode(
    schema: &Schema,
    parser_id: &'static str,
) -> Result<Vec<(Fact, FactProvenance)>, ParseError> {
    let mut enc = Encoder {
        out: Vec::new(),
        parser_id,
        bnode_counter: 0,
    };

    // Build prefix map for expansion.
    let prefix_map: BTreeMap<String, String> = schema
        .prefixes
        .iter()
        .map(|(p, iri)| (p.clone(), iri.clone()))
        .collect();

    // Encode BASE.
    if let Some(base) = &schema.base {
        enc.emit(
            SCHEMA_SUBJECT,
            pred("base"),
            &format!("\"{base}\""),
            0,
        );
    }

    // Encode PREFIX declarations.
    for (prefix, iri) in &schema.prefixes {
        let obj = format!("\"{prefix}: <{iri}>\"");
        enc.emit(SCHEMA_SUBJECT, pred("prefix"), &obj, 0);
    }

    // Encode each shape declaration.
    for decl in &schema.shapes {
        let label_iri =
            expand_label(&decl.label, &prefix_map, schema.base.as_deref())?;

        // schema → shape
        enc.emit(SCHEMA_SUBJECT, pred("shape"), &label_iri, decl.offset);

        // Encode the shape expression.
        enc.encode_shape_expr(&label_iri, &decl.expr, &prefix_map, schema.base.as_deref(), decl.offset)?;
    }

    Ok(enc.out)
}

struct Encoder {
    out: Vec<(Fact, FactProvenance)>,
    parser_id: &'static str,
    bnode_counter: usize,
}

impl Encoder {
    fn emit(&mut self, subject: &str, predicate: impl AsRef<str>, object: &str, offset: usize) {
        let predicate = predicate.as_ref();
        self.out.push((
            Fact {
                subject: subject.to_owned(),
                predicate: predicate.to_owned(),
                object: object.to_owned(),
                graph: None,
            },
            FactProvenance {
                offset: Some(offset),
                parser: self.parser_id.to_owned(),
            },
        ));
    }

    fn fresh_bnode(&mut self) -> String {
        let n = self.bnode_counter;
        self.bnode_counter += 1;
        format!("_:b{n}")
    }

    fn encode_shape_expr(
        &mut self,
        subject: &str,
        expr: &ShapeExpr,
        prefixes: &BTreeMap<String, String>,
        base: Option<&str>,
        offset: usize,
    ) -> Result<(), ParseError> {
        match expr {
            ShapeExpr::Shape(shape) => {
                if shape.closed {
                    self.emit(subject, pred("shape/closed"), "\"true\"", offset);
                }
                for ext_label in &shape.extends {
                    let ext_iri = expand_label(ext_label, prefixes, base)?;
                    self.emit(subject, pred("shape/extends"), &ext_iri, offset);
                }
                for te in &shape.triple_exprs {
                    self.encode_triple_expr(subject, te, prefixes, base)?;
                }
            }
            ShapeExpr::NodeConstraint(nc) => {
                let nc_lit = encode_node_constraint(nc, prefixes, base)?;
                self.emit(subject, pred("nodeConstraint"), &nc_lit, offset);
            }
            ShapeExpr::Ref(ref_label) => {
                let ref_iri = expand_label(ref_label, prefixes, base)?;
                self.emit(subject, pred("shape/ref"), &ref_iri, offset);
            }
            ShapeExpr::And(lhs, rhs) => {
                self.encode_shape_expr(subject, lhs, prefixes, base, offset)?;
                self.encode_shape_expr(subject, rhs, prefixes, base, offset)?;
                self.emit(subject, pred("shape/and"), "\"true\"", offset);
            }
            ShapeExpr::Or(lhs, rhs) => {
                self.encode_shape_expr(subject, lhs, prefixes, base, offset)?;
                self.encode_shape_expr(subject, rhs, prefixes, base, offset)?;
                self.emit(subject, pred("shape/or"), "\"true\"", offset);
            }
            ShapeExpr::Not(inner) => {
                self.encode_shape_expr(subject, inner, prefixes, base, offset)?;
                self.emit(subject, pred("shape/not"), "\"true\"", offset);
            }
            ShapeExpr::Any => {
                self.emit(subject, pred("nodeConstraint"), "\"ANY\"", offset);
            }
        }
        Ok(())
    }

    fn encode_triple_expr(
        &mut self,
        shape_subject: &str,
        te: &TripleExpr,
        prefixes: &BTreeMap<String, String>,
        base: Option<&str>,
    ) -> Result<(), ParseError> {
        match te {
            TripleExpr::Constraint(tc) => {
                let bn = self.fresh_bnode();
                self.emit(shape_subject, pred("shape/tripleConstraint"), &bn, tc.offset);

                // Predicate.
                let pred_iri = encode_predicate(&tc.predicate, prefixes, base)?;
                self.emit(&bn, pred("tc/predicate"), &pred_iri, tc.offset);

                // Inverse.
                if tc.inverse {
                    self.emit(&bn, pred("tc/inverse"), "\"true\"", tc.offset);
                }

                // Cardinality.
                let card_lit = encode_cardinality(tc.cardinality);
                self.emit(&bn, pred("tc/cardinality"), &card_lit, tc.offset);

                // Value expression.
                if let Some(ve) = &tc.value_expr {
                    let ve_lit = encode_inline_expr(ve, prefixes, base)?;
                    self.emit(&bn, pred("tc/valueConstraint"), &ve_lit, tc.offset);
                }
            }
            TripleExpr::OneOf(members) => {
                for m in members {
                    self.encode_triple_expr(shape_subject, m, prefixes, base)?;
                }
            }
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------
// Encoding helpers
// -----------------------------------------------------------------------

fn pred(local: &str) -> String {
    format!("<{NS}{local}>")
}

fn encode_predicate(
    p: &Predicate,
    prefixes: &BTreeMap<String, String>,
    _base: Option<&str>,
) -> Result<String, ParseError> {
    match p {
        Predicate::RdfType => Ok(format!("<{RDF_TYPE}>")),
        Predicate::Iri(iri) => Ok(format!("<{iri}>")),
        Predicate::Pname { prefix, local } => {
            let ns = prefixes.get(prefix.as_str()).ok_or_else(|| {
                ParseError::new(0, format!("undefined prefix '{prefix}:' in predicate"))
            })?;
            Ok(format!("<{ns}{local}>"))
        }
    }
}

fn encode_node_constraint(
    nc: &NodeConstraint,
    prefixes: &BTreeMap<String, String>,
    _base: Option<&str>,
) -> Result<String, ParseError> {
    let s = match nc {
        NodeConstraint::Iri => "\"IRI\"".to_owned(),
        NodeConstraint::Literal => "\"LITERAL\"".to_owned(),
        NodeConstraint::NonLiteral => "\"NONLITERAL\"".to_owned(),
        NodeConstraint::BNode => "\"BNODE\"".to_owned(),
        NodeConstraint::Datatype(dt) => {
            let iri = match dt {
                DatatypeRef::Iri(i) => format!("<{i}>"),
                DatatypeRef::Pname { prefix, local } => {
                    let ns = prefixes.get(prefix.as_str()).ok_or_else(|| {
                        ParseError::new(0, format!("undefined prefix '{prefix}:' in datatype"))
                    })?;
                    format!("<{ns}{local}>")
                }
            };
            format!("\"datatype:{iri}\"")
        }
        NodeConstraint::ValueSet(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|item| match item {
                    crate::ast::ValueSetItem::Iri(i) => format!("<{i}>"),
                    crate::ast::ValueSetItem::Pname { prefix, local } => {
                        format!("{prefix}:{local}")
                    }
                    crate::ast::ValueSetItem::Literal(l) => l.clone(),
                })
                .collect();
            format!("\"valueSet:{parts}\"", parts = parts.join(" "))
        }
    };
    Ok(s)
}

fn encode_inline_expr(
    expr: &ShapeExpr,
    prefixes: &BTreeMap<String, String>,
    base: Option<&str>,
) -> Result<String, ParseError> {
    match expr {
        ShapeExpr::NodeConstraint(nc) => encode_node_constraint(nc, prefixes, base),
        ShapeExpr::Ref(label) => {
            let iri = expand_label(label, prefixes, base)?;
            Ok(format!("\"ref:{iri}\""))
        }
        ShapeExpr::Any => Ok("\"ANY\"".to_owned()),
        ShapeExpr::Shape(_) => Ok("\"shape\"".to_owned()),
        ShapeExpr::And(_, _) => Ok("\"and\"".to_owned()),
        ShapeExpr::Or(_, _) => Ok("\"or\"".to_owned()),
        ShapeExpr::Not(_) => Ok("\"not\"".to_owned()),
    }
}

fn encode_cardinality(c: Cardinality) -> String {
    match c {
        Cardinality::One => "\"1,1\"".to_owned(),
        Cardinality::Optional => "\"0,1\"".to_owned(),
        Cardinality::Star => "\"0,*\"".to_owned(),
        Cardinality::Plus => "\"1,*\"".to_owned(),
        Cardinality::Exact(n) => format!("\"{n},{n}\""),
        Cardinality::Range(n, m) => format!("\"{n},{m}\""),
        Cardinality::AtLeast(n) => format!("\"{n},*\""),
    }
}
