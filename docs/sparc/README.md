# SPARC plan for `zed-rdf`

This directory holds the **SPARC** plan for a ground-up Rust implementation of
the RDF language family, SPARQL, and Datalog, plus the tooling that surfaces
them in Zed.

SPARC is a five-phase structured-reasoning workflow:

| Phase            | Doc                                      | Purpose                                                                 |
|------------------|------------------------------------------|-------------------------------------------------------------------------|
| **S**pecification | [`01-specification.md`](01-specification.md) | What exactly must the system do — standards, scope, NFRs, acceptance.  |
| **P**seudocode    | [`02-pseudocode.md`](02-pseudocode.md)       | Algorithmic sketches for critical paths, filled per phase.             |
| **A**rchitecture  | [`03-architecture.md`](03-architecture.md)   | DDD bounded contexts, crate topology, pipelines, cross-cutting policy. |
| **R**efinement    | [`04-refinement.md`](04-refinement.md)       | Iteration plan (phases A-I), TDD workflow, risks, benchmarks.          |
| **C**ompletion    | [`05-completion.md`](05-completion.md)       | Per-module DoD, W3C compliance gate, release + governance.             |

Architecturally significant decisions are recorded as MADR-format ADRs in
[`../adr/`](../adr/). Every SPARC doc below should reference the ADRs that
justify its claims — when a SPARC doc and an ADR disagree, **the ADR wins**
(and the SPARC doc is updated).

## Reading order

1. Start with [`01-specification.md`](01-specification.md) — in particular the
   **Scope Decisions Required** block. Nothing else is final until those are
   resolved.
2. Skim [`03-architecture.md`](03-architecture.md) for the shape of the system.
3. Refer to [`../adr/README.md`](../adr/README.md) for the ADR index.
4. [`04-refinement.md`](04-refinement.md) is the execution plan once scope is
   confirmed.

## How this plan evolves

- **Specification** is mostly static once scope is confirmed; edits require an
  ADR if they change scope.
- **Pseudocode** is filled just-in-time, one phase ahead of implementation.
- **Architecture** is living — new bounded contexts and module splits land via
  ADRs.
- **Refinement** is the rolling plan: phase plans and risk register update
  every milestone.
- **Completion** is revised whenever a W3C spec or test suite we target
  publishes a new version.
