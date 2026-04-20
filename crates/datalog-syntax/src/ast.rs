//! AST types for a Datalog program.

/// A complete Datalog program: an ordered list of statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// Ordered list of top-level statements (facts and rules).
    pub statements: Vec<Statement>,
}

/// A top-level Datalog statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// A ground fact: `relname(arg, …).`
    Fact(Atom),
    /// A rule: `head :- body.`
    Rule(Rule),
}

/// A Datalog rule: `head :- body_literal, …`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The rule head.
    pub head: Atom,
    /// One or more body literals (non-empty by grammar).
    pub body: Vec<Literal>,
}

/// A body literal — either a positive or negated atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// Positive literal: `atom`
    Positive(Atom),
    /// Negative literal: `not atom`
    Negative(Atom),
}

/// A relation application: `relname(arg, …)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    /// Relation name — must start with a lowercase letter.
    pub relname: String,
    /// Argument list (may be empty).
    pub args: Vec<Arg>,
}

/// A single argument to an atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// An unbound variable: starts with an uppercase letter.
    Variable(String),
    /// A ground constant: lowercase identifier or quoted string.
    Constant(String),
}
