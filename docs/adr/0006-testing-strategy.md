# ADR-0006: Testing strategy

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Tags:** `testing`, `quality`, `process`

## Context and Problem Statement

Our correctness claim (ADR-0005) is only as good as the tests that back
it. Parsers plus an LSP need layered coverage to stay honest: unit tests
for spec features, property tests for round-trip invariants, fuzz for
parser hardening, snapshot tests for diagnostics, W3C manifests for
compliance, and LSP integration tests for end-to-end behaviour.

## Decision

**Layered test strategy.** Each layer exists to catch a distinct class
of bug.

### Layers

1. **Unit tests.** Colocated with code. Every public function has
   happy-path + error-path tests.

2. **Property tests (`proptest`).** Invariants:
   - `parse → format → parse` produces identical facts.
   - Formatter is idempotent: `format(format(x)) == format(x)`.
   - Diagnostics are monotone under prefix extension: appending valid
     content does not invalidate earlier spans.
   - IRI: `resolve(relative, base)` is absolute and matches RFC 3986
     worked examples.
   Default: 1 024 cases per property; nightly 16 384.

3. **Fuzz targets (`cargo-fuzz`).** One per parser. Corpus seeded from
   the W3C test suite + hand-crafted pathological inputs. Release gate:
   ≥ 24 h cumulative without unique crashes since last release.

4. **Snapshot tests (`insta`).** Diagnostic output on representative
   broken inputs; formatter output on representative corpora;
   `documentSymbol` trees; hover contents for built-in vocabulary.

5. **W3C conformance manifests.** Vendored as submodules in
   `external/tests/`, pinned by commit. Harness in
   `crates/testing/rdf-testsuite/`. Fails closed (unknown test kind =
   failure).

6. **LSP integration tests.** `crates/testing/rdf-testsuite/` spawns
   `rdf-lsp` and drives it over stdio with a scripted client. Per
   language, the harness exercises diagnostics, hover, completion,
   goto-definition, document-symbols, formatting, rename, code actions,
   semantic tokens.

7. **Benchmark regression (`criterion`).** Runs on a dedicated CI
   runner. Regressions > 10 % fail the merge-queue job.

8. **Release-candidate rehearsal.** Before release: bump pinned test
   suites, run full CI including 24 h fuzz, manual smoke test of the
   Zed extension.

### CI topology

- **Fast lane** (every PR, ≤ 8 min):
  `cargo check`, `cargo clippy -D warnings`, `cargo test` (excludes
  long benches and fuzz), `cargo-deny check`, `cargo-audit`,
  `cargo-msrv`.
- **Full lane** (`main` merges + nightly): full W3C conformance suites,
  full LSP integration suite, benchmark-regression gate.
- **Extended nightly**: 24 h fuzz continuation; property tests at
  16 384 cases; `miri` on allow-listed `unsafe` crates (if any).

### Determinism

- No wall-clock assertions.
- No network in tests.
- Randomness is seeded; fuzz inputs preserved in the corpus directory.
- Tests that can't be made deterministic are `#[ignore]`-d with a
  linked issue.

### Coverage

`cargo-llvm-cov` on the full lane. Targets:

- **Parser crates**: ≥ 90 % line / ≥ 80 % branch.
- **Cross-cutting crates** (`rdf-diagnostics`, `rdf-iri`, `rdf-vocab`):
  ≥ 95 % line / ≥ 85 % branch.
- **LSP + format**: ≥ 85 % line / ≥ 75 % branch.

Coverage drops > 1 pp vs `main` require a PR justification.

### Test-suite allowlisting

Buggy upstream tests are allow-listed in `external/tests/ALLOWLIST.md`
with:

- test id,
- upstream issue link,
- justification,
- target date to re-enable.

Allow-list entries past their target date block the next release until
triaged.

## Consequences

- **Positive**: every class of bug has a targeting layer. Releases have
  measurable, anchored claims.
- **Negative**: upfront harness cost (paid once in `rdf-testsuite`).
  Nightly fuzz requires an always-on runner (cost acceptable).
- **Neutral**: strategy is ambitious; we can scale down per ADR
  amendment if capacity demands it.

## Validation

- Per-phase exit gate: layers 1-5 green at 100 % of relevant content
  (layer 5 only where a W3C suite exists).
- Release rehearsal checklist lives at `docs/release/checklist.md` by
  end of phase A.
- This ADR re-reviewed at v1.0.

## Links

- `docs/sparc/04-refinement.md` §1 TDD discipline.
- `docs/sparc/05-completion.md` §2 conformance gate.
- ADR-0005 parser correctness scope.
