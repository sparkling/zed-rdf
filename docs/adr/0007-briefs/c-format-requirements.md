---
agent: expert-c-respawn
cohort: hive-adr-0007
role: Phase B format requirements
date: 2026-04-20
---

# Brief C — Phase B format requirements

Sources consulted: `crates/rdf-turtle/src/grammar.rs` (top), `crates/testing/rdf-diff/src/lib.rs` (frozen `Parser` trait), `docs/adr/0021-phase-b-execution-plan.md` §Decision + §Consequences. WebFetch deliberately skipped per budget guard; spec citations are from memory and flagged where so.

The frozen trait surface every format must hit is small and format-agnostic: `parse(&[u8]) -> Result<ParseOutcome, Diagnostics>` emitting `Facts` (a `BTreeMap<Fact, FactProvenance>`) in the canonical form from `rdf-diff`'s module docs (angle-bracketed IRIs, `_:label` blank nodes, inline-suffixed literals, positional BNode canonicalisation). That trait does not constrain parser *technology* — it only constrains the output shape. So the grammar class of each input language is the thing that dictates the choice.

## §1 Per-format verdict cards

### rdf-xml

- **Grammar class:** event-stream over XML Infoset, plus a small attribute-grammar layer on top. Not context-free in the Turtle sense — RDF/XML §6 productions are rules over *element events* and *attribute tuples*, not over tokens. State is driven by element nesting + `rdf:parseType` mode switches.
- **Infra needed:** `quick-xml` (already allow-listed per ADR-0021 pre-flight §1.2) as the tokeniser/event source. No additional crate. `rdf-iri` for base/xml:base resolution; `rdf-diagnostics` for span carry.
- **Tool recommendation:** **hand-roll a state machine over `quick-xml` events.** A combinator library buys nothing here — the grammar is attribute-polymorphic, not token-sequential. Model it as `enum ElementMode { NodeElt, PropElt, ParseTypeLiteral, ParseTypeCollection, ParseTypeResource }` with a stack of `(mode, subject, base, lang)` frames.
- **Risk note:** the *real* gotcha is the cross-product of `rdf:about` / `rdf:ID` / `rdf:nodeID` / `rdf:resource` / `rdf:parseType="Literal|Resource|Collection"` / `xml:lang` / `xml:base` / striping (node-element vs property-element alternation) — one property element can carry five attributes that each mean a different triple shape. `parseType="Literal"` needs exclusive-canonical-XML preservation of the child subtree (a separate serialisation problem mid-parse); `parseType="Collection"` desugars into `rdf:first` / `rdf:rest` chains on freshly minted BNodes.

### rdf-jsonld

- **Grammar class:** context-sensitive tree walk over a JSON AST. Not a grammar in the parser-generator sense at all — it's a structural rewrite of `serde_json::Value` driven by an active `@context` stack. The "parse" step is already done by `serde_json`; our job is syntactic validation + triple emission.
- **Infra needed:** `serde_json` (expected to land in allow-list at the ADR-0021 pre-flight step — the execution plan calls it out as a required dep); `rdf-iri`; `rdf-diagnostics`.
- **Tool recommendation:** **tree-walk a `serde_json::Value`** with a `ContextStack` side-structure. No combinator, no hand-rolled lexer — the JSON layer is not ours. What we hand-roll is the keyword dispatcher (`@id`, `@type`, `@value`, `@language`, `@list`, `@set`, `@graph`, `@vocab`, `@base`, `@container`, `@reverse`, `@nest`, `@included`) and IRI-expansion rules.
- **Risk note:** scope creep. JSON-LD 1.1 §4 (cite from memory) defines syntax; the *semantic* layers (Expansion Algorithm §5, Compaction §6, Flattening §7, Frame, URDNA2015 canonicalisation) are published as separate algorithms. Per ADR-0021 Consequences the boundary is "syntax + `@context` well-formedness only". The concrete trap is `@context` remote-IRI loading (HTTP fetch during parse) — *forbid at the trait boundary*, treat any remote `@context` as a hard reject in Phase B.

### rdf-trix

- **Grammar class:** regular, XML-shaped. TriX is a fixed-schema XML envelope around N-Triples-style term elements (`<triple><uri/><uri/><plainLiteral/></triple>` inside `<graph>` inside `<TriX>`). It's effectively an XSD with four or five element kinds and one attribute (`xml:base`-ish).
- **Infra needed:** `quick-xml` (shared with `rdf-xml`) or — per ADR-0021 §Decision drivers — a tiny bespoke tokeniser if we want to keep `rdf-trix` independent of `rdf-xml`. Reuse `quick-xml`: cheaper.
- **Tool recommendation:** **hand-roll, trivially.** Two hundred lines of event matching. No combinator needed; no grammar ambiguity; no stripping rules.
- **Risk note:** the only gotcha is that term-element *content* carries format constraints lifted from N-Triples: IRIs must pass `rdf-iri` validation, literals must honour the `datatype=` / `xml:lang=` attributes, plain-literal whitespace is significant. Reuse `rdf-ntriples` term-level validators rather than reimplementing.

### rdf-n3

- **Grammar class:** LL(k)-extended Turtle. Turtle's grammar is LL(1) with a small amount of lookahead (the `pname` vs `IRIREF` fork, collection / bnode-propertylist openers); N3 adds a handful of productions on top of that same base. Per `crates/rdf-turtle/src/grammar.rs` top comments, the current grammar is a recursive-descent dispatcher with a single `Lexer`-driven lookahead, which is exactly the shape N3 extends.
- **Infra needed:** `rdf-turtle` as a *library dependency* (not a fork). The N3 crate must be able to drive the Turtle lexer and call into Turtle production handlers for the shared subset, then layer N3-only productions on top.
- **Tool recommendation:** **hand-roll as a recursive-descent extension of the Turtle parser.** Same technology as Turtle. A combinator here would gratuitously re-implement work already proven in Phase A.
- **Risk note:** N3's quoted formulas (`{ … }` as a term, not a graph block) and universal/existential variables (`?x`, `@forAll`, `@forSome`) break the Turtle invariant that "the subject of a triple is always an IRI / BNode". The `rdf-diff` `Fact.subject` field is a `String`, so the *wire shape* still fits — but the canonicalisation story for quoted formulas is undefined in `rdf-diff`'s current module docs. Flag this as an open question (§5).

## §2 JSON-LD scope line

- **IN scope (Phase B):** JSON syntactic validity via `serde_json`; `@context` well-formedness (keyword spellings, term-definition shape, IRI / CURIE validity of mapped values, `@base` / `@vocab` / `@language` well-formedness, detection of cyclic local contexts); triple emission for inputs that are *already* in a trivially expanded shape (explicit `@id` / `@type` / `@value`, no external context resolution required).
- **OUT of scope (Phase E):** the Expansion Algorithm, Compaction Algorithm, Flattening Algorithm, Framing, URDNA2015 canonicalisation, and remote `@context` loading. These are separate W3C recommendations and are explicitly deferred per ADR-0021 §Consequences ("no expand, no compact, no normalize. Anything more is an ADR amendment").
- **Enforcement:** the Phase-B exit gate runs only the W3C **syntax** test suite subset; the expand / compact / flatten / frame suites are `skip`-listed at manifest-runner level (not allow-listed — different mechanism, they never enter the run).

## §3 N3-on-Turtle reuse sketch

- **What `rdf-turtle` must export for `rdf-n3`:** the `Lexer` + `Tok` + `Spanned` types (currently `pub(crate)` per `grammar.rs:23`); the `Parser` state struct's prefix / base / bnode-index helpers; and the individual production handlers for shared nonterminals (`parse_subject`, `parse_object`, `parse_predicate_object_list`, collection desugaring for `rdf:first` / `rdf:rest` chains). Visibility needs to flip from `pub(crate)` to `pub` on a deliberately minimal surface — probably a new `rdf_turtle::grammar_api` module that re-exports the N3-needed symbols and is marked `#[doc(hidden)]` to discourage downstream use.
- **What N3 adds that Turtle cannot parse:** (a) quoted-formula terms `{ triples }` appearing in subject / object position (requires a new `Tok::LBrace` / `Tok::RBrace` pair and a recursive re-entry into the triple production with a nested-graph sink); (b) the N3 keyword set `@forAll`, `@forSome`, `@keywords`, and `=>` / `<=` as predicate shorthands for `log:implies` (requires keyword-mode switching that Turtle's `@prefix` / `@base` handler does not support, hence "LL(k)-extended" rather than "LL(1)").

## §4 Shadow feasibility

- **(a) rdf-xml shadow in 4–6 weeks — YES, tight.** The grammar is well-specified (W3C Rec, not a draft), a reference implementation exists in `oxrdfxml` (used as oracle per ADR-0021 §1.3), and `quick-xml` gives a free event stream. The risk is the `parseType="Literal"` exclusive-canonical-XML path — that can eat a week on its own. Disjoint-cohort (sonnet-4-6) is realistic because RDF/XML is common enough in the model's training data to not require from-scratch spec reading.
- **(b) rdf-jsonld shadow in 4–6 weeks — YES, more comfortable.** Syntactic scope only (per §2) shrinks the surface dramatically. `serde_json` removes the whole tokenising problem. The shadow can be ~1500 lines against the oracle. The real-risk moment is *defining scope clearly enough* that the shadow and main parser agree on what "accept" means (see §5 open question on `@context` remote loading).

## §5 Open questions for queen

- Does the `rdf-diff` `Facts` canonicalisation story need an extension for N3 quoted formulas, or do we pin N3-in-Phase-B to the Turtle-equivalent subset (no quoted formulas, no universals) and defer quoted-formula support to a later phase? Strong recommendation: the latter — keeps the trait frozen.
- For JSON-LD, do we hard-reject remote `@context` URLs at parse time (clean, reproducible, breaks real-world inputs), or accept-with-warning and emit an empty graph for contexts we can't resolve offline? Strong recommendation: hard-reject in Phase B; revisit in Phase E alongside expansion.
- Does `rdf-trix` get its own `quick-xml` integration, or share a thin `crates/xml-events` utility crate with `rdf-xml`? Cohort separation for shadows argues against sharing (a bug in the shared layer taints both shadows); independence argues for duplication. Recommendation: duplicate the ~200 lines of TriX event code into `rdf-trix`; do not factor out.
