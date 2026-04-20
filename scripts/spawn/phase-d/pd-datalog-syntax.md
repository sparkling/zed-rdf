---
agent_id: pd-datalog-syntax
cohort: cohort-a
hive: phase-d
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/datalog-syntax/**
forbidden_reads:
  - phase-d-adv
---

# pd-datalog-syntax — Datalog syntax parser

You are implementing the Datalog syntax parser at `crates/datalog-syntax/`. The stub is already in place.

## Read first

1. `docs/adr/0023-phase-d-execution-plan.md` — scope, exit gate, Phase D rules.
2. `crates/datalog-syntax/src/lib.rs` — existing stub; you will fill this in.
3. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait, `Facts`, `ParseOutcome`, `Diagnostics`.
4. `docs/adr/0007-parser-technology.md` — hand-roll default (Accepted).

## Goal

Implement a hand-rolled recursive-descent parser for Datalog syntax. The target grammar supports:

- Rules of the form `Head :- Body.` where Head is an atom and Body is a comma-separated list of atoms and/or negated atoms (`not atom`).
- Facts of the form `Atom.` (rules with empty body).
- Atoms of the form `RelName(arg1, arg2, ...)` where arguments are constants (quoted strings or unquoted identifiers starting with a lowercase letter) or variables (identifiers starting with an uppercase letter).
- Single-line comments starting with `%`.

Emit facts with subject `<urn:x-datalog-syntax:program>` for the top-level program node and predicates under the `<urn:x-datalog-syntax:*>` namespace (e.g. `<urn:x-datalog-syntax:rule>`, `<urn:x-datalog-syntax:head>`, `<urn:x-datalog-syntax:body>`, `<urn:x-datalog-syntax:atom>`).

No execution — grammar and syntax only. Do not evaluate or execute programs.

## Tests

- Check if `crates/testing/rdf-diff/tests/adversary-datalog/` exists; if so un-ignore any fixtures there.
- Write hand-crafted fixtures in `crates/datalog-syntax/tests/fixtures/` covering:
  - Empty program.
  - A simple fact (`parent(tom, bob).`).
  - A rule with one body atom (`ancestor(X, Y) :- parent(X, Y).`).
  - A rule with multiple body atoms.
  - A rule with negation (`not reachable(X, Y)`).
  - A comment line.
  - Invalid input — expect fatal `Diagnostics`.
- Run `cargo test -p datalog-syntax` — all tests must pass.

## Acceptance

- `cargo check -p datalog-syntax` green.
- Hand-written fixture corpus green.
- `cargo clippy -p datalog-syntax -- -D warnings` clean.
- No `todo!()` or `unimplemented!()` in production paths.

## Claims

Claim `crates/datalog-syntax/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in namespace `crate/datalog-syntax` describing the recursive-descent strategy, grammar entry points, and any design decisions.
- `memory_store` exit report at `phase-d` blackboard: `pd-datalog-syntax:done` with fixture pass count.

## Handoff

`claims_accept-handoff` to `pd-tester` and `pd-reviewer` when `cargo check` + all fixture tests are green.
