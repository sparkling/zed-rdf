# IRI adversary-corpus divergences (Phase A)

Owner: `phaseA-tester`. Scope: which `tests/adversary-iri/*` tests stay
`#[ignore]` at the end of Phase A and why. Reviewed by `phaseA-reviewer`
at handoff.

## Summary

All 9 per-fixture diff tests in
`crates/testing/rdf-diff/tests/adversary_iri.rs` remain ignored after
Phase A. None has been un-ignored because each depends on the main
parser pair landing and compiling:

- IRI-001, 002, 003a, 006 — need `rdf-turtle` (main) + `rdf-turtle-shadow`.
- IRI-003b, 004, 005, 007, 008 — need `rdf-ntriples` (main) + `rdf-ntriples-shadow`.

The structural always-on tests (`all_adversary_iri_fixtures_present`,
`adversary_iri_readme_present`, and the 8 per-fixture
`*_fixture_present` tests) continue to run on every
`cargo test --workspace`.

## Carry-over (deferred infrastructure)

`phaseA-rdf-turtle` had not completed when `phaseA-tester` ran:
`crates/rdf-turtle/src/lib.rs` was absent and the crate failed to
resolve, which in turn blocks `cargo test --workspace` from compiling.
`phaseA-rdf-iri` had landed source but `crates/rdf-iri/src/parse.rs`
triggered 40 E0658 errors (const-fn range-contains usage), so the crate
did not build either. Consequently neither of the following Phase-A
prerequisites was available when un-ignoring:

| Fixture  | Needs                                             | Status                                         |
|----------|---------------------------------------------------|------------------------------------------------|
| IRI-001  | `rdf-turtle` main parser                          | deferred — `phaseA-rdf-turtle` lib.rs missing  |
| IRI-002  | `rdf-turtle` main parser                          | deferred — same                                |
| IRI-003a | `rdf-turtle` main parser                          | deferred — same                                |
| IRI-003b | `rdf-ntriples` main parser + surrogate handling   | in-tree, ready to un-ignore once workspace builds |
| IRI-004  | `rdf-ntriples` main parser                        | in-tree, ready                                 |
| IRI-005  | `rdf-ntriples` main parser                        | in-tree, ready                                 |
| IRI-006  | `rdf-turtle` main parser                          | deferred                                       |
| IRI-007  | `rdf-ntriples` main parser                        | in-tree, ready                                 |
| IRI-008  | `rdf-ntriples` main parser + explicit NFC opt-out | in-tree, ready                                 |

Additionally, IDN / `ToASCII` beyond ASCII lowercasing is explicitly
deferred by the `rdf-iri` crate (see `docs/spec-readings/iri/
idna-host-normalisation-pin.md`). No IRI fixture in the current set
exercises IDN, but any future host-Unicode adversary (e.g. Punycode
round-trip) will remain ignored until the IDN pin lifts.

## Proposed un-ignore wave (post-Phase-A)

1. Sibling `phaseA-rdf-turtle` lands `src/lib.rs` exposing
   `TurtleParser` / `TriGParser` implementing `rdf_diff::Parser`.
2. Sibling `phaseA-rdf-iri` fixes the const-fn errors in
   `crates/rdf-iri/src/parse.rs` (rewrite `RangeInclusive::contains`
   calls as explicit comparisons in const context).
3. `cargo test --workspace` goes green.
4. A follow-up handoff walks each fixture above, drops the `#[ignore]`,
   and records any surviving real divergence here (section "Observed
   divergences" below).

## Observed divergences

_None yet._ Populated by the follow-up handoff once the main parsers
build and the diff harness can run the per-fixture assertions.

## References

- `docs/verification/adversary-findings/iri.md` — failure-mode brief.
- `docs/verification/tests/catalogue.md` — authoritative status table.
- `crates/testing/rdf-diff/tests/adversary_iri.rs` — test bodies.
- ADR-0019 §4 — adversary-corpus responsibilities.
- ADR-0020 §Validation — "zero divergences is suspicious".
