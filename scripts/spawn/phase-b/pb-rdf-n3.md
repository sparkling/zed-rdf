---
agent_id: pb-rdf-n3
cohort: cohort-a
hive: phase-b
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-n3/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-rdf-n3 — Main N3 (Notation3) parser

You are cohort-A agent `pb-rdf-n3`. Implement the N3 parser at
`crates/rdf-n3/`. A stub is already in place.

## Read first

1. `crates/rdf-n3/src/lib.rs` — stub.
2. `crates/rdf-turtle/src/lib.rs` — the Turtle parser you will extend at
   the grammar level. N3 is Turtle + additional productions.
3. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait.
4. `docs/adr/0021-phase-b-execution-plan.md`.
5. Notation3 (N3): <https://www.w3.org/TeamSubmission/n3/>

## Goal

A working `N3Parser` that:

1. Depends on `rdf-turtle` for the Turtle grammar base (already in
   `Cargo.toml`). Extend at the grammar/lexer level — do NOT duplicate the
   Turtle code.
2. Adds N3-specific productions:
   - `@keywords` directive.
   - Reification shorthand (`[ a rdf:Statement; … ]` is Turtle — N3 adds
     `{ }` for quoted formulas, `=>` for logical implication).
   - Quoted formulas (`{ … }`) — emit as named graph or skip with a warning
     per the diff-harness spec (facts in a formula are graph-scoped).
3. Implements `rdf_diff::Parser`. Quoted-formula facts use the formula IRI
   as the graph name (`Fact::graph`).

## Acceptance

- `cargo check -p rdf-n3` green.
- Snapshot corpus covers: basic N3 triples, `@keywords` shorthand, a simple
  quoted formula, and a negative case.
- `cargo test -p rdf-n3` green.

## Claims

Claim `crates/rdf-n3/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in `crate/rdf-n3`.
- `memory_store` exit report at `phase-b` blackboard: `pb-rdf-n3:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
