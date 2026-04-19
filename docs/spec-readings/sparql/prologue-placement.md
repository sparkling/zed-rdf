# Pin: SPARQL Prologue placement — `BASE` / `PREFIX` are prologue-only

- **Diagnostic code:** `SPARQL-PROLOGUE-001`
- **Language / format:** SPARQL 1.1 Query and Update.
- **Productions:** `Prologue ::= (BaseDecl | PrefixDecl)*`,
  `BaseDecl`, `PrefixDecl`, `QueryUnit`, `UpdateUnit` (§4.1; §19.8
  production [4]).
- **Spec target:** W3C SPARQL 1.1 Query
  <https://www.w3.org/TR/sparql11-query/#rPrologue>; SPARQL 1.1
  Update <https://www.w3.org/TR/sparql11-update/#Prologue>.
- **Status:** active.
- **Author:** `fe-phase-c-sparql`.
- **Date:** 2026-04-19.
- **Adversary reference:** Failure Mode 5 in
  `docs/verification/adversary-findings/sparql.md`.

## Ambiguous clause

SPARQL 1.1 §4.1 says:

> "SPARQL queries are a sequence of declarations: BASE and PREFIX
> followed by one of the query forms."

The grammar production is `Prologue ::= (BaseDecl | PrefixDecl)*`,
and the top-level productions are:

```text
Query      ::= Prologue (SelectQuery | ConstructQuery | DescribeQuery | AskQuery) ValuesClause
Update     ::= Prologue (Update1 (';' Prologue Update)?)?
```

The Prologue is structurally distinct from the query body: no `BASE`
or `PREFIX` declaration is reachable from inside a `GroupGraphPattern`
or `TriplesTemplate`. Implementations that reuse a Turtle parser for
IRI handling may accidentally permit `BASE` anywhere (because Turtle
does allow directives interleaved with triples) — that is a real
divergence and is the subject of this pin.

Between Update operations, a sub-Prologue is permitted by
`Update ::= Prologue (Update1 (';' Prologue Update)?)?`. Sub-prologues
inherit prior declarations and may add new ones; the Phase-C parser
preserves them lexically but does not scope-resolve.

## Reading chosen

The parser MUST reject `BASE` or `PREFIX` tokens encountered inside a
`GroupGraphPattern`, `SubSelect`, `TriplesTemplate`, or `QuadPattern`
body. The diagnostic code is `SPARQL-PROLOGUE-001` with severity
`error` (fatal). The error message identifies the offending keyword
and cites §4.1.

Sub-prologues at Update-unit boundaries are accepted.

## Rationale

- The grammar is context-free: a LALR or recursive-descent parser
  with the Prologue as a distinct phase will naturally reject mid-body
  `BASE`/`PREFIX`. An implementation that treats them as statements
  anywhere inside a block has conflated Turtle and SPARQL grammars.
- The adversary brief's Failure Mode 5 specifically probes this split.
- W3C test suite: see `data-sparql11/syntax-query/` manifest,
  "syntax-04" entries which include prologue-placement negatives.

## Non-goals

- This pin does **not** cover the Turtle `@prefix` directive or its
  SPARQL-style `PREFIX` spelling when used at the top — both are fine.
- It does not constrain relative-IRI resolution order; that is a
  separate concern covered by `IRI-PCT-001`.

## Diagnostic code

- **Code:** `SPARQL-PROLOGUE-001`
- **Emitted by:** `sparql-syntax`.
- **Message template:** `SPARQL-PROLOGUE-001: BASE|PREFIX declaration
  is not allowed inside a group graph pattern (SPARQL 1.1 §4.1:
  Prologue ::= (BaseDecl | PrefixDecl)*) at byte N`.
- **Fatal?** Yes.
