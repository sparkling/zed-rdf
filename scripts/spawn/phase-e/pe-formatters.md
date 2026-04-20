---
agent_id: pe-formatters
cohort: cohort-a
hive: phase-e
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-format/**
---

# pe-formatters — Turtle, N-Triples, N-Quads idempotency + formatter completion

You are cohort-A agent `pe-formatters`. Your job is to implement idempotency
property tests for the existing formatters in `crates/rdf-format/` and to
ensure all three required formatters (Turtle, N-Triples, N-Quads) are
complete with idempotency guarantees.

## Read first

1. `.claude-flow/phase-e/arch-memo.md` — architect memo defining the
   idempotency test strategy and formatter API decisions. **Read this
   before writing any code.**
2. `docs/adr/0024-phase-e-execution-plan.md` — scope: Turtle (idempotent),
   N-Triples, N-Quads. RDF/XML and JSON-LD formatters are stretch goals.
3. `crates/rdf-format/src/lib.rs` — current formatter state. All four
   writers (`NTriplesWriter`, `NQuadsWriter`, `TurtleWriter`, `TriGWriter`)
   are fully implemented with inherent impl pattern. No idempotency tests
   exist yet.
4. `crates/rdf-format/Cargo.toml` — current dependencies.

## Goal

### 1. Idempotency property tests

Add idempotency tests for each formatter. The contract is:

```
format(format(x)) == format(x)
```

For each of `NTriplesWriter`, `NQuadsWriter`, and `TurtleWriter`:

1. Write a set of deterministic fixture round-trips in
   `crates/rdf-format/tests/idempotency.rs`.
2. For each formatter: take a set of `Fact` fixtures (covering IRIs,
   blank nodes, plain literals, lang-tagged literals, datatyped literals),
   serialise to a `String`, then serialise that output again (parsing the
   string back via the corresponding Phase-A parser, then re-serialising),
   and assert the two output strings are equal.

   For `TurtleWriter`: run the idempotency test both with and without
   prefix registration to confirm the prefix header is stable.

3. Use `dev-dependencies` `rdf-ntriples` and `rdf-turtle` (already
   present in `crates/rdf-format/Cargo.toml`) as the parse-back step.

### 2. Formatter completeness review

Read `crates/rdf-format/src/lib.rs` in full. Confirm each of the three
required formatters is complete:

- `NTriplesWriter`: complete if IRI escaping, blank node pass-through,
  literal re-escaping (LF, CR, TAB), and datatype/lang-tag suffixes all
  work. Check existing tests cover these cases — if coverage gaps exist,
  add targeted tests.
- `NQuadsWriter`: complete if graph-slot emission and default-graph
  omission work. Same test check.
- `TurtleWriter`: complete if prefix header, pname compaction (longest
  match), unsafe local-part fallback, and long-literal form all work.

Do NOT rewrite the formatters unless a bug is found. The existing
implementation is considered correct per Phase A — only add tests and fix
actual bugs (if any).

### 3. Acceptance

- `cargo test -p rdf-format` green (including new idempotency tests).
- `cargo clippy -p rdf-format -- -D warnings` clean.
- `format(format(x)) == format(x)` property holds for all fixture cases
  in `tests/idempotency.rs`.
- No `todo!()` or `unimplemented!()` introduced.

## Claims

Claim `crates/rdf-format/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/formatter-idempotency` in
  `crate/rdf-format` namespace: idempotency test approach and any bugs
  found and fixed.
- `memory_store` exit report at `phase-e` blackboard:
  `pe-formatters:done` with formatter test pass counts.

## Handoff

`claims_accept-handoff` → `pe-tester` when `cargo test -p rdf-format`
and `cargo clippy -p rdf-format` are both green.
