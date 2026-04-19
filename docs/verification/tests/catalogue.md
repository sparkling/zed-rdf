# Verification-v1 test catalogue

Owner: `v1-tester` (cohort A). Written during the verification-v1 sweep
per ADR-0020 §3; reviewed by `v1-reviewer` at handoff per ADR-0006.

This catalogue maps each test to the invariant it protects and the
frozen-surface symbol(s) it exercises. New rows are added by follow-up
handoffs as shadow crates, adversary fixtures, and `xtask verify` land.

## Status conventions

- **active** — runs on every `cargo test --workspace`.
- **ignored-until-stubs** — compiles today, unignored once
  `v1-diff-core` fills `Facts::canonicalise` / `diff` / `diff_many`.
- **ignored-until-shadow** — unignored once both the matching shadow
  crate (`crates/syntax/*-shadow`) and the main parser (`crates/syntax/*`)
  exist.
- **ignored-until-xtask** — unignored once `v1-ci-wiring` lands
  `xtask/verify/**`.

## Properties (`crates/testing/rdf-diff/tests/properties.rs`)

| ID | Test                                   | Invariant                                                                                          | Status                 |
|----|----------------------------------------|----------------------------------------------------------------------------------------------------|------------------------|
| P1 | `prop_canonicalise_is_idempotent`      | `Facts::canonicalise(Facts::canonicalise(x)) == Facts::canonicalise(x)` (idempotence).             | ignored-until-stubs    |
| P2 | `prop_diff_self_is_clean`              | For every canonical `a`, `diff(a, a).unwrap().is_clean()`.                                         | ignored-until-stubs    |
| P3 | `prop_diff_commutative_at_set_level`   | Divergence-set equality: `set(diff(a, b)) == set(diff(b, a))` (list order free).                   | ignored-until-stubs    |
| P0 | `api_shape_compiles`                   | Frozen surface from ADR-0020 §1.4 remains constructible (`Fact`, `Facts`, `FactProvenance`).       | active                 |

Case count: `CASES = 256`. Seeded LCG (see module header); bump to
`16_384` on the nightly lane per ADR-0006 §Layers-2 when `proptest` is
introduced.

## Snapshots (`crates/testing/rdf-diff/tests/snapshots.rs`)

| ID | Test                                                  | Invariant                                                                                     | Status                  |
|----|-------------------------------------------------------|-----------------------------------------------------------------------------------------------|-------------------------|
| S1 | `snapshot_ntriples_shadow_vs_main_smoke`              | `diff(main.parse(x), shadow.parse(x)).is_clean()` on curated N-Triples smoke input.           | ignored-until-shadow    |
| S2 | `snapshot_turtle_shadow_vs_main_smoke`                | Same for Turtle (`SMOKE_TURTLE`).                                                             | ignored-until-shadow    |
| S3 | `snapshot_sparql_shadow_vs_main_smoke`                | Same for SPARQL 1.1 syntax (`SMOKE_SPARQL`).                                                  | ignored-until-shadow    |
| S4 | `snapshot_adversary_fixture_discovery_is_stable`      | Fixtures under `tests/adversary-<format>/` enumerate deterministically.                       | active                  |

The per-format adversary fixture roots —
`tests/adversary-ntriples/`, `tests/adversary-turtle/`,
`tests/adversary-iri/`, `tests/adversary-sparql/` — are **not** claimed
by this agent; cohort B (`v1-adv-*`) owns them per ADR-0020 §6.5.

## Integration (`crates/testing/rdf-diff/tests/xtask_verify.rs`)

| ID | Test                                                | Invariant                                                                                     | Status                |
|----|-----------------------------------------------------|-----------------------------------------------------------------------------------------------|-----------------------|
| I1 | `xtask_verify_smoke_corpus_green`                   | `cargo xtask verify --smoke` exits 0 on the smoke corpus.                                     | ignored-until-xtask   |
| I2 | `catalogue_is_discoverable_from_workspace_root`     | This very file is reachable at `docs/verification/tests/catalogue.md` from workspace root.         | active                |

## Coverage integration

`v1-ci-wiring` owns the `cargo-llvm-cov` target. When it lands, the
target invokes `cargo test --workspace -- --include-ignored` so that
P1-P3, S1-S3, and I1 contribute to the `rdf-diff` coverage number. The
target's thresholds follow ADR-0006 §Coverage for cross-cutting crates
(≥ 95 % line / ≥ 85 % branch).

## Forbidden inputs

Per `v1-tester`'s prompt: this agent does not read
`verification-v1-adv` memory, nor
`verification/adversary-findings`. Adversary-produced **fixture files**
under `tests/adversary-*/` are read through the filesystem only.

## Handoff

On completion: `claims_accept-handoff` → `v1-reviewer` for engineering
review per ADR-0006. Reviewer checklist:

1. `cargo test --workspace` green (verifies active rows).
2. `cargo test --workspace -- --include-ignored` expected to panic on
   any `todo!()`-backed body — that is the **expected** signal that
   `v1-diff-core` has not yet landed, not a test defect.
3. Rows transition from `ignored-until-*` to `active` via follow-up
   handoffs; each transition removes a single `#[ignore]` attribute.
