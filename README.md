# zed-rdf

A Zed extension and Rust LSP for the RDF family of languages:
Turtle, TriG, N-Triples, N-Quads, RDF/XML, JSON-LD, TriX, Notation3,
SPARQL, SHACL (shapes in Turtle), ShEx, and Datalog.

The extension provides tree-sitter-based highlighting, outlines,
indentation, folding, and injections. The LSP provides error-tolerant
parsing with syntactic diagnostics, hover on well-known vocabulary
(`xsd:`, `rdf:`, `rdfs:`, `owl:`, `skos:`, `sh:`, `dcterms:`, `foaf:`,
…), completion, in-file goto-definition, document symbols, rename,
deterministic formatting, code actions, and semantic tokens.

## What it is *not*

No triple store, no SPARQL query execution, no RDFS/OWL/SHACL/ShEx
reasoning or validation, no Datalog evaluation, no endpoint or network
calls. This is an editor-only language tool. See
[`docs/sparc/01-specification.md`](docs/sparc/01-specification.md) §2
for the full scope boundary.

## Status

**Pre-alpha.** Scope, architecture, and foundational ADRs are in
[`docs/sparc/`](docs/sparc/) and [`docs/adr/`](docs/adr/). No crates
yet; phase A kicks off next.

Start reading at
[`docs/sparc/01-specification.md`](docs/sparc/01-specification.md).

## Building

Will be documented once the first crates land. Requires:

- Rust **stable** (see [ADR-0001](docs/adr/0001-rust-toolchain.md) — no
  MSRV commitment; we build on whatever stable is current).
- `wasm32-wasip2` target for the Zed extension
  (`rustup target add wasm32-wasip2`).

## Licence

Dual-licensed under **Apache-2.0 OR MIT**. See
[`LICENSE-APACHE`](LICENSE-APACHE) and [`LICENSE-MIT`](LICENSE-MIT).
Contributions are accepted under the same terms — see
[`CONTRIBUTING.md`](CONTRIBUTING.md).
