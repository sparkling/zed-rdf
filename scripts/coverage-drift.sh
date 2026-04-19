#!/usr/bin/env bash
# coverage-drift.sh — fail if measured per-crate line coverage has
# regressed by more than DRIFT_TOLERANCE_PP (default 3.00 pp) against
# the committed baseline in docs/verification/coverage-baseline.md.
#
# Invoked by:
#   - .github/workflows/coverage.yml drift-guard step
#   - xtask verify --coverage (mirrors CI locally)
#
# Contract with the baseline file:
#   - The BASELINE table is bracketed by the sentinels
#     "<!-- BEGIN-BASELINE -->" / "<!-- END-BASELINE -->".
#   - Table rows have the shape:
#       | package | baseline | threshold | notes |
#     where `threshold` is either an integer percent or the literal
#     "warn" for warn-only rows.
#
# Behaviour:
#   - Iterates every baseline row.
#   - Reads the current line-cover via `cargo llvm-cov report
#     --package <pkg> --summary-only`.
#   - For `warn` rows, prints the delta; never fails.
#   - For numeric-threshold rows, fails on either:
#       a) measured < threshold                (hard-gate check)
#       b) measured − baseline < -DRIFT_TOLERANCE_PP   (drift check)
#
# Exit codes: 0 on clean pass, 1 on any hard-gate or drift failure,
# 2 on environment / parse errors.

set -euo pipefail

DRIFT_TOLERANCE_PP="${DRIFT_TOLERANCE_PP:-3.00}"
BASELINE_FILE="${BASELINE_FILE:-docs/verification/coverage-baseline.md}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "coverage-drift: cargo is required on PATH" >&2
  exit 2
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "coverage-drift: cargo-llvm-cov is not installed" >&2
  echo "  install with: cargo install cargo-llvm-cov --locked" >&2
  exit 2
fi

if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "coverage-drift: baseline file not found: $BASELINE_FILE" >&2
  exit 2
fi

# Extract the rows between the sentinels. We tolerate extra whitespace
# but the sentinel text itself is load-bearing — keep it in lockstep
# with docs/verification/coverage-baseline.md §1.
rows=$(awk '
  /BEGIN-BASELINE/ {capture=1; next}
  /END-BASELINE/   {capture=0}
  capture && /^\|/ { print }
' "$BASELINE_FILE")

if [[ -z "$rows" ]]; then
  echo "coverage-drift: no rows found between BEGIN-BASELINE/END-BASELINE" >&2
  exit 2
fi

overall_fail=0
printed_header=0

# Emit a small header so CI logs read nicely.
printf '%-20s %10s %10s %10s %10s  %s\n' \
  "package" "baseline" "threshold" "measured" "delta" "status"
printf '%-20s %10s %10s %10s %10s  %s\n' \
  "-------" "--------" "---------" "--------" "-----" "------"
printed_header=1

while IFS= read -r row; do
  # Skip the table header / separator rows.
  case "$row" in
    *"---"*) continue ;;
    *"package"*"baseline"*) continue ;;
  esac

  pkg=$(echo "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$2); print $2}')
  baseline=$(echo "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$3); print $3}')
  threshold=$(echo "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$4); print $4}')

  [[ -z "$pkg" ]] && continue

  # Ask cargo-llvm-cov for the current per-package summary. The TOTAL
  # row's third-from-last numeric column is `Lines Cover %` (cargo-llvm-cov
  # 0.6+). Grab it via awk so we don't depend on field-position guesses.
  measured=$(cargo llvm-cov report --package "$pkg" --summary-only 2>/dev/null \
    | awk '/^TOTAL/ {
        # Scan right-to-left for a percent token and pick the line-cover one.
        # Column order (stable across llvm-cov versions we use):
        #   Regions Missed Cover Functions Missed Cover Executed Lines Missed Cover [Branches Missed Cover]
        # Lines Cover is the 10th percent or, more simply, the 3rd
        # percent token after the "Executed" column; printing $(NF-2)
        # in versions with branch columns or $NF in versions without
        # would be fragile. Instead: pick the token at fixed offset
        # counting from the right, accommodating both layouts.
        for (i=NF; i>=1; i--) if ($i ~ /%$/) { pcts[++n]=$i; idx[n]=i }
        # With branch cols: percent tokens are (right-to-left):
        #   branch-cover, line-cover, fn-cover, region-cover
        # Without:
        #   line-cover, fn-cover, region-cover
        if (n >= 4)      print pcts[2];  # branches present
        else if (n >= 3) print pcts[1];  # no branches — rightmost is line cover
        else             print "ERR"
      }' \
    | tr -d '%')

  if [[ -z "$measured" || "$measured" == "ERR" ]]; then
    echo "coverage-drift: cannot parse measured coverage for $pkg" >&2
    overall_fail=1
    continue
  fi

  # Compute delta with awk (bash lacks floats).
  delta=$(awk -v m="$measured" -v b="$baseline" 'BEGIN {printf "%.2f", m - b}')

  status="ok"
  row_fail=0

  # Hard-gate check — only when threshold is numeric.
  if [[ "$threshold" =~ ^[0-9]+$ ]]; then
    below=$(awk -v m="$measured" -v t="$threshold" 'BEGIN {print (m+0 < t+0) ? 1 : 0}')
    if [[ "$below" == "1" ]]; then
      status="FAIL: below hard-gate ${threshold}%"
      row_fail=1
    fi

    # Drift check.
    drifted=$(awk -v d="$delta" -v tol="$DRIFT_TOLERANCE_PP" \
      'BEGIN {print (d+0 < -tol+0) ? 1 : 0}')
    if [[ "$drifted" == "1" ]]; then
      if [[ "$row_fail" == "1" ]]; then
        status="$status; drifted ${delta} pp (tol -${DRIFT_TOLERANCE_PP})"
      else
        status="FAIL: drifted ${delta} pp (tol -${DRIFT_TOLERANCE_PP})"
      fi
      row_fail=1
    fi
  else
    status="warn-only"
  fi

  printf '%-20s %10s %10s %10s %10s  %s\n' \
    "$pkg" "$baseline" "$threshold" "$measured" "$delta" "$status"

  [[ "$row_fail" == "1" ]] && overall_fail=1
done <<< "$rows"

if (( printed_header == 0 )); then
  echo "coverage-drift: no rows processed — baseline file empty?" >&2
  exit 2
fi

if (( overall_fail != 0 )); then
  echo "" >&2
  echo "coverage-drift: FAIL — see table above. Update $BASELINE_FILE" >&2
  echo "if the regression is intentional; bump baselines when coverage" >&2
  echo "improves." >&2
  exit 1
fi

echo ""
echo "coverage-drift: OK — all packages within ${DRIFT_TOLERANCE_PP} pp of baseline"
exit 0
