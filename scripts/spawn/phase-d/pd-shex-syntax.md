---
agent_id: pd-shex-syntax
cohort: cohort-a
hive: phase-d
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/shex-syntax/**
forbidden_reads:
  - phase-d-adv
---

# pd-shex-syntax — ShEx 2.x compact syntax parser

You are implementing the ShEx 2.x compact syntax parser at `crates/shex-syntax/`. The stub is already in place.

## Read first

1. `docs/adr/0023-phase-d-execution-plan.md` — scope, exit gate, Phase D rules.
2. `crates/shex-syntax/src/lib.rs` — existing stub; you will fill this in.
3. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait, `Facts`, `ParseOutcome`, `Diagnostics`.
4. `docs/adr/0007-parser-technology.md` — hand-roll default (Accepted).

## Goal

The ShEx 2.x compact syntax grammar is at https://shex.io/shex-semantics/#shexc.

Implement a hand-rolled recursive-descent parser that:

1. Parses ShEx compact syntax schemas (`PREFIX`, `BASE`, shape expressions, node constraints, triple constraints, etc.).
2. Emits AST structure as `rdf_diff::Facts` — use subject `<urn:x-shex-syntax:schema>` for the top-level schema node and predicates under the `<urn:x-shex-syntax:*>` namespace for structural facts (e.g. `<urn:x-shex-syntax:shapeLabel>`, `<urn:x-shex-syntax:tripleConstraint>`, `<urn:x-shex-syntax:nodeConstraint>`).
3. Produces `Diagnostics` for syntax errors with informative messages (line/column where possible).

No ShEx validation — grammar and syntax only. Do not implement the ShEx semantics or conformance checking.

## Tests

- Check if `crates/testing/rdf-diff/tests/adversary-shex/` exists; if so un-ignore any fixtures there.
- Write hand-crafted fixtures in `crates/shex-syntax/tests/fixtures/` covering:
  - Empty schema.
  - Simple shape with a triple constraint.
  - Shape with cardinality (`{1,3}`, `+`, `*`, `?`).
  - `PREFIX` and `BASE` declarations.
  - Nested shape references.
  - Invalid input — expect fatal `Diagnostics`.
- Run `cargo test -p shex-syntax` — all tests must pass.

## Acceptance

- `cargo check -p shex-syntax` green.
- Hand-written fixture corpus green.
- `cargo clippy -p shex-syntax -- -D warnings` clean.
- No `todo!()` or `unimplemented!()` in production paths.

## Claims

Claim `crates/shex-syntax/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in namespace `crate/shex-syntax` describing the recursive-descent strategy, grammar entry points, and any spec divergences.
- `memory_store` exit report at `phase-d` blackboard: `pd-shex-syntax:done` with fixture pass count.

## Handoff

`claims_accept-handoff` to `pd-tester` and `pd-reviewer` when `cargo check` + all fixture tests are green.
