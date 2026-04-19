# N-Triples / N-Quads adversary-corpus divergences (Phase A)

Owner: `phaseA-tester`. Tracks which `tests/adversary-nt/*` fixtures
stay `#[ignore]` at the end of Phase A and why. Reviewed by
`phaseA-reviewer` at handoff.

## Summary

The test module `crates/testing/rdf-diff/tests/adversary_nt.rs` was
created by `phaseA-tester` during the Phase-A verification pass. It
catalogues all 7 failure modes from
`docs/verification/adversary-findings/nt.md` with 12 per-fixture
`#[test]` entries plus 3 always-on structural tests:

- `ant0_fixture_discovery_present_and_sorted`
- `ant0b_all_expected_fixtures_present`
- `ant0c_readme_present`

All 12 per-fixture diff tests remain ignored after Phase A.

## Carry-over (deferred infrastructure)

The main `rdf-ntriples` crate (`phaseA-rdf-ntriples`) **is** on disk
and compiles cleanly (`cargo check -p rdf-ntriples` is green). However
the workspace as a whole does not build because sibling Phase-A agents
(`phaseA-rdf-turtle`, `phaseA-rdf-iri`) had not completed when
`phaseA-tester` ran — see `../iri/divergences.md` for the matching
note. Since `cargo test --workspace` cannot complete, wiring
per-fixture `main` vs `shadow` diffs that compile in the workspace
build would just move the failure from one step to another; the
un-ignore bar is "test passes under the normal `cargo test
--workspace --all-features` invocation" and that bar cannot be met
today regardless of parser correctness.

The ignored tests therefore carry a `phaseA-tester` commentary and a
wiring sketch that the next handoff agent applies verbatim.

## Fixture-to-divergence hypotheses

From `docs/verification/adversary-findings/nt.md`:

| Fixture                                | FM | Hypothesised divergence                        |
|----------------------------------------|----|------------------------------------------------|
| `fm1-eol-bare-cr.nt`                   | 1  | AcceptRejectSplit (1.1 vs 1.2 draft EOL rules) |
| `fm1-eol-crlf.nt`                      | 1  | AcceptRejectSplit on miscounted lines          |
| `fm2-relative-iri-predicate.nt`        | 2  | AcceptRejectSplit — NT forbids relative IRIs   |
| `fm2-relative-iri-graph.nq`            | 2  | Same, in N-Quads graph slot                    |
| `fm3-unicode-escape-lower.nt`          | 3  | ObjectMismatch if escapes stored raw           |
| `fm3-unicode-escape-upper.nt`          | 3  | Same                                           |
| `fm4-bnode-dot-middle.nt`              | 4  | AcceptRejectSplit (greedy regex)               |
| `fm4-bnode-trailing-dot.nt`            | 4  | AcceptRejectSplit (must reject)                |
| `fm5-datatype-relative-iri.nt`         | 5  | AcceptRejectSplit — absolute IRI required      |
| `fm6-langtag-lowercase.nt`             | 6  | 1.1-vs-1.2 canonicalisation diff               |
| `fm6-langtag-uppercase.nt`             | 6  | ObjectMismatch under 1.1 parser                |
| `fm7-comment-no-final-newline.nt`      | 7  | AcceptRejectSplit if final EOL required        |

## Proposed un-ignore wave (post-Phase-A)

1. Sibling Phase-A agents finish landing so `cargo test --workspace
   --all-features` compiles.
2. A follow-up handoff adds a `dev-dependency` edge from `rdf-diff` (or
   a new test-scope crate) onto `rdf-ntriples` + `rdf-ntriples-shadow`,
   then walks each fixture above, replaces `todo!()` with the wiring
   sketch already in place in `adversary_nt.rs`, and un-ignores.
3. Any surviving real divergence is recorded in the "Observed
   divergences" section below with a reproducer.

## Observed divergences

_None yet._ This section is filled in by the un-ignore handoff.

## References

- `docs/verification/adversary-findings/nt.md` — failure-mode brief.
- `docs/verification/tests/catalogue.md` — authoritative status table.
- `crates/testing/rdf-diff/tests/adversary_nt.rs` — test module.
- ADR-0019 §4 — adversary-corpus responsibilities.
- ADR-0020 §Validation.
