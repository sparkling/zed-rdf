---
agent: devils-advocate-respawn
cohort: hive-adr-0007
role: contrarian
date: 2026-04-20
---

# Brief — Devil's Advocate (hand-roll default)

## §1 Thesis

Phase A's hand-rolled success is survivorship bias from two trivially-regular
grammars; extending the default to `rdf-xml`, `rdf-jsonld`, `rdf-trix`, and
`rdf-n3` under Phase B's 4–6 week squeeze ships a maintenance-hostile,
diagnostic-mediocre parser stack whose real cost lands in Phase C (SPARQL)
and in LSP error-recovery quality — neither of which Phase A's gate measured.

## §2 Objections

1. **Claim:** The Phase A "prior" is not evidence for Phase B — NT is
   line-regular and Turtle's only hard production is the collection/bnode
   property-list pair, both now pinned.
   **Hook:** `crates/rdf-turtle/src/grammar.rs:76-127` — `Parser` is a flat
   recursive descent with a single `BTreeMap` state table; N3 adds quoted
   formulas (nested graph terms) and `@forAll`/`@forSome` scoping that the
   current `SubjectKind` enum (grammar.rs:39-50) cannot model without
   restructuring the dispatcher.
   **To overrule:** demonstrate that the `Parser` struct can host N3's
   formula nesting without reworking `parse_document`'s token-peek loop —
   or concede that `rdf-n3` is a rewrite, not an extension.

2. **Claim:** Hand-rolled recursive descent has a structural ceiling on
   LSP error recovery that combinator/CST crates lift for free.
   **Hook:** ADR-0004 allow-list explicitly names `rowan`/`cstree` for
   "lossless CST representation shared across parsers" (row in §"Allow-list
   (v1)"). Phase A parsers emit `(Fact, FactProvenance)` tuples, not a CST.
   A Zed LSP that can't mid-edit a malformed `@prefix` without aborting
   the whole document is a user-visible regression vs tree-sitter.
   **To overrule:** point at a hand-rolled recovery strategy in the current
   turtle grammar that survives a broken `IRIREF` mid-collection and
   returns partial facts with a diagnostic — or admit the ceiling.

3. **Claim:** ADR-0019 §3 mandates shadow implementations for `rdf-xml`
   and `rdf-jsonld`. Hand-rolling the main doubles the hand-roll cost
   because the shadow must also be hand-rolled to preserve cohort
   disjointness on *implementation technique*, not just prompt lineage.
   **Hook:** ADR-0019 §3: "Shadow ≠ main output on any input is a CI
   failure"; ADR-0021 §4 shadow roster has `pb-shadow-rdfxml` +
   `pb-shadow-jsonld` as `coder` roles with worktrees.
   **To overrule:** show that a `winnow`-main + hand-roll-shadow pair
   satisfies the independence axis *better* than hand-roll × hand-roll
   (which it does — different technique is a stronger disjointness
   signal than different prompt lineage alone).

4. **Claim:** Phase B's 4–6 week budget with 4 mains + 2 shadows in one
   spawn (ADR-0021 §2) does not absorb grammar-shape surprises. Hand-rolled
   RDF/XML is historically the genre's quicksand — `oxrdfxml` exists
   precisely because everyone who rolled their own gave up.
   **Hook:** ADR-0021 §5 adversary hive is 6 agents, at the 15-agent
   ceiling exactly. One RDF/XML "striped reification production surprised
   us" refactor burns the buffer.
   **To overrule:** show a hand-rolled RDF/XML parser in any comparable
   Rust project that landed its W3C eval suite in <4 weeks.

5. **Claim:** Diagnostic quality plateaus on hand-rolled descent without
   a CST layer. Phase A shipped diagnostics; they are anchor-and-message,
   not range-and-fixit.
   **Hook:** `grammar.rs:198-200` returns `Diag { code, message }` with a
   byte offset; no token-span tree, no recovery checkpoints. Extending
   this pattern to JSON-LD (where the LSP user edits inside a nested
   `@context` object) produces worse diagnostics than serde_json + a
   post-pass, which is the combinator-shaped approach.
   **To overrule:** exhibit a fix-it suggestion produced by the current
   turtle grammar on a mid-document error — or concede the plateau.

6. **Claim:** The SPARQL time bomb. SPARQL 1.1's grammar is materially
   larger than Turtle's and has real ambiguity (e.g. `PrefixedName` vs
   `VAR1` disambiguation in property paths). Phase C will need combinators
   or table-driven parsing; deciding "hand-roll by default" in ADR-0007
   locks in a dialect of hand-rolled helpers that Phase C then has to
   either extend or throw away.
   **Hook:** ADR-0004 §Allow-list, row "chumsky **or** winnow (ADR-0007)"
   — the row exists *because* the project anticipated this decision for
   non-trivial grammars. Deferring the decision to Phase C means paying
   the migration cost twice: once to adopt, once to refactor the Phase B
   parsers into the chosen combinator.
   **To overrule:** commit in the same ADR to hand-rolling SPARQL too,
   with a budget — or accept that Phase B is the cheapest place to
   introduce the combinator, before it becomes load-bearing.

7. **Claim:** ADR-0004 itself signalled doubt. The phrase "Writing
   Turtle/SPARQL by hand is viable but slower" is in the "Why we don't
   reimplement" column of the chumsky/winnow row — the allow-list
   *already admits* the combinator crates and defers only the pick.
   Arguing for hand-roll Phase B means arguing against the original ADR-0004
   author's hedge.
   **Hook:** ADR-0004 §"Allow-list (v1)", chumsky/winnow row.
   **To overrule:** explain why "slower" stopped being a cost now that
   Phase B has a tighter budget than Phase A did.

## §3 Concessions

Hand-rolling wins legitimately where the grammar fits on one page and the
diagnostic surface is coarse:

- **rdf-trix** — an XML wrapper over N-Triples content. The XML layer is
  `quick-xml` anyway (ADR-0021 §Context); the NT layer reuses
  `crates/rdf-ntriples`. Nothing to gain from a combinator here.
- **rdf-ntriples** (already shipped) — line-regular; hand-roll was right.
- **Turtle** (already shipped) — vindicated by the 100% W3C gate. The
  maintenance case only opens later when LSP recovery requirements tighten.

For `rdf-xml` and `rdf-jsonld`, the shape argument flips: RDF/XML's
striped syntax and JSON-LD's `@context` resolver genuinely benefit from
combinator composition, and the shadow requirement (ADR-0019 §3) makes
two hand-rolls strictly more expensive than one combinator + one
hand-roll shadow — which also gives stronger independence.

## §4 Predictions if overruled

1. **RDF/XML shadow diverges on striped-syntax reification, both are
   wrong, neither catches it.** Surfaces in `crates/testing/rdf-diff/`
   against `oxrdfxml` (oracle) in the first post-spawn integration pass
   (ADR-0021 §6). Root cause: two hand-rolled parsers from the same
   LLM prior share the same misreading of RDF/XML §2.17 (the "rdf:li"
   container-membership production), which prompt-lineage disjointness
   alone does not break. Signal: ADR-0019 §Validation's "zero divergences
   on first run is suspicious" fires, and the investigation lands here.

2. **Phase C SPARQL lands `winnow` anyway and Phase B's rdf-xml is
   rewritten.** Surfaces 8–12 weeks after `phase-b/done` tag, in the
   SPARQL property-path work. The refactor PR touches `crates/rdf-xml/`
   to share a combinator-based IRI-production helper; reviewers note the
   duplicate hand-rolled IRI validator. Cost: ~2 weeks of unplanned
   work charged to Phase C's budget, plus a silent regression window on
   rdf-xml diagnostics during the refactor.
