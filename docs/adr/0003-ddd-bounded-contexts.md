# ADR-0003: DDD bounded contexts

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Tags:** `domain`, `architecture`

## Context and Problem Statement

The project spans a dozen parser-style crates, a shared vocabulary
database, formatters, an LSP, and a Zed extension. Each RDF-family
language (Turtle, SPARQL, ShEx, Datalog, …) is governed by a different
W3C document that evolves on its own cadence. Without discipline, types
bleed across languages and a change in one spec cascades into unrelated
code.

We need an explicit bounded-context map.

## Decision Drivers

- Spec evolution should be **local** to the owning crate.
- The LSP should consume a small, stable "facts" surface from each
  parser, not the parser's internal CST.
- `rdf-vocab` is cross-cutting (used for hover across every RDF-family
  language); it must depend on nothing except IRI + core types.
- Long-lived mental model: one context = one folder = one set of words.

## Considered Options

1. **Layered only** — domain / application / infrastructure split
   without explicit contexts.
2. **One context per W3C spec group.** Each language gets its own parser
   crate; cross-cutting contexts (diagnostics, IRI, vocab, formatting,
   LSP) each get their own crate.
3. **Unified object graph** — one RDF-ish tree spanning all languages.

## Decision

**Chosen option: Option 2 — one bounded context per W3C spec group plus
cross-cutting contexts.**

Context map as in [`03-architecture.md`](../sparc/03-architecture.md) §2:

1. Diagnostics (cross-cutting)
2. IRI (cross-cutting)
3. Syntax — one sub-context per language (Turtle+TriG, N-Triples+N-Quads,
   RDF/XML, JSON-LD, TriX, N3, SPARQL, ShEx, Datalog)
4. Vocabulary (cross-cutting)
5. Formatting (per-format, grouped)
6. LSP (feature services over all of the above)
7. Editor assets (`.scm` queries, per-language `config.toml`)
8. Extension (`wasm32-wasip2` shim)
9. Testing

Rules of engagement:

- **Downstream depends on upstream only.** Enforced by `cargo-deny`.
  `foundations ← vocab, syntax ← format, lsp, testing`.
- **Boundary types are explicit.** A parser's internal CST is that
  parser's business. The LSP consumes a stable `Facts` struct
  (prefixes, symbols, references) and `Diagnostic`s. Renaming an AST
  node does not break the LSP.
- **Ubiquitous language follows the spec** — types carry their spec
  name: `Iri`, `PrefixedName`, `BlankNodeLabel`, `VariableOccurrence`,
  `ShapeExpr`, `Clause`, etc.
- **No shared mutable state.** Values in; values out.
- **Tests stay with the context.** Each crate tests its own public
  surface. Cross-crate tests live in `crates/testing/rdf-testsuite/`.

## Consequences

- **Positive**
  - Spec evolution is local — only the owning parser updates when
    SPARQL 1.2 adds a clause.
  - Reviewers know who owns what.
  - Parallel development across languages is cheap.
- **Negative**
  - Some duplication at boundaries (parser CST + LSP Facts). Acceptable:
    the duplication is the boundary.
  - Adding a language means adding a crate, not a file. Intentional.
- **Neutral**
  - The LSP sits on a wide dependency fan-in; changing any parser
    surface may bump LSP code. Acceptable because the LSP is the
    consumer of record.

## Validation

- `cargo-deny` green.
- A spec-only change touches one syntax crate + its tests.
- New contributors can name the owning crate for a feature within one
  minute of reading the architecture doc.

## Links

- `docs/sparc/03-architecture.md` §1-§2.
- Evans, *Domain-Driven Design* — context mapping patterns.
- ADR-0002 workspace topology.
