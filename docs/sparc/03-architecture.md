# SPARC-03 — Architecture

> **Supersedes** the engine-scoped v1. Rewritten 2026-04-18 for the
> Zed-extension + LSP scope.

## 1. Shape

Three tiers:

1. **Crates** — Rust libraries: per-format parsers, a shared diagnostics
   model, a well-known vocabulary database, per-format formatters.
2. **LSP** — a `tower-lsp` binary that composes the crates and speaks
   LSP 3.17 over stdio.
3. **Zed extension** — a `wasm32-wasip2` shim that launches the LSP and
   ships the tree-sitter `.scm` queries and `config.toml` for each
   language.

Data flows strictly one direction: the editor asks, the extension proxies
to the LSP, the LSP uses the parser crates, the LSP answers.

## 2. DDD bounded contexts

| # | Context         | Owns                                                                   |
|---|-----------------|------------------------------------------------------------------------|
| 1 | **Diagnostics** | Span, Severity, Label — shared across all parsers.                     |
| 2 | **IRI**         | RFC 3987 IRI parse / normalise / relative resolution.                  |
| 3 | **Syntax**      | One sub-context per format. Each owns its lexer, error-tolerant parser, concrete syntax tree (CST), and facts extracted for LSP features (prefixes, variables, symbols). |
| 4 | **Vocabulary**  | Static database of well-known RDF/SPARQL/SHACL/ShEx/OWL vocabulary terms with labels and short descriptions for hover + completion. |
| 5 | **Formatting**  | Per-format deterministic pretty-printer.                               |
| 6 | **LSP**         | Feature services — diagnostics, completion, hover, goto-definition, document symbols, rename, code actions, semantic tokens, formatting. Owns the document model (rope) and workspace state. |
| 7 | **Editor assets** | Tree-sitter `.scm` queries + per-language `config.toml` inside the extension. Not Rust code, but architecturally significant. |
| 8 | **Extension**   | `wasm32-wasip2` shim: locates `rdf-lsp`, launches it, reports status to Zed. |
| 9 | **Testing**     | W3C manifest harness (parse-only), LSP integration harness.            |

Translation rule: a parser's CST is its own type; the LSP consumes
**facts** extracted from the CST (e.g., `PrefixDecl { prefix: "foaf",
iri: "http://…", span: 12..43 }`), not the CST directly, so each parser
owns its internal shapes.

## 3. Crate topology (workspace)

~12 crates, all under one Cargo workspace.

```
zed-rdf/
├── Cargo.toml                     # workspace root
├── crates/
│   ├── foundations/
│   │   ├── rdf-diagnostics/       # Span, Label, Severity, Diagnostic
│   │   └── rdf-iri/               # RFC 3987 IRI
│   ├── syntax/
│   │   ├── rdf-turtle/            # Turtle + TriG (CST + facts for LSP)
│   │   ├── rdf-ntriples/          # N-Triples + N-Quads
│   │   ├── rdf-xml/               # RDF/XML (built on quick-xml)
│   │   ├── rdf-jsonld/            # JSON-LD syntax + context well-formedness
│   │   ├── rdf-trix/              # TriX
│   │   ├── rdf-n3/                # Notation3 syntax
│   │   ├── sparql-syntax/         # SPARQL 1.1/1.2 parser + resolver
│   │   ├── shex-syntax/           # ShExC + ShExJ
│   │   └── datalog-syntax/        # Datalog
│   ├── vocab/
│   │   └── rdf-vocab/             # xsd/rdf/rdfs/owl/skos/sh/dc*/foaf/schema/…
│   ├── format/
│   │   └── rdf-format/            # Per-language formatters
│   ├── lsp/
│   │   └── rdf-lsp/               # tower-lsp binary
│   └── testing/
│       └── rdf-testsuite/         # W3C manifest + LSP integration harness
├── extensions/
│   └── zed-rdf/                   # wasm32-wasip2; extension.toml; languages/*/
└── docs/
    ├── sparc/
    └── adr/
```

Notes:

- **SHACL** does not get its own syntax crate — SHACL shapes are Turtle.
  Recognition is a job of `rdf-vocab` (knows the `sh:` terms) plus the
  LSP (knows how to look up `sh:Property` on hover).
- **SKOS / OWL / DCAT / FOAF** similarly: they are sets of IRIs carrying
  documentation; they live in `rdf-vocab`.
- **SPARQL-star** (triple patterns with triple terms) is a feature flag
  on `sparql-syntax`.
- **Datalog** gets its own crate because its grammar differs meaningfully
  from any RDF format.

## 4. Intra-workspace dependency graph

```
rdf-diagnostics ◄── rdf-iri ◄── syntax/* (all parser crates)
                    rdf-vocab ─────► (string constants only; depends on nothing except rdf-iri)
                    rdf-format ─► syntax/*
                    rdf-lsp ─► rdf-diagnostics, rdf-iri, rdf-vocab, rdf-format, syntax/*
                    rdf-testsuite ─► syntax/*, rdf-diagnostics
                    extensions/zed-rdf ─► (build-only; no Rust dep on the engine crates — it spawns rdf-lsp)
```

Cycles forbidden, enforced by `cargo-deny`.

## 5. Pipelines

### 5.1 Parsing

```
bytes → lexer (logos) → tokens
                     → error-tolerant parser → CST + Diagnostic[]
                                             → Fact extractor → prefixes, symbols, variable bindings
```

Parsers never `unwrap` on user input. Unexpected tokens become
`Diagnostic` + recovery; the CST is always produced.

### 5.2 LSP request

```
LSP request → document rope (updated incrementally)
            → parser (re-parse; incremental where cheap)
            → feature service:
                didOpen / didChange → publishDiagnostics
                hover               → rdf-vocab lookup
                completion          → grammar-driven + vocab-driven
                definition          → in-file symbol table
                documentSymbol      → CST → symbol tree
                formatting          → rdf-format
                rename              → scope-aware CST rewrite
                codeAction          → curated fixers
                semanticTokens      → CST → LSP token stream
```

### 5.3 Tree-sitter surface

Zed reads `.scm` files directly — no Rust involvement. Rust parsers and
tree-sitter grammars are **parallel** sources of truth:

- **tree-sitter** handles the visual surface: highlighting, brackets,
  indents, outline, textobjects, injections, runnables, overrides.
- **Rust parser** handles semantic surface: diagnostics, hover targets,
  completion context, goto-definition, rename scope.

Tree-sitter grammars are pinned community grammars (ADR-TBD on which
repos / commits). We do not write grammars from scratch unless forced.

## 6. Cross-cutting

- **Error model**: `rdf-diagnostics::Diagnostic` with
  `{ severity, code, range, message, related, fixits }`. Converts to LSP
  directly.
- **Concurrency**: sync parsers, tokio/axum-style async runtime in the
  LSP. Parsing offloaded via `spawn_blocking` for huge files.
- **Unicode**: NFC normalisation for IRIs; `unicode-normalization`.
- **Feature flags** per parser: `rdf-star`, `sparql-1-2`.
- **Logging**: `tracing`; LSP logs to stderr by default.

## 7. Public API shape (LSP-facing)

The LSP talks to the crates through a few boring types:

```rust
// rdf-diagnostics
pub struct Diagnostic { severity, code, range, message, labels, fixits }

// syntax/<format>
pub fn parse(source: &str) -> ParseResult;
pub struct ParseResult { pub cst: Cst, pub diagnostics: Vec<Diagnostic>, pub facts: Facts }
pub struct Facts { pub prefixes: Vec<PrefixDecl>, pub symbols: Vec<Symbol>, pub references: Vec<Reference> }

// rdf-vocab
pub fn lookup(iri: &str) -> Option<&VocabEntry>;
pub struct VocabEntry { pub iri: &'static str, pub label: &'static str, pub description: &'static str, pub kind: Kind }

// rdf-format
pub fn format(source: &str, opts: &FormatOptions) -> Result<String, Diagnostic>;
```

No store, no graph, no triple types. This is not an RDF engine.

## 8. Zed extension

`extensions/zed-rdf/` layout:

```
Cargo.toml                 # crate-type = ["cdylib"], target wasm32-wasip2
extension.toml             # id, name, [grammars.*], [language_servers.*]
src/lib.rs                 # impl zed_extension_api::Extension; locates & launches rdf-lsp
languages/
  turtle/
    config.toml
    highlights.scm
    outline.scm
    brackets.scm
    indents.scm
    injections.scm
    textobjects.scm
    runnables.scm
    overrides.scm
  trig/       …
  ntriples/   …
  nquads/     …
  sparql/     …
  shacl/      (shacl files are turtle; config points grammar=turtle, scope-restricts sh:*)
  shex/       …
  rdfxml/     …
  jsonld/     …
  trix/       …
  n3/         …
  datalog/    …
```

Grammar pins live in `extension.toml`; per-language `config.toml` sets
path suffixes, brackets, comments, `first_line_pattern`.

## 9. What this does not yet decide

- Which parser-combinator crate (`chumsky` vs `winnow`). ADR-TBD.
- Which tree-sitter grammar for each language and whether any need a
  fork. ADR-TBD per language.
- Whether `rdf-format` shares a formatter framework across formats or
  hand-writes each. ADR-TBD.
- LSP capability matrix — exact set of code actions and semantic-token
  types. ADR-TBD.
