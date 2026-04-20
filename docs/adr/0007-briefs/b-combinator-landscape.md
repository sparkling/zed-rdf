---
agent: expert-b-respawn
cohort: hive-adr-0007
role: parser-combinator-landscape
date: 2026-04-20
---

# Brief B — Combinator landscape

## §1 Verdict (one paragraph)

**Hand-roll all four Phase B parsers; do not pull in chumsky or winnow.**
Confidence: high. Three of the four formats (NT, TriX, RDF/XML) are XML-
or line-event shaped and do not play to a combinator's strengths. JSON-LD
is a tree-walk over `serde_json::Value`, where a combinator is a category
error. That leaves only N3 — a Turtle-superset text grammar — where a
combinator would be ergonomic, and N3 is a single file against which the
cost of a new allow-list entry is not justified given Phase A's
hand-rolled Turtle already paid the recursive-descent tax.
The ADR-0004 allow-list line "`chumsky` **or** `winnow`" was a *future*
option, not a commitment; we propose closing it unless SPARQL in a later
phase reopens the question.

## §2 Head-to-head table

| Axis | chumsky 1.0 | winnow | hand-roll |
|---|---|---|---|
| Maintenance | Active; LSP/language-tooling user base | Active; nom author's successor | Ours, Phase A precedent |
| Error-recovery API | Best-in-class (`recover_with`, sync tokens) | Manual (`cut_err`, custom fallbacks) | Manual; Phase A shape known |
| Span ergonomics | First-class `Spanned<T>` everywhere | Via `Located<I>` + `Stateful` | Explicit byte-offset threading |
| Lexer composition (logos) | Clean — `Input` over token iter | Clean — `Stream` impl | N/A (we own the tokeniser) |
| XML/JSON fit | Poor — built for text grammars | Poor — byte/text oriented | Good — `quick-xml`/`serde_json` events |
| Compile time | Heavy generics; slowest of the three | Lighter; closer to `nom` baseline | Zero added |
| Binary size | Largest (heavy monomorphisation) | Medium | Smallest |
| Port effort from Phase A | High — rewrite recursive descent | Medium — closer to imperative style | Zero — reuse Turtle shape |

## §3 Compile / size data

Not measured in session. Per public rosetta-rs benchmarks and the
respective crate READMEs, the community-reported pattern is that
chumsky adds the largest compile-time hit among pure-Rust combinator
libraries (heavy generic machinery, `Parser` trait with many associated
types), winnow is in the same ballpark as nom (noticeably lighter than
chumsky — rule-of-thumb "~2× faster to compile than chumsky"), and a
hand-rolled recursive-descent parser adds nothing beyond its own
source weight. Binary-size ordering follows the same shape. None of
these numbers are decisive on their own — the real cost driver is
maintenance of a new leaf dependency across Phase B's four crates,
not the millisecond-level compile delta.

## §4 Port sketch

Turtle object-list production, chumsky-flavoured pseudocode vs
hand-rolled shape we already ship in Phase A:

```rust
// chumsky flavour
let object_list = object
    .separated_by(just(Token::Comma).padded())
    .at_least(1)
    .collect::<Vec<_>>()
    .labelled("objectList")
    .recover_with(skip_then_retry_until([Token::Semi, Token::Dot]));

// hand-rolled (the shape in crates/syntax/rdf-turtle today)
fn object_list(p: &mut Parser<'_>) -> Result<Vec<Object>, Diag> {
    let mut out = vec![p.object()?];
    while p.eat(Token::Comma) {
        out.push(p.object()?);
    }
    Ok(out)
}
```

The combinator form buys `recover_with` for free; the hand-rolled form
keeps that logic in a `sync_to(&[Token::Semi, Token::Dot])` helper we
already have. Net ergonomic gain: small, for one production, at the
cost of a new transitive dep across four crates.

## §5 Recommendation for Phase B scope

- **N-Triples / N-Quads**: hand-roll. Line-oriented, trivial grammar,
  already has a precedent shape in `rdf-turtle`. A combinator here is
  over-engineering.
- **TriX**: hand-roll over `quick-xml` events. XML SAX-style streaming
  is orthogonal to what chumsky/winnow are good at; a combinator would
  sit *above* the XML event stream and buy nothing.
- **RDF/XML**: same — `quick-xml` events + hand-rolled state machine
  for the RDF/XML production rules. This is the hardest of the four
  grammatically but the least combinator-friendly.
- **JSON-LD**: hand-roll a tree walker over `serde_json::Value`
  (already on the allow-list). JSON-LD's expansion/compaction is an
  algorithmic tree transform; parser combinators solve the wrong
  problem here.
- **N3** (if in Phase B scope at all — confirm with queen): hand-roll
  as a Turtle superset by forking the Turtle recogniser. The
  combinator-assist case is real here but weak.

The frozen `Parser` trait in `crates/testing/rdf-diff/src/lib.rs` is
agnostic to parser implementation — it only fixes the input/output
shape at the seam — so hand-rolling preserves full freedom and costs
nothing architecturally.

## §6 Open questions for the queen

- Is N3 actually in Phase B's four-parser slate, or is the "four" =
  {NT, NQ, TriX, RDF/XML, JSON-LD} minus one? The combinator decision
  flips if N3 is in and large; otherwise it does not.
- Should ADR-0004's row 67 ("chumsky **or** winnow") be **closed
  now** by this ADR (retiring the option), or left open for the
  SPARQL-query phase where grammar size and error-recovery quality
  genuinely tip the calculus?
- Is there appetite for a `logos`-only allow-list tightening — i.e.
  lexer generator yes, combinator library no — as the Phase B
  guardrail, to make "hand-roll the productions" the explicit house
  style?
