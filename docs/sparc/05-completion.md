# SPARC-05 — Completion

> **Supersedes** the engine-scoped v1. Rewritten 2026-04-18.

"Done" means each crate meets the per-crate DoD, the conformance gate
is green, and the LSP integration harness passes across every language.

## 1. Definition of done (per crate)

1. **Rustdoc clean.** `#![deny(missing_docs)]` on every public crate.
   Every public item has a doc comment and, where useful, an example.
2. **Spec mapping.** Each parser crate's `SPEC.md` maps productions /
   features to clauses of the W3C spec (or an ADR that documents a
   deviation).
3. **Conformance.** Every applicable W3C manifest entry is green, or
   allow-listed with a linked justification.
4. **Unit + property tests.** `cargo-llvm-cov` ≥ 90 % line / ≥ 80 %
   branch on parser crates.
5. **Fuzz.** Every parser has a `cargo-fuzz` target; ≥ 24 h cumulative
   without unique crashes before release.
6. **Clippy clean.** `cargo clippy --all-targets --all-features -- -D
   warnings`.
7. **Safety.** No `unsafe` outside the allow-list in ADR-0001.
8. **No flakes.** Tests are deterministic.
9. **CHANGELOG entry.** Every user-visible change noted.

## 2. Conformance gate

| Module / crate          | Suite                                               | Gate   |
|-------------------------|-----------------------------------------------------|--------|
| `rdf-ntriples`          | `w3c/rdf-tests` → `nt`, `nq`                        | 100 %  |
| `rdf-turtle`            | `w3c/rdf-tests` → `turtle`, `trig`                  | 100 %  |
| `rdf-xml`               | `w3c/rdf-tests` → `rdf-xml`                         | 100 %  |
| `rdf-jsonld`            | `w3c/json-ld-api` *syntax + context* subset         | 100 %  |
| `rdf-trix`              | hand-written corpus (no W3C suite exists)           | fixture full pass |
| `rdf-n3`                | W3C Team Submission examples + TimBL's n3 repo corpus | fixture full pass |
| `sparql-syntax`         | `w3c/rdf-tests` → `sparql11` *syntax* manifests     | 100 %  |
| SPARQL 1.2 (tracking)   | community WG draft suite                            | tracked |
| `shex-syntax`           | `shexSpec/shexTest` syntax-only entries             | 100 %  |
| `datalog-syntax`        | curated fixture corpus                              | fixture full pass |

Test suites are git submodules under `external/tests/`, pinned by
commit.

## 3. LSP integration gate

End-to-end tests run `rdf-lsp` as a subprocess and speak LSP over stdio
via `lsp-client`:

- **didOpen / didChange / publishDiagnostics** on good + broken input per
  language.
- **hover** on every well-known vocabulary term in `rdf-vocab`.
- **completion** at representative positions: after `@prefix` keyword,
  after `:` to suggest local names, after `PREFIX` in SPARQL, in
  SPARQL `WHERE { ?s a ?… }` to propose `a`/`rdf:type`, after `sh:` to
  propose the `sh:` vocabulary, and so on.
- **goto-definition** on a prefixed name → its `@prefix` declaration;
  on a SPARQL variable → its first binding.
- **documentSymbol** tree sanity check.
- **formatting** round-trips parse (parse → format → parse gives
  identical facts).
- **rename** on a prefix / variable updates every occurrence in file.
- **codeAction** "declare missing prefix" wires up for every built-in
  vocabulary.
- **semanticTokens** produced and categorised correctly.

All of these are snapshot- or structural-assert tests in
`crates/testing/rdf-testsuite/`.

## 4. Performance gate

- Parse throughput targets in `04-refinement.md` §5 met on
  `bench/data/`.
- LSP cold-open target met on a 10 k-line Turtle fixture.
- Extension install-size (unpacked) under 5 MB (grammars are fetched at
  install time by Zed; we only ship `.scm` files and the extension
  Wasm).

## 5. Release criteria

### v0.1 — end of phase A

- N-Triples, N-Quads, Turtle, TriG parse + format.
- rdf-diagnostics + rdf-iri stable.
- CLI binary wrapping the parser-only API (for debugging the harness).
- Crates published.

### v0.5 — end of phase F

- All formats parseable; LSP core features live; Zed extension usable
  via `install dev extension`.

### v1.0 — end of phase I

- Every DoD met; conformance gates green; LSP integration gate green;
  performance gate met.
- Extension published to the Zed registry.
- `CHANGELOG.md`, `SECURITY.md`, `CONTRIBUTING.md` present.
- Semver policy in force (`cargo-semver-checks` in CI).

## 6. Post-1.0 candidates (tracked, not v1.0)

- Cross-file goto-definition (resolving imports / schema references).
- Schema-driven completion (e.g., propose properties from a loaded OWL
  ontology) — **would be a new workstream** because it requires loading
  triples, which today is out of scope. Requires an ADR if revisited.
- Snippet marketplace (share SHACL / SPARQL snippet packs).
- MCP context server exposing parser diagnostics to AI assistants.
- RDF 1.2 REC promotion once W3C finalises it.

Each of these gets its own ADR and, if accepted, appears in
`docs/roadmap.md`.
