# Pin: lone-surrogate rejection in pct-encoded IRI bytes

- **Diagnostic code:** `IRI-SURROGATE-001`
- **Language / format:** cross-cutting — Turtle, N-Triples, N-Quads,
  TriG, SPARQL 1.1, RDF/XML, JSON-LD.
- **Productions:** `IRIREF` in every RDF serialisation; IRI-valued
  terms in SPARQL; value-object IRI strings in JSON-LD.
- **Spec target:** RFC 3987 §1.4 + §2.2
  <https://www.rfc-editor.org/rfc/rfc3987>; RFC 3987 Errata 3937
  <https://www.rfc-editor.org/errata/eid3937>; RFC 3629 §3 (UTF-8
  surrogate exclusion) <https://www.rfc-editor.org/rfc/rfc3629#section-3>;
  Unicode 15 §3.9 D76–D79 (surrogate code points are not scalar values).
- **Status:** active.
- **Author:** `fe-surrogate-reject` (cohort A), follow-up to
  `cu2-adversary-iri`'s latent-shared-bug finding.
- **Date:** 2026-04-19.

## Ambiguous clause

RFC 3987 §1.4 says the IRI character set is **Unicode scalar values**.
§2.2 gives the ABNF for `ucschar` but does not explicitly restate
that `pct-encoded` triplets decode to bytes in a *valid* UTF-8
sequence; a naive reading would accept any `%HH %HH %HH` as
"three opaque bytes". The grammar alone therefore does not forbid
`%ED%A0%80` (the UTF-8 encoding of U+D800).

RFC 3629 §3 ("UTF-8 definition") forbids surrogate code points
(U+D800..U+DFFF) in UTF-8 streams: they are not scalar values.
RFC 3987 Errata 3937 carries this through to IRIs explicitly:

> Surrogates (code points in the range U+D800 to U+DFFF) are not
> Unicode scalar values, and therefore cannot appear in an IRI.
> Implementations MUST NOT produce IRIs containing surrogate
> characters, and SHOULD NOT accept them in input; surrogates MUST
> NOT be generated when converting IRIs to URIs and MUST be rejected
> when reversing the mapping.

Adversary cohort B's fixture `iri-003-surrogate-rejection.nt` probes
this exact corner. `docs/verification/adversary-findings/iri/divergences.md`
previously recorded the finding as a **shared under-enforcement**:
both `rdf-iri` and `rdf-iri-shadow` admitted `%ED%A0%80` because
neither parser decoded pct-triplets at validation time. The adversary
cohort wrote "not yet a bug per our current pin". This pin closes
that gap.

## Reading chosen

Every IRI validator MUST reject any pct-encoded byte sequence that
decodes to a value in the UTF-16 surrogate range `U+D800..U+DFFF`.

**Detection rule.** A three-byte UTF-8 surrogate encoding has the
form `0xED  (0xA0..=0xBF)  (0x80..=0xBF)`. When the validator
encounters a `%HH` triplet whose decoded byte is `0xED` AND the
immediately following three bytes are a well-formed `%HH` triplet
whose decoded byte is in `0xA0..=0xBF`, the run MUST be rejected.
The third byte's value is not consulted — the first two bytes alone
prove the sequence is a surrogate half, and `0xED` followed by any
second byte outside the surrogate window (e.g. `0x9F` in the
just-below-surrogate boundary encoding `%ED%9F%BF` = U+D7FF, or
`0x80..=0x9F` generally) is a legitimate three-byte UTF-8 prefix
and MUST be accepted.

The rejection is **fatal**: the emitted diagnostic's code is
`IRI-SURROGATE-001` and the `fatal` flag on the `Diagnostics` shape
is set. The offset points at the `%` of the offending first triplet.

**Scope of the check.** The rule applies to every pct-encoded
subcomponent the validator visits — `iuserinfo`, `ireg-name` host
(i.e., excluding IP-literals in `[..]`), `ipath-*`, `iquery`,
`ifragment`. The main parser's `validate_run` helper applies to all
five; the shadow's `validate_pct_encoding` pass applies to the
authority, path, query, and fragment in turn. (Hosts that reach the
shadow via the authority slice are validated as part of that slice.)

**Interaction with `IRI-PCT-001`.** The byte-for-byte equality pin
says we do not decode pct-encoding for comparison. This pin does not
contradict that: surrogate detection decodes **only to inspect** and
does not mutate the stored IRI. Byte-for-byte string equality of
accepted IRIs is unchanged; the set of accepted IRIs shrinks.

**Non-interaction with IDN.** `idna::domain_to_ascii_strict` would
reject surrogate code points even if `encode_host` reached the
fallback, but the fallback only fires for `ireg-name`s that the
parser accepted; once parse-time rejection is in place the fallback
can no longer see a surrogate scalar. No change to the IDN pin.

## Rationale

- Errata 3937 is the authoritative source; it is binding on every
  RFC-3987-conformant implementation.
- RFC 3629 §3 forbids surrogate UTF-8 in the first place, so
  accepting `%ED%A0%80` silently ships an IRI whose URI form would
  be ill-formed UTF-8 under a strict decoder. Rejecting at parse
  keeps the diagnostic close to the user's input.
- The boundary `%ED%9F%BF` (U+D7FF) is a legitimate private-use
  assignment and appears in real content; the detection rule
  deliberately looks only at the second byte's upper nibble so
  this boundary accepts.
- The adversary-corpus divergence record called this out as a
  latent shared bug. Fixing both sides independently (main and
  shadow) restores the "zero divergences is suspicious" health
  signal (ADR-0020 §Validation) for this fixture — both reject with
  the same diagnostic code, which is the correct post-fix shape.
- `oxttl` / `oxrdfxml` / `oxsparql-syntax` oracles all reject
  `%ED%A0%80` at the IRI layer; our pre-pin behaviour was an
  outlier.

## Non-goals / out of scope

- Rejection of pct-encoded **overlong** UTF-8 (e.g. `%C0%80` = an
  alternative encoding of NUL). That is a separate RFC 3629 §3
  constraint; a future pin `IRI-OVERLONG-001` would handle it.
- Rejection of pct-encoded **non-shortest** UTF-8 forms beyond the
  surrogate case. Again, a future pin.
- Rejection of surrogate characters that appear **literally** (not
  pct-encoded) in input. The parser already rejects those via its
  UTF-8 decode — Rust `&str` cannot hold lone surrogates, and byte
  input that contains raw `0xED 0xA0..` fails `from_utf8` before
  `validate_run` / `validate_pct_encoding` is reached. This pin
  covers only the pct-encoded channel.
- Legacy "WTF-8" / UTF-16-via-UTF-8 tolerance. Some systems encode
  surrogates this way for backward compat; ADR-0020 §Reading-choice
  prefers strict conformance over legacy compatibility. A future
  amendment could add a `wtf8` feature-gated opt-in; none planned.

## Diagnostic code

- **Code:** `IRI-SURROGATE-001`
- **Emitted by:** every parser that handles IRIs (`rdf-iri`,
  `rdf-iri-shadow`, and downstream parsers that defer IRI
  construction to `rdf-iri`). Oracle adapters (`ox*`) surface the
  source library's native error; the triage hint maps them to this
  code.
- **Message template (main):**
  `IRI-SURROGATE-001: pct-encoded byte sequence decodes to a UTF-16 surrogate (U+D800..U+DFFF); forbidden by RFC 3987 Errata 3937`
- **Message template (shadow):**
  `IRI-SURROGATE-001: pct-encoded byte sequence at offset <N> decodes to a UTF-16 surrogate (U+D800..U+DFFF); forbidden by RFC 3987 Errata 3937`
- **Fatal?** Yes. The diagnostic is emitted at parse time with
  `fatal = true`; the `ParseOutcome` path is never reached.

## Forward references

- `crates/rdf-iri/src/parse.rs` — `validate_run` carries the
  detection for the main parser; `DiagnosticCode::SurrogatePct`
  stringifies as `IRI-SURROGATE-001`.
- `crates/syntax/rdf-iri-shadow/src/inner.rs` —
  `validate_pct_encoding` carries the independently-written
  detection for the shadow; `IriError::SurrogatePctEncoding`
  carries the offset.
- `crates/rdf-iri/src/tests.rs` — five unit tests cover lone high,
  lone low, encoded surrogate pair, just-below boundary accept, and
  query/fragment-position rejection.
- `crates/syntax/rdf-iri-shadow/src/inner.rs` (test module) — five
  tests with the same shape, independently asserted.
- `crates/testing/rdf-diff/tests/adversary_iri.rs` —
  `iri_003b_surrogate_rejected_by_strict_parser` now asserts
  `both-reject` with both diagnostics citing `IRI-SURROGATE-001`.
- `docs/verification/adversary-findings/iri/divergences.md` — the
  "Latent shared bug" finding is marked `RESOLVED`.
