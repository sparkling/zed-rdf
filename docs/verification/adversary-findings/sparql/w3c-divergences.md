# SPARQL — W3C Manifest Divergences

Owner: `pa-w3c-vendor`. Extends `../sparql.md` with the W3C
manifest surface area now that `external/tests/sparql/` is populated.

## Current state (vendor-in, 2026-04-19)

- Corpus root: `external/tests/sparql/` → `w3c-rdf-tests/sparql/sparql11/`
- Files discovered: 1163 (of which 1156 are `.rq` / `.srx` / `.srj` /
  `.ru` / `.ttl` / `.nt` / manifest)
- Sub-manifests: 35 (one per SPARQL 1.1 feature area, plus
  `manifest-all.ttl` and the split `manifest-sparql11-{query,update,fed,results}.ttl`)
- Harness mode: **stub**; xtask path-dep deferred to ADR-0020 §5
- Real divergences: **0** (stub-mode artefact)
- xtask exit: `1` under the global fail-closed guard (gate is **not**
  load-bearing for SPARQL at Phase A exit)

## Deferred (Phase A → Phase C)

SPARQL is out of scope for the Phase A exit gate (ADR-0018 §4). Phase
C will land SPARQL 1.1 query + update *syntax* coverage; eval,
federation, entailment and HTTP protocol layers stay deferred:

- **In-scope later (Phase C):** `syntax-query/`, `syntax-update-1/`,
  `syntax-update-2/`, `syntax-fed/` (syntax only), plus the top-level
  `manifest-sparql11-query.ttl` / `manifest-sparql11-update.ttl`
  positive/negative syntax entries.
- **Deferred indefinitely:** `entailment/` (requires an RDFS / OWL
  reasoner), `protocol/` + `http-rdf-update/` (require an HTTP client
  + endpoint harness), `service-description/` (federation
  descriptions), `csv-tsv-res/` + `json-res/` (result-format
  canonicalisation beyond syntax).
- **Eval** suites under `add/`, `bind/`, `bindings/`, `construct/`,
  `exists/`, `grouping/`, `negation/`, `subquery/`, etc. are evaluation
  tests that need a full algebra evaluator; deferred until a SPARQL
  evaluator lands (not currently scheduled).

No hypotheses are recorded here yet — they will be captured when the
`sparql-syntax` parser is wired into `rdf-diff-oracles` as
`oxsparql_adapter` (ADR-0019 §1).
