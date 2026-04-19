# SPARC-01 — Specification

> **Supersedes** the engine-scoped v1 of this document. Scope clarified
> 2026-04-18: we are building **a Zed language extension + Rust LSP** for
> the RDF family, not a full RDF/SPARQL/Datalog engine. §8
> scope-decision session resolved 2026-04-18.

## 1. Mission

Ship a Zed extension that gives the **RDF family** of languages first-class
editor support in Zed:

- **Serialisation formats**: N-Triples, N-Quads, Turtle, TriG, RDF/XML,
  JSON-LD 1.1, TriX, Notation3.
- **Query**: SPARQL 1.1 + 1.2 (latest CR) by default.
- **Shapes**: SHACL (shapes in Turtle, recognised via `sh:` vocab); ShEx
  (ShExC + ShExJ).
- **Rules**: Datalog (surface syntax only).

"First-class" means:

- Tree-sitter-based **highlighting, outline, bracket matching, indentation,
  folding, language injections, text objects, runnable markers, semantic
  tokens**.
- A Rust **Language Server** providing:
  - error-tolerant parsing with **syntactic diagnostics**,
  - **hover** for well-known RDF vocabulary (`xsd:`, `rdf:`, `rdfs:`,
    `owl:`, `skos:`, `sh:`, `dcterms:`, `foaf:`, …),
  - **completion** — keywords, prefixes, snippets, and well-known
    vocabulary terms (no user-data-driven or ML completion),
  - **goto-definition** scoped to the current file (prefix → `@prefix`
    declaration; SPARQL variable → first binding),
  - **document / workspace symbols** (in-file only),
  - **document formatting** (deterministic pretty-print per format),
  - **rename** (in-file only — prefixes, SPARQL variables, blank nodes),
  - **code actions** for common fixes (declare missing prefix, expand
    prefixed-name, add `@base`, etc.),
  - **semantic tokens**.

## 2. Hard scope boundaries

We explicitly **do not** build:

- a triple or quad store,
- a SPARQL query executor or protocol server,
- RDFS / OWL / SHACL / ShEx **reasoning** or **validation**,
- a Datalog evaluator,
- remote endpoint integration (no SPARQL over HTTP, no Linked Data
  fetching, no schema introspection),
- ML / corpus-trained predictive text; completion is driven by the spec
  grammar plus a static vocabulary file only.

If a feature would require running any of the above, it is out of scope
for v1.0.

## 3. Users

| Audience             | What they do in Zed                                                       |
|----------------------|---------------------------------------------------------------------------|
| Ontologists          | Author Turtle, SHACL shapes, SKOS vocabularies with highlighting, hover, and syntactic validation. |
| Knowledge engineers  | Write SPARQL with prefix completion, variable rename, snippet-driven query scaffolding. |
| Researchers          | Read and edit N3 and Datalog rule sets with syntax support.              |

## 4. Target standards (parse-and-highlight only)

### 4.1 Serialisation

RDF 1.2 / RDF-star is **default on** at the parser level. Users who want
to lock a parser to RDF 1.1 semantics enable the `rdf-1-1-strict` Cargo
feature on the relevant crate.

| Format     | Spec                                  | RDF 1.2 / RDF-star                       |
|------------|---------------------------------------|------------------------------------------|
| N-Triples  | RDF 1.1 + 1.2 N-Triples               | default on; `rdf-1-1-strict` opt-out     |
| N-Quads    | RDF 1.1 + 1.2 N-Quads                 | default on; `rdf-1-1-strict` opt-out     |
| Turtle     | RDF 1.1 + 1.2 Turtle                  | default on; `rdf-1-1-strict` opt-out     |
| TriG       | RDF 1.1 + 1.2 TriG                    | default on; `rdf-1-1-strict` opt-out     |
| RDF/XML    | RDF 1.1 XML Syntax                    | n/a (RDF/XML has no RDF-star form)       |
| JSON-LD    | JSON-LD 1.1 (REC 2020) — **syntax + context well-formedness only; no expand/compact** | 1.2 context keywords default on; `rdf-1-1-strict` opt-out |
| TriX       | HP TriX submission                    | n/a                                      |
| N3         | W3C Team Submission — **parse only; no reasoning, no built-ins** | n/a                                      |

Because RDF 1.2 is still in WG status as of 2026-04, the latest CR is
tracked and the CHANGELOG notes each re-pin. Breaking spec changes bump
the parser crate's minor version pre-1.0 and major version post-1.0.

### 4.2 Query

- **SPARQL 1.1 Query + Update** (REC 2013) — full syntax parse.
- **SPARQL 1.2** (latest CR) — **default on**; additive syntax
  (triple-term patterns, `VERSION`, `LATERAL`, improved `SERVICE`,
  etc.). Users pin to pure 1.1 via the `sparql-1-1-strict` Cargo
  feature on `sparql-syntax`.

### 4.3 Shapes

- **SHACL**: shapes in Turtle; the `sh:` vocabulary is recognised for hover
  and completion; we do **not** run SHACL validation.
- **ShEx 2.1**: both ShExC (compact) and ShExJ (JSON) surface syntax; parse
  only.

### 4.4 Rules

- **Datalog**: standard Prolog-ish surface syntax (clauses, facts, queries,
  stratified negation recognition for highlighting). No evaluation.

## 5. Deliverables

| Artifact          | Surface                                                                 |
|-------------------|-------------------------------------------------------------------------|
| `rdf-lsp` binary  | LSP 3.17 server (Rust, tower-lsp). **stdio transport only**.            |
| `zed-rdf`         | Zed extension (`wasm32-wasip2`). Launches `rdf-lsp`, ships tree-sitter `.scm` queries and per-language `config.toml` for each RDF-family language. |
| `rdf-*` crates    | Parsers, vocabulary data, formatters — published to crates.io so other tools can reuse them. |

The LSP and the crates it builds on are the real product; the Zed
extension is a thin shim around them.

## 6. Non-functional requirements

- **Error-tolerant parsers.** An LSP must produce useful diagnostics on
  broken input without aborting.
- **Correctness.** Every parser passes its W3C syntax test manifest at
  100 % for v1.0 (see [`05-completion.md`](05-completion.md)).
- **Fast.** Cold-open of a 10 k-line Turtle file: highlighting ≤ 100 ms,
  first diagnostics ≤ 500 ms on the developer's laptop.
- **Pure Rust, no C deps** in the LSP. The Zed extension itself is
  `wasm32-wasip2`.
- `#![forbid(unsafe_code)]` by default (per ADR-0001).
- **Determinism.** Formatters produce identical output on identical input.
- **No network.** LSP does not reach the network; no ontology fetching,
  no endpoint calls, no telemetry.
- **No unsaved-work loss.** LSP never mutates the user's files without an
  explicit edit command (formatting, rename, code action).

## 7. Acceptance criteria (v1.0)

1. Each parser passes the **W3C rdf-tests** syntax manifest for its
   format at 100 %.
2. SPARQL parser passes the **sparql11-test-suite** *syntax-only*
   entries at 100 %; SPARQL 1.2 tracked-CR tests reported.
3. Tree-sitter grammars exist for every covered language (pinned
   community grammar or our own); Zed `.scm` query files for each render
   correctly in Zed.
4. LSP end-to-end tests pass for: open-file diagnostics, hover on every
   built-in vocabulary term, completion (keyword / prefix / snippet /
   vocab), goto-definition (prefix, SPARQL variable), document-symbols,
   formatting, rename, a representative set of code actions,
   semantic-tokens.
5. `zed-rdf` installs via `zed: install dev extension` and is publishable
   to the Zed extension registry (PR-ready submodule entry).
6. All layered test levels green (see ADR-0006): unit, property, fuzz
   ≥ 24 h with no unique crashes, snapshot, W3C manifests.
7. Crates published to crates.io under our agreed prefix.

## 8. Resolved scope decisions (2026-04-18)

| # | Decision topic                         | Outcome                                                                                   | Trace       |
|---|----------------------------------------|-------------------------------------------------------------------------------------------|-------------|
| 1 | Third-party crate policy               | Allow-list as drafted; RDF/SPARQL parser crates forbidden                                 | ADR-0004    |
| 2 | MSRV                                   | **Latest stable, no MSRV commitment** (overrides earlier N-2 proposal)                    | ADR-0001    |
| 3 | Repo layout                            | Single Cargo workspace; `crates/*`, `extensions/zed-rdf/`                                 | ADR-0002    |
| 4 | Licence                                | Apache-2.0 OR MIT dual                                                                    | `LICENSE-*` |
| 5 | Tree-sitter grammars                   | **Mixed** — pin existing community grammars where good; write our own for gaps            | ADR-0009 (reserved) |
| 6 | RDF 1.2 / SPARQL 1.2 syntax            | **Default on**; `rdf-1-1-strict` / `sparql-1-1-strict` Cargo features opt out             | ADR-0014 (reserved) |
| 7 | LSP transport                          | **stdio only**                                                                            | ADR-0011 (reserved) |

Further ADRs (0007, 0008, 0009, 0010, 0011, 0012, 0013, 0014, 0015,
0016 — see [`../adr/README.md`](../adr/README.md)) are drafted as their
phases begin.
