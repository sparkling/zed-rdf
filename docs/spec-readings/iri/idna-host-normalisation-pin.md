# Spec-reading pin: host IDNA normalisation (RFC 3987 §3.1 step 3)

**Agent:** v1-shadow-iri  
**Date:** 2026-04-19  
**Coordinate with:** v1-specpins

## Ambiguous production

RFC 3987 §3.1 step 3 says:

> For each `ireg-name` in the IRI, apply the `ToASCII` operation defined
> in RFC 3490 to each component of the `ireg-name`, …

`ToASCII` (RFC 3490 §4) is the full IDNA processing algorithm which
involves:

1. Map characters using `nameprep` (RFC 3491, a Stringprep profile).
2. Check for ASCII-only labels; if so, apply case-folding to lowercase.
3. If a label contains non-ASCII after mapping, apply ACE encoding
   (Punycode, RFC 3492) with the `xn--` prefix.

## Pin decision (v1-shadow-iri)

This implementation applies **ASCII lowercase only** (step 2 without
step 3).  Full IDNA (Punycode encoding, nameprep) is **not** implemented.

Rationale:

- The diff harness targets a no-network, no-DNS context.  IRIs in RDF
  data are rarely IDNs; the normalisation goal is comparison stability,
  not network-ready serialisation.
- A full IDNA dependency (e.g., `idna` crate) was not approved in the
  workspace `[workspace.dependencies]` at the time this crate was
  written.  Adding it requires an ADR-0004 amendment.
- Lowercasing is the common subset of all IDNA processing that applies
  to ASCII host labels (RFC 3490 §4 step 3a).

## Known divergence surface

If the main `rdf-iri` crate implements full `ToASCII`, the diff harness
**will** surface divergences on IRIs whose host contains non-ASCII
Unicode labels (e.g., `http://münchen.de/`).  This is an expected and
desired divergence — it validates that the diff harness is working.

If the main implementation also applies ASCII-lowercase only, the two
implementations agree here and the harness will not fire on this class
of input.  That is acceptable but should be noted in the Phase-A
integration report.

## References

- RFC 3987 §3.1 — IRI to URI mapping, step 3 (IDNA host normalisation).
- RFC 3490 §4 — `ToASCII` algorithm.
- RFC 3491 — Nameprep: A Stringprep Profile for IDN.
- RFC 3492 — Punycode: A Bootstring Encoding.
