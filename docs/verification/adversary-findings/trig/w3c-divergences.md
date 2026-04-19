# TriG — W3C Manifest Divergences

Owner: `pa-w3c-vendor`.

## Current state (vendor-in, 2026-04-19)

- Corpus root: `external/tests/trig/` → `w3c-rdf-tests/rdf/rdf11/rdf-trig/`
- Files discovered: 470 (of which 468 are `.trig` / `.nq` / related)
- Harness mode: **stub**; xtask path-dep deferred to ADR-0020 §5
- Real divergences: **0** (stub-mode artefact)
- xtask exit: `1` (fail-closed under ADR-0019 §Validation)

## Expected per-manifest findings (forward-looking)

TriG adds named-graph framing on top of Turtle. The integration pass
will exercise `rdf-turtle::TriGParser` against both positive and
negative syntax entries plus eval tests (TriG → N-Quads).

| entry shape                       | outcome       | hypothesis                                                  |
| --------------------------------- | ------------- | ----------------------------------------------------------- |
| `rdft:TestTrigPositiveSyntax`     | accept        | graph framing, default-graph semantics                      |
| `rdft:TestTrigNegativeSyntax`     | reject        | misplaced `{ }`, bad graph-name position                    |
| `rdft:TestTrigEval`               | accept + eval | Turtle-shaped payload + graph-name must round-trip to N-Quads |
| `rdft:TestTrigNegativeEval`       | eval-mismatch | semantic rejection                                          |

First-pass hypotheses:

- Default-graph statements outside `{ }` must merge with the unnamed
  graph produced from in-block statements — ordering-sensitive.
- Graph-name as blank-node label — scope vs the surrounding document.
- Inheritance of Turtle inconsistencies (see `../ttl/w3c-divergences.md`).

## Deferred

None — TriG is in Phase A scope.
