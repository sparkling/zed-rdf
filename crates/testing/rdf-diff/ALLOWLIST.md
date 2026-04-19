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

<!--
Historical note: the 2026-04-19 Phase-A exit gate closed the two open
classes once carried here.

- Class G (TTL-BASE-001 harness-level base IRI not supplied — 68 entries,
  17 unique names × 2 manifest passes × {ttl, trig}) retired by wiring
  `mf:assumedTestBase` + action filename into `xtask verify`'s
  `parse_for_language` via `TurtleParser::parse_with_base` /
  `TriGParser::parse_with_base`. See `docs/verification/adversary-
  findings/{ttl,trig}/w3c-divergences.md` → class G.

- Class H (tolerant trailing `.` after SPARQL-style `PREFIX` / `BASE` —
  4 entries) retired by (a) dropping the stray `.` from
  `crates/testing/rdf-diff/tests/adversary-ttl/fm6-base-directive-
  replacement.ttl` and (b) tightening the `rdf-turtle` grammar to reject
  a `.` after the SPARQL-style productions per Turtle §6.5
  (`sparqlPrefix`, `sparqlBase`). See
  `docs/spec-readings/turtle/directive-terminator.md` for the updated
  pin, and the `sparql_*_with_trailing_dot_rejected` smoke tests for the
  coverage.

Goal remains zero entries. Add new entries only when a genuine harness
gap or upstream bug is captured, with the retirement plan spelled out
per the schema above.
-->

