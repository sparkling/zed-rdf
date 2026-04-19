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
| IRI-003b | `%ED%A0%80` surrogate rejected   | **RESOLVED — both reject per IRI-SURROGATE-001** — `fe-surrogate-reject` pinned `docs/spec-readings/iri/lone-surrogate-rejection.md` (code `IRI-SURROGATE-001`) and added a pct-triplet-pair decode in `rdf-iri`'s `validate_run` and `rdf-iri-shadow`'s `validate_pct_encoding`. Both now emit the `IRI-SURROGATE-001` fatal diagnostic for `%ED %A0..%BF`-initiated byte pairs; the boundary `%ED%9F%BF` (U+D7FF) continues to accept. Adversary assertion flipped to `both-reject`. |
| IRI-004  | Pct case unified                 | **`ObjectMismatch` — real divergence** — main preserves `%c3%a9`/`%C3%A9` distinct per the `percent-encoding-3986-vs-3987` pin; shadow uppercases both to `%C3%A9` on normalise, unifying them. |
| IRI-005  | `urn:`/`tag:` rejected           | **Both accept (bug #1 fixed)** — main's `validate_path` now guards the RFC 3986 §4.2 first-segment-colon rule on `!has_scheme && !has_authority`, so `urn:example:a-resource`, `urn:isbn:*`, `urn:example:foo#bar`, and `tag:example.org,2024:*` all parse. `data:,hello` is unchanged (its path never had a colon). Shadow agrees. See "Parser bugs surfaced" below for the resolution. |
| IRI-006  | Empty-base merge wrong           | **Agree (hypothesis did not fire)** — main's §5.2.3 Merge Paths implementation inserts the slash correctly; shadow agrees on the resolved normalised form. |
| IRI-007  | Host case folded                 | **Both fold (hypothesis did not fire)** — main's `normalise()` ASCII-lowercases the host per RFC 3490 §4 + 3986 §6.2.2.1; shadow does the same. The raw `Iri::parse` output preserves case, so callers comparing raw bytes retain RDF string-equality. This is documented in `docs/spec-readings/iri/idna-host-normalisation-pin.md`. |
| IRI-008  | NFC silently unifies NFC/NFD     | **Agree (hypothesis did not fire)** — neither parser applies NFC; both preserve `%C3%A9` and `cafe%CC%81` as distinct byte sequences. |

## Parser bugs surfaced

### Bug #1 — IRI-005: `validate_path` applies RFC 3986 §4.2 to absolute IRIs (FIXED)

**Status.** Fixed in `crates/rdf-iri/src/parse.rs` (`fe-urn-scheme-bug`
follow-up to this brief). IRI-005 now classifies as `both-accept-same`.
Summary kept here for provenance.

**Location.** `crates/rdf-iri/src/parse.rs::validate_path`
(`!has_authority && !slice.starts_with('/')` branch, previously lines
272–287).

**Why wrong.** RFC 3986 §4.2 forbids `:` in the first segment of a
**relative-path reference** to avoid ambiguity with a scheme-prefixed
absolute IRI. Once a scheme has already been parsed (the caller's `parts`
has `scheme.is_some()`), the reference is absolute and §4.2 no longer
applies — indeed, the ambiguity it protects against has already been
resolved by parsing the scheme. The original check guarded only on
`has_authority`, so every authority-less absolute IRI whose path
contained a colon (`urn:*:*`, `tag:*,*:*`, `mailto:user:ext@host`, etc.)
was rejected.

**Shadow behaviour.** Shadow parses these correctly: it splits on the
first `:` to produce a scheme and treats the rest as `path`, with no
§4.2 check.

**Fix applied.** The `validate_path` signature gained a `has_scheme`
argument, threaded from `parts.scheme.is_some()`. The §4.2 guard now
reads

```rust
if !has_scheme && !has_authority && !slice.starts_with('/') {
```

Before/after: `urn:example:foo`, `urn:isbn:0-486-27557-4`,
`tag:example.com,2026:bar`, `urn:example:foo#bar`, and
`tag:example.org,2024:resource-1` were all rejected with
`DiagnosticCode::Syntax` "first segment of a relative-path reference
must not contain ':'"; they are now accepted. Relative references
without a scheme (e.g. `1a:b/c`) still reject as before. The
dot-segment workaround `./foo:bar` — already accepted because `.` is a
dot-segment whose next `/` ends the first segment — remains accepted.

`crates/testing/rdf-diff/tests/adversary_iri.rs::iri_005_authority_less_iris_accepted`
now asserts `both-accept-same` for the four urn/tag cases (`data:,hello`
kept as a control). Five new unit tests in
`crates/rdf-iri/src/tests.rs` pin the behaviour:
`parse_accepts_urn_example_foo`, `parse_accepts_urn_isbn`,
`parse_accepts_tag_uri`, `parse_rejects_relative_ref_with_colon_in_first_segment`,
and `parse_accepts_relative_ref_dot_segment_workaround`.

**Latent finding — RESOLVED.** IRI-003b (surrogate via `%ED%A0%80`).
The latent shared bug was picked up by `fe-surrogate-reject`: the
pin `docs/spec-readings/iri/lone-surrogate-rejection.md` now
governs this reading (`IRI-SURROGATE-001`). Both `rdf-iri`'s
`validate_run` and `rdf-iri-shadow`'s `validate_pct_encoding` decode
pct-triplet pairs and reject whenever the first two bytes form the
UTF-8 encoding of a surrogate scalar (`0xED` followed by
`0xA0..=0xBF`). The boundary case `%ED%9F%BF` (U+D7FF) continues to
accept. Both parsers emit `IRI-SURROGATE-001` as a fatal diagnostic;
the adversary test now asserts `both-reject` with that code cited in
both diagnostics.

## Proposed follow-ups

1. ~~**Fix bug #1.**~~ Done (`fe-urn-scheme-bug`). IRI-005 now
   classifies `both-accept-same`; assertion + comment updated in
   `adversary_iri.rs`; unit regressions pinned in `rdf-iri/src/tests.rs`.
2. ~~**IRI-003b strictness.**~~ Done (`fe-surrogate-reject`). Pin
   `docs/spec-readings/iri/lone-surrogate-rejection.md`
   (`IRI-SURROGATE-001`) adopted; both parsers decode pct-triplet
   pairs at validation time and reject surrogate-encoding byte runs
   with a fatal diagnostic. The adversary assertion flipped from
   `both-accept-same` to `both-reject` and now also asserts that
   both diagnostics cite the `IRI-SURROGATE-001` code.
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
