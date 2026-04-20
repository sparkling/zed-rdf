---
agent_id: pf-lsp-features
cohort: cohort-a
hive: phase-f
role: backend-dev
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-lsp/src/features/**
---

# pf-lsp-features — LSP feature handlers: hover, completion, goto-definition, documentSymbol, formatting

You are cohort-A agent `pf-lsp-features`. Your job is to implement the 5
LSP feature handlers in `crates/rdf-lsp/src/features/`. You own
`src/features/**`.

## Read first

1. `.claude-flow/phase-f/arch-memo.md` — architect memo. **Read this
   before writing any code.** It defines the trait boundary between the
   dispatch layer (owned by `pf-lsp-protocol`) and your feature modules.
2. `docs/adr/0025-phase-f-execution-plan.md` — scope.
3. `crates/rdf-lsp/Cargo.toml` — available dependencies.
4. `crates/rdf-lsp/src/features/mod.rs` — module stubs to fill.
5. `crates/rdf-vocab/src/lib.rs` — term model for hover lookups.

## Goal

Implement all 5 feature handlers in accordance with the R-6 trait
boundary from the arch memo. Each handler takes `(text: &str,
position: lsp_types::Position, language: Language)` and returns the
appropriate `lsp_types` response type wrapped in `Option`.

### 1. Hover (`features/hover.rs`)

- Locate the token at `position` in `text`.
- If the token looks like a full IRI or a prefixed name, look it up in
  the appropriate `rdf_vocab` module (e.g., `rdf_vocab::rdf`,
  `rdf_vocab::rdfs`, `rdf_vocab::owl`, `rdf_vocab::xsd`, `rdf_vocab::sh`,
  etc.).
- Return `HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown,
  value: "<label>\n\n<comment>" })` if found; `None` otherwise.
- Signature: `pub fn handle(text: &str, pos: lsp_types::Position,
  lang: Language) -> Option<lsp_types::Hover>`.

### 2. Completion (`features/completion.rs`)

- Detect the completion context at `position` (use the algorithm from
  `docs/sparc/02-pseudocode.md` §9).
- For each language, return keyword completions:
  - Turtle/TriG: `@prefix`, `@base`, `a`, `true`, `false`, common XSD
    datatype suffixes.
  - SPARQL: `SELECT`, `CONSTRUCT`, `ASK`, `DESCRIBE`, `WHERE`, `FILTER`,
    `OPTIONAL`, `UNION`, `GRAPH`, `PREFIX`, `BASE`, etc.
  - ShEx: `PREFIX`, `BASE`, `EXTERNAL`, `ABSTRACT`, `IRI`, `LITERAL`,
    `BNODE`, `NONLITERAL`, `AND`, `OR`, `NOT`.
  - Datalog: `:-`, `.` (rule terminator), built-in predicate names.
  - NT/NQ/RDF-XML/JSON-LD/TriX/N3: minimal (prefix keywords where
    applicable).
- Also suggest terms from `rdf_vocab` for any language when a prefix is
  recognised (e.g., after `rdf:` suggest all `rdf_vocab::rdf` terms).
- Signature: `pub fn handle(text: &str, pos: lsp_types::Position,
  lang: Language) -> Option<lsp_types::CompletionResponse>`.

### 3. Document symbols (`features/document_symbols.rs`)

- Parse document with the appropriate parser.
- Extract top-level names:
  - NT/NQ: distinct subjects (as IRIs).
  - Turtle/TriG: subjects of top-level triples.
  - SPARQL: query variables (`?var`, `$var`).
  - ShEx: shape labels.
  - Datalog: relation names (head predicate of each rule).
  - Others: empty list is acceptable.
- Return `DocumentSymbolResponse::Flat(Vec<SymbolInformation>)`.
- Signature: `pub fn handle(text: &str, lang: Language) ->
  Option<lsp_types::DocumentSymbolResponse>`.

### 4. Formatting (`features/formatting.rs`)

- Call the appropriate serialiser from `rdf-format` on the parsed facts.
- If no serialiser exists for the language (RDF/XML, JSON-LD — Phase E
  stretch goals), return `None` (graceful degradation per ADR-0025
  §Consequences/Negative).
- Return `Some(vec![TextEdit { range: full_document_range, new_text:
  formatted }])`.
- Signature: `pub fn handle(text: &str, lang: Language) ->
  Option<Vec<lsp_types::TextEdit>>`.

### 5. Goto-definition (`features/goto_definition.rs`)

- For prefix names (e.g., `rdf:`, `owl:`), return the location of the
  `@prefix`/`PREFIX` declaration in the document.
- Scan `text` for the declaration line, compute `lsp_types::Location`.
- Signature: `pub fn handle(text: &str, pos: lsp_types::Position,
  lang: Language) -> Option<lsp_types::GotoDefinitionResponse>`.

## Acceptance

- `cargo build -p rdf-lsp` green with all 5 features registered in
  `features/mod.rs` and callable.
- `cargo clippy -p rdf-lsp -- -D warnings` clean.
- Each handler compiles and returns a sensible stub for at least one
  language (full implementation is Phase F's complete deliverable, but
  the signatures must be correct and callable).

## Claims

Claim `crates/rdf-lsp/src/features/**` before editing. Release on
completion. Do NOT touch `src/main.rs`, `src/lib.rs`, or
`src/dispatch.rs` — those are owned by `pf-lsp-protocol`.

## Memory

- `memory_store` at `implementation/feature-handlers` in `phase-f`
  namespace: list of implemented features and language coverage per
  feature.
- `memory_store` exit report at `phase-f` blackboard:
  `pf-lsp-features:done` with feature list and build status.

## Handoff

`claims_accept-handoff` → `pf-tester` when `cargo build -p rdf-lsp`
is green with all 5 feature handlers present.
