---
agent_id: pc-sparql-wiring
cohort: cohort-a
hive: phase-c
role: backend-dev
model: claude-opus-4-7
worktree: true
claims:
  - crates/sparql-syntax/**
  - xtask/verify/**
forbidden_reads:
  - phase-c-adv
  - verification/adversary-findings/sparql
---

# pc-sparql-wiring — xtask/verify SPARQL wiring + W3C integration tests

You are cohort-A agent `pc-sparql-wiring`. The `sparql-syntax` crate
already exists (4562 lines, compiles clean). Your job is to wire it to
the W3C test suite via `xtask/verify` and create integration tests so
that `cargo run -p xtask -- verify sparql` exits 0 at 100% pass.

## Read first

1. `crates/sparql-syntax/src/lib.rs` — public API entry point.
2. `xtask/verify/src/manifest.rs` — existing language cases (`nt`, `nq`,
   `ttl`, `trig`, `rdfxml`, `jsonld`); you will add `sparql-query` and
   `sparql-update` here.
3. `external/tests/sparql/manifest-sparql11-query.ttl` (first 50 lines)
   — W3C manifest structure: `mf:name`, `qt:query`, `mf:result`
   (positive), `mf:action` (negative syntax tests).
4. `external/tests/sparql/manifest-sparql11-update.ttl` (first 50 lines)
   — update manifest structure: `ut:request` in place of `qt:query`.
5. `docs/adr/0022-phase-c-execution-plan.md` — scope, exit gate, Phase C
   rules.
6. `docs/adr/0007-parser-technology.md` — hand-roll default (Accepted).
7. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait, `Facts`,
   `ParseOutcome`, `Diagnostics`.

## Goal

### 1. Extend `xtask/verify/src/manifest.rs`

Add `sparql-query` and `sparql-update` language cases to
`parse_for_language`. The SPARQL manifests differ from RDF manifests:

- Positive syntax tests: entry has `mf:name` + `qt:query` (or
  `ut:request` for update) but no `mf:result` triple — the absence of
  `mf:result` means "parse must succeed".
- Negative syntax tests: entry has `mf:action` pointing at the query
  file — parse must fail.

Use the same `ManifestEntry` shape already in use for other languages.
Mirror the `rdfxml` case as the closest template.

### 2. Create `crates/sparql-syntax/tests/w3c_syntax.rs`

An integration test file that:

1. Loads `external/tests/sparql/manifest-sparql11-query.ttl` (and the
   update manifest).
2. Iterates entries, calling `SparqlParser::new()` (or the equivalent
   public constructor from `crates/sparql-syntax/src/lib.rs`) on each
   test file.
3. Asserts positive entries parse without fatal diagnostics; asserts
   negative entries return a fatal error.
4. Reports the pass/fail count; fails the test if any entry is
   unexpected.

Wire via `#[test]` functions or a `#[test_case]`-style macro — whatever
fits the existing test style in `crates/sparql-syntax/`.

### 3. Fix conformance failures

Run `cargo run -p xtask -- verify sparql` and iterate until 100% pass.
Allow-list entries only where the spec is genuinely ambiguous; each
allow-list entry needs a one-line retirement plan comment.

## Acceptance

- `cargo check -p sparql-syntax` green.
- `cargo test -p sparql-syntax` green (including `w3c_syntax.rs`).
- `cargo run -p xtask -- verify sparql` exits 0 with 100% pass rate
  (or a documented allow-list).
- No new `todo!()` or `unimplemented!()` introduced.

## Claims

Claim `crates/sparql-syntax/**` and `xtask/verify/**` before editing.
Release both on completion.

## Memory

- `memory_store` at `implementation/approach` in `crate/sparql-syntax`
  describing the manifest wiring strategy and any spec divergences.
- `memory_store` exit report at `phase-c` blackboard:
  `pc-sparql-wiring:done` with W3C pass/fail counts for query and
  update manifests separately.

## Handoff

`claims_accept-handoff` → `pc-reviewer` when `cargo check` + all tests
are green.
