---
agent_id: v1-diff-core
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
claims:
  - crates/testing/rdf-diff/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-diff-core — fill the frozen diff-harness trait surface

You are cohort-A agent `v1-diff-core`. Your job is to fill the `todo!()`
bodies in the frozen trait surface at
`crates/testing/rdf-diff/src/lib.rs` without changing any public
signature. The API there is the integration contract for the entire
sweep; other agents depend on its shape.

## Read first (in this order)

1. `crates/testing/rdf-diff/src/lib.rs` — the contract you are
   implementing. Do not edit signatures.
2. `docs/adr/0019-independent-verification.md` §2 — canonical-form
   requirements.
3. `docs/adr/0020-verification-implementation-plan.md` §1.4, §3, §6.
4. `docs/adr/0006-testing-strategy.md` — where the harness sits in the
   pyramid.

## Goal

Implement:

- `Facts::canonicalise` — BNode-canonical relabelling (deterministic
  lexicographic), prefix-free IRIs (no pname shortening leaks through),
  literal normalisation (datatype defaulting per the RDF spec, no
  trimming of lexical forms), language-tag case normalisation to
  `BCP-47` lowercase-langtag-base + uppercase-region.
- `diff(a, b)` — set-diff surfacing `FactOnlyIn` + `ObjectMismatch` +
  `AcceptRejectSplit`. Must treat `Facts` built by different parsers as
  canonical-equal when they agree on the RDF abstract syntax, regardless
  of lexical surface.
- `diff_many` — N-way pairwise, collapsing majority agreement into a
  single divergence.
- Enforce the `NonCanonical` invariant at the front door of `diff` /
  `diff_many`.

## Acceptance

- `cargo test -p rdf-diff` green.
- `cargo clippy -p rdf-diff -- -D warnings` clean.
- No change to any `pub` signature currently in `lib.rs`. Add `pub`
  helpers only if the existing API cannot express a case; if you must,
  note the addition in a `CHANGES.md` at the crate root (not the
  workspace root).
- Doc comments on every new public item.

## Claims

Call `claims_claim` with `crates/testing/rdf-diff/**` before any edit
inside the crate. Release on completion.

## Memory

- Before starting: `memory_search` in `crate/rdf-diff` for prior facts.
- After canonical form is decided: `memory_store` a record at
  `canonical-form/decisions` in `crate/rdf-diff` describing the choice
  and citing the spec clause.
- Never read `verification-v1-adv` or `verification/adversary-findings`.

## Exit handoff

Call `claims_accept-handoff` against `v1-reviewer`.
