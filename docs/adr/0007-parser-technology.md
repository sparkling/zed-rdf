# ADR-0007: Parser technology for the RDF family — hand-rolled default, combinator library deferred

- **Status:** Accepted
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen (queen, orchestrator session); hive
  briefs from Expert A (Phase A precedent), Expert B (combinator
  landscape), Expert C (Phase B format requirements), Expert D
  (dependency policy), and Devil's Advocate.
- **Supersedes:** —
- **Instantiates:** [ADR-0004](0004-third-party-crate-policy.md)
  §Allow-list row naming "`chumsky` **or** `winnow` (ADR-0007)".
- **Resolves:** the deferred choice in ADR-0004 line 67.
- **Review:** same-session decision; Devil's Advocate final review
  incorporated per
  [`0007-briefs/devils-advocate-final-review.md`](0007-briefs/devils-advocate-final-review.md)
  (Block × 1, Amend × 5, Note × 3 — all addressed).
- **Tags:** `policy`, `parsers`, `tooling`, `phase-b`

## Context and Problem Statement

ADR-0004 §Allow-list admitted "`chumsky` **or** `winnow` (ADR-0007)"
as a single deferred entry — parser-combinator libraries for complex
grammars, with the choice postponed pending empirical signal. Phase A
shipped `rdf-ntriples` (1189 lines, hand-rolled, single file) and
`rdf-turtle` (2166 lines, hand-rolled lexer + grammar + iri + diag
modules; no combinator library) through the W3C strict exit gate at
tag `phase-a/done` with zero allow-list entries
([commit b8c236d](../../)). ADR-0021 §1.1 requires ADR-0007 be
Accepted before the Phase B single-shot spawn, so Phase B coders know
whether to extend the Phase A shape or switch to a combinator
toolchain.

Phase B's four formats (`rdf-xml`, `rdf-jsonld`, `rdf-trix`, `rdf-n3`)
have heterogeneous grammar shapes: RDF/XML is an XML-event-driven
attribute grammar; JSON-LD is a tree walk over `serde_json::Value`;
TriX is a tiny XML envelope around N-Triples content; N3 is a Turtle
superset. Parser-combinator libraries optimise for one of those shapes
(token-stream text grammars); the other three do not play to
combinator strengths. The decision must be grounded in the grammars
themselves, not tool preference.

Out of scope for this ADR: LSP error-recovery retrofit (Phase F),
SPARQL parser technology (Phase C — see §Reopen triggers), ADR-0021's
operational plan.

## Decision Drivers

- **Grammar-shape fit.** Each Phase B format has distinct parser
  requirements; one-size-fits-all tooling is unlikely to be optimal.
- **Phase A empirical signal.** Two hand-rolled parsers cleared the
  strict W3C exit gate at 100 %; the shape is proven for the classes
  it covered.
- **Supply-chain posture (ADR-0004).** Every added runtime dep is
  attack surface; admission is deliberately frictional.
- **WASM / Zed-extension portability.** The workspace ships a Zed
  extension targeting wasm32. Deps that pull C toolchains
  (`cc` / `psm` / `libc`) compromise that story.
- **Shadow disjointness (ADR-0019 §3).** `rdf-xml` and `rdf-jsonld`
  ship with disjoint-cohort shadows for independent verification.
- **LSP quality plateau.** LSP-grade error recovery is the unmet
  quality bar; neither Phase A parser carries it yet.
- **Phase B wall-clock budget.** 4–6 weeks, 15 agents in one spawn at
  the ADR-0017 ceiling; grammar-shape surprises have no slack.

## Considered Options

1. **Hand-roll all Phase B parsers**, continuing the Phase A pattern.
   - *Pros.* Zero supply-chain delta; Phase A code shape and review
     muscle carry over; direct grammar-to-code mapping where spec
     language is most load-bearing (NT trailing-dot; Turtle number /
     terminator disambiguation); exit cost zero.
   - *Cons.* Duplicated hand-coded backtracking, lookahead, and
     header-dispatch patterns that a combinator library would make
     declarative; no built-in error-recovery API; maintenance drag
     scales with grammar size.

2. **Adopt `chumsky` 1.0 uniformly** for text-grammar parsers
   (`rdf-turtle`, `rdf-n3`, future `sparql-syntax`), hand-roll the
   XML/JSON shims (`rdf-xml`, `rdf-trix`, `rdf-jsonld`).
   - *Pros.* Best-in-class error-recovery API (`recover_with`,
     synchronisation tokens); declarative span carry
     (`Spanned<T>`); uniform diagnostic shape across Turtle-family
     parsers; ergonomic fit for LSP retrofit.
   - *Cons.* Heavy compile-time cost; default features pull `stacker`
     → `psm` → `cc` build-dep → `libc` / `windows-sys` (per Expert D
     §2), which requires a C toolchain and compromises the WASM
     story unless `default-features = false` is pinned;
     monomorphisation-heavy binary; significant port effort for the
     already-shipped `rdf-turtle`; medium exit cost.

3. **Adopt `winnow` uniformly** for text-grammar parsers, hand-roll
   XML/JSON shims.
   - *Pros.* Small pure-Rust dep graph (≈1–3 transitives); nom
     author's successor, active maintenance; WASM-clean; compile-time
     cost closer to hand-roll than chumsky; ergonomics closer to
     imperative recursive descent, which lowers port effort.
   - *Cons.* Error-recovery API is manual (`cut_err` + bespoke
     fallbacks), narrowing the LSP-quality benefit that is the main
     reason to adopt a combinator library at all; still a runtime
     dep expansion, still has an exit cost.

4. **Mixed — combinator library (chumsky or winnow) for rdf-n3 and
   rdf-xml main parsers, hand-roll elsewhere**, with hand-rolled
   shadows for independent verification.
   - *Pros.* Addresses the Devil's Advocate §2 shadow-cost objection
     (different technique = stronger disjointness than prompt
     lineage alone); concentrates combinator investment on the two
     formats where error-recovery benefit is highest; bounds the
     blast radius of the dep admission.
   - *Cons.* Two parser dialects in the workspace to maintain;
     Expert C §1 notes RDF/XML is attribute-polymorphic over XML
     events, not token-sequential — combinator value there is
     weaker than the intuition suggests; N3 quoted-formula /
     graph-block ambiguity (Expert A §3, Expert C §5) is real but
     Phase B scopes N3 to the Turtle-equivalent subset, which the
     hand-roll already handles.

## Decision

**Chosen option: Option 1 — hand-roll all Phase B parsers.** Close
the deferred ADR-0004 "`chumsky` **or** `winnow`" row by replacing
it with an explicit "hand-roll; revisit per ADR-0007" note. Keep
the existing allow-list infrastructure crates (`logos`, `quick-xml`,
`serde_json`) that each Phase B parser consumes; the decision affects
only the grammar layer above those tokenisers.

**Per-format binding** (follows Expert C's §1 verdict cards):

- **`rdf-xml`** — hand-rolled state machine over `quick-xml` events.
  Model `enum ElementMode { NodeElt, PropElt, ParseTypeLiteral,
  ParseTypeCollection, ParseTypeResource }` with a stack of
  `(mode, subject, base, lang)` frames.
- **`rdf-jsonld`** — hand-rolled tree walk over `serde_json::Value`
  with a `ContextStack` side structure. Scope: syntax +
  `@context` well-formedness only; expand / compact / flatten /
  frame / URDNA2015 deferred to Phase E per ADR-0021 §Consequences.
  Remote `@context` loading is hard-rejected at parse time.
- **`rdf-trix`** — hand-rolled event matcher over `quick-xml`.
  ~200 lines of XML event matching sits below the factoring
  threshold; a shared utility crate would re-export `quick-xml`
  types and add a dep edge for no meaningful code-reduction
  benefit. **No shadow** (ADR-0021 §Consequences line 231–232), so
  independence disjointness is not a rationale here — the decision
  is purely factoring economics. Term-level validators reuse
  `rdf-ntriples`'s.
- **`rdf-n3`** — hand-rolled recursive-descent extension of
  `rdf-turtle`. **Phase B scopes N3 to the Turtle-equivalent
  subset** — quoted formulas (`{ … }` as a term), universal /
  existential variables (`@forAll`, `@forSome`, `?x`), and rule
  productions (`=>` / `<=`) are deferred to a separate ADR.
  Rationale: `rdf-diff`'s frozen `Fact` canonicalisation
  (ADR-0020 §1.4) does not define behaviour for quoted formulas.
  **Adversary-brief scope pin:** the `pb-adv-n3` brief and the
  resulting snapshot corpus (ADR-0021 §5, §6.3 line 185) MUST be
  scoped to this Turtle-equivalent subset; quoted-formula /
  `@forAll` / rule fixtures may be *tracked* but are explicitly
  **not gating** for `phase-b/done`. Cross-reference this pin
  when drafting `scripts/spawn/phase-b/pb-adv-n3.md`, otherwise
  the Phase B exit gate and ADR-0007 scope diverge.
  **Grammar-API surface:** `rdf-turtle` must expose a minimal
  API for `rdf-n3` to consume — `Lexer`, `Tok`, `Spanned`, and a
  concrete set of production entry points. The surface must be
  pinned in a sibling brief (`0007-briefs/grammar-api-surface.md`,
  authored by the orchestrator or by `pb-rdf-turtle` as its first
  deliverable) **before** the Phase B single-shot spawn reads this
  ADR; `#[doc(hidden)]` is not a visibility control and cannot
  substitute for the API pin. Absent that brief, `pb-rdf-n3` and
  `pb-rdf-turtle` will race to define it and the integration pass
  will land visibility-widening edits without review.

This decision is bounded by Phase B. SPARQL parser technology is
**explicitly deferred** to Phase C and is this ADR's primary reopen
trigger (see below).

## Consequences

- **Positive.**
  - Zero supply-chain delta for Phase B. No new runtime dep; no
    ADR-0004 allow-list addition; no `deny.toml` edit.
  - WASM / Zed-extension portability preserved — no C toolchain
    required for the parser layer.
  - Phase A code shape and review practice carry directly.
    `crates/rdf-turtle/src/{lexer.rs,grammar.rs}` is the reference
    template; Phase B coders mirror the split.
  - Shadow independence is carried by three explicit axes from
    ADR-0019 §3 + §4: (a) cohort separation (cohort-A vs cohort-B
    prompt lineages), (b) base-model override
    (`claude-sonnet-4-6` for shadows per ADR-0021 §4 vs
    `claude-opus-4-7` for mains), and (c) the oracle layer
    (`oxrdfxml`, `oxjsonld` — ADR-0019 §1 carve-out). ADR-0019
    §Validation treats "zero divergences on first run" as a
    tripwire, not a ranking; this ADR does **not** assert that any
    one axis dominates the others. The Devil's Advocate's
    "technique disjointness" concern (brief §2.3) is acknowledged
    as a weaker fourth axis that Phase B forgoes; if integration-
    pass divergence analysis shows the three retained axes are
    insufficient, the reopen trigger for this ADR fires (§Reopen
    triggers #2).
  - The ADR-0004 deferred row is closed — the workspace no longer
    carries an open "pending tool choice" signal in its policy
    layer.

- **Negative.**
  - LSP-grade error recovery is not delivered by this ADR. Phase A
    parsers emit single byte offsets (`FactProvenance { offset }`
    in `rdf-ntriples`; `Diag { code, message, offset }` in
    `rdf-turtle` — neither integrates with the `rdf-diagnostics`
    crate yet, per Expert A §2). Phase B parsers inherit the same
    shape unless the LSP retrofit ADR (Phase F) lands first.
    **This is the Devil's Advocate's strongest retained objection
    (§2.2, §2.5); it is accepted as a known gap.**
  - Hand-coded backtracking patterns (`grammar.rs:193-207,
    239-256` — `reject_dot`, `looks_like_graph_block`) and
    near-duplicated production functions (`parse_triple_stmt` vs
    `parse_triple_stmt_in_block`, `grammar.rs:368-445`) are
    patterns Phase B will replicate. The `DiagnosticCode` → pin-ID
    pattern (`crates/rdf-turtle/src/diag.rs:42-54` — e.g.
    `TTL-LITESC-001`) is Phase A's one reusable idiom and MUST be
    carried to every new Phase B parser.
  - N3's quoted-formula / graph-block ambiguity (Expert A §3,
    Expert C §5) is not solved by Phase B scope — it is deferred.
    Users needing full N3 must wait for the quoted-formula ADR.
  - Diagnostic quality is unified at Phase A's level, not above it.
    No fix-it suggestions. No token-span tree. No recovery
    checkpoints. Phase B parsers must not plateau below Phase A
    but are not required to exceed it.

- **Neutral.**
  - This ADR commits only Phase B. Phase C (SPARQL) reopens the
    question — see reopen triggers. The "hand-roll is the house
    style" reading is not a permanent commitment; it is the
    present state.
  - `rdf-diagnostics` integration for existing Phase A parsers is
    orthogonal debt (Expert A §2). Tracking: open question, not
    resolved here.

- **Acknowledged risks (Devil's Advocate residual objections).**
  - **No cited precedent for a hand-rolled Rust RDF/XML parser
    landing its W3C eval suite in ≤4 weeks** (Devil's Advocate
    brief §2.4). The working schedule estimate for `rdf-xml` is
    Phase A's hand-roll velocity on Turtle — ≈2166 lines across
    lexer + grammar + iri + diag modules in one sweep — applied
    to RDF/XML's smaller production set plus a `quick-xml`-driven
    state machine. That is an analogy, not a precedent. The
    `rdf-xml` worktree is therefore the chief schedule risk of
    the Phase B sweep and is the first item to surface in any
    mid-sweep review.
  - **"Slower" remains a cost** (Devil's Advocate brief §2.7).
    ADR-0004's "by hand is viable but slower" hedge is not
    repealed by this ADR. The cost is accepted because (i)
    Phase A demonstrated the hand-roll velocity on a grammar of
    comparable specification-density (Turtle) and (ii) the
    combinator alternative carries its own costs (supply-chain,
    WASM, error-recovery API mismatch for the XML/JSON three of
    four formats). The trade is not "slower is free"; it is
    "slower beats the alternatives for this scope."

## Reopen triggers

This ADR is automatically reconsidered when **any** of the following
occur:

1. **Phase C (SPARQL) scope freeze** — tag `phase-c/scope-frozen`.
   SPARQL 1.1's grammar is materially larger than Turtle's and
   carries genuine ambiguity (property paths, `PrefixedName` vs
   `VAR1`, aggregation inside expressions). The Devil's Advocate's
   §2.6 "SPARQL time bomb" objection is accepted as a future
   trigger, not a current decision driver. ADR-0007's successor
   must pick the SPARQL parser technology independently. The
   predicted refactor cost if Phase C lands `winnow` (Devil's
   Advocate brief §4.2: ≈2 weeks unplanned work + a silent
   regression window on `rdf-xml` diagnostics) is accepted as
   **future Phase C budget, not current Phase B budget**. Phase B
   parsers do not pre-emptively shape their helpers for combinator
   lift — doing so would leak an unresolved decision into the
   Phase B implementation.
2. **LSP error-recovery quality gate fails.** If the SPARC-phase LSP
   quality gate reports recovery-diagnostic scores below the agreed
   bar (definition pending LSP retrofit ADR), the trigger fires and
   combinator adoption is reconsidered for the formats that caused
   the failure.
3. **N3 full-language requested.** If a consumer requires quoted
   formulas, universals, or rule productions, a new ADR extends
   `rdf-diff`'s canonicalisation story and reconsiders N3's parser
   technology.
4. **`cargo deny` drift or CVE incident** on any currently
   allow-listed infrastructure crate (`logos`, `quick-xml`,
   `serde_json`). Combinator adoption becomes comparatively less
   costly if a peer dep already forces a supply-chain review.

## ADR-0004 edit (this ADR carries)

The `chumsky` / `winnow` row in ADR-0004 §Allow-list (v1) is
replaced. The canonical diff:

```diff
-| `chumsky` **or** `winnow` (ADR-0007) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
+| — (deferred to ADR-0007; resolved 2026-04-20: hand-roll default; combinator admission reopens per ADR-0007 §Reopen triggers) | n/a | Phase A + Phase B formats ship hand-rolled; see ADR-0007 | Hand-written recursive descent |
```

The row stays in the table as a historical marker so future readers
see the decision path; it no longer names a tool. All four table
columns are populated so the Markdown table renders cleanly.
Rationale: a removed row is invisible in review; a resolved row
preserves the audit trail. A dated "resolved" marker in the first
column lets a future ADR supersede this decision with a new row
rather than editing this one. The ADR-0004 "Amended-by" frontmatter
carries the ADR-0007 entry so the amendment is discoverable from the
top of that file.

## Validation

- **`cargo check --workspace --all-features`** green after the
  ADR-0004 edit and any resulting `[workspace.dependencies]` tidy.
- **`cargo deny check`** green; no new admissions.
- **No chumsky or winnow in `Cargo.lock`.** Grep asserts this; if a
  transitive accidentally drags one in, the reopen trigger fires.
- **Phase B exit gate** (ADR-0021 §6.3): `rdfxml` + `jsonld` W3C
  syntax suites at 100 %, `trix` + `n3` snapshot corpus green — the
  measurable consequence of the technology choice.
- **Shadow divergence first-run non-zero** (ADR-0019 §Validation)
  for `rdf-xml` and `rdf-jsonld`; if zero, cohort separation is the
  prime suspect, not parser technology.
- **No cross-parser CST dependency** — hand-rolled parsers do not
  share a CST type across crates. If a shared CST emerges, it must
  be gated behind its own ADR (`rowan` / `cstree` are allow-listed
  by ADR-0004 for LSP, not for parsers).

## Open questions (carried forward to LSP retrofit ADR)

From the briefs:

1. **`rdf-diagnostics` integration for Phase A parsers** — Expert A §5
   Q1, §2. Phase A parsers emit stringly-typed diagnostics on
   single byte offsets, not ranges. Cross-cutting retrofit; not in
   Phase B scope.
2. **Parser-performance bar** — Expert A §5 Q3. SPARC 04 §5 commits
   to ≥80 MB/s on prettified Turtle; `crates/rdf-turtle/benches/`
   does not yet exist. If the hand-roll misses that bar the
   decision to stay hand-rolled must be re-examined on perf
   grounds, orthogonal to this ADR's policy drivers.
3. **`deny-regression` negative assertions** — Expert D §6. Should
   `crates/testing/deny-regression/` carry an explicit negative
   check that `psm`, `cc`, `stacker` never enter the runtime
   closure? Recommended; out of scope here.

## Links

- [`0004-third-party-crate-policy.md`](0004-third-party-crate-policy.md)
  §Allow-list — this ADR closes the deferred row.
- [`0017-execution-model.md`](0017-execution-model.md) — phase cadence.
- [`0019-independent-verification.md`](0019-independent-verification.md)
  §3 — shadow disjointness and cohort-separation requirement.
- [`0020-verification-implementation-plan.md`](0020-verification-implementation-plan.md)
  §1.4 — frozen `rdf_diff::Parser` trait (agnostic to parser tech).
- [`0021-phase-b-execution-plan.md`](0021-phase-b-execution-plan.md)
  §1.1 — pre-flight dependency on this ADR being Accepted.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 — Phase B
  budget + exit gate that measures this decision's consequences.
- Briefs (hive, 2026-04-20):
  - [`0007-briefs/a-phase-a-precedent.md`](0007-briefs/a-phase-a-precedent.md)
  - [`0007-briefs/b-combinator-landscape.md`](0007-briefs/b-combinator-landscape.md)
  - [`0007-briefs/c-format-requirements.md`](0007-briefs/c-format-requirements.md)
  - [`0007-briefs/d-policy-alignment.md`](0007-briefs/d-policy-alignment.md)
  - [`0007-briefs/devils-advocate.md`](0007-briefs/devils-advocate.md)
