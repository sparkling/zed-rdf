# Runbook — coverage with `cargo-llvm-cov`

Owner: `cu-coverage` (ADR-0020 §1.2 carry-over).
Scope: per-crate line-coverage measurement + the Phase-A 70 % hard gate.
Companions: [`.github/workflows/coverage.yml`](../../.github/workflows/coverage.yml),
[`xtask/verify/src/main.rs`](../../xtask/verify/src/main.rs).

## 1. What this is

Coverage is measured with
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov), which
drives Rust's source-based coverage (`-C instrument-coverage`) and
collates per-crate `.profdata` into line-, region-, and function-level
reports.

Two entry points:

| Entry point                                                | Who uses it                          |
|------------------------------------------------------------|--------------------------------------|
| `cargo run -p xtask -- verify --coverage`                  | Local developers                     |
| `.github/workflows/coverage.yml`                           | CI (every PR, weekly cron, manual)   |

Both share the same threshold matrix (§3) and the same `cargo llvm-cov`
invocation, so a green local run predicts a green CI run.

## 2. Running locally

### 2.1 Prerequisites

You need `cargo-llvm-cov` and the `llvm-tools-preview` rustup component:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

If `cargo-llvm-cov` is missing, `xtask verify --coverage` exits 1 with
the install hint above.

### 2.2 Full run

```bash
cargo run -p xtask -- verify --coverage
```

This does three things in order:

1. `cargo llvm-cov --workspace --all-features --locked --lcov
   --output-path lcov.info` — builds every member with coverage
   instrumentation, runs the workspace test suite, and writes an LCOV
   report to `lcov.info` in the repo root.
2. Runs `cargo llvm-cov report --package <pkg> --fail-under-lines 70`
   for each package in the hard-gate set (§3). The first failing gate
   does **not** short-circuit — the wrapper keeps going so one run
   surfaces every failing crate.
3. Prints a warn-only `cargo llvm-cov report --workspace` summary.

Exit code: `0` on all gates passing, `1` on any gate failure or on
cargo-llvm-cov itself returning non-zero.

### 2.3 Running `cargo llvm-cov` directly

The xtask wrapper is pure convenience; the underlying tool is
self-contained. You can target a single crate with:

```bash
cargo llvm-cov --package rdf-turtle --html
open target/llvm-cov/html/index.html
```

`--html` produces a per-file browser-readable report that the lcov
artefact does not. Use it when triaging a specific regression.

## 3. Threshold matrix

| Package           | Gate              | Rationale                                                                         |
|-------------------|-------------------|-----------------------------------------------------------------------------------|
| `rdf-iri`         | **hard**, 70 %    | Phase-A parser (ADR-0017 §4). Drives IRI resolution across every downstream lang. |
| `rdf-ntriples`    | **hard**, 70 %    | Phase-A parser (ADR-0017 §4). Smallest surface; anything below 70 % is a smell.   |
| `rdf-turtle`      | **hard**, 70 %    | Phase-A parser (ADR-0017 §4). Largest grammar in Phase A; the hardest to keep up. |
| `rdf-diagnostics` | **hard**, 70 %    | Shared diagnostic infra (ADR-0018 Wave 1). Regressions here silently poison UX.   |
| everything else   | warn-only         | Test-harness, shadow, and xtask crates — exercised indirectly.                    |

Two sources of truth to keep in sync when tuning:

1. `COVERAGE_HARD_GATED_PACKAGES` / `COVERAGE_FAIL_UNDER_LINES` in
   `xtask/verify/src/main.rs`.
2. The four `Gate <pkg>` steps in `.github/workflows/coverage.yml`.

When you change one, change the other in the same PR — a drift is
always a bug.

### 3.1 Tuning the threshold

Two reasons you might want to move the 70 % floor:

- **Raise it.** Preferred direction once a parser stabilises. Bump in
  both files, land the PR, leave the new floor documented here.
- **Lower it (transiently).** Only acceptable to unblock a landing of a
  planned refactor. Open an issue, note the expected recovery timeline,
  and revert the threshold within two sweeps. Do not lower below 60 %
  without an ADR amendment — below that we lose the signal.

Changing the gated-crate set requires an ADR amendment (ADR-0020 §1.2
is explicit about which four crates are hard-gated).

## 4. Interpreting reports

### 4.1 lcov.info artefact

CI uploads `lcov.info` on every run (PR, cron, manual dispatch) at:

> Actions → coverage → `<run>` → Artifacts → `lcov-report`

Consume locally with any lcov reader:

```bash
# genhtml ships with lcov (brew install lcov)
genhtml lcov.info --output-directory coverage-html
open coverage-html/index.html
```

The LCOV format is line- and branch-granular. Pay attention to the
"UNC" (uncovered-line) markers in crates that just dropped below the
gate — they point at the exact ranges to cover.

### 4.2 Per-crate summaries

Each `cargo llvm-cov report --package <pkg>` in CI prints a block like:

```
Filename                    Regions   Missed Regions  Cover  Functions  Missed Functions  Executed  Lines   Missed Lines  Cover
----------------------------------------------------------------------------------------------------------------------------
src/parser.rs                   132              18  86.36%         14                 1   92.86%    541            39  92.79%
src/lib.rs                       12               0 100.00%          2                 0  100.00%     41             0 100.00%
----------------------------------------------------------------------------------------------------------------------------
TOTAL                           144              18  87.50%         16                 1   93.75%    582            39  93.30%
```

The `--fail-under-lines 70` step keys off the `TOTAL` line's line-cover
percentage.

### 4.3 Deliberately ignored targets

`cargo-llvm-cov` excludes test binaries, build scripts, and
doc-examples from the instrumented build automatically — we do not
pass any `--ignore-filename-regex` flags because the defaults already
match our convention (`target/**`, `tests/**`, `xtask/**`). If that
changes upstream, add the explicit flag here and in the workflow
rather than relying on the default.

## 5. CI reports

- **Trigger**: every PR targeting `main`, weekly cron (Mon 03:17 UTC),
  and `workflow_dispatch`.
- **Where**: Actions → coverage.
- **Artefact**: `lcov-report` (retention: 14 days).
- **No external upload**: Codecov / Coveralls integration is deferred
  pending an ADR-0020 amendment. When that lands, wire the upload step
  between "Upload lcov.info artefact" and the gating steps — keep the
  artefact upload regardless so we have an offline fallback.

## 6. Verifying the gate fires

During the Phase-A sweep we deliberately broke a test to confirm the
gate actually blocks a merge. Reproduce with:

```bash
# In a scratch branch:
# 1. Comment out enough of a covered code path in rdf-ntriples so that
#    its line coverage drops below 70%.
# 2. Run the wrapper.
cargo run -p xtask -- verify --coverage

# Expected:
# xtask verify --coverage: rdf-ntriples below 70% line coverage
# exit 1
```

Push the branch, confirm the `coverage` workflow run goes red, then
revert. **Never land the transient break** — it is a one-shot
acceptance check.

## 7. Known deviations / TODO

- Codecov upload is intentionally absent until ADR-0020 is amended.
- `--all-features` is used rather than a feature matrix because
  Phase-A crates do not yet expose non-default features that gate
  coverage-visible code; revisit when the feature graph grows.
- Branch-coverage (`--branch`) is not currently enforced — Rust's
  source-based branch coverage is still flagged experimental upstream.
  Track the stabilisation and promote when ready.
