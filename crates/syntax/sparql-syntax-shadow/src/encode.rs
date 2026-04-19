//! AST-as-Facts encoding for `sparql-syntax-shadow`.
//!
//! # Encoding specification
//!
//! Every node in the SPARQL AST is assigned a synthetic subject IRI of the
//! form `_shadow:N` (where N is a monotonically-increasing integer). The
//! document root is always `_shadow:0`.
//!
//! Structural relationships are expressed as triples:
//!
//! ```text
//! <subject>  <predicate>            <object>
//! _shadow:N  sparql:type            sparql:Select
//! _shadow:N  sparql:projection      sparql:Star
//! _shadow:N  sparql:var             "x"
//! _shadow:N  sparql:triple          _shadow:M
//! _shadow:M  sparql:subject         ...
//! _shadow:M  sparql:predicate       ...
//! _shadow:M  sparql:object          ...
//! ```
//!
//! The `sparql:` prefix expands to `https://shadow.sparql/ast#`.
//!
//! ## Canonical form rules
//!
//! 1. All IRIs are stored in angle-bracket form: `<https://...>`.
//! 2. Prefixed names are stored as-is (not expanded) because the diff
//!    harness compares at the grammar level, not the semantic level.
//!    Prefix resolution is a separate semantic step.
//! 3. Literal lexical forms are stored exactly as written (including
//!    surrounding quotes). Language tags are lower-cased per RFC 5646.
//! 4. Blank-node labels are stored as `_:label`.
//! 5. Variable names are stored as `?name`.
//! 6. Anonymous blank nodes are assigned sequential labels `_:anon0`,
//!    `_:anon1`, … within the document.
//! 7. The `rdf:type` shorthand `a` is stored as `<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>`.
//! 8. Numeric literals carry their XSD datatype in the object field:
//!    `"42"^^<...xsd#integer>`.
//! 9. Graph names for triples: patterns inside `GRAPH <g>` blocks are
//!    emitted with `graph = Some("<g>")`.
//!
//! ## Fact namespace
//!
//! | Predicate | Meaning |
//! |-----------|---------|
//! | `sparql:type` | RDF type of the node |
//! | `sparql:queryForm` | query form (Select/Construct/Ask/Describe) |
//! | `sparql:projection` | Star or list anchor |
//! | `sparql:projectionVar` | projected variable |
//! | `sparql:projectionExpr` | (expr AS ?alias) pair |
//! | `sparql:modifier` | DISTINCT / REDUCED |
//! | `sparql:dataset` | dataset clause node |
//! | `sparql:named` | true if FROM NAMED |
//! | `sparql:graphIri` | IRI of the named graph |
//! | `sparql:where` | where-clause node |
//! | `sparql:element` | element within a group pattern |
//! | `sparql:index` | 0-based position within a list |
//! | `sparql:triple` | a triple-pattern node |
//! | `sparql:subject` | triple subject |
//! | `sparql:predicate` | triple predicate |
//! | `sparql:object` | triple object |
//! | `sparql:filter` | filter expression node |
//! | `sparql:bind` | bind expression node |
//! | `sparql:bindVar` | target variable of BIND |
//! | `sparql:optional` | optional block node |
//! | `sparql:minus` | minus block node |
//! | `sparql:union` | union node (two children) |
//! | `sparql:left` | left child of union |
//! | `sparql:right` | right child of union |
//! | `sparql:service` | service node |
//! | `sparql:silent` | true if SILENT |
//! | `sparql:endpoint` | service endpoint |
//! | `sparql:graph` | GRAPH name |
//! | `sparql:subquery` | subquery node |
//! | `sparql:values` | VALUES clause node |
//! | `sparql:valuesVar` | variable in VALUES |
//! | `sparql:valuesRow` | one row in VALUES |
//! | `sparql:valuesCell` | one cell in VALUES |
//! | `sparql:path` | property path node |
//! | `sparql:pathKind` | path kind string |
//! | `sparql:pathChild` | child path |
//! | `sparql:pathLeft` | left child of binary path |
//! | `sparql:pathRight` | right child of binary path |
//! | `sparql:groupBy` | GROUP BY list anchor |
//! | `sparql:orderBy` | ORDER BY list anchor |
//! | `sparql:direction` | ASC or DESC |
//! | `sparql:limit` | LIMIT value |
//! | `sparql:offset` | OFFSET value |
//! | `sparql:updateOp` | an update operation node |
//! | `sparql:opKind` | kind of update op |
//! | `sparql:source` | LOAD source |
//! | `sparql:into` | LOAD INTO graph |
//! | `sparql:expr` | expression sub-node |
//! | `sparql:op` | binary operator string |
//! | `sparql:lhs` | left-hand side |
//! | `sparql:rhs` | right-hand side |
//! | `sparql:literal` | a literal value string |
//! | `sparql:iri` | an IRI value string |
//! | `sparql:var` | a variable name |
//! | `sparql:base` | BASE IRI |
//! | `sparql:prefix` | prefix declaration node |
//! | `sparql:prefixLabel` | prefix label |
//! | `sparql:prefixIri` | prefix IRI |

use std::collections::BTreeMap;

use rdf_diff::{Fact, FactProvenance, Facts};

use crate::ast::*;

const SPARQL: &str = "https://shadow.sparql/ast#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DEC: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DBL: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOL: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Encode an AST [`Document`] into a [`Facts`] set.
pub fn encode_document(doc: &Document) -> Facts {
    let mut enc = Encoder::new();
    enc.encode_doc(doc);
    enc.into_facts()
}

struct Encoder {
    counter: usize,
    facts: Vec<(Fact, FactProvenance)>,
    prefixes: BTreeMap<String, String>,
}

impl Encoder {
    fn new() -> Self {
        Self {
            counter: 0,
            facts: Vec::new(),
            prefixes: BTreeMap::new(),
        }
    }

    fn fresh(&mut self) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("_shadow:{n}")
    }

    fn emit(&mut self, subject: &str, predicate: &str, object: &str, graph: Option<&str>) {
        self.facts.push((
            Fact {
                subject: subject.to_owned(),
                predicate: predicate.to_owned(),
                object: object.to_owned(),
                graph: graph.map(ToOwned::to_owned),
            },
            FactProvenance {
                offset: None,
                parser: "sparql-syntax-shadow".to_owned(),
            },
        ));
    }

    fn p(&self, local: &str) -> String {
        format!("<{SPARQL}{local}>")
    }

    fn into_facts(self) -> Facts {
        // NOTE: Facts::canonicalise is todo!() in rdf-diff (filled by
        // v1-diff-core). We directly build a Facts by inserting into the
        // BTreeMap ourselves. This is correct because our output is already
        // in canonical form per the encoding spec above.
        let mut set = BTreeMap::new();
        for (fact, prov) in self.facts {
            set.insert(fact, prov);
        }
        Facts {
            set,
            prefixes: self.prefixes,
        }
    }

    // ── Document ─────────────────────────────────────────────────────────

    fn encode_doc(&mut self, doc: &Document) {
        let root = "_shadow:0".to_owned();
        self.emit(&root, &format!("<{RDF_TYPE}>"), &self.p("Document"), None);

        // Prologue
        self.encode_prologue(&root, &doc.prologue);

        // Body
        match &doc.body {
            DocumentBody::Query(q) => {
                let qn = self.fresh();
                self.emit(&root, &self.p("query"), &qn, None);
                self.encode_query(&qn, q, None);
            }
            DocumentBody::Update(ops) => {
                for (i, op) in ops.iter().enumerate() {
                    let on = self.fresh();
                    self.emit(&root, &self.p("updateOp"), &on, None);
                    self.emit(&on, &self.p("index"), &format!("{i}"), None);
                    self.encode_update_op(&on, op);
                }
            }
        }
    }

    fn encode_prologue(&mut self, parent: &str, prologue: &Prologue) {
        if let Some(base) = &prologue.base {
            self.emit(parent, &self.p("base"), &format!("<{base}>"), None);
            self.prefixes
                .insert("(base)".to_owned(), base.clone());
        }
        for decl in &prologue.prefixes {
            let dn = self.fresh();
            self.emit(parent, &self.p("prefix"), &dn, None);
            self.emit(&dn, &self.p("prefixLabel"), &format!("\"{}\"", decl.prefix), None);
            self.emit(&dn, &self.p("prefixIri"), &format!("<{}>", decl.iri), None);
            self.prefixes
                .insert(decl.prefix.clone(), decl.iri.clone());
        }
    }

    // ── Query ─────────────────────────────────────────────────────────────

    fn encode_query(&mut self, node: &str, q: &Query, ctx_graph: Option<&str>) {
        self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Query"), ctx_graph);

        // Form
        match &q.form {
            QueryForm::Select(sc) => self.encode_select(node, sc, ctx_graph),
            QueryForm::Construct(ct) => self.encode_construct(node, ct, ctx_graph),
            QueryForm::Ask => {
                self.emit(node, &self.p("queryForm"), &self.p("Ask"), ctx_graph);
            }
            QueryForm::Describe(items) => {
                self.emit(node, &self.p("queryForm"), &self.p("Describe"), ctx_graph);
                for (i, item) in items.iter().enumerate() {
                    let vn = self.fresh();
                    self.emit(node, &self.p("describeItem"), &vn, ctx_graph);
                    self.emit(&vn, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_var_or_iri(&vn, item, ctx_graph);
                }
            }
        }

        // Dataset
        for (i, ds) in q.dataset.iter().enumerate() {
            let dn = self.fresh();
            self.emit(node, &self.p("dataset"), &dn, ctx_graph);
            self.emit(&dn, &self.p("index"), &format!("{i}"), ctx_graph);
            self.emit(
                &dn,
                &self.p("named"),
                if ds.named { "\"true\"" } else { "\"false\"" },
                ctx_graph,
            );
            let iri_str = iri_to_string(&ds.iri);
            self.emit(&dn, &self.p("graphIri"), &iri_str, ctx_graph);
        }

        // WHERE clause
        let wn = self.fresh();
        self.emit(node, &self.p("where"), &wn, ctx_graph);
        self.encode_group_graph_pattern(&wn, &q.where_clause, ctx_graph);

        // Modifiers
        self.encode_solution_modifiers(node, &q.modifiers, ctx_graph);

        // VALUES
        if let Some(vals) = &q.values {
            let vn = self.fresh();
            self.emit(node, &self.p("values"), &vn, ctx_graph);
            self.encode_values_clause(&vn, vals, ctx_graph);
        }
    }

    fn encode_select(&mut self, node: &str, sc: &SelectClause, ctx_graph: Option<&str>) {
        self.emit(node, &self.p("queryForm"), &self.p("Select"), ctx_graph);

        if let Some(m) = &sc.modifier {
            let mstr = match m {
                SelectModifier::Distinct => "\"DISTINCT\"",
                SelectModifier::Reduced => "\"REDUCED\"",
            };
            self.emit(node, &self.p("modifier"), mstr, ctx_graph);
        }

        match &sc.projection {
            SelectProjection::Star => {
                self.emit(node, &self.p("projection"), &self.p("Star"), ctx_graph);
            }
            SelectProjection::Vars(vars) => {
                self.emit(node, &self.p("projection"), &self.p("VarList"), ctx_graph);
                for (i, sv) in vars.iter().enumerate() {
                    let vn = self.fresh();
                    self.emit(node, &self.p("projectionItem"), &vn, ctx_graph);
                    self.emit(&vn, &self.p("index"), &format!("{i}"), ctx_graph);
                    match sv {
                        SelectVar::Var(name) => {
                            self.emit(&vn, &self.p("var"), &format!("?{name}"), ctx_graph);
                        }
                        SelectVar::Alias { expr, alias } => {
                            let en = self.fresh();
                            self.emit(&vn, &self.p("expr"), &en, ctx_graph);
                            self.encode_expr(&en, expr, ctx_graph);
                            self.emit(&vn, &self.p("alias"), &format!("?{alias}"), ctx_graph);
                        }
                    }
                }
            }
        }
    }

    fn encode_construct(&mut self, node: &str, ct: &ConstructTemplate, ctx_graph: Option<&str>) {
        self.emit(node, &self.p("queryForm"), &self.p("Construct"), ctx_graph);
        match ct {
            ConstructTemplate::Where => {
                self.emit(node, &self.p("constructTemplate"), &self.p("Where"), ctx_graph);
            }
            ConstructTemplate::Template(triples) => {
                let tn = self.fresh();
                self.emit(node, &self.p("constructTemplate"), &tn, ctx_graph);
                for (i, tp) in triples.iter().enumerate() {
                    let tpn = self.fresh();
                    self.emit(&tn, &self.p("triple"), &tpn, ctx_graph);
                    self.emit(&tpn, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_triple_pattern(&tpn, tp, ctx_graph);
                }
            }
        }
    }

    // ── Group Graph Pattern ───────────────────────────────────────────────

    fn encode_group_graph_pattern(
        &mut self,
        node: &str,
        ggp: &GroupGraphPattern,
        ctx_graph: Option<&str>,
    ) {
        match ggp {
            GroupGraphPattern::Group(elems) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("GroupGraphPattern"), ctx_graph);
                for (i, elem) in elems.iter().enumerate() {
                    let en = self.fresh();
                    self.emit(node, &self.p("element"), &en, ctx_graph);
                    self.emit(&en, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_graph_pattern_element(&en, elem, ctx_graph);
                }
            }
            GroupGraphPattern::SubQuery(q) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("SubQuery"), ctx_graph);
                let qn = self.fresh();
                self.emit(node, &self.p("subquery"), &qn, ctx_graph);
                self.encode_query(&qn, q, ctx_graph);
            }
        }
    }

    fn encode_graph_pattern_element(
        &mut self,
        node: &str,
        elem: &GraphPatternElement,
        ctx_graph: Option<&str>,
    ) {
        match elem {
            GraphPatternElement::Triple(tp) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("TriplePattern"), ctx_graph);
                self.encode_triple_pattern(node, tp, ctx_graph);
            }
            GraphPatternElement::Filter(expr) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Filter"), ctx_graph);
                let en = self.fresh();
                self.emit(node, &self.p("filter"), &en, ctx_graph);
                self.encode_expr(&en, expr, ctx_graph);
            }
            GraphPatternElement::Optional(ggp) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Optional"), ctx_graph);
                let inner = self.fresh();
                self.emit(node, &self.p("optional"), &inner, ctx_graph);
                self.encode_group_graph_pattern(&inner, ggp, ctx_graph);
            }
            GraphPatternElement::Minus(ggp) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Minus"), ctx_graph);
                let inner = self.fresh();
                self.emit(node, &self.p("minus"), &inner, ctx_graph);
                self.encode_group_graph_pattern(&inner, ggp, ctx_graph);
            }
            GraphPatternElement::Union(left, right) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Union"), ctx_graph);
                let ln = self.fresh();
                let rn = self.fresh();
                self.emit(node, &self.p("left"), &ln, ctx_graph);
                self.emit(node, &self.p("right"), &rn, ctx_graph);
                self.encode_group_graph_pattern(&ln, left, ctx_graph);
                self.encode_group_graph_pattern(&rn, right, ctx_graph);
            }
            GraphPatternElement::Graph { name, pattern } => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Graph"), ctx_graph);
                let gname_str = match name {
                    VarOrIri::Var(v) => format!("?{v}"),
                    VarOrIri::Iri(iri) => iri_to_string(iri),
                };
                self.emit(node, &self.p("graph"), &gname_str, ctx_graph);
                // Emit inner patterns with the graph name as ctx_graph
                let new_ctx = match name {
                    VarOrIri::Iri(iri) => {
                        let s = iri_to_string(iri);
                        // Use angle-bracket form as graph name
                        Some(s)
                    }
                    _ => ctx_graph.map(ToOwned::to_owned),
                };
                let inner = self.fresh();
                self.emit(node, &self.p("graphPattern"), &inner, ctx_graph);
                self.encode_group_graph_pattern(
                    &inner,
                    pattern,
                    new_ctx.as_deref(),
                );
            }
            GraphPatternElement::Service {
                endpoint,
                silent,
                pattern,
            } => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Service"), ctx_graph);
                self.emit(
                    node,
                    &self.p("silent"),
                    if *silent { "\"true\"" } else { "\"false\"" },
                    ctx_graph,
                );
                let ep_str = match endpoint {
                    VarOrIri::Var(v) => format!("?{v}"),
                    VarOrIri::Iri(iri) => iri_to_string(iri),
                };
                self.emit(node, &self.p("endpoint"), &ep_str, ctx_graph);
                let inner = self.fresh();
                self.emit(node, &self.p("servicePattern"), &inner, ctx_graph);
                self.encode_group_graph_pattern(&inner, pattern, ctx_graph);
            }
            GraphPatternElement::Bind { expr, var } => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("Bind"), ctx_graph);
                let en = self.fresh();
                self.emit(node, &self.p("bind"), &en, ctx_graph);
                self.encode_expr(&en, expr, ctx_graph);
                self.emit(node, &self.p("bindVar"), &format!("?{var}"), ctx_graph);
            }
            GraphPatternElement::InlineData(vals) => {
                self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("InlineData"), ctx_graph);
                let vn = self.fresh();
                self.emit(node, &self.p("values"), &vn, ctx_graph);
                self.encode_values_clause(&vn, vals, ctx_graph);
            }
        }
    }

    // ── Triple patterns ───────────────────────────────────────────────────

    fn encode_triple_pattern(
        &mut self,
        node: &str,
        tp: &TriplePattern,
        ctx_graph: Option<&str>,
    ) {
        let subj = term_to_string(&tp.subject);
        self.emit(node, &self.p("subject"), &subj, ctx_graph);

        match &tp.predicate {
            Predicate::Term(t) => {
                let pred = term_to_string(t);
                self.emit(node, &self.p("predicate"), &pred, ctx_graph);
            }
            Predicate::Path(path) => {
                let pn = self.fresh();
                self.emit(node, &self.p("predicatePath"), &pn, ctx_graph);
                self.encode_path(&pn, path, ctx_graph);
            }
        }

        let obj = term_to_string(&tp.object);
        self.emit(node, &self.p("object"), &obj, ctx_graph);
    }

    // ── Property paths ────────────────────────────────────────────────────

    fn encode_path(&mut self, node: &str, path: &PathExpr, ctx_graph: Option<&str>) {
        match path {
            PathExpr::Iri(iri) => {
                self.emit(node, &self.p("pathKind"), "\"iri\"", ctx_graph);
                self.emit(node, &self.p("iri"), &iri_to_string(iri), ctx_graph);
            }
            PathExpr::Inverse(inner) => {
                self.emit(node, &self.p("pathKind"), "\"inverse\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
            PathExpr::Negated(inner) => {
                self.emit(node, &self.p("pathKind"), "\"negated\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
            PathExpr::Sequence(l, r) => {
                self.emit(node, &self.p("pathKind"), "\"sequence\"", ctx_graph);
                let ln = self.fresh();
                let rn = self.fresh();
                self.emit(node, &self.p("pathLeft"), &ln, ctx_graph);
                self.emit(node, &self.p("pathRight"), &rn, ctx_graph);
                self.encode_path(&ln, l, ctx_graph);
                self.encode_path(&rn, r, ctx_graph);
            }
            PathExpr::Alternative(l, r) => {
                self.emit(node, &self.p("pathKind"), "\"alternative\"", ctx_graph);
                let ln = self.fresh();
                let rn = self.fresh();
                self.emit(node, &self.p("pathLeft"), &ln, ctx_graph);
                self.emit(node, &self.p("pathRight"), &rn, ctx_graph);
                self.encode_path(&ln, l, ctx_graph);
                self.encode_path(&rn, r, ctx_graph);
            }
            PathExpr::ZeroOrOne(inner) => {
                self.emit(node, &self.p("pathKind"), "\"zeroOrOne\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
            PathExpr::ZeroOrMore(inner) => {
                self.emit(node, &self.p("pathKind"), "\"zeroOrMore\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
            PathExpr::OneOrMore(inner) => {
                self.emit(node, &self.p("pathKind"), "\"oneOrMore\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
            PathExpr::Group(inner) => {
                self.emit(node, &self.p("pathKind"), "\"group\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("pathChild"), &cn, ctx_graph);
                self.encode_path(&cn, inner, ctx_graph);
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn encode_expr(&mut self, node: &str, expr: &Expr, ctx_graph: Option<&str>) {
        match expr {
            Expr::Var(v) => {
                self.emit(node, &self.p("exprKind"), "\"var\"", ctx_graph);
                self.emit(node, &self.p("var"), &format!("?{v}"), ctx_graph);
            }
            Expr::Iri(iri) => {
                self.emit(node, &self.p("exprKind"), "\"iri\"", ctx_graph);
                self.emit(node, &self.p("iri"), &iri_to_string(iri), ctx_graph);
            }
            Expr::Literal(lit) => {
                self.emit(node, &self.p("exprKind"), "\"literal\"", ctx_graph);
                let obj = literal_to_string(lit);
                self.emit(node, &self.p("literal"), &obj, ctx_graph);
            }
            Expr::Bool(b) => {
                self.emit(node, &self.p("exprKind"), "\"literal\"", ctx_graph);
                let obj = format!(
                    "\"{}\"^^<{XSD_BOOL}>",
                    if *b { "true" } else { "false" }
                );
                self.emit(node, &self.p("literal"), &obj, ctx_graph);
            }
            Expr::Integer(n) => {
                self.emit(node, &self.p("exprKind"), "\"literal\"", ctx_graph);
                let obj = format!("\"{}\"^^<{XSD_INT}>", n);
                self.emit(node, &self.p("literal"), &obj, ctx_graph);
            }
            Expr::Decimal(s) => {
                self.emit(node, &self.p("exprKind"), "\"literal\"", ctx_graph);
                let obj = format!("\"{}\"^^<{XSD_DEC}>", s);
                self.emit(node, &self.p("literal"), &obj, ctx_graph);
            }
            Expr::Double(s) => {
                self.emit(node, &self.p("exprKind"), "\"literal\"", ctx_graph);
                let obj = format!("\"{}\"^^<{XSD_DBL}>", s);
                self.emit(node, &self.p("literal"), &obj, ctx_graph);
            }
            Expr::Not(inner) => {
                self.emit(node, &self.p("exprKind"), "\"not\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("expr"), &cn, ctx_graph);
                self.encode_expr(&cn, inner, ctx_graph);
            }
            Expr::Neg(inner) => {
                self.emit(node, &self.p("exprKind"), "\"neg\"", ctx_graph);
                let cn = self.fresh();
                self.emit(node, &self.p("expr"), &cn, ctx_graph);
                self.encode_expr(&cn, inner, ctx_graph);
            }
            Expr::BinOp { op, lhs, rhs } => {
                self.emit(node, &self.p("exprKind"), "\"binop\"", ctx_graph);
                self.emit(node, &self.p("op"), &format!("\"{}\"", binop_str(op)), ctx_graph);
                let ln = self.fresh();
                let rn = self.fresh();
                self.emit(node, &self.p("lhs"), &ln, ctx_graph);
                self.emit(node, &self.p("rhs"), &rn, ctx_graph);
                self.encode_expr(&ln, lhs, ctx_graph);
                self.encode_expr(&rn, rhs, ctx_graph);
            }
            Expr::BuiltIn { name, args } => {
                self.emit(node, &self.p("exprKind"), "\"builtin\"", ctx_graph);
                self.emit(node, &self.p("funcName"), &format!("\"{}\"", name), ctx_graph);
                for (i, arg) in args.iter().enumerate() {
                    let an = self.fresh();
                    self.emit(node, &self.p("arg"), &an, ctx_graph);
                    self.emit(&an, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_expr(&an, arg, ctx_graph);
                }
            }
            Expr::Aggregate { name, distinct, args, star, separator } => {
                self.emit(node, &self.p("exprKind"), "\"aggregate\"", ctx_graph);
                self.emit(node, &self.p("funcName"), &format!("\"{}\"", name), ctx_graph);
                self.emit(
                    node,
                    &self.p("distinct"),
                    if *distinct { "\"true\"" } else { "\"false\"" },
                    ctx_graph,
                );
                self.emit(
                    node,
                    &self.p("star"),
                    if *star { "\"true\"" } else { "\"false\"" },
                    ctx_graph,
                );
                if let Some(sep) = separator {
                    self.emit(node, &self.p("separator"), &format!("\"{sep}\""), ctx_graph);
                }
                for (i, arg) in args.iter().enumerate() {
                    let an = self.fresh();
                    self.emit(node, &self.p("arg"), &an, ctx_graph);
                    self.emit(&an, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_expr(&an, arg, ctx_graph);
                }
            }
            Expr::Exists(ggp) => {
                self.emit(node, &self.p("exprKind"), "\"exists\"", ctx_graph);
                let inner = self.fresh();
                self.emit(node, &self.p("pattern"), &inner, ctx_graph);
                self.encode_group_graph_pattern(&inner, ggp, ctx_graph);
            }
            Expr::NotExists(ggp) => {
                self.emit(node, &self.p("exprKind"), "\"notExists\"", ctx_graph);
                let inner = self.fresh();
                self.emit(node, &self.p("pattern"), &inner, ctx_graph);
                self.encode_group_graph_pattern(&inner, ggp, ctx_graph);
            }
            Expr::In { lhs, rhs } => {
                self.emit(node, &self.p("exprKind"), "\"in\"", ctx_graph);
                let ln = self.fresh();
                self.emit(node, &self.p("lhs"), &ln, ctx_graph);
                self.encode_expr(&ln, lhs, ctx_graph);
                for (i, r) in rhs.iter().enumerate() {
                    let rn = self.fresh();
                    self.emit(node, &self.p("inMember"), &rn, ctx_graph);
                    self.emit(&rn, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_expr(&rn, r, ctx_graph);
                }
            }
            Expr::NotIn { lhs, rhs } => {
                self.emit(node, &self.p("exprKind"), "\"notIn\"", ctx_graph);
                let ln = self.fresh();
                self.emit(node, &self.p("lhs"), &ln, ctx_graph);
                self.encode_expr(&ln, lhs, ctx_graph);
                for (i, r) in rhs.iter().enumerate() {
                    let rn = self.fresh();
                    self.emit(node, &self.p("notInMember"), &rn, ctx_graph);
                    self.emit(&rn, &self.p("index"), &format!("{i}"), ctx_graph);
                    self.encode_expr(&rn, r, ctx_graph);
                }
            }
        }
    }

    // ── VALUES ────────────────────────────────────────────────────────────

    fn encode_values_clause(
        &mut self,
        node: &str,
        vals: &ValuesClause,
        ctx_graph: Option<&str>,
    ) {
        self.emit(node, &format!("<{RDF_TYPE}>"), &self.p("ValuesClause"), ctx_graph);
        for (i, v) in vals.vars.iter().enumerate() {
            let vn = self.fresh();
            self.emit(node, &self.p("valuesVar"), &vn, ctx_graph);
            self.emit(&vn, &self.p("index"), &format!("{i}"), ctx_graph);
            self.emit(&vn, &self.p("var"), &format!("?{v}"), ctx_graph);
        }
        for (ri, row) in vals.rows.iter().enumerate() {
            let rn = self.fresh();
            self.emit(node, &self.p("valuesRow"), &rn, ctx_graph);
            self.emit(&rn, &self.p("index"), &format!("{ri}"), ctx_graph);
            for (ci, cell) in row.iter().enumerate() {
                let cn = self.fresh();
                self.emit(&rn, &self.p("valuesCell"), &cn, ctx_graph);
                self.emit(&cn, &self.p("index"), &format!("{ci}"), ctx_graph);
                match cell {
                    None => {
                        self.emit(&cn, &self.p("undef"), "\"true\"", ctx_graph);
                    }
                    Some(term) => {
                        let ts = term_to_string(term);
                        self.emit(&cn, &self.p("value"), &ts, ctx_graph);
                    }
                }
            }
        }
    }

    // ── Solution Modifiers ────────────────────────────────────────────────

    fn encode_solution_modifiers(
        &mut self,
        node: &str,
        mods: &SolutionModifiers,
        ctx_graph: Option<&str>,
    ) {
        if !mods.group_by.is_empty() {
            let gn = self.fresh();
            self.emit(node, &self.p("groupBy"), &gn, ctx_graph);
            for (i, cond) in mods.group_by.iter().enumerate() {
                let cn = self.fresh();
                self.emit(&gn, &self.p("condition"), &cn, ctx_graph);
                self.emit(&cn, &self.p("index"), &format!("{i}"), ctx_graph);
                match cond {
                    GroupCondition::Var(v) => {
                        self.emit(&cn, &self.p("var"), &format!("?{v}"), ctx_graph);
                    }
                    GroupCondition::Expr { expr, alias } => {
                        let en = self.fresh();
                        self.emit(&cn, &self.p("expr"), &en, ctx_graph);
                        self.encode_expr(&en, expr, ctx_graph);
                        if let Some(a) = alias {
                            self.emit(&cn, &self.p("alias"), &format!("?{a}"), ctx_graph);
                        }
                    }
                }
            }
        }

        if !mods.having.is_empty() {
            let hn = self.fresh();
            self.emit(node, &self.p("having"), &hn, ctx_graph);
            for (i, expr) in mods.having.iter().enumerate() {
                let en = self.fresh();
                self.emit(&hn, &self.p("condition"), &en, ctx_graph);
                self.emit(&en, &self.p("index"), &format!("{i}"), ctx_graph);
                self.encode_expr(&en, expr, ctx_graph);
            }
        }

        if !mods.order_by.is_empty() {
            let on = self.fresh();
            self.emit(node, &self.p("orderBy"), &on, ctx_graph);
            for (i, cond) in mods.order_by.iter().enumerate() {
                let cn = self.fresh();
                self.emit(&on, &self.p("condition"), &cn, ctx_graph);
                self.emit(&cn, &self.p("index"), &format!("{i}"), ctx_graph);
                let dir = match cond.direction {
                    OrderDirection::Asc => "\"ASC\"",
                    OrderDirection::Desc => "\"DESC\"",
                };
                self.emit(&cn, &self.p("direction"), dir, ctx_graph);
                let en = self.fresh();
                self.emit(&cn, &self.p("expr"), &en, ctx_graph);
                self.encode_expr(&en, &cond.expr, ctx_graph);
            }
        }

        if let Some(l) = mods.limit {
            self.emit(
                node,
                &self.p("limit"),
                &format!("\"{}\"^^<{XSD_INT}>", l),
                ctx_graph,
            );
        }

        if let Some(o) = mods.offset {
            self.emit(
                node,
                &self.p("offset"),
                &format!("\"{}\"^^<{XSD_INT}>", o),
                ctx_graph,
            );
        }
    }

    // ── Utility ───────────────────────────────────────────────────────────

    fn encode_var_or_iri(&mut self, node: &str, item: &VarOrIri, ctx_graph: Option<&str>) {
        match item {
            VarOrIri::Var(v) => {
                self.emit(node, &self.p("var"), &format!("?{v}"), ctx_graph);
            }
            VarOrIri::Iri(iri) => {
                self.emit(node, &self.p("iri"), &iri_to_string(iri), ctx_graph);
            }
        }
    }

    // ── Update Ops ────────────────────────────────────────────────────────

    fn encode_update_op(&mut self, node: &str, op: &UpdateOp) {
        match op {
            UpdateOp::Load { silent, source, into_graph } => {
                self.emit(node, &self.p("opKind"), "\"Load\"", None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                self.emit(node, &self.p("source"), &format!("<{source}>"), None);
                if let Some(g) = into_graph {
                    self.emit(node, &self.p("into"), &format!("<{g}>"), None);
                }
            }
            UpdateOp::Clear { silent, graph_ref }
            | UpdateOp::Drop { silent, graph_ref: graph_ref @ GraphRef::All }
            | UpdateOp::Drop { silent, graph_ref: graph_ref @ GraphRef::Default }
            | UpdateOp::Drop { silent, graph_ref: graph_ref @ GraphRef::Named2 }
            | UpdateOp::Drop { silent, graph_ref } => {
                let kind = if matches!(op, UpdateOp::Clear { .. }) {
                    "\"Clear\""
                } else {
                    "\"Drop\""
                };
                self.emit(node, &self.p("opKind"), kind, None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                match graph_ref {
                    GraphRef::Named(iri) => {
                        self.emit(node, &self.p("graphRef"), &format!("<{iri}>"), None);
                    }
                    GraphRef::Default => {
                        self.emit(node, &self.p("graphRef"), "\"DEFAULT\"", None);
                    }
                    GraphRef::Named2 => {
                        self.emit(node, &self.p("graphRef"), "\"NAMED\"", None);
                    }
                    GraphRef::All => {
                        self.emit(node, &self.p("graphRef"), "\"ALL\"", None);
                    }
                }
            }
            UpdateOp::Create { silent, iri } => {
                self.emit(node, &self.p("opKind"), "\"Create\"", None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                self.emit(node, &self.p("iri"), &format!("<{iri}>"), None);
            }
            UpdateOp::Add { silent, from, to } => {
                self.emit(node, &self.p("opKind"), "\"Add\"", None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                self.emit(node, &self.p("from"), &graph_or_default_str(from), None);
                self.emit(node, &self.p("to"), &graph_or_default_str(to), None);
            }
            UpdateOp::Move { silent, from, to } => {
                self.emit(node, &self.p("opKind"), "\"Move\"", None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                self.emit(node, &self.p("from"), &graph_or_default_str(from), None);
                self.emit(node, &self.p("to"), &graph_or_default_str(to), None);
            }
            UpdateOp::Copy { silent, from, to } => {
                self.emit(node, &self.p("opKind"), "\"Copy\"", None);
                self.emit(node, &self.p("silent"), bool_str(*silent), None);
                self.emit(node, &self.p("from"), &graph_or_default_str(from), None);
                self.emit(node, &self.p("to"), &graph_or_default_str(to), None);
            }
            UpdateOp::InsertData(triples) => {
                self.emit(node, &self.p("opKind"), "\"InsertData\"", None);
                for (i, tp) in triples.iter().enumerate() {
                    let tn = self.fresh();
                    self.emit(node, &self.p("triple"), &tn, None);
                    self.emit(&tn, &self.p("index"), &format!("{i}"), None);
                    self.encode_triple_pattern(&tn, tp, None);
                }
            }
            UpdateOp::DeleteData(triples) => {
                self.emit(node, &self.p("opKind"), "\"DeleteData\"", None);
                for (i, tp) in triples.iter().enumerate() {
                    let tn = self.fresh();
                    self.emit(node, &self.p("triple"), &tn, None);
                    self.emit(&tn, &self.p("index"), &format!("{i}"), None);
                    self.encode_triple_pattern(&tn, tp, None);
                }
            }
            UpdateOp::DeleteWhere(ggp) => {
                self.emit(node, &self.p("opKind"), "\"DeleteWhere\"", None);
                let inner = self.fresh();
                self.emit(node, &self.p("where"), &inner, None);
                self.encode_group_graph_pattern(&inner, ggp, None);
            }
            UpdateOp::Modify { with, delete, insert, using, where_pattern } => {
                self.emit(node, &self.p("opKind"), "\"Modify\"", None);
                if let Some(w) = with {
                    self.emit(node, &self.p("with"), &format!("<{w}>"), None);
                }
                for (i, tp) in delete.iter().enumerate() {
                    let tn = self.fresh();
                    self.emit(node, &self.p("deleteTriple"), &tn, None);
                    self.emit(&tn, &self.p("index"), &format!("{i}"), None);
                    self.encode_triple_pattern(&tn, tp, None);
                }
                for (i, tp) in insert.iter().enumerate() {
                    let tn = self.fresh();
                    self.emit(node, &self.p("insertTriple"), &tn, None);
                    self.emit(&tn, &self.p("index"), &format!("{i}"), None);
                    self.encode_triple_pattern(&tn, tp, None);
                }
                for (i, ds) in using.iter().enumerate() {
                    let dn = self.fresh();
                    self.emit(node, &self.p("using"), &dn, None);
                    self.emit(&dn, &self.p("index"), &format!("{i}"), None);
                    self.emit(
                        &dn,
                        &self.p("named"),
                        if ds.named { "\"true\"" } else { "\"false\"" },
                        None,
                    );
                    self.emit(&dn, &self.p("graphIri"), &iri_to_string(&ds.iri), None);
                }
                let inner = self.fresh();
                self.emit(node, &self.p("where"), &inner, None);
                self.encode_group_graph_pattern(&inner, where_pattern, None);
            }
        }
    }
}

// ── Free functions ───────────────────────────────────────────────────────────

fn iri_to_string(iri: &Iri) -> String {
    match iri {
        Iri::Absolute(s) => s.clone(), // already has < >
        Iri::Prefixed(s) => s.clone(), // keep as-is
        Iri::A => format!(
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        ),
    }
}

fn term_to_string(term: &Term) -> String {
    match term {
        Term::Var(v) => format!("?{v}"),
        Term::Iri(iri) => iri_to_string(iri),
        Term::BlankNode(label) => label.clone(),
        Term::AnonBlankNode => "_:anon".to_owned(),
        Term::Literal(lit) => literal_to_string(lit),
        Term::BNodePropList(_) => "_:bnode_prop_list".to_owned(),
        Term::Collection(_) => "_:collection".to_owned(),
    }
}

fn literal_to_string(lit: &Literal) -> String {
    match &lit.annotation {
        LiteralAnnotation::Plain => {
            // SPARQL 1.1 §3.1.1: plain literals are xsd:string
            format!("{}^^<{XSD_STRING}>", lit.lexical)
        }
        LiteralAnnotation::Lang(tag) => {
            format!("{}@{}", lit.lexical, tag.to_ascii_lowercase())
        }
        LiteralAnnotation::Datatype(iri) => {
            format!("{}^^{}", lit.lexical, iri_to_string(iri))
        }
    }
}

fn binop_str(op: &BinOp) -> &'static str {
    match op {
        BinOp::Or => "||",
        BinOp::And => "&&",
        BinOp::Eq => "=",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
    }
}

fn bool_str(b: bool) -> &'static str {
    if b { "\"true\"" } else { "\"false\"" }
}

fn graph_or_default_str(g: &GraphOrDefault) -> String {
    match g {
        GraphOrDefault::Default => "\"DEFAULT\"".to_owned(),
        GraphOrDefault::Named(iri) => format!("<{iri}>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::tokenise, parser::parse_document};

    fn encode_str(src: &str) -> Facts {
        let tokens = tokenise(src).expect("lex");
        let (doc, _) = parse_document(&tokens, src).expect("parse");
        encode_document(&doc)
    }

    #[test]
    fn facts_non_empty_for_select() {
        let facts = encode_str("SELECT * WHERE { }");
        assert!(!facts.set.is_empty());
    }

    #[test]
    fn root_node_is_shadow_0() {
        let facts = encode_str("SELECT * WHERE { }");
        assert!(facts.set.keys().any(|f| f.subject == "_shadow:0"));
    }

    #[test]
    fn select_star_encoded() {
        let facts = encode_str("SELECT * WHERE { }");
        let has_star = facts.set.keys().any(|f| {
            f.predicate.contains("projection")
                && f.object.contains("Star")
        });
        assert!(has_star, "expected sparql:projection -> sparql:Star");
    }

    #[test]
    fn insert_data_encoded() {
        let facts = encode_str("INSERT DATA { <http://s> <http://p> <http://o> . }");
        let has_insert = facts
            .set
            .keys()
            .any(|f| f.object.contains("InsertData"));
        assert!(has_insert);
    }

    #[test]
    fn service_endpoint_encoded() {
        let facts = encode_str(
            "SELECT * WHERE { SERVICE <http://ep.example/sparql> { ?s ?p ?o } }",
        );
        let has_endpoint = facts.set.keys().any(|f| {
            f.predicate.contains("endpoint") && f.object.contains("ep.example")
        });
        assert!(has_endpoint, "expected endpoint fact");
    }

    #[test]
    fn property_path_encoded() {
        let facts = encode_str(
            "SELECT ?x WHERE { ?x <http://a>/<http://b> ?y }",
        );
        let has_seq = facts.set.keys().any(|f| {
            f.predicate.contains("pathKind") && f.object.contains("sequence")
        });
        assert!(has_seq, "expected sequence path kind");
    }

    #[test]
    fn literal_lang_encoded() {
        let facts = encode_str(
            "SELECT * WHERE { ?x <http://label> \"hello\"@en }",
        );
        let has_lang = facts.set.keys().any(|f| f.object.contains("@en"));
        assert!(has_lang, "expected lang tag in object");
    }

    #[test]
    fn prefixes_captured() {
        let facts = encode_str(
            "PREFIX ex: <http://example.org/> SELECT * WHERE { }",
        );
        assert!(facts.prefixes.contains_key("ex"));
    }
}
