# Turtle / TriG adversary-corpus divergences (Phase A)

Owner: `phaseA-tester`. Tracks which `tests/adversary-ttl/*` fixtures
stay `#[ignore]` at the end of Phase A and why.

## Summary

All 13 per-fixture tests in
`crates/testing/rdf-diff/tests/adversary_ttl.rs` (`AT1`–`AT13`) were
un-ignored during the Phase-A pass once sibling `phaseA-rdf-turtle`
finished landing `crates/rdf-turtle/src/lib.rs` and the Phase-A
coordinator added a `[dev-dependencies]` edge from `rdf-diff` onto
`rdf-turtle`. All 15 tests in the module now run on every
`cargo test --workspace --all-features`.

`phaseA-tester` did **not** edit `adversary_ttl.rs` for the un-ignore
transition — the sibling coordinator had already applied it by the
time the pass completed. The module header now documents in-process
smoke-parsing; cross-parser diff (main vs shadow vs oxttl) continues
to live in `xtask verify`.

## Earlier carry-over (now cleared)

At the start of the Phase-A pass `crates/rdf-turtle/` had
`Cargo.toml`, `src/iri.rs`, `src/lexer.rs`, `src/grammar.rs`, and
`src/diag.rs`, but **no `src/lib.rs`** — so `cargo test -p rdf-turtle`
failed with "can't find lib rdf_turtle at path …" and `cargo test
--workspace` could not complete. That blocker lifted before handoff
and the un-ignore wave proceeded immediately.

## Fixture-to-hypothesis map

From `docs/verification/adversary-findings/ttl.md` (see the existing
wiring comments in `adversary_ttl.rs` for detail):

| Test ID | Fixture                                    | Hypothesis           |
|---------|--------------------------------------------|----------------------|
| AT1     | `fm1-leading-digit-local.ttl`              | AcceptRejectSplit    |
| AT2     | `fm1-prefix-redefinition.ttl`              | FactOnlyIn           |
| AT3     | `fm2-percent-encoding-local.ttl`           | ObjectMismatch       |
| AT4     | `fm3-keyword-scope.ttl`                    | AcceptRejectSplit    |
| AT5     | `fm4-empty-collection.ttl`                 | FactOnlyIn           |
| AT6     | `fm4-nested-collection.ttl`                | FactOnlyIn           |
| AT7     | `fm5-long-string-newline.ttl`              | AcceptRejectSplit    |
| AT8     | `fm5-short-string-newline-invalid.ttl`     | AcceptRejectSplit    |
| AT9     | `fm6-base-directive-replacement.ttl`       | FactOnlyIn           |
| AT10    | `fm6-chained-base.ttl`                     | FactOnlyIn           |
| AT11    | `fm7-trailing-semicolon.ttl`               | AcceptRejectSplit    |
| AT12    | `fm8-trig-bnode-scope.trig`                | FactOnlyIn           |
| AT13    | `fm9-numeric-literal-types.ttl`            | ObjectMismatch       |

## Un-ignore wave (complete)

1. `phaseA-rdf-turtle` lands `src/lib.rs`. ✓
2. `[dev-dependencies]` edge added from `rdf-diff` onto `rdf-turtle`. ✓
3. Each AT fixture rewrites to `parse_ttl_expect_ok` /
   `parse_trig_expect_ok` + explicit fact-set assertions. ✓
4. No test currently fails → zero observed divergences. This is
   acceptable today because the tests exercise the main parser in
   isolation; the diff-vs-shadow signal remains the job of
   `xtask verify`.

## Observed divergences

_None yet._ The in-process tests are main-only; divergences surface
in `xtask verify` output once `rdf-diff-oracles` + shadow-feature
enablement ship for the harness.

## References

- `docs/verification/adversary-findings/ttl.md` — failure-mode brief.
- `docs/verification/tests/catalogue.md` — authoritative status table.
- `crates/testing/rdf-diff/tests/adversary_ttl.rs` — test module.
- ADR-0019 §4 — adversary-corpus responsibilities.
