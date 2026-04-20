//! AST node types for ShEx 2.x compact syntax (ShExC).
//!
//! These types are an internal representation; they are not part of the
//! public API. The `parser` module fills them in; `encode` flattens them
//! into `(Fact, FactProvenance)` pairs.

/// A complete ShEx schema document.
#[derive(Debug, Clone, Default)]
pub(crate) struct Schema {
    /// Optional base IRI, set by `BASE <iri>`.
    pub(crate) base: Option<String>,
    /// Prefix bindings declared with `PREFIX name: <iri>`.
    pub(crate) prefixes: Vec<(String, String)>,
    /// Shape declarations in document order.
    pub(crate) shapes: Vec<ShapeDecl>,
}

/// A single shape declaration: `<label> <expr>` or `<label> EXTENDS @<ref> <expr>`.
#[derive(Debug, Clone)]
pub(crate) struct ShapeDecl {
    /// Shape label (IRI or BNode).
    pub(crate) label: Label,
    /// The shape expression body.
    pub(crate) expr: ShapeExpr,
    /// Byte offset of the label token.
    pub(crate) offset: usize,
}

/// A shape label — the LHS of a shape declaration.
#[derive(Debug, Clone)]
pub(crate) enum Label {
    /// Angle-bracketed IRI reference: `<iri>`.
    Iri(String),
    /// Prefixed name: `ex:Foo`.
    Pname { prefix: String, local: String },
    /// Blank node: `_:label`.
    BNode(String),
}

/// A shape expression (recursive).
#[derive(Debug, Clone)]
pub(crate) enum ShapeExpr {
    /// `{ tripleExpr* }` — a shape with triple constraints.
    Shape(Shape),
    /// Node constraint — `xsd:string`, `IRI`, `LITERAL`, etc.
    NodeConstraint(NodeConstraint),
    /// Shape reference `@<label>`.
    Ref(Label),
    /// `<a> AND <b>`
    And(Box<Self>, Box<Self>),
    /// `<a> OR <b>`
    Or(Box<Self>, Box<Self>),
    /// `NOT <a>`
    Not(Box<Self>),
    /// Explicit `ANY` / `.` — matches any node.
    Any,
}

/// A `{ tripleExpr* }` shape.
#[derive(Debug, Clone)]
pub(crate) struct Shape {
    /// The triple constraints in the shape.
    pub(crate) triple_exprs: Vec<TripleExpr>,
    /// CLOSED keyword present?
    pub(crate) closed: bool,
    /// EXTENDS references.
    pub(crate) extends: Vec<Label>,
}

/// A triple expression — either a single constraint or a one-of group.
#[derive(Debug, Clone)]
pub(crate) enum TripleExpr {
    /// A single triple constraint.
    Constraint(TripleConstraint),
    /// `( expr1 | expr2 | … )` — a one-of.
    OneOf(Vec<Self>),
}

/// A single triple constraint: `<predicate> <valueExpr> <cardinality>`.
#[derive(Debug, Clone)]
pub(crate) struct TripleConstraint {
    /// `INVERSE` keyword was present.
    pub(crate) inverse: bool,
    /// Predicate IRI (or `a` for rdf:type).
    pub(crate) predicate: Predicate,
    /// Value expression (shape expr on the object).
    pub(crate) value_expr: Option<Box<ShapeExpr>>,
    /// Cardinality.
    pub(crate) cardinality: Cardinality,
    /// Byte offset.
    pub(crate) offset: usize,
}

/// A predicate reference in a triple constraint.
#[derive(Debug, Clone)]
pub(crate) enum Predicate {
    /// `a` keyword (rdf:type).
    RdfType,
    /// `<iri>`.
    Iri(String),
    /// `prefix:local`.
    Pname { prefix: String, local: String },
}

/// A node constraint — a restriction on the type of the node.
#[derive(Debug, Clone)]
pub(crate) enum NodeConstraint {
    /// `IRI` — node must be an IRI.
    Iri,
    /// `LITERAL` — node must be a literal.
    Literal,
    /// `NONLITERAL` — node must be an IRI or blank node.
    NonLiteral,
    /// `BNODE` — node must be a blank node.
    BNode,
    /// Datatype constraint: `xsd:string`, `<iri>`, etc.
    Datatype(DatatypeRef),
    /// Value set: `[ val1 val2 … ]`.
    ValueSet(Vec<ValueSetItem>),
}

/// A datatype IRI or prefixed name.
#[derive(Debug, Clone)]
pub(crate) enum DatatypeRef {
    /// `<iri>`.
    Iri(String),
    /// `prefix:local`.
    Pname { prefix: String, local: String },
}

/// A value in a value set `[ … ]`.
#[derive(Debug, Clone)]
pub(crate) enum ValueSetItem {
    /// `<iri>`.
    Iri(String),
    /// `prefix:local`.
    Pname { prefix: String, local: String },
    /// `"lexical"`, `"lexical"^^<iri>`, `"lexical"@lang`.
    Literal(String),
}

/// Cardinality of a triple constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cardinality {
    /// Exactly 1 (default, no modifier).
    One,
    /// `?` — 0 or 1.
    Optional,
    /// `*` — 0 or more.
    Star,
    /// `+` — 1 or more.
    Plus,
    /// `{n}` — exactly n.
    Exact(u32),
    /// `{n,m}` — between n and m.
    Range(u32, u32),
    /// `{n,}` — n or more (unbounded upper).
    AtLeast(u32),
}
