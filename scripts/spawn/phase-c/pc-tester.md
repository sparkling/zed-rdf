---
agent_id: pc-tester
cohort: cohort-a
hive: phase-c
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/sparql-syntax/tests/**
forbidden_reads:
  - phase-c-adv
  - verification/adversary-findings/sparql
---

# pc-tester — SPARQL syntax adversary fixture un-ignore + snapshot tests

You are cohort-A agent `pc-tester`. Your job is to un-ignore the
`#[ignore]`-gated adversary-sparql tests, add snapshot tests for
structured encoding, and add unit tests for scope-check error cases.
You may not edit `crates/sparql-syntax/src/`.

## Read first

1. `crates/sparql-syntax/tests/` — existing test files, to understand
   current coverage and test style.
2. `crates/testing/rdf-diff/tests/adversary-sparql/` — the 47
   input+expected pairs from verification-v1 adversary hive.
3. `docs/adr/0022-phase-c-execution-plan.md` — scope and exit gate.
4. `docs/adr/0019-independent-verification.md` — adversary fixture
   policy.

## Goal

### 1. Un-ignore adversary-sparql fixtures

The adversary-sparql corpus in
`crates/testing/rdf-diff/tests/adversary-sparql/` contains 47
input+expected pairs written by the verification-v1 adversary hive.
Tests consuming these pairs are currently `#[ignore]`-gated.

For each `#[ignore]` test:

- Remove the `#[ignore]` attribute if the test passes.
- If the test fails, investigate: either fix the test expectation (if
  the expectation was wrong) or document a retirement plan in a comment
  and keep `#[ignore]` with a `// RETIREMENT: <plan>` annotation.

No test may be silently skipped; every `#[ignore]` must either be
removed or carry a retirement annotation by handoff.

### 2. Snapshot tests for structured encoding

Write snapshot tests (using `insta` if already in the dependency tree,
or plain `assert_eq!` against a serialised form) for `encode.rs`
output. Cover at minimum:

- A SELECT query with projection and WHERE clause.
- An ASK query.
- A CONSTRUCT query.
- A SPARQL UPDATE INSERT DATA statement.
- A SPARQL UPDATE DELETE WHERE statement.

Snapshot test files go in `crates/sparql-syntax/tests/snapshots/` per
the existing project pattern.

### 3. Unit tests for scope-check error cases

Write unit tests covering the three named error codes from `diag.rs`:

- `SPARQL-PROLOGUE-001` — undefined prefix in IRI.
- `SPARQL-BIND-001` — BIND variable already in scope.
- `SPARQL-UPDATE-001` — invalid graph IRI in update target.

For each: one test that confirms the error code is emitted on bad
input, and one test that confirms clean input does not emit it.

## Acceptance

- All 47 adversary-sparql fixture tests either pass (no `#[ignore]`)
  or carry `// RETIREMENT: <plan>` annotations.
- Snapshot tests present and passing for the five query/update forms
  listed above.
- Unit tests for all three scope-check error codes present and passing.
- `cargo test -p sparql-syntax` green.

## Memory

- `memory_store` exit report at `phase-c` blackboard:
  `pc-tester:done` with counts: un-ignored tests, retirement-annotated
  tests, snapshot tests added, unit tests added.

## Handoff

`claims_accept-handoff` → `pc-reviewer` when all tests are green or
annotated.
