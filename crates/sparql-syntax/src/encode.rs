//! Encoder: AST → `rdf_diff::Fact`s.
//!
//! The encoding is structural: each salient AST node produces one fact
//! `<request> <predicate> "payload"`. The `request` subject is a fixed
//! sentinel so that two requests can be diffed fact-for-fact. Payloads
//! are printable strings (plain literals) that serialise the
//! sub-structure. The goal is a canonical, comparable **shape**
//! signature — NOT a Turtle / JSON-LD rendering of the tree.
//!
//! Predicates live under `urn:x-sparql-syntax:`, which is independent
//! of the shadow's encoding namespace. The diff harness compares via
//! the canonical `Facts` set, which for this crate is the multiset of
//! `(predicate, payload)` tuples. If the shadow encodes differently,
//! that surfaces as `FactOnlyIn` divergences — acceptable and
//! documented in `divergences.md`.

use rdf_diff::{Fact, FactProvenance};

use crate::ast::{
    CopyMoveKind, DatasetClause, Expr, FuncName, GraphOrDefault, GraphTarget, GroupCondition,
    GroupGraphPattern, GroupPatternElement, InlineData, Literal, LiteralKind, NegatedAtom,
    OrderCondition, Path, PathOrPredicate, PathPrim, Projection, QuadTriple, Query, QueryForm,
    Request, SelectClause, SelectModifier, SolutionModifier, TermOrPath, TriplePattern, UpdateOp,
    UpdateRequest, VarOrIri,
};

/// Stable sentinel subject for all SPARQL-syntax facts.
const REQUEST_SUBJECT: &str = "<urn:x-sparql-syntax:request>";

/// Predicate namespace prefix.
const NS: &str = "urn:x-sparql-syntax:";

/// Encode a parsed [`Request`] into a list of `(fact, provenance)` pairs
/// ready for `Facts::canonicalise`.
pub(crate) fn encode_request(
    request: &Request,
    parser_id: &'static str,
) -> Vec<(Fact, FactProvenance)> {
    let mut out = Vec::new();
    match request {
        Request::Query(q) => encode_query(q, &mut out),
        Request::Update(u) => encode_update(u, &mut out),
    }
    out.into_iter()
        .enumerate()
        .map(|(i, (pred, payload))| {
            (
                Fact {
                    subject: REQUEST_SUBJECT.to_owned(),
                    predicate: format!("<{NS}{pred}>"),
                    object: format!("\"{}\"", escape_literal(&payload)),
                    graph: None,
                },
                FactProvenance {
                    offset: Some(i),
                    parser: parser_id.to_owned(),
                },
            )
        })
        .collect()
}

// ------------- Query ------------------------------------------------

fn encode_query(q: &Query, out: &mut Vec<(&'static str, String)>) {
    out.push(("kind", "query".to_owned()));
    if let Some(base) = &q.base {
        out.push(("base", base.clone()));
    }
    for (p, iri) in &q.prefixes {
        out.push(("prefix", format!("{p} -> {iri}")));
    }
    match &q.form {
        QueryForm::Select(s) => encode_select(s, out),
        QueryForm::Construct(c) => encode_construct(c, out),
        QueryForm::Ask => out.push(("form", "ASK".to_owned())),
        QueryForm::Describe { targets } => {
            out.push(("form", "DESCRIBE".to_owned()));
            for t in targets {
                out.push(("describe-target", fmt_var_or_iri(t)));
            }
        }
    }
    for d in &q.dataset {
        out.push(("dataset", fmt_dataset(d)));
    }
    encode_ggp(&q.where_clause, "where", out);
    encode_modifiers(&q.modifiers, out);
    if let Some(vals) = &q.values_clause {
        out.push(("values", fmt_values(vals)));
    }
}

fn encode_select(s: &SelectClause, out: &mut Vec<(&'static str, String)>) {
    out.push(("form", "SELECT".to_owned()));
    if let Some(m) = s.modifier {
        out.push((
            "select-modifier",
            match m {
                SelectModifier::Distinct => "DISTINCT".to_owned(),
                SelectModifier::Reduced => "REDUCED".to_owned(),
            },
        ));
    }
    match &s.projection {
        None => out.push(("projection", "*".to_owned())),
        Some(list) => {
            for p in list {
                out.push((
                    "projection",
                    match p {
                        Projection::Var(v) => format!("?{v}"),
                        Projection::Expr { expr, var } => {
                            format!("({} AS ?{})", fmt_expr(expr), var)
                        }
                    },
                ));
            }
        }
    }
}

fn encode_construct(c: &crate::ast::ConstructClause, out: &mut Vec<(&'static str, String)>) {
    out.push(("form", "CONSTRUCT".to_owned()));
    match &c.template {
        Some(tpl) => {
            for t in tpl {
                out.push(("construct-template", fmt_triple(t)));
            }
        }
        None => out.push(("construct-template", "<short-form>".to_owned())),
    }
}

fn encode_modifiers(m: &SolutionModifier, out: &mut Vec<(&'static str, String)>) {
    for g in &m.group_by {
        out.push(("group-by", fmt_group(g)));
    }
    for h in &m.having {
        out.push(("having", fmt_expr(h)));
    }
    for o in &m.order_by {
        out.push(("order-by", fmt_order(o)));
    }
    if let Some(l) = m.limit {
        out.push(("limit", l.to_string()));
    }
    if let Some(o) = m.offset {
        out.push(("offset", o.to_string()));
    }
}

fn encode_ggp(g: &GroupGraphPattern, pred: &'static str, out: &mut Vec<(&'static str, String)>) {
    out.push((pred, format!("{{ {} }}", fmt_ggp(g))));
}

fn fmt_ggp(g: &GroupGraphPattern) -> String {
    let mut pieces: Vec<String> = Vec::new();
    for e in &g.elements {
        pieces.push(fmt_ggp_elem(e));
    }
    pieces.join(" . ")
}

fn fmt_ggp_elem(e: &GroupPatternElement) -> String {
    match e {
        GroupPatternElement::Triples(ts) => ts
            .iter()
            .map(fmt_triple)
            .collect::<Vec<_>>()
            .join(" . "),
        GroupPatternElement::Optional(g) => format!("OPTIONAL {{ {} }}", fmt_ggp(g)),
        GroupPatternElement::Minus(g) => format!("MINUS {{ {} }}", fmt_ggp(g)),
        GroupPatternElement::Union(alts) => alts
            .iter()
            .map(|a| format!("{{ {} }}", fmt_ggp(a)))
            .collect::<Vec<_>>()
            .join(" UNION "),
        GroupPatternElement::Group(g) => format!("{{ {} }}", fmt_ggp(g)),
        GroupPatternElement::Filter(e) => format!("FILTER({})", fmt_expr(e)),
        GroupPatternElement::Bind { expr, var } => {
            format!("BIND({} AS ?{})", fmt_expr(expr), var)
        }
        GroupPatternElement::Values(v) => format!("VALUES {}", fmt_values(v)),
        GroupPatternElement::Service {
            silent,
            endpoint,
            pattern,
        } => format!(
            "SERVICE{} {} {{ {} }}",
            if *silent { " SILENT" } else { "" },
            fmt_var_or_iri(endpoint),
            fmt_ggp(pattern)
        ),
        GroupPatternElement::Graph { name, pattern } => {
            format!("GRAPH {} {{ {} }}", fmt_var_or_iri(name), fmt_ggp(pattern))
        }
        GroupPatternElement::SubQuery(q) => format!("SUBQUERY[{}]", fmt_subquery(q)),
    }
}

fn fmt_subquery(q: &Query) -> String {
    let mut pieces = Vec::new();
    match &q.form {
        QueryForm::Select(s) => {
            pieces.push("SELECT".to_owned());
            if let Some(m) = s.modifier {
                pieces.push(format!("{m:?}"));
            }
            match &s.projection {
                None => pieces.push("*".to_owned()),
                Some(list) => {
                    for p in list {
                        pieces.push(match p {
                            Projection::Var(v) => format!("?{v}"),
                            Projection::Expr { expr, var } => {
                                format!("({} AS ?{})", fmt_expr(expr), var)
                            }
                        });
                    }
                }
            }
        }
        _ => pieces.push("<non-select-subquery>".to_owned()),
    }
    pieces.push(format!("WHERE {{ {} }}", fmt_ggp(&q.where_clause)));
    pieces.join(" ")
}

fn fmt_triple(t: &TriplePattern) -> String {
    format!(
        "{} {} {}",
        fmt_term(&t.subject),
        fmt_pred(&t.predicate),
        fmt_term(&t.object)
    )
}

fn fmt_pred(p: &PathOrPredicate) -> String {
    match p {
        PathOrPredicate::A => "a".to_owned(),
        PathOrPredicate::Predicate(t) => fmt_term(t),
        PathOrPredicate::Path(path) => fmt_path(path),
    }
}

fn fmt_path(p: &Path) -> String {
    match p {
        Path::Prim(pp) => fmt_path_prim(pp),
        Path::Opt(inner) => format!("({}?)", fmt_path(inner)),
        Path::ZeroOrMore(inner) => format!("({}*)", fmt_path(inner)),
        Path::OneOrMore(inner) => format!("({}+)", fmt_path(inner)),
        Path::Inverse(inner) => format!("^({})", fmt_path(inner)),
        Path::Alt(alts) => {
            let parts: Vec<String> = alts.iter().map(fmt_path).collect();
            format!("({})", parts.join(" | "))
        }
        Path::Seq(seq) => {
            let parts: Vec<String> = seq.iter().map(fmt_path).collect();
            format!("({})", parts.join(" / "))
        }
        Path::Negated(atoms) => {
            let parts: Vec<String> = atoms
                .iter()
                .map(|a| match a {
                    NegatedAtom::Fwd(p) => fmt_path_prim(p),
                    NegatedAtom::Inv(p) => format!("^{}", fmt_path_prim(p)),
                })
                .collect();
            format!("!({})", parts.join(" | "))
        }
    }
}

fn fmt_path_prim(p: &PathPrim) -> String {
    match p {
        PathPrim::A => "a".to_owned(),
        PathPrim::Iri(i) => format!("<{i}>"),
        PathPrim::Prefixed { prefix, local } => format!("{prefix}:{local}"),
    }
}

fn fmt_term(t: &TermOrPath) -> String {
    match t {
        TermOrPath::Iri(s) => format!("<{s}>"),
        TermOrPath::PrefixedName { prefix, local } => format!("{prefix}:{local}"),
        TermOrPath::Var(v) => format!("?{v}"),
        TermOrPath::BNodeLabel(l) => format!("_:{l}"),
        TermOrPath::BNodeAnon => "[]".to_owned(),
        TermOrPath::Literal(l) => fmt_literal(l),
        TermOrPath::NumericLit(s) => s.clone(),
        TermOrPath::BoolLit(b) => {
            if *b {
                "true".to_owned()
            } else {
                "false".to_owned()
            }
        }
        TermOrPath::Nil => "()".to_owned(),
        TermOrPath::Collection(items) => {
            let parts: Vec<String> = items.iter().map(fmt_term).collect();
            format!("({})", parts.join(" "))
        }
        TermOrPath::BNodePropertyList(props) => {
            let parts: Vec<String> = props
                .iter()
                .map(|(p, objs)| {
                    let os: Vec<String> = objs.iter().map(fmt_term).collect();
                    format!("{} {}", fmt_pred(p), os.join(" , "))
                })
                .collect();
            format!("[ {} ]", parts.join(" ; "))
        }
    }
}

fn fmt_literal(l: &Literal) -> String {
    let lex = escape_inner_literal(&l.lexical);
    match &l.kind {
        LiteralKind::Simple => format!("\\\"{lex}\\\""),
        LiteralKind::Lang(t) => format!("\\\"{lex}\\\"@{t}"),
        LiteralKind::Typed(iri) => format!("\\\"{lex}\\\"^^<{iri}>"),
        LiteralKind::TypedPrefixed { prefix, local } => {
            format!("\\\"{lex}\\\"^^{prefix}:{local}")
        }
    }
}

fn fmt_var_or_iri(v: &VarOrIri) -> String {
    match v {
        VarOrIri::Var(s) => format!("?{s}"),
        VarOrIri::Iri(s) => format!("<{s}>"),
        VarOrIri::Prefixed { prefix, local } => format!("{prefix}:{local}"),
    }
}

fn fmt_expr(e: &Expr) -> String {
    match e {
        Expr::Or(a, b) => format!("({} || {})", fmt_expr(a), fmt_expr(b)),
        Expr::And(a, b) => format!("({} && {})", fmt_expr(a), fmt_expr(b)),
        Expr::Eq(a, b) => format!("({} = {})", fmt_expr(a), fmt_expr(b)),
        Expr::NotEq(a, b) => format!("({} != {})", fmt_expr(a), fmt_expr(b)),
        Expr::Lt(a, b) => format!("({} < {})", fmt_expr(a), fmt_expr(b)),
        Expr::LtEq(a, b) => format!("({} <= {})", fmt_expr(a), fmt_expr(b)),
        Expr::Gt(a, b) => format!("({} > {})", fmt_expr(a), fmt_expr(b)),
        Expr::GtEq(a, b) => format!("({} >= {})", fmt_expr(a), fmt_expr(b)),
        Expr::In(a, l) => format!(
            "({} IN ({}))",
            fmt_expr(a),
            l.iter().map(fmt_expr).collect::<Vec<_>>().join(", ")
        ),
        Expr::NotIn(a, l) => format!(
            "({} NOT IN ({}))",
            fmt_expr(a),
            l.iter().map(fmt_expr).collect::<Vec<_>>().join(", ")
        ),
        Expr::Add(a, b) => format!("({} + {})", fmt_expr(a), fmt_expr(b)),
        Expr::Sub(a, b) => format!("({} - {})", fmt_expr(a), fmt_expr(b)),
        Expr::Mul(a, b) => format!("({} * {})", fmt_expr(a), fmt_expr(b)),
        Expr::Div(a, b) => format!("({} / {})", fmt_expr(a), fmt_expr(b)),
        Expr::UnaryNot(a) => format!("!({})", fmt_expr(a)),
        Expr::UnaryPos(a) => format!("(+{})", fmt_expr(a)),
        Expr::UnaryNeg(a) => format!("(-{})", fmt_expr(a)),
        Expr::Var(v) => format!("?{v}"),
        Expr::Iri(i) => format!("<{i}>"),
        Expr::Prefixed { prefix, local } => format!("{prefix}:{local}"),
        Expr::Literal(l) => fmt_literal(l),
        Expr::NumericLit(s) => s.clone(),
        Expr::BoolLit(b) => b.to_string(),
        Expr::Func {
            name,
            args,
            distinct,
        } => {
            let n = match name {
                FuncName::Builtin(s) => s.clone(),
                FuncName::Iri(i) => format!("<{i}>"),
                FuncName::Prefixed { prefix, local } => format!("{prefix}:{local}"),
            };
            let ds = if *distinct { "DISTINCT " } else { "" };
            let a = args.iter().map(fmt_expr).collect::<Vec<_>>().join(", ");
            format!("{n}({ds}{a})")
        }
        Expr::Exists(g) => format!("EXISTS {{ {} }}", fmt_ggp(g)),
        Expr::NotExists(g) => format!("NOT EXISTS {{ {} }}", fmt_ggp(g)),
        Expr::CountStar { distinct } => {
            if *distinct {
                "COUNT(DISTINCT *)".to_owned()
            } else {
                "COUNT(*)".to_owned()
            }
        }
    }
}

fn fmt_dataset(d: &DatasetClause) -> String {
    match d {
        DatasetClause::Default(s) => format!("FROM <{s}>"),
        DatasetClause::Named(s) => format!("FROM NAMED <{s}>"),
    }
}

fn fmt_group(g: &GroupCondition) -> String {
    match g {
        GroupCondition::Var(v) => format!("?{v}"),
        GroupCondition::Expr(e) => fmt_expr(e),
        GroupCondition::ExprAs { expr, var } => format!("({} AS ?{})", fmt_expr(expr), var),
    }
}

fn fmt_order(o: &OrderCondition) -> String {
    let d = if o.descending { "DESC" } else { "ASC" };
    format!("{d} {}", fmt_expr(&o.expr))
}

fn fmt_values(v: &InlineData) -> String {
    let vars = v
        .vars
        .iter()
        .map(|s| format!("?{s}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut parts = vec![format!("({vars})")];
    for row in &v.rows {
        let cells: Vec<String> = row
            .iter()
            .map(|c| match c {
                None => "UNDEF".to_owned(),
                Some(t) => fmt_term(t),
            })
            .collect();
        parts.push(format!("({})", cells.join(" ")));
    }
    parts.join(" ")
}

// ------------- Update -----------------------------------------------

fn encode_update(u: &UpdateRequest, out: &mut Vec<(&'static str, String)>) {
    out.push(("kind", "update".to_owned()));
    if let Some(base) = &u.base {
        out.push(("base", base.clone()));
    }
    for (p, iri) in &u.prefixes {
        out.push(("prefix", format!("{p} -> {iri}")));
    }
    for op in &u.operations {
        encode_update_op(op, out);
    }
}

fn encode_update_op(op: &UpdateOp, out: &mut Vec<(&'static str, String)>) {
    match op {
        UpdateOp::Load {
            silent,
            source,
            dest,
        } => {
            let s = if *silent { " SILENT" } else { "" };
            let d = dest
                .as_deref()
                .map(|d| format!(" INTO GRAPH <{d}>"))
                .unwrap_or_default();
            out.push(("op", format!("LOAD{s} <{source}>{d}")));
        }
        UpdateOp::Clear { silent, target } => {
            let s = if *silent { " SILENT" } else { "" };
            out.push(("op", format!("CLEAR{s} {}", fmt_target(target))));
        }
        UpdateOp::Create { silent, graph } => {
            let s = if *silent { " SILENT" } else { "" };
            out.push(("op", format!("CREATE{s} GRAPH <{graph}>")));
        }
        UpdateOp::Drop { silent, target } => {
            let s = if *silent { " SILENT" } else { "" };
            out.push(("op", format!("DROP{s} {}", fmt_target(target))));
        }
        UpdateOp::CopyMoveAdd {
            op,
            silent,
            source,
            target,
        } => {
            let kw = match op {
                CopyMoveKind::Copy => "COPY",
                CopyMoveKind::Move => "MOVE",
                CopyMoveKind::Add => "ADD",
            };
            let s = if *silent { " SILENT" } else { "" };
            out.push((
                "op",
                format!(
                    "{kw}{s} {} TO {}",
                    fmt_graph_or_default(source),
                    fmt_graph_or_default(target)
                ),
            ));
        }
        UpdateOp::InsertData(qs) => {
            out.push(("op", "INSERT_DATA".to_owned()));
            for q in qs {
                out.push(("insert-data", fmt_quad(q)));
            }
        }
        UpdateOp::DeleteData(qs) => {
            out.push(("op", "DELETE_DATA".to_owned()));
            for q in qs {
                out.push(("delete-data", fmt_quad(q)));
            }
        }
        UpdateOp::DeleteWhere(qs) => {
            out.push(("op", "DELETE_WHERE".to_owned()));
            for q in qs {
                out.push(("delete-where", fmt_quad(q)));
            }
        }
        UpdateOp::Modify {
            with,
            delete,
            insert,
            using,
            where_clause,
        } => {
            out.push(("op", "MODIFY".to_owned()));
            if let Some(w) = with {
                out.push(("modify-with", w.clone()));
            }
            if let Some(d) = delete {
                for q in d {
                    out.push(("modify-delete", fmt_quad(q)));
                }
            }
            if let Some(i) = insert {
                for q in i {
                    out.push(("modify-insert", fmt_quad(q)));
                }
            }
            for u in using {
                out.push(("modify-using", fmt_dataset(u)));
            }
            encode_ggp(where_clause, "modify-where", out);
        }
    }
}

fn fmt_target(t: &GraphTarget) -> String {
    match t {
        GraphTarget::Graph(s) => format!("GRAPH <{s}>"),
        GraphTarget::Default => "DEFAULT".to_owned(),
        GraphTarget::Named => "NAMED".to_owned(),
        GraphTarget::All => "ALL".to_owned(),
    }
}

fn fmt_graph_or_default(g: &GraphOrDefault) -> String {
    match g {
        GraphOrDefault::Default => "DEFAULT".to_owned(),
        GraphOrDefault::Graph(s) => format!("GRAPH <{s}>"),
    }
}

fn fmt_quad(q: &QuadTriple) -> String {
    let g = q
        .graph
        .as_ref()
        .map(|n| format!("GRAPH {} ", fmt_var_or_iri(n)))
        .unwrap_or_default();
    let t = q
        .triples
        .iter()
        .map(fmt_triple)
        .collect::<Vec<_>>()
        .join(" . ");
    format!("{g}{{ {t} }}")
}

// --- Literal escaping ------------------------------------------------

fn escape_literal(s: &str) -> String {
    // Escape for embedding into the outer `"..."` literal in a Fact.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn escape_inner_literal(s: &str) -> String {
    // Escape for appearing inside a nested "..." rendered to a payload
    // that itself is escaped again by `escape_literal`.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}
