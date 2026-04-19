//! Abstract syntax tree for SPARQL 1.1.
//!
//! Closely follows the SPARQL 1.1 Query Language grammar productions
//! (W3C Rec §19). Each variant names the grammar production it maps to.
//! The AST is intentionally minimal: it captures structure that affects
//! the AST-as-Facts encoding but discards syntactic sugar (e.g. `a` is
//! expanded to `rdf:type` during fact encoding, not here).

/// Top-level document: either a query or an update sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Prologue declarations (BASE + PREFIX).
    pub prologue: Prologue,
    /// The body — a query or update.
    pub body: DocumentBody,
}

/// Prologue: BASE and PREFIX declarations in document order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prologue {
    /// Optional BASE IRI (last BASE wins per SPARQL grammar).
    pub base: Option<String>,
    /// PREFIX declarations in order. Overrides accumulate left-to-right.
    pub prefixes: Vec<PrefixDecl>,
}

/// A single PREFIX declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixDecl {
    /// Prefix label without trailing colon, e.g. `"ex"` or `""` (default).
    pub prefix: String,
    /// The IRI to which the prefix expands (angle brackets stripped).
    pub iri: String,
}

/// Document body: mutually exclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentBody {
    /// A SPARQL 1.1 query.
    Query(Box<Query>),
    /// A SPARQL 1.1 Update request (sequence of update operations).
    Update(Vec<UpdateOp>),
}

/// A SPARQL query (SELECT, CONSTRUCT, ASK, DESCRIBE).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Dataset declaration (FROM / FROM NAMED clauses).
    pub dataset: Vec<DatasetClause>,
    /// The query form.
    pub form: QueryForm,
    /// WHERE clause group graph pattern.
    pub where_clause: GroupGraphPattern,
    /// Solution modifiers.
    pub modifiers: SolutionModifiers,
    /// Inline VALUES block at the end of a query (SPARQL 1.1).
    pub values: Option<ValuesClause>,
}

/// Query form variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryForm {
    /// SELECT projection.
    Select(SelectClause),
    /// CONSTRUCT template.
    Construct(ConstructTemplate),
    /// ASK (no projection).
    Ask,
    /// DESCRIBE resources.
    Describe(Vec<VarOrIri>),
}

/// SELECT clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectClause {
    /// `DISTINCT` or `REDUCED` modifier.
    pub modifier: Option<SelectModifier>,
    /// `*` or explicit projection.
    pub projection: SelectProjection,
}

/// DISTINCT / REDUCED.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectModifier {
    /// `DISTINCT`
    Distinct,
    /// `REDUCED`
    Reduced,
}

/// Projection list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectProjection {
    /// `SELECT *`
    Star,
    /// `SELECT ?x (?expr AS ?v) …`
    Vars(Vec<SelectVar>),
}

/// One projected item in a SELECT clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectVar {
    /// Plain `?x` variable.
    Var(String),
    /// `(?expr AS ?alias)` expression projection.
    Alias {
        /// The expression to evaluate.
        expr: Expr,
        /// The bound alias variable name.
        alias: String,
    },
}

/// CONSTRUCT template: either a `{ ... }` triple template or `WHERE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructTemplate {
    /// `CONSTRUCT { template }`.
    Template(Vec<TriplePattern>),
    /// `CONSTRUCT WHERE { ... }` shorthand.
    Where,
}

/// A resource in DESCRIBE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarOrIri {
    /// A variable.
    Var(String),
    /// An IRI (absolute or prefixed).
    Iri(Iri),
}

/// Dataset clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetClause {
    /// The graph IRI.
    pub iri: Iri,
    /// `true` → FROM NAMED, `false` → FROM.
    pub named: bool,
}

/// Solution modifiers (GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SolutionModifiers {
    /// GROUP BY conditions.
    pub group_by: Vec<GroupCondition>,
    /// HAVING conditions.
    pub having: Vec<Expr>,
    /// ORDER BY conditions.
    pub order_by: Vec<OrderCondition>,
    /// LIMIT value.
    pub limit: Option<u64>,
    /// OFFSET value.
    pub offset: Option<u64>,
}

/// A GROUP BY condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupCondition {
    /// A built-in or expression (possibly with alias).
    Expr {
        /// The expression.
        expr: Expr,
        /// Optional alias: `(?expr AS ?v)`.
        alias: Option<String>,
    },
    /// A plain variable.
    Var(String),
}

/// An ORDER BY condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderCondition {
    /// Direction.
    pub direction: OrderDirection,
    /// The expression to order by.
    pub expr: Expr,
}

/// ASC or DESC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderDirection {
    /// `ASC`
    Asc,
    /// `DESC`
    Desc,
}

/// VALUES clause (inline data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesClause {
    /// Variables.
    pub vars: Vec<String>,
    /// Rows of bound values (`None` = UNDEF).
    pub rows: Vec<Vec<Option<RdfTerm>>>,
}

/// A group graph pattern (the core recursive structure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupGraphPattern {
    /// `{ patterns ... }`
    Group(Vec<GraphPatternElement>),
    /// `{ SELECT ... }` subquery.
    SubQuery(Box<Query>),
}

/// One element within a group graph pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphPatternElement {
    /// A triple pattern or property path triple.
    Triple(TriplePattern),
    /// A `FILTER` expression.
    Filter(Expr),
    /// An `OPTIONAL { ... }` block.
    Optional(GroupGraphPattern),
    /// A `MINUS { ... }` block.
    Minus(GroupGraphPattern),
    /// A `UNION` of two patterns.
    Union(GroupGraphPattern, GroupGraphPattern),
    /// A `GRAPH iri { ... }` pattern.
    Graph {
        /// Graph name: IRI or variable.
        name: VarOrIri,
        /// Inner pattern.
        pattern: GroupGraphPattern,
    },
    /// A `SERVICE [SILENT] iri { ... }` pattern.
    Service {
        /// Service endpoint.
        endpoint: VarOrIri,
        /// Whether `SILENT` was specified.
        silent: bool,
        /// Inner pattern.
        pattern: GroupGraphPattern,
    },
    /// A `BIND(expr AS ?var)` clause.
    Bind {
        /// The expression being bound.
        expr: Expr,
        /// The variable name.
        var: String,
    },
    /// An inline `VALUES` block.
    InlineData(ValuesClause),
}

/// A triple pattern: (subject, predicate, object).
///
/// Property paths appear in the predicate position as [`PathExpr`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriplePattern {
    /// Subject.
    pub subject: Term,
    /// Predicate (IRI, variable, or property path).
    pub predicate: Predicate,
    /// Object.
    pub object: Term,
}

/// Predicate of a triple pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// Simple IRI or variable.
    Term(Term),
    /// A property path expression.
    Path(PathExpr),
}

/// A property path expression (SPARQL 1.1 §18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathExpr {
    /// `iri` — a single IRI step.
    Iri(Iri),
    /// `^path` — inverse path.
    Inverse(Box<Self>),
    /// `!path` — negated property set.
    Negated(Box<Self>),
    /// `path / path` — sequence.
    Sequence(Box<Self>, Box<Self>),
    /// `path | path` — alternative.
    Alternative(Box<Self>, Box<Self>),
    /// `path?` — zero or one.
    ZeroOrOne(Box<Self>),
    /// `path*` — zero or more.
    ZeroOrMore(Box<Self>),
    /// `path+` — one or more.
    OneOrMore(Box<Self>),
    /// `(path)` — parenthesised path.
    Group(Box<Self>),
}

/// A term that may appear in subject or object position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// A variable `?x`.
    Var(String),
    /// An IRI.
    Iri(Iri),
    /// A blank-node label `_:b0`.
    BlankNode(String),
    /// An anonymous blank node `[]`.
    AnonBlankNode,
    /// An RDF literal.
    Literal(Literal),
    /// A nested blank-node property list `[ pred obj ; ... ]`.
    BNodePropList(Vec<(Predicate, Term)>),
    /// A collection `( t1 t2 ... )`.
    Collection(Vec<Term>),
}

/// An IRI — either absolute (angle-bracket form) or prefixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Iri {
    /// `<https://...>` — the stored string includes `<` and `>`.
    Absolute(String),
    /// `ex:Foo` — as lexed; requires prefix resolution for full IRI.
    Prefixed(String),
    /// The `a` shorthand for `rdf:type`.
    A,
}

/// An RDF literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    /// The lexical form including surrounding quotes and escape sequences
    /// (not yet decoded — decoding happens during encoding if needed).
    pub lexical: String,
    /// Literal annotation.
    pub annotation: LiteralAnnotation,
}

/// Literal type annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralAnnotation {
    /// No annotation — implicitly `xsd:string` per RDF 1.1.
    Plain,
    /// `@lang-tag`.
    Lang(String),
    /// `^^IRI`.
    Datatype(Iri),
}

/// An RDF term (concrete — no property paths).
pub type RdfTerm = Term;

// ── Update AST ─────────────────────────────────────────────────────────────

/// A SPARQL 1.1 Update operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOp {
    /// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`
    Load {
        /// Whether SILENT was specified.
        silent: bool,
        /// Source IRI.
        source: String,
        /// Optional destination graph.
        into_graph: Option<String>,
    },
    /// `CLEAR [SILENT] GraphRef`
    Clear {
        /// Whether SILENT was specified.
        silent: bool,
        /// Which graph(s) to clear.
        graph_ref: GraphRef,
    },
    /// `DROP [SILENT] GraphRef`
    Drop {
        /// Whether SILENT was specified.
        silent: bool,
        /// Which graph(s) to drop.
        graph_ref: GraphRef,
    },
    /// `CREATE [SILENT] GRAPH <iri>`
    Create {
        /// Whether SILENT was specified.
        silent: bool,
        /// Graph IRI.
        iri: String,
    },
    /// `ADD [SILENT] GraphOrDefault TO GraphOrDefault`
    Add {
        /// Whether SILENT was specified.
        silent: bool,
        /// Source.
        from: GraphOrDefault,
        /// Destination.
        to: GraphOrDefault,
    },
    /// `MOVE [SILENT] GraphOrDefault TO GraphOrDefault`
    Move {
        /// Whether SILENT was specified.
        silent: bool,
        /// Source.
        from: GraphOrDefault,
        /// Destination.
        to: GraphOrDefault,
    },
    /// `COPY [SILENT] GraphOrDefault TO GraphOrDefault`
    Copy {
        /// Whether SILENT was specified.
        silent: bool,
        /// Source.
        from: GraphOrDefault,
        /// Destination.
        to: GraphOrDefault,
    },
    /// `INSERT DATA { triples }`
    InsertData(Vec<TriplePattern>),
    /// `DELETE DATA { triples }`
    DeleteData(Vec<TriplePattern>),
    /// `DELETE WHERE { pattern }`
    DeleteWhere(GroupGraphPattern),
    /// `[WITH iri] DELETE { template } [INSERT { template }] [USING ...] WHERE { pattern }`
    Modify {
        /// Optional WITH graph IRI.
        with: Option<String>,
        /// DELETE template (may be empty).
        delete: Vec<TriplePattern>,
        /// INSERT template (may be empty).
        insert: Vec<TriplePattern>,
        /// USING / USING NAMED graphs.
        using: Vec<DatasetClause>,
        /// WHERE clause.
        where_pattern: GroupGraphPattern,
    },
}

/// A graph reference in CLEAR / DROP operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphRef {
    /// `GRAPH <iri>`
    Named(String),
    /// `DEFAULT`
    Default,
    /// `NAMED`
    Named2,
    /// `ALL`
    All,
}

/// DEFAULT or a named graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphOrDefault {
    /// `DEFAULT`
    Default,
    /// `GRAPH <iri>` or bare `<iri>`
    Named(String),
}

// ── Expressions ─────────────────────────────────────────────────────────────

/// A SPARQL expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A variable reference.
    Var(String),
    /// An IRI constant.
    Iri(Iri),
    /// A literal constant.
    Literal(Literal),
    /// Boolean `true` / `false`.
    Bool(bool),
    /// Integer literal.
    Integer(i64),
    /// Decimal literal (stored as string to avoid precision loss).
    Decimal(String),
    /// Double literal.
    Double(String),
    /// Unary `!expr`
    Not(Box<Self>),
    /// Unary `-expr`
    Neg(Box<Self>),
    /// Binary operator.
    BinOp {
        /// Operator.
        op: BinOp,
        /// Left operand.
        lhs: Box<Self>,
        /// Right operand.
        rhs: Box<Self>,
    },
    /// A built-in function call.
    BuiltIn {
        /// Function name (upper-case canonical).
        name: String,
        /// Arguments.
        args: Vec<Self>,
    },
    /// An aggregate.
    Aggregate {
        /// Aggregate function name.
        name: String,
        /// `DISTINCT` flag.
        distinct: bool,
        /// Arguments (`*` represented as empty vec + `star: true`).
        args: Vec<Self>,
        /// `*` was specified.
        star: bool,
        /// SEPARATOR for `GROUP_CONCAT`.
        separator: Option<String>,
    },
    /// `EXISTS { pattern }`
    Exists(Box<GroupGraphPattern>),
    /// `NOT EXISTS { pattern }`
    NotExists(Box<GroupGraphPattern>),
    /// `expr IN (list)`
    In {
        /// Left-hand side.
        lhs: Box<Self>,
        /// List of right-hand values.
        rhs: Vec<Self>,
    },
    /// `expr NOT IN (list)`
    NotIn {
        /// Left-hand side.
        lhs: Box<Self>,
        /// List of right-hand values.
        rhs: Vec<Self>,
    },
}

/// Binary operators in SPARQL expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    /// `||`
    Or,
    /// `&&`
    And,
    /// `=`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
}
