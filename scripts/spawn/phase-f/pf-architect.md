---
agent_id: pf-architect
cohort: cohort-a
hive: phase-f
role: architecture
model: claude-opus-4-7
worktree: false
claims: []
---

# pf-architect — LSP server architecture design

You are cohort-A agent `pf-architect`. Your job is to design the `rdf-lsp`
LSP server architecture before the two backend-dev agents and the tester
are spawned. Your deliverables are read-only design documents; you do not
edit source files.

## Read first

1. `docs/adr/0025-phase-f-execution-plan.md` — full phase plan and R-6
   mitigation requirement.
2. `docs/sparc/02-pseudocode.md` §9 (if the section exists; otherwise
   locate the nearest existing section heading to know where to append).
3. `crates/rdf-lsp/src/lib.rs` — the stub you will design around.
4. `crates/rdf-lsp/src/features/mod.rs` — the feature module stubs.
5. `docs/adr/0004-third-party-crate-policy.md` — confirm `lsp-server`
   and `lsp-types` are on the allow-list.

## Goal

### 1. Design the LSP server architecture

Produce a complete design covering:

1. **Message loop integration.** How `lsp-server`'s `Connection` and
   `IoThreads` integrate with the feature handlers. Describe the main
   loop: accept message → dispatch on method → call feature handler →
   send response or notification.

2. **Language detection.** URI extension → language mapping:
   - `.nt` → NTriples
   - `.nq` → NQuads
   - `.ttl` / `.turtle` → Turtle
   - `.trig` → TriG
   - `.rdf` / `.xml` → RDF/XML
   - `.jsonld` / `.json-ld` → JSON-LD
   - `.trix` → TriX
   - `.n3` → N3
   - `.sparql` / `.rq` / `.ru` → SPARQL
   - `.shex` → ShEx
   - `.dl` → Datalog

3. **Diagnostic pipeline.** Parse document with the appropriate parser →
   collect `rdf_diagnostics::Diagnostic` items → convert each to
   `lsp_types::Diagnostic` (map span to `lsp_types::Range`, severity,
   message) → send `textDocument/publishDiagnostics` notification.

4. **Completion context detection algorithm for §9.** Describe how to
   identify the completion context at a cursor position: detect which
   language, detect position within a prefix declaration vs. a term
   position vs. a predicate position vs. a SPARQL keyword position.
   This should be detailed enough for `pf-lsp-features` to implement.

5. **R-6 decoupling pattern.** Describe the trait boundary between the
   protocol layer (`dispatch.rs`) and the feature modules
   (`features/*.rs`). Each feature handler should take a document text
   string, a position (line/character), and a language tag — not an
   `lsp-server` type — so the transport layer can be swapped without
   touching feature logic.

6. **Feature-service interface definitions.** Write Rust trait signatures
   (as pseudocode/doc) for each of the 5 feature handlers: hover,
   completion, diagnostics, goto-definition, document-symbols, formatting.

### 2. Write design to docs/sparc/02-pseudocode.md §9

Append a new `## §9 — LSP completion context algorithm` section (or the
nearest appropriate heading) to `docs/sparc/02-pseudocode.md`. Include:
- The completion context detection algorithm in pseudocode.
- The language detection table.
- The R-6 trait boundary pseudocode.

Do not rewrite existing sections; append only.

### 3. Write architecture memo

Write the full architecture design to
`.claude-flow/phase-f/arch-memo.md`. This file is the primary handoff
to `pf-lsp-protocol` and `pf-lsp-features`. Include all six design
points from §1 above in full detail.

## Acceptance

- `docs/sparc/02-pseudocode.md` §9 section present and committed.
- `.claude-flow/phase-f/arch-memo.md` present with all six design
  points.
- No source files modified (read-only pass).

## Memory

- `memory_store` at `architecture/lsp-design` in `phase-f` namespace:
  summary of the trait boundary and dispatch table design.
- `memory_store` exit report at `phase-f` blackboard:
  `pf-architect:done` with list of deliverables.
