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

## Triage log (phase-a-triage / pat-ntriples, 2026-04-19)

The `xtask verify` integration pass landed and surfaced two distinct
divergences against the W3C `rdf-n-triples` suite (each counted twice
in `diff-report-nt.json` because both the `rdf11` and `rdf12`
manifests include the same fixture):

### nt-syntax-bad-bnode-01

- **Test-id:** `nt-syntax-bad-bnode-01` (from both `rdf11` and `rdf12`
  manifests).
- **Fixture:** `external/tests/nt/nt-syntax-bad-bnode-01.nt` — input
  `_::a  <http://example/p> <http://example/o> .`.
- **Kind:** `TestNTriplesNegativeSyntax`. W3C manifest comment:
  "Colon in bnode label not allowed (negative test)".
- **Pre-fix report:** `AcceptRejectSplit — expected-reject but
  accepted`.
- **Root cause:** `is_pn_chars_u` in `crates/rdf-ntriples/src/lib.rs`
  treated `:` as a member of `PN_CHARS_U`. That definition is
  correct for **Turtle** (where prefixed names need it) but wrong
  for N-Triples / N-Quads — §2.3 of RDF 1.1 N-Triples defines
  `PN_CHARS_U ::= PN_CHARS_BASE | '_'`. The parser therefore
  accepted `_::a` because the first post-`_:` character `':'` was
  wrongly classified as a valid label starter.
- **Classification:** **Parser bug.**
- **Action taken:**
  1. Narrowed `is_pn_chars_u` to `PN_CHARS_BASE | '_'` — commented
     with the W3C §2.3 citation.
  2. Added regression tests
     `tests::bnode_label_first_char_colon_rejected` and
     `tests::nquads_bnode_label_first_char_colon_rejected`.
  3. Extended the `blank-node-labels.md` pin with an explicit
     colon-exclusion sub-clause (NT-BN-002) and a cross-reference to
     the W3C negative tests.
- **Status after fix:** `xtask verify` reports zero divergence;
  the parser now emits `NT-BN-002: illegal first character in
  blank-node label at byte offset 2`.

### nt-syntax-bad-bnode-02

- **Test-id:** `nt-syntax-bad-bnode-02` (both manifests).
- **Fixture:** `external/tests/nt/nt-syntax-bad-bnode-02.nt` — input
  `_:abc:def  <http://example/p> <http://example/o> .`.
- **Kind:** `TestNTriplesNegativeSyntax`. W3C manifest comment:
  "Colon in bnode label not allowed (negative test)".
- **Pre-fix report:** `AcceptRejectSplit — expected-reject but
  accepted`.
- **Root cause:** Same root cause as `bad-bnode-01` via the
  transitive inclusion `is_pn_chars = is_pn_chars_u | …`. The
  interior `:` was greedily absorbed into the label.
- **Classification:** **Parser bug (same fix).**
- **Action taken:** Same single-line change to `is_pn_chars_u` as
  above. Added regression tests
  `tests::bnode_label_interior_colon_rejected` and
  `tests::nquads_bnode_label_interior_colon_rejected`.
- **Status after fix:** the label lexer now stops at `abc`; the
  trailing `:def` is then rejected by the statement parser as an
  unexpected character before the predicate (`NT-STMT-*` family).

### Summary

- Divergences closed: 2 distinct (4 manifest counts).
- Classification: 2 parser bugs / 0 allow-list entries / 0 new pins
  authored (existing pin `blank-node-labels.md` clarified).
- Files touched: `crates/rdf-ntriples/src/lib.rs`,
  `docs/spec-readings/ntriples/blank-node-labels.md`.
- Regression tests added: 4 (2 for NT, 2 for NQ).
