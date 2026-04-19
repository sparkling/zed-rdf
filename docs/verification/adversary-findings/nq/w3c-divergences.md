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

## Triage log (phase-a-triage / pat-ntriples, 2026-04-19)

The `xtask verify` integration pass surfaced the N-Triples
`bad-bnode-01` / `bad-bnode-02` fixtures a second time inside the
N-Quads corpus (N-Quads vendors the same negative tests verbatim —
see `external/tests/nq/nt-syntax-bad-bnode-0*.nq`). Each was counted
twice (`rdf11` + `rdf12` manifests), giving 4 divergence rows in
`diff-report-nq.json`.

Both share the **same root cause and fix** as the N-Triples triage —
see `docs/verification/adversary-findings/nt/w3c-divergences.md` for
the full technical breakdown. Cross-references below.

### nt-syntax-bad-bnode-01 (via NQ corpus)

- **Test-id:** `nt-syntax-bad-bnode-01` (re-run under NQuadsParser).
- **Fixture:** `external/tests/nq/nt-syntax-bad-bnode-01.nq` — input
  `_::a  <http://example/p> <http://example/o> .`. Byte-identical to
  the NT fixture.
- **Pre-fix report:** `AcceptRejectSplit — expected-reject but
  accepted`.
- **Classification:** **Parser bug** (shared with NT — single
  `is_pn_chars_u` function drives both parsers via `Mode::{NTriples,
  NQuads}`).
- **Action taken:** Same fix to `is_pn_chars_u` in
  `crates/rdf-ntriples/src/lib.rs` closes both NT and NQ cases.
  Regression test `tests::nquads_bnode_label_first_char_colon_rejected`
  locks the NQ path in.

### nt-syntax-bad-bnode-02 (via NQ corpus)

- **Test-id:** `nt-syntax-bad-bnode-02` (re-run under NQuadsParser).
- **Fixture:** `external/tests/nq/nt-syntax-bad-bnode-02.nq` — input
  `_:abc:def  <http://example/p> <http://example/o> .`.
- **Pre-fix report:** `AcceptRejectSplit — expected-reject but
  accepted`.
- **Classification:** **Parser bug** (shared fix).
- **Action taken:** Same fix. Regression test
  `tests::nquads_bnode_label_interior_colon_rejected`.

### Summary

- Divergences closed: 2 distinct (4 manifest counts).
- Classification: 2 parser bugs (shared with NT) / 0 allow-list / 0
  new pins.
- Regression tests added: 2 NQ-specific (plus 2 NT-side — see NT
  triage doc).
