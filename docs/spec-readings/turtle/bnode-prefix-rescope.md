# Pin: Turtle blank-node scoping across `@prefix` redefinitions and TriG graph blocks

- **Diagnostic code:** `TTL-BNPFX-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Productions:** `BLANK_NODE_LABEL`, `anon`, `@prefix`/`PREFIX`
  directives, TriG `graphNameNode` / `wrappedGraph` (Turtle §2.6,
  TriG §2.2, §3).
- **Spec target:** RDF 1.1 Turtle <https://www.w3.org/TR/turtle/>;
  RDF 1.1 TriG <https://www.w3.org/TR/trig/>.
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From Turtle §2.6 "RDF Blank Nodes":

> "Blank nodes are given labels in the form `_:abc`. A blank node
> label is local to the document it is in. Outside of the document
> scope, arbitrary blank node labels have no meaning."

From TriG §3 "Quads":

> "Each blank node identifier is local to the document it is in."
> (TriG §2.2 describes graph blocks.)

Two distinct ambiguities are mixed under "local to the document" in
practice:

1. **Prefix redefinition and blank-node scope.** A document may
   redefine `@prefix ex: <…>` mid-stream. `@prefix` binds **prefix
   names to IRIs** — it does **not** bind or re-scope blank-node
   labels. But a naive implementation that bundles "naming tables"
   (prefixes + bnodes) under one lexical scope may inadvertently
   rescope `_:b` on a later `@prefix` re-declaration.
2. **TriG graph-block scope.** TriG §3 says blank-node identifiers
   are "local to the document"; TriG §2.2 describes distinct graph
   blocks. The spec is silent on whether `_:b` inside graph
   `<http://example/g1>` is the **same** blank node as `_:b` inside
   graph `<http://example/g2>` or a **different** one.

## Reading chosen

1. **`@prefix` / `PREFIX` / `@base` / `BASE` redefinitions DO NOT
   rescope blank-node labels.** A blank-node label is scoped to the
   **whole document** once and for all. Redefining a prefix mid-way
   through the document produces a different IRI for future prefixed
   names, but `_:b` before and after the redefinition is the **same**
   blank node.
2. **TriG graph blocks DO NOT rescope blank-node labels either.**
   `_:b` in graph `<g1>` and `_:b` in graph `<g2>` inside the same
   TriG document refer to the **same** blank node. If an author
   wants two distinct blank nodes they must use two distinct labels
   (`_:b1`, `_:b2`) or rely on `[]`/`anon` freshness. This is the
   document-scope reading, consistent with both Turtle §2.6 and
   TriG §3's "local to the document".

Both readings produce predictable round-tripping against the canonical
fact set defined by `crates/testing/rdf-diff/src/lib.rs`.

## Rationale

- Turtle §2.6 says "local to the document"; it does not say "local to
  the smallest lexical scope in which it appears". `@prefix` /
  `PREFIX` directives are prefix-IRI bindings (Turtle §2.4), not
  naming scopes for anything else. Conflating them is a refactor
  hazard that shows up when a parser maintains a single `HashMap`
  of "named things" reset at every directive.
- The cohort-B adversary brief `docs/verification/adversary-findings/ttl.md`
  Failure Mode 8 argues the **opposite** reading for TriG — that
  each graph block is its own bnode scope — and cites TriG §2.2. That
  reading is a plausible one; we reject it here and record the
  rejection explicitly:
  - TriG §3 repeats Turtle's "local to the document" phrasing; no
    sentence in TriG narrows it to per-graph-block. If the WG had
    intended per-block scoping the spec would say so (as SPARQL
    CONSTRUCT §10.3 does for its own blank-node scoping).
  - The RDF 1.1 TriG test manifest includes
    `trig-syntax-bnode-09.trig` which shares `_:a` across graph
    blocks and expects the **same** blank node. Per-block scoping
    would fail that manifest entry.
  - Adopting per-block scoping would also diverge from what
    `oxttl` emits on its TriG adapter (the ADR-0019 §1 oracle),
    which would make the diff harness report "real" divergences
    that are purely a spec-reading choice. Pinning the document-
    scope reading keeps the harness clean.
- The cohort-B veto log records `TTL-008` as a High finding against
  cohort A. With this pin in place, the "Reading chosen" is the
  authoritative arbitration; the adversary fixture moves from
  "veto" to "regression test for the pinned reading".

## Diagnostic code

- **Code:** `TTL-BNPFX-001`
- **Emitted by:** `rdf-turtle`, `rdf-turtle-shadow`, `oxttl` oracle
  adapter.
- **Message template:**
  `TTL-BNPFX-001: bnode label '_:<label>' resolved under document-scope rule at byte <offset>`
  (non-fatal trace emitted when a parser disagrees with its peer on
  this exact pin).
- **Fatal?** No. This pin governs a **choice**, not a rejection; the
  code surfaces in `DiffReport.triage_hint` when a divergence looks
  bnode-scope-shaped.

## Forward references

- `crates/syntax/rdf-turtle/SPEC.md` — TODO: cite `TTL-BNPFX-001`
  under "Pinned readings".
- `crates/syntax/rdf-turtle-shadow/` must implement the document-
  scope rule identically.
- Adversary fixtures: `tests/adversary-turtle/ttl-bnpfx-001-trig-*.trig`
  (cohort B) — positive fixtures where same-label-across-blocks is
  equal; negative fixtures where an author clearly wants two blank
  nodes and used two labels.
