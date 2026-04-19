# Pin: JSON-LD keyword aliasing — `@id`, `@type`, `@container`, `@vocab` aliases

- **Diagnostic code:** `JSONLD-KWALIAS-001`
- **Language / format:** JSON-LD 1.1 (toRDF / fromRDF API surface);
  applies equally to JSON-LD 1.0 with narrower keyword set.
- **Productions:** context processing §4.1, keyword aliasing §4.1.3,
  term definition §4.2, expansion algorithm §5.1, value-object
  §4.6 in JSON-LD 1.1.
- **Spec target:** W3C JSON-LD 1.1
  <https://www.w3.org/TR/json-ld11/>; JSON-LD 1.1 API
  <https://www.w3.org/TR/json-ld11-api/>; JSON-LD 1.0
  <https://www.w3.org/TR/json-ld/> (legacy, narrower alias set).
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From JSON-LD 1.1 §4.1.3 "Aliasing Keywords":

> "Each of the JSON-LD keywords, except for `@context`, may be
> aliased to application-specific keywords. This feature allows
> legacy JSON content to be utilized by JSON-LD."

From §9.4 "Context Definitions", term-definition algorithm:

> "If the value is `@type`, `@id`, … the term definition is a
> keyword alias."

Two ambiguities land on parsers:

1. **Which keywords are aliasable?** JSON-LD 1.1 §4.1.3 says "all
   except `@context`". JSON-LD 1.0 §6.5 allows a narrower subset
   (`@id`, `@type`, `@language`, `@value`, `@list`, `@set`,
   `@reverse`, `@graph` only). A document declaring
   `"@version": 1.0` in its context must refuse aliases for
   1.1-introduced keywords (`@included`, `@nest`, `@json`,
   `@direction`, `@protected`, etc.).
2. **Transitive aliasing.** May a term be aliased to another alias
   (`{"id": "@id", "identifier": "id"}`)? JSON-LD 1.1 §4.1.3 says:
   "An alias must expand directly to a JSON-LD keyword." Chained
   aliases are NOT permitted. A parser that resolves the alias
   chain transitively by walking the term table would silently
   accept `"identifier"` as an `@id` alias; the spec requires that
   it be rejected as an invalid term definition.

## Reading chosen

1. **Aliasable keyword set is spec-version-dependent.** Under a
   `{"@version": 1.1}` context (the default once the processing
   mode is "json-ld-1.1"), every keyword EXCEPT `@context` is
   aliasable. Under a `{"@version": 1.0}` context, only the
   JSON-LD 1.0 §6.5 set is aliasable; attempting to alias
   `@included`, `@nest`, `@json`, `@direction`, `@protected`, or
   any other 1.1-introduced keyword is an **invalid IRI mapping**
   error (JSON-LD 1.1 §4.2 term-definition algorithm, step that
   raises `invalid IRI mapping`).
2. **No alias chains.** A term definition whose value is itself
   the label of an existing alias (not a keyword string starting
   with `@`) is an **invalid term definition** error. The
   term-definition algorithm must check that the value starts
   with `@` AND names a recognised keyword; anything else falls
   through to regular IRI-mapping logic, not alias logic.
3. **`@context` is never aliasable.** Even under 1.1, attempting
   `{"ctx": "@context"}` is an **invalid term definition** error.
4. **Alias application is bidirectional.** On **expansion** (JSON-LD
   →   RDF), the parser maps the alias key back to its keyword
   before interpreting the value. On **compaction** (RDF →
   JSON-LD), the serialiser prefers the alias over the bare
   keyword when the active context defines one. For this
   verification sweep we only cover the toRDF direction
   (expansion); the fromRDF pin will be authored when the
   serialiser lands.

Canonical form after expansion is independent of the alias choice:
`{"id": "http://example/s", "@type": "…"}` with an `id → @id`
alias produces the same `Fact`s as the literal-keyword form.

## Rationale

- JSON-LD 1.1 §4.1.3 and §9.4 are both explicit on "expand directly
  to a JSON-LD keyword"; the term-definition algorithm in the
  JSON-LD 1.1 API (§Create Term Definition) raises
  `invalid IRI mapping` for a mismatched value.
- `oxjsonld` (ADR-0019 §1 oracle) and the official W3C JSON-LD 1.1
  test suite (`toRdf-manifest.jsonld` entries `#t0042`, `#te005`,
  `#te111`, `#tli04` and friends) enforce the no-chain and
  version-aware alias-set rules. An implementation that resolves
  chains transitively will fail those entries.
- The JSON-LD WG's handling of `@version` is recorded in the JSON-LD
  1.1 Primer and in the JSON-LD Community Group's threaded
  discussion "Processing mode and keyword aliasing" (w3c/json-ld-api
  issue #123 and #189); both reaffirm the version-aware reading.
- `@context` non-aliasability is explicit in §4.1.3 ("except for
  `@context`").
- The cohort-B adversary briefs do not yet carry a JSON-LD file;
  when `docs/verification/adversary-findings/jsonld.md` lands, fixtures
  targeting this pin will live under `tests/adversary-jsonld/` and
  carry the slug `jsonld-kwalias-001-*.jsonld`.

## Diagnostic code

- **Code:** `JSONLD-KWALIAS-001`
- **Emitted by:** `rdf-jsonld` (main parser) and its forthcoming
  shadow; `oxjsonld` oracle adapter surfaces the same code prefix.
- **Message templates:**
  `JSONLD-KWALIAS-001: invalid keyword alias '<term>' → '<value>' (not a keyword)`
  `JSONLD-KWALIAS-001: keyword '<kw>' not aliasable under @version 1.0`
  `JSONLD-KWALIAS-001: '@context' is not aliasable`
  `JSONLD-KWALIAS-001: alias chain '<term>' → '<term2>' not permitted`
- **Fatal?** Yes for all four message forms — they correspond to
  JSON-LD 1.1 `invalid term definition` / `invalid IRI mapping`
  errors.

## Forward references

- `crates/syntax/rdf-jsonld/SPEC.md` — TODO: add "Pinned readings"
  citing `JSONLD-KWALIAS-001`.
- A JSON-LD shadow crate will be added to
  `crates/syntax/rdf-jsonld-shadow/` in a later sweep (JSON-LD
  was not in the verification-v1 shadow scope; the `[dev-deps]`
  `oxjsonld` oracle carries the differential load until then).
  That future shadow must emit the same code.
- Adversary fixtures (pending cohort-B brief):
  `tests/adversary-jsonld/jsonld-kwalias-001-chain.jsonld`,
  `tests/adversary-jsonld/jsonld-kwalias-001-context-alias.jsonld`,
  `tests/adversary-jsonld/jsonld-kwalias-001-v10-new-keyword.jsonld`.
