# Pin: Turtle relative IRI without a base

- **Diagnostic code:** `TTL-BASE-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** any `IRIREF` occurrence in a position requiring an
  absolute IRI (§6.3 "IRIs"; §3 "RDF Term Constructors").
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Turtle §6.3:

> "Relative IRIs are resolved against the `In-Scope Base IRI` as
> specified in RFC 3986 §5.1 and §5.2."

The spec defines five sources for the base (§6.3 enumeration);
where none is in scope, a relative IRI cannot be resolved. The
spec is silent on what the parser must do in that case, giving two
plausible readings:

1. **Accept and emit the relative IRI as-is.** Some libraries accept
   a relative IRI term when no base is in scope, producing a
   non-absolute IRI in the emitted RDF — which subsequent layers
   must handle or reject.
2. **Reject the document.** The RDF 1.1 Concepts §3.2 requirement
   that RDF IRIs be absolute makes any relative-without-base
   scenario a terminal error.

## Reading chosen

**Reject.** `TTL-BASE-001` is fatal. The parser rejects any
relative `IRIREF` encountered in an RDF-term position when no base
IRI is in scope (no `@base` / `BASE` directive, no externally
supplied base, no document-URI fallback). The rejection points at
the relative IRI's byte offset.

## Rationale

- RDF 1.1 Concepts §3.2 requires absolute IRIs for every
  subject / predicate / object / graph-name IRI; emitting a relative
  IRI to downstream layers is therefore a silent-corruption risk,
  not a loud parse error — exactly the hazard the ADR-0019 pin
  regime is designed to eliminate.
- Cohort-B adversary brief
  `docs/verification/adversary-findings/turtle.md` flags
  "relative-IRI-without-base" as a historical divergence surface
  across libraries; a pinned rejection here kills the divergence
  class.
- Positive tests in the Turtle test suite (e.g.
  `turtle-eval-struct-03`) always pair relative IRIs with a
  declared or externally supplied base; the negative path is
  exercised by `turtle-syntax-bad-base-*`.

## Diagnostic code

- **Code:** `TTL-BASE-001`
- **Emitted by:** `rdf-turtle` parser (see
  `crates/rdf-turtle/src/diag.rs:51` for the code definition).
- **Message template:**
  `TTL-BASE-001: relative IRI <...> used but no base IRI is in scope (byte <N>)`.
- **Fatal?** Yes.
