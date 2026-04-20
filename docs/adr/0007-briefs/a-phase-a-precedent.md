---
agent_id: expert-a-phase-a-precedent
cohort: hive-adr-0007
model: claude-opus-4-7[1m]
date: 2026-04-20
adr: 0007
role: expert-a
sources_read:
  - crates/rdf-ntriples/src/lib.rs
  - crates/rdf-turtle/src/{lexer.rs,grammar.rs,iri.rs,diag.rs,lib.rs}
  - crates/rdf-turtle/tests/{smoke.rs,proptest.rs}
  - crates/rdf-ntriples/tests/{fixtures.rs,proptest.rs}
  - crates/rdf-turtle/Cargo.toml, crates/rdf-ntriples/Cargo.toml
  - crates/rdf-diagnostics/src/lib.rs (+ siblings)
  - docs/adr/0017-execution-model.md, 0018, 0019, 0020, 0021
  - docs/sparc/04-refinement.md
  - external/tests/PINS.md
---

# §1 — Executive verdict

**Mixed, leaning hand-roll, with one hard caveat.** The two Phase A parsers
are well-shaped hand-rolls that cleared the W3C exit gate at 100 % with a
modest LOC budget (ntriples 1189, turtle 2166) and zero parser-combinator
dependency. However, **neither parser integrates with the workspace
`rdf-diagnostics` crate** — both emit stringly-typed diagnostics or a
local `Diag` struct, and neither carries `Span` range information.
LSP-grade error surface is a future retrofit, not a Phase A deliverable.
For Phase B: `rdf-n3` (Turtle superset) is a trivial hand-roll extension;
`rdf-trix` is XML-events, shape-distinct; `rdf-xml` and `rdf-jsonld` need
*infrastructure* tokenisers (quick-xml, serde_json) anyway, so the parser
above the tokeniser is small in both cases and the hand-roll precedent
carries. **Confidence: high that hand-roll works for N3/TriX/RDF-XML;
medium that it is the best shape for JSON-LD** (tree-walk, not token
stream — a combinator gives no leverage either). Confidence overall:
**0.75** for "continue hand-rolling" as the default for Phase B.

# §2 — Evidence table

| Q | Finding | Primary citations |
|---|---------|-------------------|
| **Q1 — structural shape (NT)** | Single-file state machine; no separate lexer. Byte-indexed cursor `ParseState { src, bytes, pos, base_offset, mode, parser_id }` at `lib.rs:166-173`. Hot function `parse_statement` `lib.rs:275-335` (61 lines). `run_parse` driver `lib.rs:91-139` (49 lines). Spans are **single byte offsets** not ranges — stored on `FactProvenance { offset: Option<usize> }` at `lib.rs:330-333`. No error recovery — first `Err` returns `fatal: true` and aborts (`lib.rs:127`). | `crates/rdf-ntriples/src/lib.rs:166-335` |
| **Q1 — structural shape (Turtle)** | Clean **lexer / grammar split** over 4 modules (lexer 848, grammar 785, iri 263, diag 87). Lexer `Tok` enum has 22 variants (`lexer.rs:28-87`) including keyword tokens (`KwA`, `KwTrue`, `KwFalse`, `KwGraph`, `SparqlPrefix`, `SparqlBase`). Hot fns: `Lexer::next` **124 lines**, annotated `#[allow(clippy::too_many_lines)]` (`lexer.rs:147-271`); `Parser::parse_document` dispatches on peek-token (`grammar.rs:104-127`); `trig_graph_block` **76 lines** — a deeply nested `match` on 7 possible header-token shapes (`grammar.rs:259-334`). Spans: every `Spanned { tok, start, end }` carries a byte range (`lexer.rs:90-98`). No recovery — `Diag.fatal=true` everywhere (`diag.rs:73`). | `crates/rdf-turtle/src/lexer.rs:28-98,147-271`; `grammar.rs:104-127,259-334` |
| **Q2 — diagnostic surface** | **Critical gap.** Neither parser imports `rdf-diagnostics`. `rdf-ntriples` uses `fatal(format!(...))` into `Vec<String>` on `rdf_diff::Diagnostics` (`lib.rs:141-146`, `lib.rs:100-105`). `rdf-turtle` has its own `DiagnosticCode` enum with 10 variants (`diag.rs:10-36`) and `Diag.render()` formats to `"CODE: msg at byte N"` (`diag.rs:77-87`) — then `lib.rs:170` flattens that string back into `rdf_diff::Diagnostics.messages`. `rdf-diagnostics` with `Span { start, end }` / `Severity` / LSP bridge is defined (`crates/rdf-diagnostics/src/lib.rs:59-63`, 634 total lines) but only `rdf-iri`, `rdf-format`, `sparql-syntax` reference it (grep result above). Messages are NOT LSP-quality: single byte offsets, no range, no related-information, no severity tiers. The `DiagnosticCode` → pin-ID mapping (`diag.rs:42-54`: `TTL-LITESC-001`, `TTL-BNPFX-001`, ...) is the **one pattern worth carrying** to XML/JSON grammars. | `crates/rdf-turtle/src/diag.rs:10-87`; `rdf-ntriples/src/lib.rs:141-146`; `rdf-diagnostics/src/lib.rs:59-63` |
| **Q3 — test burden** | **rdf-ntriples: 52 test cases** — 33 inline unit (`src/lib.rs`, grep count 33 `#[test]`), 17 fixture (`tests/fixtures.rs`), 2 proptest (`tests/proptest.rs:108,137`). Adversary-NT fixtures go through a `.expected`-side-car harness (`fixtures.rs:97-137`). **rdf-turtle: 39 test cases** — 37 inline smoke (`tests/smoke.rs`), 2 proptest (`tests/proptest.rs:71,182`). Property generators are small (≤8 triples, 48–64 cases — `rdf-ntriples/tests/proptest.rs:30-32,104`; `rdf-turtle/tests/proptest.rs:20,62,175`). Both crates also run the W3C manifest via `xtask verify` out-of-crate (not a `#[test]`). **No cursor-introspection tests** — tests treat the parser as black-box (`NTriplesParser.parse(bytes)` → `Result<ParseOutcome, Diagnostics>`). This means the test burden **is not locked to the hand-roll shape**; swapping to combinators would not force test rewrites. | `crates/rdf-ntriples/src/lib.rs:843-1187`; `rdf-ntriples/tests/{fixtures.rs:139-211,proptest.rs:102-155}`; `rdf-turtle/tests/{smoke.rs:42-461,proptest.rs:61-208}` |
| **Q4 — pain points** | **Almost no TODO/FIXME.** Workspace-wide grep for TODO/FIXME/XXX/HACK in these parsers returned 0 actionable markers — only doc-comment backslash escapes that matched false-positively (`lexer.rs:471`, `lib.rs:15,612` are spec references, not TODOs). **But** the real pain is elsewhere: (a) **hand-coded backtracking** at `grammar.rs:193,207,239,248,255` — `reject_dot` saves `lex.offset()`, peeks, and conditionally seeks back; `looks_like_graph_block` uses a 2-token lookahead with an explicit rewind (`grammar.rs:239-256`); this is the shape combinators handle out of the box. (b) **Open-coded match chains** on `Tok` — 26 `matches!(...*.tok)` call sites in `grammar.rs` (grep count above), many of the form `matches!(peek.as_ref().map(|s| &s.tok), Some(Tok::Dot | Tok::RBracket | ...))` (`grammar.rs:473-476`). (c) **Duplicated lookahead helpers**: `parse_triple_stmt` and `parse_triple_stmt_in_block` differ only in the terminator set — 78 lines near-duplicated (`grammar.rs:368-445`). (d) Numeric-literal `.` disambiguation vs statement terminator (`lexer.rs:587-614`) is grammar-encoded inline; a PEG / chumsky would express it declaratively. (e) `lex_pname` carries the `had_colon` out-param to disambiguate bare keywords from pnames (`lexer.rs:684-785`) — a code-smell a Pratt / combinator parser with explicit alternation avoids. | `crates/rdf-turtle/src/grammar.rs:192-257,368-445,473-476`; `lexer.rs:147-271,587-614,684-785` |
| **Q5 — scaling signal** | See §3 below. Summary: **Turtle-shape hand-roll keeps working** for N3 (direct extension of the existing grammar module) and for the "inner" parser of TriX / RDF-XML (after `quick-xml` provides events). **JSON-LD syntax** is a *tree walk* over `serde_json::Value`, which is neither a hand-roll nor a combinator problem — it's visitor code regardless of decision. | derived; see §3 |
| **Q6 — contrarian evidence** | See §4 below. **Two combinator wins**: `trig_graph_block` header-dispatch `match` (`grammar.rs:259-334`, 76 lines across 7 arms with duplicated "expect LBrace" suffix in each); and `parse_triple_stmt_in_block` vs `parse_triple_stmt` duplication (`grammar.rs:368-445`). **Two hand-roll wins**: NT literal trailing-dot backoff (`lib.rs:488-502` — tight byte-level cursor rewind with a specific W3C negative-test FM4-b that a combinator PEG would struggle to express cleanly); and `Lexer::next` numeric vs `.` terminator disambiguation (`lexer.rs:210-229,587-614` — the parser-state-aware one-byte lookahead is cheaper and clearer than encoding the full decimal/double grammar alternation declaratively). | `crates/rdf-turtle/src/grammar.rs:259-334,368-445`; `rdf-ntriples/src/lib.rs:488-502`; `rdf-turtle/src/lexer.rs:210-229,587-614` |

# §3 — Scaling predictions per Phase B format

ADR-0021 lines 22–48 commits to four Phase B formats. Per-format signal:

## rdf-xml

- **Grammar shape:** tiny — RDF/XML is defined over an XML event stream,
  not a token stream. The grammar is *7 productions* (§7 of the RDF/XML
  spec). `quick-xml` is pre-approved (ADR-0021:34). The actual "parser"
  is a dispatch on `quick-xml`'s `Event::{Start, End, Empty, Text, ...}`
  with a property-element state machine.
- **Hand-roll verdict:** trivially hand-rolled. The Turtle-style lexer
  is replaced by `quick-xml::Reader::read_event()`; everything above is
  a small state machine with ~5 modes. **Hand-roll wins.** A combinator
  library over XML events would be ceremony without payoff.
- **Confidence:** 0.9.

## rdf-jsonld

- **Grammar shape:** parse `serde_json::Value` first, then walk the tree.
  `@context` well-formedness (ADR-0021:236) is a schema-validation pass
  over the `serde_json::Map`, not a parse at all.
- **Hand-roll verdict:** the decision is orthogonal — tree-walk code is
  the same shape in either world. The combinator library `chumsky` /
  `winnow` buys *nothing* here because the tokeniser is `serde_json`,
  not a byte stream. **Hand-roll wins by default** (no dep added for
  zero benefit).
- **Confidence:** 0.85.

## rdf-trix

- **Grammar shape:** XML wrapper around N-Triples content per
  ADR-0021:39. Two layers: (1) tiny XML tokeniser (either reuse
  `rdf-xml`'s or `quick-xml` directly — ADR-0021:38-40 is undecided),
  (2) forward each `<triple>` body to `NTriplesParser`.
- **Hand-roll verdict:** composition of existing hand-rolled parts.
  **Hand-roll wins** — nothing to gain from combinators on a 3-element
  XML envelope. Reuse `rdf-ntriples::NTriplesParser::parse` verbatim.
- **Confidence:** 0.95.

## rdf-n3

- **Grammar shape:** Turtle superset. ADR-0021:42-44 notes it "consumes
  the `rdf-turtle` grammar and adds N3-specific productions" (`@keywords`,
  reification `{…}` formulas, quoted triples `<<…>>`). ~15 new productions
  per the cohort prompt. The existing lexer already has `KwGraph` and
  brace tokens.
- **Hand-roll verdict:** **hand-roll wins — but only with a refactor.**
  The current grammar is a concrete `Parser<'a>` struct, not generic
  over "dialect" in the way a combinator library would naturally
  express. N3's quoted-formula `{ p o ; q r }` collides syntactically
  with TriG's graph-block `{ triples }` at `grammar.rs:235-257` — the
  `looks_like_graph_block` heuristic is already fragile (two-token
  lookahead with rewind), and N3 formulas inside predicate positions
  would force either a third `Dialect` variant or a fork. Neither is
  clean but both are cheaper than porting all of Turtle to combinators.
- **Confidence:** 0.7 that hand-roll is still the right call; 0.3 that
  N3's formula-vs-graph-block ambiguity is the moment a PEG-style
  grammar earns its keep.

**Aggregate:** 3 of 4 Phase B formats keep the hand-roll pattern
cleanly. RDF/N3 is the edge case. The format that would benefit *most*
from combinators (unambiguous declarative grammar) is the one Phase B
explicitly ships *without* a W3C conformance suite (ADR-0021:28, 185),
so the risk of a combinator-induced divergence from prose-specified
behaviour is lower here than for the other three — paradoxically
making combinators *safer* to try on N3 than on rdf-xml or jsonld.

# §4 — Hand-roll wins and losses

## Wins (≥2)

1. **Trailing-dot disambiguation in blank-node labels** — `rdf-ntriples/src/lib.rs:488-502`.
   The label lexer greedily consumes `PN_CHARS | '.'`, then backs off
   one byte at a time while the last byte is `.`, so the dot is
   restored as the statement terminator. A declarative PEG has to either
   lookahead an unbounded distance or split the `.` handling across
   two alternatives. The hand-roll is 6 lines of cursor arithmetic and
   exactly satisfies W3C `nt-syntax-bad-bnode-01/02` plus FM4-b
   (citation in same file: `lib.rs:1044-1051`).

2. **Numeric-vs-terminator `.` disambiguation** —
   `rdf-turtle/src/lexer.rs:210-229, 587-614`. The lexer inspects the
   byte *after* the dot and commits to "statement terminator" unless a
   digit or (after a digit-run) `e`/`E` follows. The comment at
   `lexer.rs:587-606` cites the exact W3C test (`turtle-syntax-number-11`)
   that pins this choice. A combinator grammar would express this as
   `INTEGER | DECIMAL | DOUBLE | DOT` alternation with commit-on-first-
   char; it would work, but the hand-rolled version makes the *why*
   explicit and keeps the "read one byte, decide" fast path visible.

3. **Document-scope blank-node table (TTL-BNPFX-001)** —
   `grammar.rs:69-71, 714-722`. Single `BTreeMap<String, String>` on
   `Parser`, same map used whether we are inside `@prefix` redef, inside
   a TriG graph block, or at top level. Pinned by the PTT1 property
   test at `rdf-turtle/tests/proptest.rs:71-123`. A combinator library
   would still need the same side-table; the hand-roll doesn't fight it.

## Losses (≥2)

1. **`trig_graph_block` header-dispatch** —
   `grammar.rs:259-334` (76 lines, 7 arms). Five of the seven arms
   repeat the pattern "consume graph-name, then expect `{`, else
   syntax error". A combinator written as
   `graph_name.then(lbrace).ignore_then(body)` would reduce this to
   four lines. Current code compiles clean but is a duplication sink.

2. **`parse_triple_stmt` vs `parse_triple_stmt_in_block`** —
   `grammar.rs:368-445`. Two near-identical 38-line functions; the
   only semantic delta is whether `}` terminates the *last* triple
   (TriG §2.5, ADR-0021 deferred). A combinator supporting optional
   alternative terminators would collapse this to one function with a
   `terminator: &[Tok]` parameter. The current code has a
   `saw_rbrace` out-param (`grammar.rs:355-358, 412-445`) threaded up
   one call level — small smell, real duplication.

3. **Backtracking implemented by hand** — `grammar.rs:193-207, 239-256`.
   `reject_dot` and `looks_like_graph_block` both save `lex.offset()`,
   probe, and seek back. Five call sites in all. A parser-combinator's
   `try { ... }.or(...)` abstraction is precisely this pattern; doing
   it by hand means any future mistake (forgetting a `seek` in one
   branch) is silent grammar drift. Partially mitigated by the
   "defensive symmetry" comment at `grammar.rs:205-207` but this is a
   comment, not a type.

4. **Diagnostic surface does not use `rdf-diagnostics`** —
   already in §2. This is a loss regardless of hand-roll vs combinator
   (neither path forces the right answer) but it is the single biggest
   latent debt in Phase A. Phase B parsers will inherit the same shape
   unless ADR-0007 calls it out as orthogonal.

# §5 — Open questions for the queen

1. **Is ADR-0007 really a parser-technology decision, or two decisions?**
   The evidence separates cleanly: (a) "hand-roll vs combinator for the
   tokeniser/grammar" (b) "how does the parser emit diagnostics".
   Phase A settled (a) and *did not* settle (b). A single ADR that only
   answers (a) risks re-opening (b) in Phase F (LSP core — ADR-0021:35
   refinement plan, phase F at SPARC 04 §2).

2. **How much weight to give the N3 formula-vs-graph-block risk?**
   N3 has no W3C conformance suite (ADR-0021:185). If we hand-roll it
   and the grammar drifts from the de-facto reference (cwm), the
   differential signal is oracle-only (ADR-0019 §3 shadow carve-out).
   A combinator grammar would be auditable by pattern match against
   the spec's BNF; the hand-roll would not. Is this risk worth a
   per-crate technology split, or do we accept it?

3. **Performance bar.** SPARC 04 §5 commits to ≥80 MB/s on prettified
   Turtle. Current Turtle parser is not benched yet (no
   `crates/rdf-turtle/benches/` exists — glob confirmed). If the
   hand-roll comes in at 30 MB/s we may be forced to the hand-roll
   anyway for reasons orthogonal to the decision in this ADR. Do we
   bench first and decide, or decide on aesthetics now?

4. **Does the existing Phase A integration-contract freeze
   (`rdf_diff::Parser`) constrain the choice?** ADR-0020 §1.4 froze
   the trait as `parse(&self, &[u8]) -> Result<ParseOutcome, Diagnostics>`.
   Combinator vs hand-roll is invisible to that contract. But if
   ADR-0007 also answers (b) and moves to `rdf-diagnostics`, the trait
   *does* change. Is that in-scope for ADR-0007 or deferred?

5. **Retirement trigger.** ADR-0021 lines 240-241 explicitly says "on
   `phase-b/done` it becomes historical". If ADR-0007 commits to
   hand-roll *for Phase A precedent only*, we still owe a re-evaluation
   ADR before Phase C (SPARQL — grammar size is comparable to Turtle
   but has much deeper expression trees). Should ADR-0007 carry its
   own retirement-on trigger, or is "per-phase ADR review" the policy?

---

*End brief. 282 lines (within the 400-line cap).*
