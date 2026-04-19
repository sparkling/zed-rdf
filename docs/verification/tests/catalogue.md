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

| ID  | Test                                    | Invariant                                                                                              | Status |
|-----|-----------------------------------------|--------------------------------------------------------------------------------------------------------|--------|
| P0  | `api_shape_compiles`                    | Frozen surface from ADR-0020 §1.4 remains constructible (`Fact`, `Facts`, `FactProvenance`).           | active |
| P1  | `prop_canonicalise_is_idempotent`       | `Facts::canonicalise(Facts::canonicalise(x)) == Facts::canonicalise(x)` (idempotence).                 | active |
| P2  | `prop_diff_self_is_clean`               | For every canonical `a`, `diff(a, a).unwrap().is_clean()`.                                             | active |
| P3  | `prop_diff_commutative_at_set_level`    | Divergence-set equality: `set(diff(a, b)) == set(diff(b, a))` (list order free).                       | active |
| P4  | `prop_canonicalise_bounds_cardinality`  | `|canonicalise(raw).set| ≤ |raw|`; doubling a raw input does not grow the canonical set.               | active |
| P5  | `prop_canonicalise_order_insensitive`   | Deterministic-shuffle of a raw input leaves the canonical fact-key set and `diff` clean.               | active |
| P6  | `prop_absolute_iri_wrap_is_idempotent`  | Bare and angle-wrapped absolute IRIs canonicalise to the same `Facts`.                                 | active |
| P6b | `prop_canonical_facts_self_round_trip`  | Feeding a canonical `Facts` back through `canonicalise` returns the same set (Facts-layer round-trip). | active |

Case count: `CASES = 256`. Seeded LCG (see module header); bump to
`16_384` on the nightly lane per ADR-0006 §Layers-2 when `proptest` is
introduced. `phaseA-tester` un-ignored P1–P3 once `Facts::canonicalise`
and `diff` were filled by the `v1-diff-core` pass, and added P4–P6b.

## Snapshots (`crates/testing/rdf-diff/tests/snapshots.rs`)

| ID | Test                                                  | Invariant                                                                                     | Status                                          |
|----|-------------------------------------------------------|-----------------------------------------------------------------------------------------------|-------------------------------------------------|
| S1 | `snapshot_ntriples_shadow_vs_main_smoke`              | `diff(main.parse(x), shadow.parse(x)).is_clean()` on curated N-Triples smoke input.           | active (self-diff; upgraded to main-vs-shadow post Phase A) |
| S2 | `snapshot_turtle_shadow_vs_main_smoke`                | Same for Turtle (`SMOKE_TURTLE`).                                                             | active (self-diff; upgraded post Phase A)       |
| S3 | `snapshot_sparql_shadow_vs_main_smoke`                | Same for SPARQL 1.1 syntax (`SMOKE_SPARQL`).                                                  | active (self-diff; upgraded post Phase A)       |
| S4 | `snapshot_adversary_fixture_discovery_is_stable`      | Fixtures under `tests/adversary-<format>/` enumerate deterministically.                       | active                                          |

The per-format adversary fixture roots —
`tests/adversary-ntriples/`, `tests/adversary-turtle/`,
`tests/adversary-iri/`, `tests/adversary-sparql/` — are **not** claimed
by this agent; cohort B (`v1-adv-*`) owns them per ADR-0020 §6.5.

## Integration (`crates/testing/rdf-diff/tests/xtask_verify.rs`)

| ID | Test                                                | Invariant                                                                                     | Status |
|----|-----------------------------------------------------|-----------------------------------------------------------------------------------------------|--------|
| I1 | `xtask_verify_smoke_corpus_green`                   | `cargo xtask verify --smoke` exits 0 on the smoke corpus.                                     | active |
| I2 | `catalogue_is_discoverable_from_workspace_root`     | This very file is reachable at `docs/verification/tests/catalogue.md` from workspace root.    | active |

## Adversary — N-Triples / N-Quads (`crates/testing/rdf-diff/tests/adversary_nt.rs`)

Created by `phaseA-tester`. Structural always-on rows first, then one
per-FM row (ignored until the main `rdf-ntriples` parser integration
lands).

| ID     | Test                                         | Covers                                        | Status                                  |
|--------|----------------------------------------------|-----------------------------------------------|-----------------------------------------|
| ANT0   | `ant0_fixture_discovery_present_and_sorted`  | Directory present + sorted; ≥14 files.        | active                                  |
| ANT0b  | `ant0b_all_expected_fixtures_present`        | All 12 `(input, expected)` pairs on disk.     | active                                  |
| ANT0c  | `ant0c_readme_present`                       | `adversary-nt/README.md` on disk.             | active                                  |
| ANT1a  | `ant1a_fm1_eol_bare_cr`                      | FM1 — bare CR as line terminator.             | ignored — awaits `rdf-ntriples` main    |
| ANT1b  | `ant1b_fm1_eol_crlf`                         | FM1 — CRLF as single EOL.                     | ignored — awaits `rdf-ntriples` main    |
| ANT2a  | `ant2a_fm2_relative_iri_predicate_rejected`  | FM2 — relative IRI in predicate slot.         | ignored — awaits `rdf-ntriples` main    |
| ANT2b  | `ant2b_fm2_relative_iri_graph_rejected`      | FM2 — relative IRI in N-Quads graph slot.     | ignored — awaits `rdf-ntriples` main    |
| ANT3a  | `ant3a_fm3_unicode_escape_case_lower`        | FM3 — `\u00e9` decodes to U+00E9.             | ignored — awaits `rdf-ntriples` main    |
| ANT3b  | `ant3b_fm3_unicode_escape_case_upper`        | FM3 — `\u00E9` decodes to U+00E9.             | ignored — awaits `rdf-ntriples` main    |
| ANT4a  | `ant4a_fm4_bnode_dot_middle_accepted`        | FM4 — `_:b.1` is valid.                       | ignored — awaits `rdf-ntriples` main    |
| ANT4b  | `ant4b_fm4_bnode_trailing_dot_rejected`      | FM4 — `_:b1.` must be rejected.               | ignored — awaits `rdf-ntriples` main    |
| ANT5   | `ant5_fm5_datatype_relative_iri_rejected`    | FM5 — datatype IRI absoluteness.              | ignored — awaits `rdf-ntriples` main    |
| ANT6a  | `ant6a_fm6_langtag_lowercase`                | FM6 — lowercase language tag.                 | ignored — awaits `rdf-ntriples` main    |
| ANT6b  | `ant6b_fm6_langtag_uppercase`                | FM6 — uppercase language tag (1.1 vs 1.2).    | ignored — awaits `rdf-ntriples` main    |
| ANT7   | `ant7_fm7_comment_no_final_newline_accepted` | FM7 — optional trailing EOL.                  | ignored — awaits `rdf-ntriples` main    |

Deferred-feature note: see
`docs/verification/adversary-findings/nt/divergences.md` for the
fixture-by-fixture carry-over table and the proposed un-ignore wave.

## Un-ignore ledger (phaseA-tester delta)

Counts recorded as (before, after) for each test binary:

- `properties.rs` — (1 active, 3 ignored) → (8 active, 0 ignored). Δ = +3 un-ignored, +4 new invariants.
- `snapshots.rs` — (1 active, 3 ignored) → (4 active, 0 ignored). Δ = +3 un-ignored (self-diff mode).
- `xtask_verify.rs` — (1 active, 1 ignored) → (2 active, 0 ignored). Δ = +1 un-ignored.
- `adversary_ttl.rs` — (2 active, 13 ignored) → (15 active, 0 ignored). Δ = +13 un-ignored; wired against `rdf-turtle` main via a `[dev-dependencies]` edge on `rdf-diff`. (Landed mid-handoff by the Phase-A sibling pair; phaseA-tester observed the delta and recorded it here.)
- `adversary_iri.rs` — unchanged (11 active, 9 ignored). Carry-over: needs a sibling `[dev-dependencies]` edge onto `rdf-iri` + `rdf-turtle` to wire in the same pattern; see `adversary-findings/iri/divergences.md`.
- `adversary_sparql.rs` — unchanged (5 active, 13 ignored); SPARQL main parser is not in Phase A.
- `adversary_nt.rs` — new file (3 active, 12 ignored). Carry-over: needs `[dev-dependencies]` edge onto `rdf-ntriples` to un-ignore; see `adversary-findings/nt/divergences.md`.

Mirror key: `verification/tests/catalogue` in the workspace memory
namespace (see `phaseA-tester` report for the stored JSON).

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
