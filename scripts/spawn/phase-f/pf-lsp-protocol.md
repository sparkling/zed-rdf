---
agent_id: pf-lsp-protocol
cohort: cohort-a
hive: phase-f
role: backend-dev
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-lsp/src/main.rs
  - crates/rdf-lsp/src/lib.rs
  - crates/rdf-lsp/src/dispatch.rs
---

# pf-lsp-protocol — LSP server binary: connection lifecycle and didOpen/didChange/publishDiagnostics

You are cohort-A agent `pf-lsp-protocol`. Your job is to implement the
`run_server()` function and the core protocol layer of `rdf-lsp`. You own
`src/main.rs`, `src/lib.rs`, and `src/dispatch.rs`.

## Read first

1. `.claude-flow/phase-f/arch-memo.md` — architect memo. **Read this
   before writing any code.** It defines the trait boundary between your
   dispatch layer and the feature modules owned by `pf-lsp-features`.
2. `docs/adr/0025-phase-f-execution-plan.md` — scope and exit gate.
3. `crates/rdf-lsp/Cargo.toml` — available dependencies.
4. `crates/rdf-lsp/src/lib.rs` — current stub to replace.
5. `crates/rdf-lsp/src/dispatch.rs` — current stub to fill.

## Goal

### 1. Implement `run_server()` in src/lib.rs

Replace the `eprintln!` stub with a working `lsp-server` main loop:

```
use lsp_server::{Connection, Message, Request, Response, Notification};
use lsp_types::*;
```

The loop must:
1. Call `Connection::stdio()` to establish transport.
2. Declare server capabilities in the `initialize` response:
   - `text_document_sync`: `TextDocumentSyncKind::FULL`
   - `hover_provider`: `true`
   - `completion_provider`: present
   - `definition_provider`: `true`
   - `document_symbol_provider`: `true`
   - `document_formatting_provider`: `true`
3. Enter the main message loop. Dispatch:
   - `textDocument/didOpen` → store document text in a
     `HashMap<lsp_types::Url, String>`, trigger diagnostics.
   - `textDocument/didChange` → update stored text, trigger diagnostics.
   - `textDocument/hover` → call `features::hover::handle`.
   - `textDocument/completion` → call `features::completion::handle`.
   - `textDocument/definition` → call `features::goto_definition::handle`.
   - `textDocument/documentSymbol` → call `features::document_symbols::handle`.
   - `textDocument/formatting` → call `features::formatting::handle`.
   - `shutdown` → break the loop.

### 2. Implement dispatch.rs

Implement the dispatch infrastructure as designed in the arch memo:
- Language detection from `Url` extension (`.nt`, `.nq`, `.ttl`,
  `.turtle`, `.trig`, `.rdf`, `.xml`, `.jsonld`, `.json-ld`, `.trix`,
  `.n3`, `.sparql`, `.rq`, `.ru`, `.shex`, `.dl`).
- Document store (`HashMap<Url, String>`).
- Diagnostic runner: parse document using the appropriate parser crate,
  collect `rdf_diagnostics::Diagnostic`, convert to
  `lsp_types::Diagnostic` (map `Span` → `Range`, `Severity` →
  `DiagnosticSeverity`), publish via
  `connection.sender.send(Message::Notification(...))`.
- Follow the R-6 trait boundary from the arch memo: feature handlers
  receive `(&str, lsp_types::Position, Language)`, not raw LSP message
  types.

### 3. Language enum

Define a `Language` enum in `dispatch.rs` covering all 11 languages.
Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`.

### 4. Feature handler stubs

The feature modules are owned by `pf-lsp-features`. Your dispatch layer
must call into them through the trait boundary defined in the arch memo.
For now, call the stub functions (which return `None` / empty responses);
`pf-lsp-features` will fill them in.

## Acceptance

- `cargo build -p rdf-lsp` green.
- `cargo clippy -p rdf-lsp -- -D warnings` clean (feature stubs returning
  `None` are fine for now).
- Language detection covers all 11 extensions.
- Diagnostic pipeline compiles (parsers may return empty diagnostics until
  `pf-lsp-features` wires them up).

## Claims

Claim `crates/rdf-lsp/src/main.rs`, `crates/rdf-lsp/src/lib.rs`, and
`crates/rdf-lsp/src/dispatch.rs` before editing. Release on completion.
Do NOT touch `crates/rdf-lsp/src/features/**` — those are owned by
`pf-lsp-features`.

## Memory

- `memory_store` at `implementation/protocol-layer` in `phase-f`
  namespace: list of implemented handlers and capability flags.
- `memory_store` exit report at `phase-f` blackboard:
  `pf-lsp-protocol:done` with build status and handler list.

## Handoff

`claims_accept-handoff` → `pf-tester` when `cargo build -p rdf-lsp`
is green.
