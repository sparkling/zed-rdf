# Pin: SPARQL BIND variable scoping

- **Diagnostic code:** `SPARQL-BIND-001`
- **Language / format:** SPARQL 1.1 Query (also applies to the
  WHERE clause of Update's Modify).
- **Productions:** `Bind ::= 'BIND' '(' Expression 'AS' Var ')'`
  (§19.8 production [60]), GroupGraphPattern (§17.1),
  in-scope variable algebra (§18.2.1).
- **Spec target:** W3C SPARQL 1.1 Query
  <https://www.w3.org/TR/sparql11-query/#assignment>; §18.2.1
  "Variable Scope".
- **Status:** active.
- **Author:** `fe-phase-c-sparql`.
- **Date:** 2026-04-19.
- **Adversary reference:** Failure Mode 11b in
  `docs/verification/adversary-findings/sparql.md`.

## Ambiguous clause

SPARQL 1.1 §18.2.1 says:

> "The variable assigned by BIND must not have been used in the
> group graph pattern up to the point of use in BIND."

The "up to the point of use" qualifier has invited three readings:

1. **Strict textual.** Any earlier appearance of `?x` in the
   surrounding GGP — whether in a triple pattern, BIND, VALUES, or
   even in a MINUS or OPTIONAL — forbids `BIND(... AS ?x)`.
2. **Bound-set only.** Only variables that would *bind* (triple
   positions, VALUES, subquery projections) count, not variables that
   only appear in FILTER or in the expression of an earlier BIND.
3. **Lenient / lazy.** Any order works; implementations re-order
   BINDs at evaluation time.

Reading 3 is wrong (the spec is explicit); reading 2 is too narrow
(FILTER uses of `?x` do establish "use in the GGP").

## Reading chosen

Reading 1 (strict textual) — a `BIND(... AS ?x)` is rejected if
`?x` already appears anywhere earlier in the enclosing
GroupGraphPattern, including in triple patterns, `VALUES`, a prior
`BIND`, or a preceding `FILTER`. Nested GGPs (OPTIONAL, MINUS, GRAPH,
SERVICE, UNION alternatives) each establish their own scope for this
rule — a variable appearing inside OPTIONAL is NOT considered "in
scope" for the enclosing GGP's BIND check unless it also appears in
the enclosing GGP's triples.

The parser tracks an in-scope set per GGP:

- When a triples block, VALUES, or BIND introduces a variable, it is
  added to the set.
- `BIND(expr AS ?x)` checks the set before inserting; any match
  raises `SPARQL-BIND-001`.

## Rationale

- The §18.2.1 clause is prescriptive; failing to check this at parse
  time lets implementation-specific re-ordering quietly change
  semantics.
- W3C test suite: `data-sparql11/bind/` manifest includes negative
  tests for this.

## Non-goals

- This pin does NOT enforce cross-GGP scoping (e.g., a variable
  bound in an outer OPTIONAL is visible to inner patterns). That is
  a semantic concern handled by evaluation.
- It does NOT check aggregate-scope rules (that is the separate
  `SPARQL-AGG-001` pin, currently reserved).

## Diagnostic code

- **Code:** `SPARQL-BIND-001`
- **Emitted by:** `sparql-syntax`.
- **Message template:** `SPARQL-BIND-001: BIND(... AS ?x) introduces
  variable already in scope (SPARQL 1.1 §18.2.1) at byte N`.
- **Fatal?** Yes.
