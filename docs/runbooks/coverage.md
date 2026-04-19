# Runbook — coverage with `cargo-llvm-cov`

Owner: `cu-coverage` (ADR-0020 §1.2) + `cu2-cov-thresholds` (carry-over-v2).
Scope: per-crate line-coverage measurement, per-crate hard gates tuned
in the baseline file, and the drift-guard ratchet.
Companions:

- [`.github/workflows/coverage.yml`](../../.github/workflows/coverage.yml)
- [`xtask/verify/src/main.rs`](../../xtask/verify/src/main.rs)
- [`scripts/coverage-drift.sh`](../../scripts/coverage-drift.sh)
- [`docs/verification/coverage-baseline.md`](../verification/coverage-baseline.md)
  — authoritative ratchet file.

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

This does four things in order:

1. `cargo llvm-cov --workspace --all-features --locked --lcov
   --output-path lcov.info` — builds every member with coverage
   instrumentation, runs the workspace test suite, and writes an LCOV
   report to `lcov.info` in the repo root.
2. Runs `cargo llvm-cov report --package <pkg> --fail-under-lines N`
   for each row in the hard-gate set (§3), where `N` is the row's
   `threshold`. The first failing gate does **not** short-circuit — the
   wrapper keeps going so one run surfaces every failing crate.
3. Runs [`scripts/coverage-drift.sh`](../../scripts/coverage-drift.sh),
   which compares today's measured line-cover against the committed
   baseline in
   [`docs/verification/coverage-baseline.md`](../verification/coverage-baseline.md)
   and fails if any tracked crate has regressed by more than
   `DRIFT_TOLERANCE_PP` (default 3 pp).
4. Prints a warn-only `cargo llvm-cov report --workspace` summary.

Exit code: `0` on all gates + drift guard passing, `1` on any gate
failure, drift regression, or cargo-llvm-cov itself returning non-zero.

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

Thresholds are tuned per-crate in the **authoritative baseline file**:
[`docs/verification/coverage-baseline.md`](../verification/coverage-baseline.md).
Current matrix (reproduced here for convenience; the baseline file is
the source of truth — if they disagree, the baseline file wins):

| Package           | Gate              | Baseline | Rationale                                                                             |
|-------------------|-------------------|---------:|---------------------------------------------------------------------------------------|
| `rdf-diagnostics` | **hard**, 80 %    |    91.19 | Shared diagnostic infra (ADR-0018 Wave 1). Small + simple — should easily clear.      |
| `rdf-iri`         | **hard**, 70 %    |    84.39 | Phase-A parser. IDN fallback + resolve edges still need future coverage.              |
| `rdf-ntriples`    | **hard**, 75 %    |    80.38 | Phase-A parser. Single-file; tight floor keeps accidental regressions visible.        |
| `rdf-turtle`      | **hard**, 70 %    |    77.84 | Phase-A parser. Largest grammar in Phase A; the hardest to keep up.                   |
| `rdf-diff`        | warn-only         |    94.04 | Harness crate, exercised via downstream parser tests.                                 |
| everything else   | warn-only         |        — | Shadow, oracle, and xtask crates — exercised indirectly.                              |

Three sources of truth to keep in sync when tuning:

1. [`docs/verification/coverage-baseline.md`](../verification/coverage-baseline.md)
   — the `BASELINE` table (parsed by `scripts/coverage-drift.sh`).
2. `COVERAGE_HARD_GATES` in `xtask/verify/src/main.rs`.
3. The `Gate <pkg>` steps in `.github/workflows/coverage.yml`.

When you change one, change the others in the same PR — a drift is
always a bug.

### 3.1 Tuning the threshold

Procedure for moving a hard-gate threshold (raise or lower):

1. Edit the `threshold` column in the baseline file for the affected
   row.
2. Edit the corresponding entry in `COVERAGE_HARD_GATES`
   (`xtask/verify/src/main.rs`).
3. Edit the corresponding `Gate <pkg>` step in
   `.github/workflows/coverage.yml`.
4. Append a short paragraph to the baseline file's §4 (Adjustments log)
   with the rationale — when coverage moves, we want the history.

Direction guidance:

- **Raise the threshold.** Preferred direction once a parser stabilises.
  Ratchets the floor; matches the baseline ratchet (§3.3 below).
- **Lower the threshold (transiently).** Only acceptable to unblock a
  landing of a planned refactor. Open an issue, note the expected
  recovery timeline, and revert the threshold within two sweeps. Do
  not lower below 60 % without an ADR amendment — below that we lose
  the signal.
- **Day-one adjustment.** If a newly-added hard-gated crate has a
  baseline below its proposed threshold, lower the threshold to
  `baseline − 5 pp` and document the expected recovery in §4 of the
  baseline file. (Not needed for the initial 2026-04-19 baseline — all
  four hard-gated crates are already above their thresholds.)

Changing the gated-crate **set** (adding or removing a package from
the hard-gate list) requires an ADR amendment (ADR-0020 §1.2 is
explicit about which crates are hard-gated).

### 3.2 Drift guard (ratchet)

The [`scripts/coverage-drift.sh`](../../scripts/coverage-drift.sh)
step re-reads the `BASELINE` table in the baseline file on every CI
run and every `xtask verify --coverage` invocation. For each row:

- **hard-gated rows** (numeric `threshold`) fail the gate when
  - `measured < threshold` (the existing `--fail-under-lines` rule, and
    the drift-guard's sanity check), *or*
  - `measured − baseline < -3.00` (regression larger than tolerance).
- **warn-only rows** print the delta but never fail.

The tolerance (`DRIFT_TOLERANCE_PP`, default `3.00`) is overridable by
env var for debugging but must not be loosened in CI without an entry
in the baseline file's §4.

### 3.3 Ratcheting the baseline

When coverage legitimately improves (e.g. new tests landed), bump the
`baseline` column for the affected row in the same PR that lands the
improvement. The drift guard then holds the new, higher floor — any
future regression greater than 3 pp against the improved baseline
fails CI. This is how the ratchet earns its keep: you never need to
remember to raise a threshold — just keep the baseline honest.

If coverage legitimately regresses (e.g. a deliberate refactor removed
a test path), lower the `baseline` column **and** add a row in the
baseline file's §4 explaining why. Do not touch `threshold` for a
legitimate regression unless the regression pushed the baseline below
the threshold; in that case see §3.1.

### 3.4 Regenerating the baseline

See the baseline file's §5 — a short script-in-the-clear that dumps
per-package line cover and is meant to be copy-pasted. Always run it
from a clean tree (`cargo llvm-cov clean --workspace`) so stale
profdata from previous partial runs cannot leak in.

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
gate actually blocks a merge. Two failure modes to exercise:

**Hard-gate failure.** In a scratch branch, comment out enough of a
covered code path in `rdf-ntriples` so that its line coverage drops
below 75 %. Run the wrapper:

```bash
cargo run -p xtask -- verify --coverage
# Expected:
# xtask verify --coverage: rdf-ntriples below 75% line coverage
# exit 1
```

**Drift failure.** Edit the `baseline` column of `rdf-turtle` in
`docs/verification/coverage-baseline.md` up by more than 3 pp
(e.g. 77.84 → 82.00). Run the wrapper:

```bash
cargo run -p xtask -- verify --coverage
# Expected drift-guard table row:
# rdf-turtle   82.00   70   77.84   -4.16   FAIL: drifted -4.16 pp (tol -3.00)
# exit 1
```

Push each scratch branch, confirm the `coverage` workflow run goes red,
then revert. **Never land either transient break** — they are one-shot
acceptance checks.

## 7. Known deviations / TODO

- Codecov upload is intentionally absent until ADR-0020 is amended.
- `--all-features` is used rather than a feature matrix because
  Phase-A crates do not yet expose non-default features that gate
  coverage-visible code; revisit when the feature graph grows.
- Branch-coverage (`--branch`) is not currently enforced — Rust's
  source-based branch coverage is still flagged experimental upstream.
  Track the stabilisation and promote when ready.
