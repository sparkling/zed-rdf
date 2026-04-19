# N-Triples — W3C Manifest Divergences

Owner: `pa-w3c-vendor`. Captures divergences surfaced by running the
`xtask verify` harness against the vendored W3C `rdf-n-triples` suite
(see `external/tests/PINS.md`).

## Current state (vendor-in, 2026-04-19)

`cargo run -p xtask -- verify` discovers the vendored suite at
`external/tests/nt/` (74 entries, 73 test/manifest files; pinned W3C
`rdf-tests` SHA `1d6be01`) and walks every file into the corpus list.

The harness itself is still in **stub mode** — per
`xtask/verify/src/main.rs` the `rdf-diff-oracles` registry is present
but xtask's path-dependency on it is deferred to ADR-0020 §5's
integration pass. As a consequence:

- `corpora_count = 73` in `target/verification-reports/diff-report-nt.json`
- `divergences = 0`
- `stub_reason` field is populated (non-`null`), so the reader can
  distinguish "ran clean" from "did not run"
- xtask exits `1` under the ADR-0019 §Validation fail-closed guard
  (zero divergences on a non-smoke run is treated as suspicious)

**No real parser-vs-oracle diffs have been produced yet**; the list
below is the expected surface area once the integration pass lands.

## Expected per-manifest findings (forward-looking)

The upstream manifest declares the entries below. Each will be fed to
`rdf-ntriples::NTriplesParser` and the shadow oracle (`oxttl`) in the
integration pass.

| test-id prefix                 | input                      | expected outcome  | hypothesis                                  |
| ------------------------------ | -------------------------- | ----------------- | ------------------------------------------- |
| `nt-syntax-file-*`             | empty / whitespace-only    | positive (accept) | smoke — no divergence expected              |
| `nt-syntax-uri-*`              | absolute IRI subjects      | positive          | cross-check with `rdf-iri` IDN/URN handling |
| `nt-syntax-string-*`           | string literal forms       | positive          | `\u` / `\U` escape coverage                 |
| `nt-syntax-str-esc-*`          | escape sequences           | positive          | carry-over from `../nt/divergences.md`      |
| `nt-syntax-bnode-*`            | blank-node forms           | positive          | blank-node scope per document               |
| `nt-syntax-datatypes-*`        | datatype IRIs              | positive          | smoke                                       |
| `nt-syntax-bad-uri-*`          | malformed IRIs             | **negative**      | overlaps with `adversary_iri` fixtures      |
| `nt-syntax-bad-string-*`       | unterminated strings       | **negative**      |                                             |
| `nt-syntax-bad-lang-*`         | malformed language tags    | **negative**      |                                             |

Suites here are exclusively "positive / negative syntax" — there is no
eval step; output need only parity-match accept/reject with the
oracle.

## Deferred

None — the N-Triples suite is fully in Phase A scope (ADR-0018 §4).
Any divergence surfaced here after the integration pass is a
parser-correctness bug.
