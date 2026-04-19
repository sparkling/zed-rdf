//! Abstract syntax tree for SPARQL 1.1 Query and Update.
//!
//! The AST is intentionally shallow: enough structure to encode the parse
//! into stable facts and to surface the nine adversary failure modes at
//! grammar level. It is NOT a full algebra tree and does NOT encode any
//! evaluation semantics.
//!
//! Encoding into `rdf_diff::Fact`s is handled by `encode.rs`.

/// A fully parsed SPARQL request — either a Query or an Update (§2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    Query(Query),
    Update(UpdateRequest),
}

/// A SPARQL query (§4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Query {
    /// Base IRI in the Prologue (§4.1), if declared.
    pub base: Option<String>,
    /// Prefix mappings declared in the Prologue.
    pub prefixes: Vec<(String, String)>,
    /// Query form.
    pub form: QueryForm,
    /// `FROM` clauses — dataset declaration (§13.2).
    pub dataset: Vec<DatasetClause>,
    /// WHERE pattern (§17).
    pub where_clause: GroupGraphPattern,
    /// Solution modifiers (§15).
    pub modifiers: SolutionModifier,
    /// VALUES clause at the query level, if any (§10.2.1).
    pub values_clause: Option<InlineData>,
}

/// The outer form of a SPARQL query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QueryForm {
    /// `SELECT ... WHERE ...`
    Select(SelectClause),
    /// `CONSTRUCT { template } WHERE { ... }` or short `CONSTRUCT WHERE { ... }`.
    Construct(ConstructClause),
    /// `ASK { ... }` — no projection.
    Ask,
    /// `DESCRIBE <iri|var> ... { ... }`.
    Describe {
        /// Targets; may be `*` (empty vec).
        targets: Vec<VarOrIri>,
    },
}

/// SELECT projection (§10.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectClause {
    /// Modifier: `DISTINCT` / `REDUCED` or none.
    pub modifier: Option<SelectModifier>,
    /// Projection list; `None` = `SELECT *`.
    pub projection: Option<Vec<Projection>>,
}

/// Projection item: either a plain variable or `(expr AS ?var)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Projection {
    Var(String),
    Expr { expr: Expr, var: String },
}

/// DISTINCT/REDUCED modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectModifier {
    Distinct,
    Reduced,
}

/// CONSTRUCT template or short form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConstructClause {
    /// Template; `None` for short CONSTRUCT which reuses WHERE as template.
    pub template: Option<Vec<TriplePattern>>,
}

/// Dataset `FROM` / `FROM NAMED` clause (§13.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DatasetClause {
    Default(String),
    Named(String),
}

/// Solution modifiers — GROUP BY / HAVING / ORDER BY / LIMIT / OFFSET.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SolutionModifier {
    pub group_by: Vec<GroupCondition>,
    pub having: Vec<Expr>,
    pub order_by: Vec<OrderCondition>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// GROUP BY condition (§11.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupCondition {
    Var(String),
    Expr(Expr),
    ExprAs { expr: Expr, var: String },
}

/// ORDER BY condition (§15.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrderCondition {
    pub expr: Expr,
    pub descending: bool,
}

// ---------------- Group graph pattern --------------------------------

/// A group graph pattern (§17.1) — `{ ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GroupGraphPattern {
    pub elements: Vec<GroupPatternElement>,
}

/// One element of a group graph pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupPatternElement {
    /// Triples block.
    Triples(Vec<TriplePattern>),
    /// `OPTIONAL { ... }`.
    Optional(GroupGraphPattern),
    /// `MINUS { ... }`.
    Minus(GroupGraphPattern),
    /// `UNION` of alternatives.
    Union(Vec<GroupGraphPattern>),
    /// Nested `{ ... }`.
    Group(GroupGraphPattern),
    /// `FILTER(expr)`.
    Filter(Expr),
    /// `BIND(expr AS ?var)`.
    Bind { expr: Expr, var: String },
    /// `VALUES` clause — inline data.
    Values(InlineData),
    /// `SERVICE [SILENT] iri { ... }`.
    Service {
        silent: bool,
        endpoint: VarOrIri,
        pattern: GroupGraphPattern,
    },
    /// `GRAPH (iri|?var) { ... }`.
    Graph {
        name: VarOrIri,
        pattern: GroupGraphPattern,
    },
    /// Nested SELECT subquery.
    SubQuery(Box<Query>),
}

/// A triple pattern — s, p, o with possibly variable or path predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriplePattern {
    pub subject: TermOrPath,
    pub predicate: PathOrPredicate,
    pub object: TermOrPath,
}

/// A term or a collection/bnode term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TermOrPath {
    Iri(String),
    PrefixedName { prefix: String, local: String },
    Var(String),
    BNodeLabel(String),
    BNodeAnon,
    Literal(Literal),
    NumericLit(String),
    BoolLit(bool),
    /// `rdf:nil` collection.
    Nil,
    /// A collection `( a b c )` flattened to a placeholder; used only in
    /// syntactic sugar.
    Collection(Vec<TermOrPath>),
    /// A blank-node property list `[ p o ... ]`.
    BNodePropertyList(Vec<(PathOrPredicate, Vec<TermOrPath>)>),
}

/// Predicate position: either `a`, an IRI/pname, a variable, or a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathOrPredicate {
    A,
    Predicate(TermOrPath),
    Path(Path),
}

/// A property path expression (§9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Path {
    /// Simple predicate (IRI / `a`).
    Prim(Box<PathPrim>),
    /// `P?` zero-or-one.
    Opt(Box<Path>),
    /// `P*` zero-or-more.
    ZeroOrMore(Box<Path>),
    /// `P+` one-or-more.
    OneOrMore(Box<Path>),
    /// `^P` inverse.
    Inverse(Box<Path>),
    /// `P | Q` alternative.
    Alt(Vec<Path>),
    /// `P / Q` sequence.
    Seq(Vec<Path>),
    /// `!(p1 | p2 | ...)` negated property set.
    Negated(Vec<NegatedAtom>),
}

/// A leaf path element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathPrim {
    A,
    Iri(String),
    Prefixed { prefix: String, local: String },
}

/// An atom inside a negated property set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NegatedAtom {
    /// Forward atom.
    Fwd(PathPrim),
    /// Inverse atom: `^p`.
    Inv(PathPrim),
}

/// A literal with optional language tag or datatype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Literal {
    pub lexical: String,
    pub kind: LiteralKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LiteralKind {
    Simple,
    Lang(String),
    Typed(String),
    TypedPrefixed { prefix: String, local: String },
}

/// A variable or IRI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VarOrIri {
    Var(String),
    Iri(String),
    Prefixed { prefix: String, local: String },
}

/// `VALUES` inline data (§10.2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineData {
    pub vars: Vec<String>,
    pub rows: Vec<Vec<Option<TermOrPath>>>,
}

// ---------------- Expressions ----------------------------------------

/// A SPARQL expression (§17.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr {
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    NotEq(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    LtEq(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    GtEq(Box<Expr>, Box<Expr>),
    In(Box<Expr>, Vec<Expr>),
    NotIn(Box<Expr>, Vec<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    UnaryNot(Box<Expr>),
    UnaryPos(Box<Expr>),
    UnaryNeg(Box<Expr>),
    Var(String),
    Iri(String),
    Prefixed { prefix: String, local: String },
    Literal(Literal),
    NumericLit(String),
    BoolLit(bool),
    /// Function call — either built-in or IRI.
    Func {
        name: FuncName,
        args: Vec<Expr>,
        distinct: bool,
    },
    /// `EXISTS { ... }`.
    Exists(GroupGraphPattern),
    /// `NOT EXISTS { ... }`.
    NotExists(GroupGraphPattern),
    /// `COUNT(*)` sentinel.
    CountStar { distinct: bool },
}

/// Function name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FuncName {
    Builtin(String),
    Iri(String),
    Prefixed { prefix: String, local: String },
}

// ---------------- Update ---------------------------------------------

/// A SPARQL Update request (§3) — a sequence of operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateRequest {
    pub base: Option<String>,
    pub prefixes: Vec<(String, String)>,
    pub operations: Vec<UpdateOp>,
}

/// One update operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UpdateOp {
    /// `LOAD [SILENT] iri [INTO GRAPH iri]`.
    Load {
        silent: bool,
        source: String,
        dest: Option<String>,
    },
    /// `CLEAR [SILENT] (GRAPH iri | DEFAULT | NAMED | ALL)`.
    Clear {
        silent: bool,
        target: GraphTarget,
    },
    /// `CREATE [SILENT] GRAPH iri`.
    Create { silent: bool, graph: String },
    /// `DROP [SILENT] (GRAPH iri | DEFAULT | NAMED | ALL)`.
    Drop {
        silent: bool,
        target: GraphTarget,
    },
    /// `COPY`/`MOVE`/`ADD` [SILENT] src TO dest.
    CopyMoveAdd {
        op: CopyMoveKind,
        silent: bool,
        source: GraphOrDefault,
        target: GraphOrDefault,
    },
    /// `INSERT DATA { ... }` (§3.1.1).
    InsertData(Vec<QuadTriple>),
    /// `DELETE DATA { ... }` (§3.1.2).
    DeleteData(Vec<QuadTriple>),
    /// `DELETE WHERE { ... }` (§3.1.3).
    DeleteWhere(Vec<QuadTriple>),
    /// `[WITH iri] (DELETE { ... }|INSERT { ... })+ [USING iri|USING NAMED iri]* WHERE { ... }` (§3.1.4).
    Modify {
        with: Option<String>,
        delete: Option<Vec<QuadTriple>>,
        insert: Option<Vec<QuadTriple>>,
        using: Vec<DatasetClause>,
        where_clause: GroupGraphPattern,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyMoveKind {
    Copy,
    Move,
    Add,
}

/// A concrete graph target for CLEAR / DROP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GraphTarget {
    Graph(String),
    Default,
    Named,
    All,
}

/// Source/target for COPY/MOVE/ADD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GraphOrDefault {
    Default,
    Graph(String),
}

/// A triple or quad in an update data block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuadTriple {
    pub graph: Option<VarOrIri>,
    pub triples: Vec<TriplePattern>,
}
