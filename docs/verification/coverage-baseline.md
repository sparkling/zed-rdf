# Coverage baseline — per-crate ratchet

Owner: `cu2-cov-thresholds` (carry-over-v2 sweep).
Scope: machine-consumed ratchet file for `.github/workflows/coverage.yml`
and `xtask verify --coverage`. Also human-readable — keep both uses in
mind when editing.

## Why this file is tracked

`.claude-flow/` is gitignored (see `.gitignore` line 40), so any
ratchet data stored there would not survive a fresh clone and CI would
have nothing to compare against. We therefore persist the baseline at
this tracked path. The drift-guard step in the coverage workflow reads
the `BASELINE` table below; any regression greater than the drift
tolerance (§3) fails the gate.

## 1. Format contract

The `BASELINE` table is parsed by the drift-guard step. **Do not
rename the table or its headers.** Each row:

| Column      | Meaning                                                                                     |
|-------------|---------------------------------------------------------------------------------------------|
| `package`   | Cargo package name (lockstep with `--package <pkg>` in the workflow + xtask).               |
| `baseline`  | Committed line-coverage percentage (float, 2 decimals). Raise only; never lower silently.   |
| `threshold` | Hard-gate floor in percent. Set to `warn` for warn-only crates.                             |
| `notes`     | Short justification — cited when the threshold or baseline moves.                           |

Drift tolerance: **3 percentage points**. A run whose measured
line-cover is more than 3 pp below the committed `baseline` for any
row with a numeric `threshold` fails the drift-guard step.

## 2. BASELINE

<!-- BEGIN-BASELINE — parsed by scripts/coverage-drift.sh; do not reformat. -->

| package           | baseline | threshold | notes                                                                                                   |
|-------------------|---------:|----------:|---------------------------------------------------------------------------------------------------------|
| rdf-diagnostics   |    91.19 |        80 | Small crate, simple; actual well above floor. Threshold per ADR-0020 carry-over.                        |
| rdf-iri           |    84.39 |        70 | IDN fallback + resolve edges still have uncovered paths; threshold leaves headroom for those to land.   |
| rdf-ntriples      |    80.38 |        75 | Single-file parser, high coverage already; tight floor keeps accidental regressions visible.            |
| rdf-turtle        |    77.84 |        70 | Largest grammar in Phase A; lexer still has 75.92% line cover. 70% floor ratchets as coverage improves. |
| rdf-diff          |    94.04 | warn      | Harness crate; exercised via downstream parser tests. Warn-only until the real diff harness lands.      |

<!-- END-BASELINE -->

## 3. Drift guard

For every row whose `threshold` is a number, the drift-guard step
computes `measured − baseline`. If that delta is less than `−3.00`,
the step fails with a message naming the package, the committed
baseline, the measured value, and the delta. Rows with `threshold =
warn` are informational only (printed, never fail).

When coverage legitimately improves, bump the `baseline` column in the
same PR that lands the improvement — this ratchets the floor. When it
legitimately regresses (e.g. a refactor removed a test path), adjust
`baseline` down **and** add a row in §4 documenting why. Do not move
`threshold` without an accompanying entry in §4.

## 4. Adjustments log

Append-only. Newest entries at the bottom.

- **2026-04-19 — initial baselines** (`cu2-cov-thresholds`). First
  measurement under the `cu2-cov-thresholds` tuning pass. All four
  hard-gated crates are already above their proposed thresholds so no
  `baseline − 5pp` adjustment was needed. `rdf-diff` stays warn-only
  because its coverage is driven almost entirely by downstream tests
  (the harness surface is thin); a hard floor there would be a proxy
  for parser-test coverage, which is already gated elsewhere.

## 5. How to regenerate

```bash
# Prerequisites: rustup component add llvm-tools-preview
#                cargo install cargo-llvm-cov --locked
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-features --summary-only
# Read TOTAL rows from per-package summaries:
for pkg in rdf-diagnostics rdf-iri rdf-ntriples rdf-turtle rdf-diff; do
  printf '%s: ' "$pkg"
  cargo llvm-cov report --package "$pkg" --summary-only 2>/dev/null \
    | awk '/^TOTAL/ {print $(NF-2)}'
done
```

Then edit the BASELINE table in §2 with the new `Lines Cover` numbers
and add an entry to §4. Run `cargo run -p xtask -- verify --coverage`
locally to confirm the new thresholds pass.
