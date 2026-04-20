---
agent: devils-advocate-final
cohort: hive-adr-0007
role: contrarian-review
date: 2026-04-20
phase: post-draft
---

# Final Devil's Advocate Review — ADR-0007 draft

## §1 Overall verdict

Survives with amendments: the decision is defensible and the per-format
bindings are the strongest part, but the draft (a) smuggles in
load-bearing claims without citation, (b) quietly narrows N3 out of the
Phase B exit gate, (c) leaves two §2 objections un-rebutted, and (d)
has frontmatter/status inconsistencies that should be fixed before the
single-shot spawn reads it.

## §2 Flaws — numbered

### Flaw 1 — "Oracle provides the dominant independence signal" is asserted, not grounded

- **Claim:** The §Consequences "Positive" bullet rebuts Devil's
  Advocate §2.3 with "the oracle provides the dominant independence
  signal." ADR-0019 §3 is name-dropped but not quoted. ADR-0019's
  actual §Validation clause cited in ADR-0021 line 250 only says
  "zero divergences on first run is suspicious" — that is a
  *tripwire*, not a ranking of signals. The "dominance" claim is new
  in this ADR and load-bearing for dismissing the hand-roll × hand-roll
  shadow cost.
- **Hook:** ADR-0007 lines 170–172.
- **Severity:** Amend.
- **Suggested remedy:** Either cite ADR-0019 §3 with the exact phrase
  that establishes oracle primacy, or soften to "we accept that the
  oracle + cohort separation + base-model override in combination
  carry the independence load; technique disjointness is a weaker
  fourth axis." Do not assert an axis ranking the referenced ADR does
  not make.

### Flaw 2 — N3 scope narrowing silently contradicts ADR-0021's exit gate

- **Claim:** ADR-0007 line 143–150 pins `rdf-n3` to the
  Turtle-equivalent subset. ADR-0021 §6.3 exit gate (line 185) reads
  "`n3`: snapshot corpus green; no W3C suite exists." If Phase B's
  N3 snapshot corpus is seeded by an adversary brief that includes
  quoted formulas or `@forAll` (reasonable — those are the
  distinguishing N3 features), the corpus is unsatisfiable by the
  scoped parser and either (a) the gate is trivially green because
  the corpus was pre-filtered to the Turtle subset (in which case
  `rdf-n3` is `rdf-turtle-with-a-different-file-extension` and the
  Phase B deliverable is theatre), or (b) the gate fails. The ADR
  does not specify which.
- **Hook:** ADR-0007 lines 143–150 vs ADR-0021 line 185 and line 161
  (`pb-adv-n3`).
- **Severity:** Block.
- **Suggested remedy:** Add a single sentence to §Decision per-format
  binding for `rdf-n3`: "The `pb-adv-n3` adversary brief and the
  resulting snapshot corpus MUST be scoped to the Turtle-equivalent
  subset; quoted-formula / `@forAll` / rule fixtures are deferred
  fixtures, tracked but not gating." Cross-link ADR-0021 so the
  adversary brief author sees the scope. Without this the spawn
  reads a contradiction.

### Flaw 3 — §2.4 (RDF/XML quicksand) and §2.7 ("slower stopped being a cost") are unanswered

- **Claim:** The §Consequences section name-checks §2.2, §2.3, §2.5
  and §2.6, but §2.4 and §2.7 go unmentioned. §2.4 asked for "a
  hand-rolled RDF/XML parser in any comparable Rust project that
  landed its W3C eval suite in <4 weeks"; the ADR provides no such
  precedent and instead asserts "4–6 week budget accommodates
  hand-rolled RDF/XML" as a decision driver (line 60–61) with no
  citation. §2.7 asked why "slower" stopped being a cost under a
  tighter budget; the ADR's answer is implicit (Phase A shipped, so
  it's possible) but never written.
- **Hook:** ADR-0007 Decision Drivers line 60–61; Consequences
  section makes no §2.4 or §2.7 reference. Devil's Advocate brief
  lines 55–64 (§2.4) and 91–99 (§2.7).
- **Severity:** Amend.
- **Suggested remedy:** Add two sentences to §Consequences Negative,
  or a one-paragraph §Acknowledged-risks block: "No comparable
  hand-rolled Rust RDF/XML parser is cited; the 4–6 week budget for
  `rdf-xml` is the chief schedule risk and is the first item to
  surface in the Phase B mid-point review. 'Slower' remains a cost;
  we accept it because Phase A's hand-roll velocity on Turtle (~2166
  lines in one sweep) is the working estimate for RDF/XML." Honest
  about the gap is better than silent.

### Flaw 4 — "Duplication preserves shadow independence" for rdf-trix is backwards

- **Claim:** Line 137–140 instructs `rdf-trix` to duplicate
  ~200 lines of `quick-xml` event matching rather than share with
  `rdf-xml`, citing "duplication preserves shadow independence." But
  rdf-trix has no shadow (ADR-0021 §Negative, line 231–232:
  "`rdf-trix` and `rdf-n3` have no shadow"). There is nothing to
  preserve independence against. The rationale is a cargo-culted
  application of §2.3's shadow argument to a format where the
  shadow axis does not apply.
- **Hook:** ADR-0007 line 137–140; ADR-0021 line 231–232.
- **Severity:** Amend.
- **Suggested remedy:** Either keep the duplication with a different
  rationale ("~200 lines of XML event matching is below the factoring
  threshold; a shared utility crate would re-export `quick-xml` types
  and add a dep edge for no code-reduction benefit"), or allow
  `rdf-trix` to call a `pub(crate)` helper inside `rdf-xml` and drop
  the duplication. The current rationale is incorrect on its own
  terms.

### Flaw 5 — Status "Accepted" with same-day decision skips the Proposed state

- **Claim:** Line 3 marks the ADR Accepted and line 5 names "hive
  briefs from Expert A … convened today." ADR-0021 line 2 is
  "Proposed"; ADR-0004 line 3 shows the amended-by-ADR-0007 edit
  (2026-04-20) already applied. The ordering implies ADR-0007 was
  drafted and flipped to Accepted within the same orchestrator
  session without a written Proposed window for external review.
  That is process-legal under ADR-0017's orchestrator-decides model
  but it means the Devil's Advocate's §2 objections have had one
  pass, not two. Claiming "Accepted" on a same-day brief without a
  Proposed interval is the kind of signal that undermines the audit
  trail the ADR-0004-edit justification (line 249–250) explicitly
  invokes.
- **Hook:** Line 3 vs lines 5–8; ADR-0021 line 2 (comparison).
- **Severity:** Note.
- **Suggested remedy:** Either flip to "Proposed" pending a single
  sleep-on-it + one merged comment pass, or add a one-liner under
  Status: "Same-session decision; Devil's Advocate final review
  incorporated per `0007-briefs/devils-advocate-final-review.md`."
  The latter is cheaper and preserves the audit trail.

### Flaw 6 — `Retires-on` vs `Reopen triggers` are two mechanisms conflated

- **Claim:** Frontmatter line 13 says
  `Retires-on: tag phase-c/scope-frozen`; body §Reopen triggers line
  216 lists that same tag as trigger #1 of four. Retirement (the ADR
  becomes historical, not consulted) and reopening (the ADR is
  reconsidered, potentially superseded) are distinct lifecycle
  states. ADR-0021 line 240–241 uses "retirement trigger" for
  `phase-b/done` in the sense of "becomes historical." ADR-0007
  conflates the two.
- **Hook:** Line 13 vs lines 216–235.
- **Severity:** Amend.
- **Suggested remedy:** Drop the `Retires-on:` frontmatter line (this
  ADR is a policy decision with reopen triggers, not a
  time-boxed execution plan like ADR-0021). Keep §Reopen triggers.
  Alternatively, if retirement is genuinely intended, rename
  `Retires-on` to `Supersedes-triggered-by` and enumerate all four
  triggers, not just one.

### Flaw 7 — The `rdf_turtle::grammar_api` module is visibility-fragile

- **Claim:** Line 144 says "expose a minimal grammar API (`Lexer`,
  `Tok`, `Spanned`, shared production handlers) via a new
  `rdf_turtle::grammar_api` module marked `#[doc(hidden)]`." This is
  the load-bearing interface between two crates. `#[doc(hidden)]` is
  a documentation hint, not a visibility control — the items will
  still be `pub` and part of the semver surface of `rdf-turtle`.
  "Shared production handlers" is also handwave: if N3 needs to
  intercept token-peek decisions mid-production (and the current
  `grammar.rs:193-207` `reject_dot` / `looks_like_graph_block`
  patterns suggest it will), the API balloons to expose parser
  internals.
- **Hook:** Line 144.
- **Severity:** Amend.
- **Suggested remedy:** Either (a) pin a concrete API shape in a
  sibling brief (`0007-briefs/grammar-api-surface.md`) before the
  Phase B spawn, so `pb-rdf-n3` and `pb-rdf-turtle` are not racing
  to define it, or (b) make `rdf-n3` a path dep that consumes
  `rdf-turtle` as a dev-time internal-path dep with
  `#[cfg(feature = "internal-grammar-api")]` guards. Leaving this
  as prose risks a visibility-war PR landing in week 3.

### Flaw 8 — ADR-0004 edit: the resolved row has empty trailing columns

- **Claim:** The diff in line 242–245 replaces the
  chumsky-or-winnow row with a single cell that spans — or rather,
  does not span — the four columns. The new row is rendered with
  three empty `|   |` cells. Markdown tables do not merge cells;
  reviewers will see an odd stub row. The ADR-0004 file at line 71
  shows the edit already applied in the current working tree, and
  the empty cells are visible.
- **Hook:** ADR-0007 lines 242–245; ADR-0004 line 71 (as applied).
- **Severity:** Note.
- **Suggested remedy:** Fill the three trailing columns with "n/a",
  "Phase A + Phase B formats ship hand-rolled; see ADR-0007", and
  "Hand-written recursive descent" — which is actually what the
  applied file already does. The ADR-0007 diff block itself is
  misleading: it shows empty cells but the applied edit has
  content. Sync the diff in ADR-0007 to match what landed in
  ADR-0004, otherwise a future reader replaying the diff produces a
  different file than the one on disk.

### Flaw 9 — Phase C refactor cost is named as a reopen trigger but not priced

- **Claim:** §Reopen triggers #1 (SPARQL) accepts Devil's Advocate
  §2.6 "as a future trigger, not a current decision driver." That is
  rhetorically clean but numerically empty: §2.6's prediction was
  "~2 weeks of unplanned work charged to Phase C's budget, plus a
  silent regression window on rdf-xml diagnostics during the
  refactor." The ADR accepts the prediction without contesting the
  cost, and without any mitigation (e.g. "Phase B parsers will use
  a shared `rdf-syntax-core` production helper module designed for
  combinator lift, not hand-roll ergonomics"). Accepting the risk
  while hiding the number is a tell.
- **Hook:** ADR-0007 lines 216–222; Devil's Advocate lines 131–137.
- **Severity:** Note.
- **Suggested remedy:** Add one sentence to the trigger: "The
  predicted refactor cost (~2 weeks, per Devil's Advocate §4.2) is
  accepted as future Phase C budget, not current Phase B budget;
  Phase B parsers do not pre-emptively shape their helpers for
  combinator lift."

## §3 Survives

Things the draft genuinely gets right — the queen should not rewrite
these:

1. **Grammar-shape partition** (lines 31–38). The observation that
   three of the four Phase B formats are not token-stream text
   grammars is the strongest single argument in the ADR and cuts
   the chumsky/winnow debate down to two formats (N3 + Turtle
   evolution), not four.
2. **Per-format bindings with concrete data structures**
   (lines 130–150). `ElementMode` enum for rdf-xml, `ContextStack`
   for rdf-jsonld — this is the level of specificity that lets a
   Phase B coder start without a second clarification round.
3. **Scope pins for `rdf-jsonld`** (line 133–134). Hard-reject
   remote `@context` at parse time, defer expand/compact/flatten/
   frame/URDNA2015. The boundary is crisp.
4. **The `DiagnosticCode` → pin-ID carry** (line 191–193). Naming
   `TTL-LITESC-001` as the Phase A idiom to replicate is concrete
   and reviewable.
5. **Validation § is measurable** (lines 253–268). "No chumsky or
   winnow in `Cargo.lock`" and "no cross-parser CST dependency"
   are grep-able assertions, not aspirational.

## §4 If overruled (predictions)

Two near-term failure modes I still expect, even with every flaw
above fixed:

1. **The `rdf_turtle::grammar_api` surface becomes the merge
   conflict** (Flaw 7 cashes out). By week 3 of Phase B,
   `pb-rdf-n3`'s worktree needs a function that `pb-rdf-turtle`
   did not export, and the two agents cross-edit
   `crates/rdf-turtle/src/grammar.rs` visibility markers. The
   orchestrator integration pass (ADR-0021 §6) resolves it by
   widening `pub(crate)` to `pub` without a review ADR, and the
   `rdf-turtle` semver surface silently grows. Surfaces in the
   first post-`phase-b/done` `cargo public-api` diff.

2. **`rdf-xml` and `rdf-xml-shadow` disagree on striped syntax and
   the oracle agrees with neither** (Devil's Advocate §4.1,
   unchanged). The draft's "oracle provides dominant independence"
   rebuttal (Flaw 1) is the exact claim that will be tested here.
   If oracle-vs-both-parsers divergence lands in the first
   integration pass, the ADR's decision survives but its rationale
   clause needs a patch, and the patch will reveal that
   "technique disjointness" was load-bearing after all.

---

**Path:** `/Users/henrik/source/zed-rdf/docs/adr/0007-briefs/devils-advocate-final-review.md`
**Overall verdict:** Survives with amendments.
**Flaw counts:** Block 1 / Amend 5 / Note 3.
