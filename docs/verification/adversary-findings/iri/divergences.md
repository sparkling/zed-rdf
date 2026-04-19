# IRI adversary-corpus divergences (Phase A)

Owner: `cu2-adversary-iri`. Scope: per-fixture findings now that the 11
previously-ignored diff tests in
`crates/testing/rdf-diff/tests/adversary_iri.rs` are all active. Reviewed
by `phaseA-reviewer` at handoff.

## Summary

All 20 tests in `adversary_iri.rs` run on every `cargo test -p rdf-diff
--all-features --test adversary_iri`:

- 9 structural always-on tests (fixture presence + README).
- 11 per-fixture / per-IRI diff tests, all un-ignored and passing.

The tests compare the main parser
([`rdf_iri::Iri::parse`](../../../crates/rdf-iri/src/lib.rs) +
[`Iri::normalise`](../../../crates/rdf-iri/src/lib.rs) +
[`Iri::resolve`](../../../crates/rdf-iri/src/resolve.rs)) against the
shadow parser
([`rdf_iri_shadow::parse`](../../../crates/syntax/rdf-iri-shadow/src/inner.rs)
+ `rdf_iri_shadow::normalise`) on the IRIs named by each
`docs/verification/adversary-findings/iri.md` failure-mode brief.

Why per-IRI rather than full-document diffs: the `rdf_iri::IriParser`
implementation of `rdf_diff::Parser` treats its byte input as a single
IRI. Feeding a `.ttl`/`.nt` document whole would only exercise the
outermost envelope, not the IRI-specific claim. The per-IRI shape lets
each test target the failure mode it is named for.

## Fixture-level findings

| Fixture  | Brief's hypothesis              | Outcome                                         |
|----------|----------------------------------|-------------------------------------------------|
| IRI-001  | Wrong `../../..` above-root      | **Agree (hypothesis did not fire)** — main clamps at root correctly; shadow agrees on normalised resolved form. |
| IRI-002  | Double-fragment leak             | **Agree (hypothesis did not fire)** — main's `Iri::resolve` replaces fragment cleanly. |
| IRI-003a | U+E001 private-use rejected      | **`AcceptRejectSplit` (inverted)** — main strictly rejects per RFC 3987 §2.2 (ucschar excludes iprivate for ipath-*); shadow accepts and pct-encodes on normalise. Strictness here is correctness, not a bug. |
| IRI-003a | Pct-encoded supplementary PU     | **Agree** — both parsers pass `%F3%B0%80%81` through unchanged. |
| IRI-003b | `%ED%A0%80` surrogate rejected   | **Shared under-enforcement** — both parsers accept; hypothesised `AcceptRejectSplit` did not fire. RFC 3987 Errata 3937 requires rejection; neither parser decodes pct-encoding to check. Recorded as a latent finding. |
| IRI-004  | Pct case unified                 | **`ObjectMismatch` — real divergence** — main preserves `%c3%a9`/`%C3%A9` distinct per the `percent-encoding-3986-vs-3987` pin; shadow uppercases both to `%C3%A9` on normalise, unifying them. |
| IRI-005  | `urn:`/`tag:` rejected           | **`AcceptRejectSplit` — real main bug** — main's `validate_path` applies the RFC 3986 §4.2 "first segment of a relative-path reference must not contain `:`" rule even when a scheme is present, so `urn:example:a-resource`, `urn:isbn:*`, `urn:example:foo#bar`, and `tag:example.org,2024:*` are all rejected. Shadow accepts. `data:,hello` escapes because its path has no colon. See "Parser bugs surfaced" below. |
| IRI-006  | Empty-base merge wrong           | **Agree (hypothesis did not fire)** — main's §5.2.3 Merge Paths implementation inserts the slash correctly; shadow agrees on the resolved normalised form. |
| IRI-007  | Host case folded                 | **Both fold (hypothesis did not fire)** — main's `normalise()` ASCII-lowercases the host per RFC 3490 §4 + 3986 §6.2.2.1; shadow does the same. The raw `Iri::parse` output preserves case, so callers comparing raw bytes retain RDF string-equality. This is documented in `docs/spec-readings/iri/idna-host-normalisation-pin.md`. |
| IRI-008  | NFC silently unifies NFC/NFD     | **Agree (hypothesis did not fire)** — neither parser applies NFC; both preserve `%C3%A9` and `cafe%CC%81` as distinct byte sequences. |

## Parser bugs surfaced

### Bug #1 — IRI-005: `validate_path` applies RFC 3986 §4.2 to absolute IRIs

**Location.** `crates/rdf-iri/src/parse.rs` `validate_path`, around lines
272–287 (`!has_authority && !slice.starts_with('/')` branch).

**Why wrong.** RFC 3986 §4.2 forbids `:` in the first segment of a
**relative-path reference** to avoid ambiguity with a scheme-prefixed
absolute IRI. Once a scheme has already been parsed (the caller's `parts`
has `scheme.is_some()`), the reference is absolute and §4.2 no longer
applies — indeed, the ambiguity it protects against has already been
resolved by parsing the scheme. The current check guards only on
`has_authority`, so every authority-less absolute IRI whose path contains
a colon (`urn:*:*`, `tag:*,*:*`, `mailto:user:ext@host`, etc.) is
rejected.

**Shadow behaviour.** Shadow parses these correctly: it splits on the
first `:` to produce a scheme and treats the rest as `path`, with no
§4.2 check.

**Proposed fix.** Change the guard at `parse.rs` §validate_path's §4.2
branch from

```rust
if !has_authority && !slice.starts_with('/') {
```

to

```rust
if scheme.is_none() && !has_authority && !slice.starts_with('/') {
```

This requires threading `parts.scheme.is_none()` down to
`validate_path`. After the fix, IRI-005 will flip from
`AcceptRejectSplit` to `both-accept-same`; the test comment should be
updated to remove the "bug still stands" caveat, and the assertion
updated to `both-accept-same`.

**Latent finding.** IRI-003b (surrogate via `%ED%A0%80`). Both parsers
accept; neither decodes pct-encoding to check whether the resulting
byte sequence encodes a lone UTF-16 surrogate scalar. RFC 3987 Errata
3937 says surrogates must never appear as scalar values. A future
RFC-3987-strict pass would decode pct-sequences during validation and
reject when the decoded byte sequence forms a UTF-8 surrogate encoding.
Not yet a bug per our current pin, but recorded here so a future
adversary cohort can pick it up.

## Proposed follow-ups

1. **Fix bug #1.** `rdf-iri` maintainer flips IRI-005 from split to
   same-accept; update the assertion + test comment in
   `adversary_iri.rs`.
2. **IRI-003b strictness.** Decide whether to decode pct-sequences
   in `validate_path`/`validate_userinfo`/`validate_host` to reject
   surrogate-encoding byte runs. Most likely a `DiagnosticCode` of
   its own (e.g. `IRI-PCT-002`). Track via a new pin in
   `docs/spec-readings/iri/pct-surrogate-strictness.md`.
3. **IRI-007 normalisation pin clarity.** The pin already says host
   case is folded by `normalise()`. Callers comparing raw bytes for
   RDF string-equality must go through `Iri::as_str()`, not
   `Iri::normalise().as_str()`. Double-check downstream call sites.

## References

- `docs/verification/adversary-findings/iri.md` — failure-mode brief.
- `docs/verification/tests/catalogue.md` — authoritative status table.
- `crates/testing/rdf-diff/tests/adversary_iri.rs` — test bodies.
- `docs/spec-readings/iri/percent-encoding-3986-vs-3987.md` —
  `IRI-PCT-001` pin (basis for IRI-004 asymmetry).
- `docs/spec-readings/iri/idna-host-normalisation-pin.md` —
  host-case fold pin (basis for IRI-007 normalise shape).
- ADR-0019 §4 — adversary-corpus responsibilities.
- ADR-0020 §Validation — "zero divergences is suspicious".
