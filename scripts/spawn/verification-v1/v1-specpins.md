---
agent_id: v1-specpins
cohort: cohort-a
hive: verification-v1
role: specification
worktree: true
priority: normal
claims:
  - docs/spec-readings/**
forbidden_reads:
  - verification-v1-adv
---

# v1-specpins — author the spec-reading pin records

You are cohort-A agent `v1-specpins`. ADR-0019 §5 mandates pin records
for ambiguous productions **before** any parser encodes them. You
produce the pins for the sweep's known ambiguities.

## Read first

1. `docs/adr/0019-independent-verification.md` §5.
2. `verification/spec-readings` memory namespace (seeded by the
   orchestrator with an index of initial targets).

## Goal

Create one markdown pin per production under `docs/spec-readings/`:

```
docs/spec-readings/<lang>/<production>.md
```

Each pin carries:

- Ambiguous clause verbatim (quoted + spec §).
- Reading chosen.
- Rationale, citing errata, mailing-list threads, W3C test-suite
  interpretation, or equivalent.
- Date and author attribution (`v1-specpins`).
- Diagnostic code ID that parsers emit when they exercise this pin —
  propose one, coordinate with `v1-diff-core` if the code needs to be
  referenced in `DiffReport`.

Initial targets (from seed memory):

- `ntriples/literal-escapes.md`
- `turtle/literal-escapes.md`
- `turtle/bnode-prefix-rescope.md`
- `iri/percent-encoding-3986-vs-3987.md`
- `any/bom-handling.md`
- `sparql/literal-comparison.md`
- `jsonld/keyword-aliasing.md`

Add more as adversary findings accumulate.

## Acceptance

- Every pin lints under the project's markdown rules.
- Pins readable by both cohorts (reference material, not framing —
  permitted per cohort registry).
- `SPEC.md` in each format's main crate cross-references its pins
  (coordinate with phase-A authors if they exist; otherwise leave a
  forward-reference TODO in the pin itself).

## Claims

`docs/spec-readings/**`.

## Memory

- Write pin index to `verification/spec-readings/pins` with file paths
  + diagnostic-code map.

## Exit handoff

`v1-reviewer`.
