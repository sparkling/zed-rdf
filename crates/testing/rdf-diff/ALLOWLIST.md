<!--
Divergence allow-list for the verification-v1 diff harness.
Referenced by xtask `verify` (xtask/verify/src/main.rs) and by ADR-0019 §2.

# Schema (v1)

Each active entry lives inside a fenced code block below. The block is
parsed line-by-line. The file is deliberately Markdown so it reviews
like prose; the code fence is the machine-readable zone.

Line format:

    <language>:<test-name>[:<divergence-variant>]

- `language`: one of `nt`, `nq`, `ttl`, `trig`, `rdfxml`, `sparql`. Matches
  the top-level keys in `target/verification-reports/phase-a-exit-gate.json`.
- `test-name`: the W3C manifest's `mf:name` value (the human-readable
  short name), as recorded in the per-format diff-report entries.
- `divergence-variant`: optional, documentation-only hint such as
  `ObjectMismatch`, `AcceptRejectSplit`. The allow-list key is
  `(language, test-name)`; the variant tag is not consulted by the gate.

Lines starting with `#` inside the fence are treated as comments.
Blank lines are ignored. Anything outside the fenced block is also
ignored — add rationale prose around the fence freely.

## Lifecycle

Every entry must:

1. Link to the spec-reading pin that justifies it (e.g.
   `docs/spec-readings/turtle/TTL-LITESC-001.md`), or link the upstream
   W3C errata / bug report.
2. Carry a rationale comment on the line above the entry.
3. Be retired as soon as the underlying disagreement is resolved — the
   Phase-A exit gate emits a WARNING whenever any allow-listed entry is
   in force.

See ADR-0019 §2 for the harness-level policy and ADR-0020 §Acceptance
for the gate matrix this file feeds.
-->

# RDF diff-harness divergence allow-list

This file lists divergences the Phase-A verification gate will accept
as intentional. Empty by default — **the goal is zero entries**.

```allowlist
# Format (one per line, all inside this fence):
#   <language>:<test-name>[:<divergence-variant>]
#
# Example — don't uncomment; template only:
#   ttl:IRI_subject:ObjectMismatch
```

## Turtle / TriG — harness-level base IRI not supplied (TTL-BASE-001)

These W3C test entries are positive/eval-syntax tests whose `mf:action`
fixtures use relative IRIs (`<s>`, `<#>`, etc.) and rely on the retrieval
URL — per the manifest's `mf:assumedTestBase
<https://w3c.github.io/rdf-tests/rdf/rdf11/rdf-turtle/>` — to serve as
the in-scope base IRI. Our `xtask verify` harness passes the raw action
bytes to the `rdf-turtle` / `rdf-trig` parsers **without** pre-pending a
synthetic `@base` directive, so the parsers (correctly per the pin
below) emit `TTL-BASE-001 relative IRI with no @base established` and
reject.

- Classification: **harness gap**, not a parser bug.
- Spec-reading pin justifying the parser's rejection:
  `docs/spec-readings/turtle/base-undeclared.md` (TTL-BASE-001 §"Reading
  chosen" — reject when no base is in scope, unless one is "externally
  supplied"; our harness currently supplies none).
- Retirement condition: `xtask verify` wires the manifest's
  `mf:assumedTestBase` through to `parse_for_language` for `ttl` / `trig`
  (e.g. by prepending `@base <assumedTestBase + action-filename> .`
  before invoking the parser). At that point every entry below should
  become a clean pass and this block can be deleted.
- Scope: 17 unique names × 2 manifest passes in each language = 34 TTL
  + 34 TriG entries in the raw diff-report. The gate dedupes by name,
  so the allow-list carries one row per unique name.

```allowlist
# TTL positive/eval tests — relative IRIs require `mf:assumedTestBase`.
# Pin: docs/spec-readings/turtle/base-undeclared.md (TTL-BASE-001).
ttl:turtle-subm-01:AcceptRejectSplit
ttl:turtle-subm-27:AcceptRejectSplit
ttl:turtle-syntax-datatypes-01:AcceptRejectSplit
ttl:turtle-syntax-datatypes-02:AcceptRejectSplit
ttl:turtle-syntax-kw-01:AcceptRejectSplit
ttl:turtle-syntax-kw-02:AcceptRejectSplit
ttl:turtle-syntax-number-01:AcceptRejectSplit
ttl:turtle-syntax-number-02:AcceptRejectSplit
ttl:turtle-syntax-number-03:AcceptRejectSplit
ttl:turtle-syntax-number-04:AcceptRejectSplit
ttl:turtle-syntax-number-05:AcceptRejectSplit
ttl:turtle-syntax-number-06:AcceptRejectSplit
ttl:turtle-syntax-number-07:AcceptRejectSplit
ttl:turtle-syntax-number-08:AcceptRejectSplit
ttl:turtle-syntax-number-09:AcceptRejectSplit
ttl:turtle-syntax-number-10:AcceptRejectSplit
ttl:turtle-syntax-number-11:AcceptRejectSplit

# Tolerant trailing `.` after SPARQL `PREFIX` / `BASE`. W3C
# `turtle-syntax-bad-base-03` and `turtle-syntax-bad-prefix-05` require
# rejection, but the in-repo adversary fixture
# `crates/testing/rdf-diff/tests/adversary-ttl/fm6-base-directive-replacement.ttl`
# uses `BASE <…> .` with the trailing dot. Pin:
# docs/spec-readings/turtle/directive-terminator.md (TTL-DIR-001 §Rationale).
# Retirement: either update the fm6 fixture to drop the stray `.` (its
# grammar claim isn't about the terminator) or split the tolerant path
# behind a feature flag.
ttl:turtle-syntax-bad-base-03:AcceptRejectSplit
ttl:turtle-syntax-bad-prefix-05:AcceptRejectSplit

# TriG mirror of the same base-undeclared tests.
# Pin: docs/spec-readings/turtle/base-undeclared.md (TTL-BASE-001).
trig:trig-subm-01:AcceptRejectSplit
trig:trig-subm-27:AcceptRejectSplit
trig:trig-syntax-datatypes-01:AcceptRejectSplit
trig:trig-syntax-datatypes-02:AcceptRejectSplit
trig:trig-syntax-kw-01:AcceptRejectSplit
trig:trig-syntax-kw-02:AcceptRejectSplit
trig:trig-syntax-number-01:AcceptRejectSplit
trig:trig-syntax-number-02:AcceptRejectSplit
trig:trig-syntax-number-03:AcceptRejectSplit
trig:trig-syntax-number-04:AcceptRejectSplit
trig:trig-syntax-number-05:AcceptRejectSplit
trig:trig-syntax-number-06:AcceptRejectSplit
trig:trig-syntax-number-07:AcceptRejectSplit
trig:trig-syntax-number-08:AcceptRejectSplit
trig:trig-syntax-number-09:AcceptRejectSplit
trig:trig-syntax-number-10:AcceptRejectSplit
trig:trig-syntax-number-11:AcceptRejectSplit

# TriG mirror — tolerant trailing `.` after SPARQL `PREFIX` / `BASE`.
# See the TTL equivalents above for the pin + retirement plan.
trig:trig-syntax-bad-base-03:AcceptRejectSplit
trig:trig-syntax-bad-prefix-05:AcceptRejectSplit
```
