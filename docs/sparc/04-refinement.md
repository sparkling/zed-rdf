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

## Verification sweep retro (`verification-v1`, 2026-04-19)

The `verification-v1` sweep under [ADR-0020](../adr/0020-verification-implementation-plan.md)
landed ADR-0019's independence infrastructure. 19 agents, two cohorts
(`verification-v1` implementing + `verification-v1-adv` adversary),
`hierarchical-mesh` topology, 15-agent ceiling.

**Landed (cohort A, 13 agents).**

- `crates/testing/rdf-diff/` — frozen trait surface filled by
  `v1-diff-core` (canonicalise + diff + diff_many).
- `crates/testing/rdf-diff-oracles/` — 5 adapter modules
  (`oxttl`, `oxrdfxml`, `oxjsonld`, `spargebra` substituting for the
  unpublished `oxsparql-syntax` role, `sophia_*`) — all
  `[dev-dependencies]` only; `cargo tree -e normal` clean.
- `crates/syntax/rdf-iri-shadow/` (33 tests), `rdf-ntriples-shadow/`
  (87), `rdf-turtle-shadow/` (94), `sparql-syntax-shadow/` (24) —
  shadow parsers by `claude-sonnet-4-6` (cohort-A default `claude-opus-4-7`
  disjointness per ADR-0019 §3).
- `crates/testing/deny-regression/` — BFS over `cargo metadata` normal
  edges; proof-of-failure captured in memory.
- `deny.toml` expanded with `exclude-dev = true` + explicit bans.
- `.github/workflows/fact-oracles.yml` — JVM out-of-process pipeline
  (Jena + rdf4j, JDK 21 Temurin).
- `.github/workflows/verification.yml` + `xtask/verify/` — PR gate.
- `docs/spec-readings/` — 7 pins (NT, Turtle, IRI, BOM, SPARQL,
  JSON-LD).
- `scripts/memory-hygiene/` — TTL sweep, falsification hook, cohort
  guard; runbook; hooks.toml registration for `v1-ci-wiring` to mirror.

**Landed (cohort B, 6 agents).**

- 4 adversary briefs (`docs/verification/adversary-findings/*.md`): 33
  failure modes across NT, Turtle, IRI, SPARQL.
- Fixture corpora:
  `crates/testing/rdf-diff/tests/adversary-{nt,ttl,iri,sparql}/` — 47
  input+expected pairs (12 NT, 13 TTL, 9 IRI, 13 SPARQL).
- `.claude-flow/audit/adversary-veto/register.md` — 33 findings
  registered, **24 vetoes fired**, 0 spurious, 9 not-yet-vetoed.
  Veto fires far exceeds the ADR-0019 §Validation "≥1 per sweep" bar.

**Validation (against ADR-0020 §Validation).**

- One spawn message: partially held — the initial 19-agent spawn
  split into a 4-agent subset + 15-agent follow-up across a session
  restart (repo had no `.git` at start-of-session; runtime cached
  phantom branch metadata). Logged as the deviation.
- Cohort separation: held. No cohort-B agent read `verification-v1`
  namespaces; no cohort-A agent read `verification-v1-adv`. `v1-memory-ttl`'s
  guard enforces the rule mechanically from this point forward.
- Adversary veto fires ≥1: **far exceeded (24)**.
- All 19 agents complete: held.
- Wall-clock ≤ 48 h: held (sweep completed inside the day).
- **Differential-signal validation (non-zero divergences on Phase-A
  run): DEFERRED.** Phase-A main parsers (`rdf-ntriples`, `rdf-turtle`,
  `rdf-iri`, `sparql-syntax`) do not yet exist, so shadow-vs-main
  cannot run. 48 parser-wired tests across the fixture corpora are
  currently `#[ignore]`-gated pending Phase A. When Phase A lands, each
  parser's conformance gate must produce ≥1 material divergence per
  ADR-0019 §Validation before being declared green. This ADR (0020)
  is still Accepted because the infrastructure is load-bearing and
  correctly wired; the deferred bar is carried on each Phase A parser's
  own acceptance.

**Integration deviations.**

- Root-level `verification/` directory was written by cohort-A/B agents
  conflating memory namespaces with on-disk paths. Relocated to
  `docs/verification/` during the integration pass (`int-verification-relocate`);
  paths rewritten across tests, prompts, pins, xtask, veto register.
- `claims_accept-handoff` schema mismatch: agents could not call
  handoff via MCP tool. Handoffs recorded in memory + agent reports
  instead. Audit trail intact but dispersed; `v1-reviewer` re-run is
  deferred pending a handoff-tool investigation.
- Three shadow crates initially used a hard `compile_error!` guard that
  broke `cargo check --workspace` (no features); unified with
  `rdf-iri-shadow`'s empty-shell pattern during the integration pass
  (`int-shadow-emptyshell`).

**Gates cleared on integration.**

- `cargo test --workspace --all-features --no-fail-fast` → all green
  (238 shadow tests, 22 rdf-diff integration, 12 deny-regression, 2
  rdf-diff-oracles smoke, 48 Phase-A-ignored).
- `cargo clippy --workspace --all-features -- -D warnings` → clean.
- `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo run -p xtask -- verify` → `6 language(s) checked, 0 divergence(s),
  smoke=true`.

**Retirement.** On `git tag verification-v1/done` both ADRs become
historical; future sweeps get a fresh cohort registry and prompt set.

## 6. Engineering workflow

- `main` always releasable.
- Feature branches; PRs include failing test → implementation → docs →
  ADR link if applicable.
- Conventional Commits.
- Pre-1.0 release cadence: monthly.
- Phase exits add a short **Phase X retro** note below.

## Phase C retro (2026-04-20)

ADR-0022 executed as single-shot parallel swarm (3 agents, hierarchical topology).
`sparql-syntax` crate was pre-existing (4562 lines, compiles clean from Phase B scaffolding).
Phase C work: xtask/verify wiring (`manifest.rs` + `main.rs`), W3C integration test file (`w3c_syntax.rs`), grammar fixes for 19 conformance failures (VALUES clause, property paths, GROUP BY scope, UPDATE IRI forms, semicolon-separated operations, multi-graph LOAD/CLEAR).
Exit gate met: sparql 149/149 W3C syntax entries pass, 0 divergences. 63 additional tests (adversary fixtures, snapshots, scope-checks) all green.
Wall-clock: ~1 session. Tagged `phase-c/done`.

## Phase D retro (2026-04-20)

ADR-0023 executed as single-shot parallel swarm (4 agents, mesh topology).
`shex-syntax`: hand-rolled recursive-descent ShEx 2.x compact syntax parser (AST + lexer + parser + encoder). 43 tests green. Key decisions: `@` disambiguation for shape refs vs lang tags, cardinality brace lookahead, undefined-prefix fatal error.
`datalog-syntax`: hand-rolled Datalog parser (rules, facts, negation, comments, quoted constants). 18 tests green. Borrow-checker issue in encode.rs fixed during tester pass.
Exit gate met: both fixture corpora green, `cargo test --workspace` clean, clippy clean.
Wall-clock: ~1 session. Tagged `phase-d/done`.

## 7. Budget overrun policy

If a phase exceeds its estimate by > 50 %:

1. Stop, write a one-page note in `docs/retros/`.
2. Decide scope trim (ADR amendment) or accept new budget (update this
   doc).
3. Re-flow downstream phases; no silent slip.
