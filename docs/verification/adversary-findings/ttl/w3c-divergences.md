# Turtle — W3C Manifest Divergences

Owner: `pa-w3c-vendor`. Captures divergences surfaced by running the
`xtask verify` harness against the vendored W3C `rdf-turtle` suite
(see `external/tests/PINS.md`).

## Current state (vendor-in, 2026-04-19)

- Corpus root: `external/tests/ttl/` → `w3c-rdf-tests/rdf/rdf11/rdf-turtle/`
- Files discovered: 433 (of which 431 are `.ttl` / `.nt` / related)
- Harness mode: **stub** (`rdf-diff-oracles` registry present, xtask
  path-dep deferred to ADR-0020 §5)
- Real divergences produced: **0** (stub-mode side-effect)
- xtask exit code: `1` (fail-closed under ADR-0019 §Validation —
  zero-divergence on a non-smoke run is suspicious)

## Expected per-manifest findings (forward-looking)

The Turtle manifest mixes three outcome classes. The integration pass
will exercise `rdf-turtle::TurtleParser` against each and diff against
the `oxttl` shadow + reference N-Triples eval output.

| entry shape                       | count (approx) | outcome       | hypothesis                                           |
| --------------------------------- | -------------- | ------------- | ---------------------------------------------------- |
| `rdft:TestTurtlePositiveSyntax`   | ~130           | accept        | covers prefixes, IRI forms, literal forms, bnodes    |
| `rdft:TestTurtleNegativeSyntax`   | ~65            | reject        | overlaps with `docs/verification/adversary-findings/ttl.md` hypotheses |
| `rdft:TestTurtleEval`             | ~140           | accept + eval | Turtle input must round-trip to provided N-Triples   |
| `rdft:TestTurtleNegativeEval`     | ~5             | eval-mismatch | semantic rejection                                   |

First-pass hypotheses for likely divergences (to be confirmed when the
integration pass runs):

- IRI relative-reference resolution (`@base`, `@prefix` interaction)
  — overlap with `../iri/divergences.md` fixtures.
- PN_LOCAL grammar edge cases (percent-encoded local parts).
- Numeric literal canonicalisation: `1.0e0` vs `1.0E0` when emitting
  N-Triples for eval.
- Blank-node labelling across separate bodies inside the same file.

## Deferred

None — Turtle is in Phase A scope (ADR-0018 §4). Any divergence
surfaced post-integration is a parser-correctness bug.
