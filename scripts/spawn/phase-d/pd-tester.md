---
agent_id: pd-tester
cohort: cohort-a
hive: phase-d
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/shex-syntax/tests/**
  - crates/datalog-syntax/tests/**
---

# pd-tester — Integration and snapshot tests for Phase D parsers

You write integration tests and snapshot tests for the ShEx and Datalog parsers once `pd-shex-syntax` and `pd-datalog-syntax` have completed their handoffs.

## Read first

1. `docs/adr/0023-phase-d-execution-plan.md` — Phase D scope and exit gate.
2. `crates/shex-syntax/src/lib.rs` — public API of the ShEx parser.
3. `crates/datalog-syntax/src/lib.rs` — public API of the Datalog parser.
4. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait surface.

## Goal

### 1. Adversary fixtures

- Check if `crates/testing/rdf-diff/tests/adversary-shex/` exists. If it does, un-ignore any `#[ignore]`-tagged tests in `crates/shex-syntax/tests/` that reference those fixtures.
- Check if `crates/testing/rdf-diff/tests/adversary-datalog/` exists. If it does, un-ignore any `#[ignore]`-tagged tests in `crates/datalog-syntax/tests/` that reference those fixtures.

### 2. Encode-output snapshot tests

Add snapshot tests that feed representative inputs to each parser and assert the encoded `rdf_diff::Facts` output matches a checked-in snapshot. Use insta or a simple `assert_eq!` against a string constant — whichever fits the codebase style already in use. Cover:

- ShEx: a schema with at least one shape, one node constraint, and one triple constraint.
- Datalog: a program with at least one fact and one rule with multiple body atoms.

### 3. Integration tests

Write integration tests in `crates/shex-syntax/tests/` and `crates/datalog-syntax/tests/` that verify:

- Round-trip: parse then re-encode produces stable output on repeated calls.
- Error paths: malformed inputs produce `ParseOutcome::Err` with `fatal: true`.

## Acceptance

- `cargo test -p shex-syntax -p datalog-syntax` green.
- All previously-ignored adversary tests enabled (or documented as absent).
- No `todo!()` in test code.

## Claims

Claim `crates/shex-syntax/tests/**` and `crates/datalog-syntax/tests/**` before editing. Release on completion.

## Memory

- `memory_store` exit report at `phase-d` blackboard: `pd-tester:done` with test counts for shex-syntax and datalog-syntax.

## Handoff

`claims_accept-handoff` to `pd-reviewer` when all tests are green.
