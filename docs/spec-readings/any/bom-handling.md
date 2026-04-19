# Pin: UTF-8 BOM handling at top of file for every textual RDF format

- **Diagnostic code:** `ANY-BOM-001`
- **Language / format:** cross-cutting — Turtle, N-Triples, N-Quads,
  TriG, RDF/XML, JSON-LD, SPARQL 1.1 query and update.
- **Productions:** not a grammar production; an encoding-layer
  concern that sits before the lexer sees its first token.
- **Spec target:** RDF 1.1 Turtle §6.1 and §2; RDF 1.1 N-Triples §2;
  RDF/XML §2; JSON-LD 1.1 (RFC 8259 / RFC 7159 JSON); SPARQL 1.1
  Query §4.1; Unicode 15.0 §23.8 "Byte Order Mark".
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

The RDF-family specs are individually explicit about UTF-8 encoding
but uneven on what to do with a leading U+FEFF "Byte Order Mark"
(BOM, serialised as bytes `EF BB BF` in UTF-8):

- RDF 1.1 Turtle §6.1 "Encoding": "The encoding of a Turtle document
  is always UTF-8." — silent on the BOM.
- RDF 1.1 N-Triples §2: specifies US-ASCII subset of UTF-8, silent
  on BOM.
- RFC 8259 §8.1 (JSON): "Implementations MUST NOT add a byte order
  mark … Implementations … MAY ignore the presence of a byte order
  mark rather than treating it as an error."
- SPARQL 1.1 Query §4.1: specifies UTF-8, silent on BOM.
- Unicode §23.8: a BOM at the start of a UTF-8 stream is a signature
  only; it carries no textual content.

Two readings are plausible:

1. **Strict grammar reading.** The grammar's start production does
   not allow an initial `#xFEFF`; the parser rejects files that
   start with a BOM.
2. **Tolerant-consumer reading.** Per Unicode §23.8 and RFC 8259, a
   BOM at byte 0 is a no-op signature; the parser skips it before
   handing bytes to the lexer.

## Reading chosen

**Tolerant consumer, strict producer.** For every textual format in
scope:

1. **On parse:** if the input byte stream begins with the three-byte
   UTF-8 BOM (`EF BB BF`), the parser silently consumes those three
   bytes before its lexer sees the first token, as if the input had
   started at byte 3. A BOM anywhere **other than** byte offset 0
   is **not** consumed; it participates as a normal U+FEFF code
   point and is subject to the grammar's usual rules (which in
   practice reject it everywhere it is not allowed as a literal
   character).
2. **On serialise:** our serialisers never emit a BOM. Output
   starts with the first grammar-level token (a triple, a `@prefix`
   directive, a JSON object open brace, etc.).
3. **Byte-offset accounting:** `FactProvenance::offset` (frozen
   surface, `rdf-diff/src/lib.rs`) is the offset into the **original
   input**, including the skipped BOM bytes. This keeps error
   positions stable between parsers that skip the BOM and oracles
   that do the same, and means "byte offset 0" and "byte offset 3"
   both point at the first logical character correctly from the
   user's perspective.

## Rationale

- Every mainstream oracle the ADR-0019 §1 table lists (`oxttl`,
  `oxrdfxml`, `oxjsonld`, `oxsparql-syntax`) skips the leading BOM
  silently. If we rejected it we would produce
  `AcceptRejectSplit` divergences on every real-world file
  produced by a Windows-origin editor. That is cosmetic
  divergence, not a correctness signal.
- RFC 8259 §8.1 for JSON-LD explicitly permits the tolerant reading.
  For the JSON-LD parser we adopt the permissive posture because it
  is RFC-sanctioned; extending the same posture to the N-Triples /
  Turtle / SPARQL family keeps behaviour uniform and avoids a
  per-format BOM rule table.
- Unicode §23.8 treats the leading BOM as a signature, not
  character content. Skipping it is not a spec violation as long
  as the parser does not do so elsewhere in the stream.
- Strict producer discipline (no BOM on output) avoids the
  symmetrical problem: a BOM on output breaks round-tripping
  against consumers that reject it.
- There is no formal erratum mandating this reading; the pin
  records the arbitration so cohort A's parsers and cohort B's
  adversary fixtures agree on the target.

## Diagnostic code

- **Code:** `ANY-BOM-001`
- **Emitted by:** every textual parser and serialiser: `rdf-ntriples`,
  `rdf-turtle`, `sparql-syntax`, `rdf-jsonld`, `rdf-xml`, plus their
  shadows and oracles.
- **Message template (warning, non-fatal):**
  `ANY-BOM-001: leading UTF-8 BOM skipped at byte offset 0`
  (parsers MAY choose not to emit this warning; if they do, the
  prefix is fixed).
- **Message template (fatal):**
  `ANY-BOM-001: stray U+FEFF at byte offset <offset>` when the
  grammar rejects a mid-stream BOM.
- **Fatal?** No for a leading BOM; yes for a non-leading U+FEFF
  where the grammar does not admit it.

## Forward references

- `crates/syntax/rdf-*/SPEC.md` — each format's SPEC.md must
  cite `ANY-BOM-001` (shared pin) rather than re-stating the rule.
- `crates/testing/rdf-diff-oracles/` — oracle adapters must apply the
  same leading-BOM skip before handing bytes to the oracle parser,
  otherwise the diff harness reports spurious divergences.
- Adversary fixtures:
  `tests/adversary-ntriples/any-bom-001-leading.nt`,
  `tests/adversary-turtle/any-bom-001-midstream.ttl`,
  `tests/adversary-sparql/any-bom-001-leading.rq`.
