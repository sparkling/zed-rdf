# SPARC-04 — Refinement

> **Supersedes** the engine-scoped v1. Rewritten 2026-04-18.

## 1. TDD discipline

Every crate is test-first. For parsers, "test-first" means:

1. Add the W3C manifest entry (or a hand-written fixture for formats
   without a suite, e.g., N3, Datalog).
2. Test fails.
3. Minimum code to pass.
4. Refactor.
5. Commit.

Layered test strategy per ADR-0006: unit → property (round-trip
invariants) → fuzz → snapshot → W3C manifest → LSP end-to-end.

## 2. Phase plan

Durations are order-of-magnitude estimates assuming the execution model
in [ADR-0017](../adr/0017-execution-model.md) — ruflo-orchestrated
parallel agent swarms with 3–6 concurrent workers per phase. Serial
execution (mode A of that ADR) blows these budgets and should only be
used for trivial changes.

| Phase | Name               | What lands                                                                                   | Duration      | Exit gate                                                              |
|-------|--------------------|----------------------------------------------------------------------------------------------|---------------|------------------------------------------------------------------------|
| **A** | Foundations        | `rdf-diagnostics`, `rdf-iri`, `rdf-ntriples`, `rdf-turtle`, `rdf-format` (NT + Turtle), `rdf-testsuite` harness | 3–4 weeks     | N-Triples, N-Quads, Turtle, TriG manifests 100 % green                 |
| **B** | Remaining RDF syntax | `rdf-xml`, `rdf-jsonld` (syntax + context well-formedness), `rdf-trix`, `rdf-n3`           | 4–6 weeks     | RDF/XML + JSON-LD syntax suites 100 %; snapshot coverage for TriX + N3 |
| **C** | SPARQL syntax       | `sparql-syntax` (1.1 full + 1.2 behind feature), resolver for prefix/var scope               | 3–4 weeks     | sparql11-test-suite syntax entries 100 %                                |
| **D** | Shapes + rules syntax | `shex-syntax`, `datalog-syntax`; SHACL via rdf-vocab recognition over Turtle                | 2–3 weeks     | ShEx test suite syntax-only entries 100 %; Datalog fixture corpus green |
| **E** | Vocab + formatters  | `rdf-vocab` complete (xsd / rdf / rdfs / owl / skos / sh / dcterms / dcat / foaf / schema.org / prov), per-format formatters in `rdf-format` | 2–3 weeks | Hover-docs snapshot locked; formatter idempotency tests green          |
| **F** | LSP core            | `rdf-lsp` bin: didOpen/didChange/publishDiagnostics, hover, completion, goto-definition, documentSymbol, formatting | 4–5 weeks | LSP integration harness green across all languages                     |
| **G** | LSP polish          | Rename, code actions, semantic tokens, workspace symbols, incremental parsing               | 2–3 weeks     | Per-feature integration test green; perf targets met                   |
| **H** | Zed extension       | `extensions/zed-rdf/` with all language `config.toml` files and `.scm` queries, grammar pins, extension `lib.rs` launches LSP | 2–3 weeks | `zed: install dev extension` works end-to-end on every language        |
| **I** | Publish + harden    | Publish crates; publish extension to Zed registry; fuzz 24 h clean; docs + examples         | 1–2 weeks     | v1.0 tagged                                                             |

Phases A–E are the parser/foundation stack. F–G build the LSP on top.
H wraps it for Zed. Phases can overlap once A is done (parsers are
independent).

## 3. Per-phase milestones

Each phase:

1. **Kick-off**: fill the relevant section of
   [`02-pseudocode.md`](02-pseudocode.md) for non-trivial pieces; update
   or author ADRs; create issues.
2. **Walking skeleton**: minimal end-to-end happy path (for phase A: parse a
   single N-Triple, emit highlighting facts).
3. **Feature breadth**: conformance-driven — add failing test, add code.
4. **Gate green**: W3C manifest / integration harness passes.
5. **Docs + benchmarks**: rustdoc complete; parser throughput measured;
   examples in `examples/`.
6. **Retro**: update this doc's retro section and the risk register.

## 4. Risk register

| ID  | Risk                                                                                     | Likelihood | Impact | Mitigation                                                                                                                | Retirement signal                         |
|-----|------------------------------------------------------------------------------------------|------------|--------|---------------------------------------------------------------------------------------------------------------------------|-------------------------------------------|
| R-1 | Community tree-sitter grammars lag spec (RDF 1.2, SPARQL 1.2)                            | Medium     | Medium | Pin by commit; fork and upstream when blocked; ADR per language                                                           | Every language has a green grammar pin    |
| R-2 | JSON-LD 1.1 context processing still too large to do "just for syntax"                   | Medium     | Medium | Keep it to well-formedness of `@context` values; skip expand/compact; revisit if needed                                   | JSON-LD syntax suite 100 %                |
| R-3 | Error recovery in the Turtle parser produces noisy diagnostics                            | High       | Low    | Resynchronise at `.` statement terminators; snapshot-test diagnostics on broken inputs                                    | Snapshot corpus stable                    |
| R-4 | LSP performance regresses on big files                                                   | Medium     | Medium | Incremental parse for Turtle/TriG/SPARQL; rope + line-diff; bench in CI                                                   | 10 k-line Turtle < 100 ms highlight       |
| R-5 | RDF 1.2 spec moves underneath us                                                         | Medium     | Low    | Feature-flag 1.2 syntax; track latest CR; CHANGELOG on every re-pin                                                       | RDF 1.2 REC shipped                       |
| R-6 | `tower-lsp` or alternative LSP crate ecosystem churn                                     | Medium     | Medium | Thin LSP glue, feature-service code decoupled; migration is mostly re-registering handlers                                | LSP swap rehearsal run once               |
| R-7 | Zed `zed_extension_api` breaking changes                                                 | Medium     | Medium | Track the crate's release notes; pin `extension_api` version; extension itself is tiny so migrations are cheap            | Two consecutive Zed releases without work |
| R-8 | Scope creep back into engine-shaped features ("we could just add a little SPARQL exec…") | Medium     | High   | Section §2 of the specification is the sign on the door; any drift requires an ADR amendment                              | v1.0 shipped without engine features      |
| R-9 | Tree-sitter query files (`.scm`) bitrot across grammar updates                           | Medium     | Low    | CI job: tree-sitter queries parse against the pinned grammar; update in the same PR that moves the grammar pin            | CI job green                              |

## 5. Benchmarking discipline

- `criterion` benches per parser in `crates/<parser>/benches/`.
- Baselines committed under `bench/baselines/`.
- CI fails on > 10 % regression.
- Targets:
  - N-Triples parse: ≥ 200 MB/s.
  - Turtle parse: ≥ 80 MB/s on prettified input.
  - SPARQL parse: ≥ 1000 queries/s on a realistic corpus.
  - LSP cold-open 10 k-line Turtle: highlight ≤ 100 ms, first diagnostics
    ≤ 500 ms.

## 6. Engineering workflow

- `main` always releasable.
- Feature branches; PRs include failing test → implementation → docs →
  ADR link if applicable.
- Conventional Commits.
- Pre-1.0 release cadence: monthly.
- Phase exits add a short **Phase X retro** note below.

## 7. Budget overrun policy

If a phase exceeds its estimate by > 50 %:

1. Stop, write a one-page note in `docs/retros/`.
2. Decide scope trim (ADR amendment) or accept new budget (update this
   doc).
3. Re-flow downstream phases; no silent slip.
