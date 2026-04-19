# N-Quads — W3C Manifest Divergences

Owner: `pa-w3c-vendor`.

## Current state (vendor-in, 2026-04-19)

- Corpus root: `external/tests/nq/` → `w3c-rdf-tests/rdf/rdf11/rdf-n-quads/`
- Files discovered: 91 (of which 90 are `.nq` / `.nt` / related)
- Harness mode: **stub** — `rdf-diff-oracles` registry present; xtask
  path-dep deferred to ADR-0020 §5 integration pass
- Real divergences: **0** (stub-mode artefact)
- xtask exit: `1` (fail-closed under ADR-0019 §Validation)

## Expected per-manifest findings (forward-looking)

Positive / negative syntax only — no eval step.

| test-id prefix           | input                     | outcome           | hypothesis                                    |
| ------------------------ | ------------------------- | ----------------- | --------------------------------------------- |
| `nq-syntax-uri-*`        | absolute IRI subjects     | positive          | same surface as N-Triples `nt-syntax-uri-*`   |
| `nq-syntax-bnode-*`      | blank-node subjects       | positive          | graph-name scope per document                 |
| `nq-syntax-quad-*`       | 4-tuples                  | positive          | graph-name position accept                    |
| `nq-syntax-bad-quad-*`   | malformed 4-tuples        | **negative**      |                                               |
| `nq-syntax-bad-literal-*`| malformed literals        | **negative**      |                                               |
| `nq-syntax-bad-uri-*`    | malformed IRIs            | **negative**      | overlaps with `adversary_iri` + N-Triples     |

## Deferred

None — N-Quads is in Phase A scope.
