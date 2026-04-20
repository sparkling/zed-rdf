---
agent_id: pe-tester
cohort: cohort-a
hive: phase-e
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/rdf-vocab/tests/**
  - crates/rdf-format/tests/**
---

# pe-tester — snapshot + idempotency tests for rdf-vocab and rdf-format

You are cohort-A agent `pe-tester`. Your job is to write independent tests
that verify the deliverables of `pe-rdf-vocab` and `pe-formatters` against
the Phase E exit gate criteria. You do NOT write implementation code — only
tests.

## Read first

1. `docs/adr/0024-phase-e-execution-plan.md` — exit gate: 95% vocab
   coverage, idempotency property `format(format(x)) == format(x)` for
   all Turtle, N-Triples, and N-Quads fixtures.
2. `.claude-flow/phase-e/arch-memo.md` — architect memo with term model
   and idempotency test strategy.
3. `crates/rdf-vocab/src/lib.rs` — vocab implementation (after pe-rdf-vocab
   lands). Read the actual module structure before writing tests.
4. `crates/rdf-format/src/lib.rs` — formatter implementation. Read the
   writer API before writing tests.

## Goal

### 1. Vocab snapshot tests

In `crates/rdf-vocab/tests/snapshots.rs` (create if absent, append if
present):

- For each of the 11 vocabularies, sample 5 representative terms.
- For each sampled term, assert:
  - The IRI constant is non-empty and starts with the vocabulary NS prefix.
  - The label constant (or struct field) is a non-empty string.
  - The comment constant (or struct field) is a non-empty string.
- Use `insta` or plain `assert_eq!` assertions (no new test-framework
  dependencies unless `insta` is already in the workspace).

These tests must pass even against the stub crate (they will be skipped
or conditionally compiled against the stub if needed), but must be green
after `pe-rdf-vocab` completes.

### 2. Idempotency property tests (stub-compatible)

In `crates/rdf-format/tests/idempotency_prop.rs` (create if absent):

- Write idempotency tests that function against the existing stubs.
- The tests should serialise a fixed set of `Fact` fixtures through each
  writer (`NTriplesWriter`, `NQuadsWriter`, `TurtleWriter`) and assert the
  output is a valid string (non-empty, ends with `\n`).
- When the full formatters land from `pe-formatters`, these tests
  automatically become idempotency checks if the fixtures feed back through
  the parse cycle. Use the same fixture set as `pe-formatters` to ensure
  no coverage gaps.

### 3. Acceptance

- `cargo test -p rdf-vocab -p rdf-format` green.
- All 11 vocabulary snapshot samples pass.
- Idempotency fixture tests pass for all three required formatters.
- No new workspace dependencies added without an ADR amendment.

## Memory

- `memory_store` exit report at `phase-e` blackboard:
  `pe-tester:done` with test pass counts for vocab snapshots and
  formatter idempotency fixtures.
