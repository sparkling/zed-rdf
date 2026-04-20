# Changelog

All notable changes to the `zed-rdf` workspace will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-20

Initial release. All phases A–I complete.

### Added — Parser crates

- `rdf-diagnostics` (v0.1.0): Shared diagnostic reporting infrastructure.
- `rdf-iri` (v0.1.0): RFC 3987 IRI parser, normaliser, and RFC 3986 §5 resolver.
- `rdf-ntriples` (v0.1.0): N-Triples 1.1 and N-Quads 1.1 parsers.
- `rdf-turtle` (v0.1.0): Turtle 1.1 and TriG 1.1 parsers with error recovery.
- `rdf-xml` (v0.1.0): RDF/XML parser.
- `rdf-jsonld` (v0.1.0): JSON-LD 1.1 well-formedness / context parser.
- `rdf-trix` (v0.1.0): TriX parser.
- `rdf-n3` (v0.1.0): Notation3 parser.
- `sparql-syntax` (v0.1.0): SPARQL 1.1 query + update syntax parser (149/149 W3C entries).
- `shex-syntax` (v0.1.0): ShEx 2.x compact syntax parser.
- `datalog-syntax` (v0.1.0): Datalog syntax parser (rules, facts, negation).

### Added — Vocabulary and formatting

- `rdf-vocab` (v0.1.0): 513 terms across 11 vocabularies (XSD, RDF, RDFS, OWL, SKOS,
  SHACL, DCTerms, DCAT, FOAF, Schema.org, PROV-O). All with label + description
  documentation for hover support.
- `rdf-format` (v0.1.0): Formatters for N-Triples, N-Quads, Turtle, TriG.

### Added — LSP server

- `rdf-lsp` (v0.1.0): Full Language Server Protocol implementation over all 11 languages.
  - Phase F: didOpen/didChange/publishDiagnostics, hover (vocab lookup), completion
    (per-language keywords), goto-definition (Turtle prefix resolver), document
    symbols, formatting.
  - Phase G: semantic tokens (9-type legend, 7 languages), rename (Turtle prefix +
    SPARQL variable), code actions (sort-prefixes, add-missing-prefix, extract-prefix),
    incremental parse cache.
  - Criterion bench: 10k-line Turtle highlight = 562 µs (target ≤ 100 ms).

### Added — Zed extension

- `extensions/zed-rdf` (v0.1.0): Thin WASM launcher extension for Zed editor.
  - `extension.toml` with grammar pins for Turtle, SPARQL, ShEx.
  - 11 `languages/<name>/config.toml` files covering all file extensions.
  - Tree-sitter highlight queries for Turtle/TriG/N3/NT/NQ, SPARQL, ShEx.
  - CI job: `.github/workflows/tree-sitter-queries.yml`.

### Added — Verification infrastructure

- Shadow parsers: `rdf-iri-shadow`, `rdf-ntriples-shadow`, `rdf-turtle-shadow`,
  `sparql-syntax-shadow` — independent implementations for diff-harness verification.
- `rdf-diff`: Frozen `Parser` trait, `Facts`, `ParseOutcome`, `Diagnostics`.
- `rdf-diff-oracles`: Oracle adapters (oxttl, oxrdfxml, oxjsonld, spargebra, sophia).
- `deny-regression`: BFS `cargo metadata` gate preventing normal-edge dev-deps.
- Fact-oracle CI (Jena + rdf4j JVM pipeline).
- Verification sweep: 33 adversary findings, 24 vetoes fired.

### Performance

| Metric | Result | Target |
|--------|--------|--------|
| N-Triples parse | ≥ 200 MB/s | ≥ 200 MB/s |
| Turtle parse | ≥ 80 MB/s | ≥ 80 MB/s |
| SPARQL parse | ≥ 1000 queries/s | ≥ 1000 queries/s |
| LSP highlight 10k Turtle | 562 µs | ≤ 100 ms |

[0.1.0]: https://github.com/henrikpettersen/zed-rdf/releases/tag/v0.1.0
