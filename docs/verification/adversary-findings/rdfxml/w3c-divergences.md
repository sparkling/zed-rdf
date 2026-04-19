# RDF/XML — W3C Manifest Divergences

Owner: `pa-w3c-vendor`.

## Current state (vendor-in, 2026-04-19)

- Corpus root: `external/tests/rdfxml/` → `w3c-rdf-tests/rdf/rdf11/rdf-xml/`
- Files discovered: 307 (of which 306 are `.rdf` / `.nt` / manifest)
- Harness mode: **stub**; no main RDF/XML parser landed yet
- Real divergences: **0**
- xtask exit: `1` under the global fail-closed guard (gate is **not**
  load-bearing for RDF/XML at Phase A exit)

## Deferred (Phase A → Phase B)

RDF/XML is **out of scope** for the Phase A exit gate (ADR-0018 §4
names N-Triples, N-Quads, Turtle, TriG only). The vendored suite is
present so downstream phases can pick it up without re-vendoring.

The suite is organised as subdirectories each with their own
`manifest.ttl` (e.g. `amp-in-url/`, `datatypes/`, `rdf-charmod-literals/`,
`rdf-charmod-uris/`, `rdfms-*`, `xmlbase/`, `xml-canon/`, ...). Most
entries are `rdft:TestXMLEval` — RDF/XML input must round-trip to the
reference N-Triples output.

No hypotheses are recorded here yet; they will be captured when the
`rdf-rdfxml` crate (Phase B) lands alongside its shadow adapter
`oxrdfxml_adapter` in `rdf-diff-oracles`.
