# Pin: IRI percent-encoding — RFC 3986 vs RFC 3987 normalisation

- **Diagnostic code:** `IRI-PCT-001`
- **Language / format:** cross-cutting — Turtle, N-Triples, N-Quads,
  TriG, SPARQL 1.1, RDF/XML, JSON-LD.
- **Productions:** `IRIREF` in every RDF serialisation; IRI-valued
  terms in SPARQL; value-object IRI strings in JSON-LD.
- **Spec target:** RDF 1.1 Concepts §3.1
  <https://www.w3.org/TR/rdf11-concepts/#section-IRIs>; RFC 3987
  <https://www.rfc-editor.org/rfc/rfc3987>; RFC 3986
  <https://www.rfc-editor.org/rfc/rfc3986>.
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Concepts §3.1 "IRIs":

> "Two IRIs are equal if and only if they are equivalent under Simple
> String Comparison according to section 5.1 of [RFC3987]."

From RFC 3986 §6.2.2.1 "Case Normalization":

> "The hexadecimal digits within a percent-encoding triplet (e.g.,
> `%3a` versus `%3A`) are case-insensitive and therefore should be
> normalized to use uppercase letters for the digits A-F."

From RFC 3987 §3.1 "Converting IRIs to URIs":

> Specifies a procedure that percent-encodes non-ASCII characters to
> produce a URI from an IRI. Does **not** mandate that RDF
> implementations normalise percent-encoding before comparison.

Tension: RFC 3986 recommends case-normalising `%hh` to uppercase for
comparison; RDF 1.1 Concepts uses **Simple String Comparison** of the
IRI character sequence. A strict SSC reading makes
`<http://example/caf%c3%a9>` and `<http://example/caf%C3%A9>` two
**different** IRIs. A "helpful" normaliser would unify them.

The symmetrical question for non-ASCII: should an IRI containing a
literal `é` be unified with one containing `%C3%A9`? RFC 3986 §6.2.3
"Percent-Encoding Normalization" recommends decoding unreserved
percent-encoded octets; RDF 1.1 Concepts does **not**.

## Reading chosen

The parser MUST treat IRI equality as **byte-for-byte Simple String
Comparison of the IRI character sequence** after grammar-level
acceptance and base-IRI resolution. In particular:

1. **No hex case folding inside percent-encoding.** `%2F` and `%2f`
   are **different** IRI characters; `<http://example/a%2Fb>` and
   `<http://example/a%2fb>` are distinct IRIs.
2. **No percent-encoding decode for comparison.** `<…/caf%C3%A9>`
   and `<…/café>` are distinct IRIs. The parser does **not** decode
   the percent-encoding before storing the IRI.
3. **No host case folding.** `<http://EXAMPLE.COM/>` and
   `<http://example.com/>` are distinct.
4. **No Unicode NFC/NFD normalisation.** `<…/café>` (NFC, U+00E9)
   and `<…/cafe\u0301>` (NFD, `e` + COMBINING ACUTE) are distinct.
5. **Base-IRI resolution per RFC 3986 §5.2.2** (including the "remove
   dots" algorithm with the "cannot ascend above root" clarification
   of RFC 3986 errata ID 4005) DOES happen at parse time, because
   the base-resolution step is grammar-mandated in Turtle §6.3 and
   SPARQL §3.1. Base resolution is separate from normalisation.

Output of the parser is the **byte-for-byte IRI character sequence**
produced by that mandated base resolution and nothing else. Any
downstream layer (e.g. a reasoner) that wants RFC 3986 §6.2 or RFC
3987 §5.3 equivalence must call that out explicitly.

## Rationale

- RDF 1.1 Concepts §3.1 says "Simple String Comparison according to
  section 5.1 of RFC3987". RFC 3987 §5.1 is literally a byte-for-byte
  comparison of the IRI character sequence; it does **not** invoke
  the §5.3 normalisation ladder. Using §6.2 of RFC 3986 for
  comparison contradicts §5.1 of RFC 3987.
- W3C public-rdf-comments thread "IRI normalization and comparison"
  (2011, revisited 2022) explicitly confirms the WG's intent:
  normalisation is an application concern, not an RDF-parser concern.
- The cohort-B adversary brief `docs/verification/adversary-findings/iri.md`
  Failure Modes 4, 7, 8 call out the three normalisation traps
  (hex case, host case, NFC). Pinning "no normalisation at parse
  time" resolves all three in one decision.
- The `oxttl` / `oxrdfxml` / `oxjsonld` / `oxsparql-syntax` oracles
  (ADR-0019 §1) share the no-normalisation posture; cross-diff
  against them is silent when we follow this pin and noisy if we
  deviate.
- Base-resolution errata alignment: RFC 3986 errata 4005 corrects an
  ambiguity in §5.2.4 step 2C (`/../` at the path root collapses to
  `/`, not an error). Adopting the errata brings us in line with
  both oracles and with W3C test suite expectations.

## Non-goals / out of scope

This pin does **not** decide:

- Whether a reasoner or SPARQL query engine may choose to expose
  normalisation-aware `sameAs` semantics at the application layer.
  That is out of scope for the parser-level equality rule this pin
  governs.
- Whether a canonicalisation step at serialisation time may
  uppercase hex digits for output tidiness. Serialisation
  canonicalisation is governed by ADR-0006 §Snapshot rather than
  by this pin.

## Diagnostic code

- **Code:** `IRI-PCT-001`
- **Emitted by:** every parser that handles IRIs (`rdf-iri`,
  `rdf-iri-shadow`, `rdf-ntriples`, `rdf-turtle`, `sparql-syntax`,
  their shadows, all `ox*` oracle adapters).
- **Message template:**
  `IRI-PCT-001: IRI equality is byte-for-byte post base-resolution; <detail>`
  (usually emitted as a non-fatal trace in `DiffReport.triage_hint`
  when a divergence smells like normalisation disagreement).
- **Fatal?** No. A **violation** of the pin (e.g. the parser folded
  hex case) shows up as a diff-harness `ObjectMismatch` or
  `FactOnlyIn` with this code in the triage hint.

## Forward references

- `crates/syntax/rdf-iri/SPEC.md` — TODO: cite `IRI-PCT-001` under
  "Pinned readings"; cross-ref RFC 3986 errata 4005.
- `crates/syntax/rdf-iri-shadow/` implements the same byte-for-byte
  rule.
- Every downstream parser that carries its own IRI state (Turtle,
  N-Triples, SPARQL) must defer IRI construction to `rdf-iri` and
  inherit this pin transitively.
- Adversary fixtures:
  `tests/adversary-iri/iri-pct-001-hex-case-{lower,upper}.nt`,
  `tests/adversary-iri/iri-pct-001-nfc-nfd.nt`,
  `tests/adversary-iri/iri-pct-001-host-case.nt`.
