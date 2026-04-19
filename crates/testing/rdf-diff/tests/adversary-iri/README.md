# Adversary IRI Fixtures

Cohort: cohort-B (`v1-adv-iri`)
Sweep: verification-v1
ADR references: ADR-0019 §4, ADR-0020 §6.5
Spec references: RFC 3986, RFC 3987, RDF Concepts §3.1

These fixtures probe 8 IRI normalisation / resolution failure modes
identified in `docs/verification/adversary-findings/iri.md`.  Each fixture
is exercised by `crates/testing/rdf-diff/tests/adversary_iri.rs` under
the `adversary-iri` label that `xtask verify` tracks.

---

## Fixture Index

| ID       | File                                    | Finding | Format | Expected outcome |
|----------|-----------------------------------------|---------|--------|-----------------|
| IRI-001  | iri-001-remove-dots-above-root.ttl      | FM-1    | Turtle | Resolved IRIs must not escape root; `../../../d` → `http://example/d` |
| IRI-002  | iri-002-pure-fragment-resolution.ttl    | FM-2    | Turtle | `#section2` → `http://example/doc#section2` (base path preserved, fragment replaced) |
| IRI-003a | iri-003-surrogate-and-private-use.ttl   | FM-3    | Turtle | Private-use U+E001 accepted; surrogate via percent-encoding variant tested separately |
| IRI-003b | iri-003-surrogate-rejection.nt          | FM-3    | N-Triples | `%ED%A0%80` (lone surrogate encoded as UTF-8 of surrogate scalar) rejected as ill-formed |
| IRI-004  | iri-004-percent-encoding-case.nt        | FM-4    | N-Triples | `%c3%a9` and `%C3%A9` are DISTINCT subjects; no unification |
| IRI-005  | iri-005-urn-absoluteness.nt             | FM-5    | N-Triples | `urn:`, `tag:`, `data:` accepted as absolute IRIs without `://` |
| IRI-006  | iri-006-empty-base-path-merge.ttl       | FM-6    | Turtle | `<foo>` against `<http://example>` (no trailing slash) → `http://example/foo` |
| IRI-007  | iri-007-host-case-folding.nt            | FM-7    | N-Triples | `http://EXAMPLE.COM/s` and `http://example.com/s` remain DISTINCT subjects |
| IRI-008  | iri-008-nfc-normalization.nt            | FM-8    | N-Triples | `%C3%A9` (NFC) and `cafe%CC%81` (NFD) remain DISTINCT subjects |

---

## Per-Fixture Hypothesis

### IRI-001: remove-dots above-root path (FM-1)

**Spec:** RFC 3986 §5.2.4, IETF Errata 4005

The `remove_dot_segments` algorithm must clamp at the root.  A path of
`../../../d` against `http://example/a/b/c` must yield `http://example/d`,
not `http://d`.  Step 2C of the algorithm requires that a leading `/../`
that cannot be resolved further is treated as `/`.

**Likely miss:** Hand-rolled resolvers iterate `..` steps without checking
whether the remaining path stack is already empty, allowing the authority
boundary to be crossed.

---

### IRI-002: pure fragment reference (FM-2)

**Spec:** RFC 3986 §5.2.2

A pure-fragment reference `#foo` keeps the base scheme, authority, path,
and query.  Only the fragment is replaced (`T.fragment = R.fragment`).
The base path MUST be preserved.

**Likely miss A:** An implementation that strips the base fragment first
but then re-appends the old base fragment when R.fragment is present will
double-apply, producing `http://example/doc#section1section2`.

**Likely miss B:** An implementation that discards the base path when
resolving a fragment-only reference produces `http://example#section2`
(path `""` instead of `/doc`).

---

### IRI-003: surrogate and private-use code points (FM-3)

**Spec:** RFC 3987 §2.2, RFC 3987 Errata 3937

Private-use characters (U+E000–U+F8FF, supplementary planes) are
explicitly allowed in IRI strings.  Surrogate code points
(U+D800–U+DFFF) are explicitly forbidden as scalar values.

**Likely miss A (surrogate accepted):** A UTF-16 runtime or a validator
that does not exclude the surrogate range will accept `%ED%A0%80` (the
byte sequence that a naive UTF-16-to-UTF-8 transcoder produces for
surrogate U+D800).

**Likely miss B (private-use rejected):** An overly strict Unicode
printability guard rejects characters in the private-use area.

---

### IRI-004: percent-encoding case unification (FM-4)

**Spec:** RFC 3986 §2.1, §6.2.2.1; RDF Concepts §3.1

RFC 3986 §6.2.2.1 *recommends* uppercasing hex digits in percent sequences
for normalization.  RDF Concepts §3.1 mandates *string equality* for IRI
comparison — no normalization.  `%c3%a9` and `%C3%A9` are distinct RDF IRIs.

**Likely miss:** A parser that uppercases hex digits during tokenisation or
storage will merge the two subjects, producing one subject where two exist.

---

### IRI-005: authority-less absolute IRIs (FM-5)

**Spec:** RFC 3986 §3; RFC 2141 (URN); W3C RDF Concepts §3.1

`urn:`, `tag:`, `data:` and similar schemes are absolute without `://`.
Absoluteness means presence of a scheme, not presence of `://`.

**Likely miss:** An absoluteness check implemented as a regex matching
`^[a-zA-Z][a-zA-Z0-9+\-.]*://` (or equivalent) incorrectly rejects
conformant `urn:` IRIs.

---

### IRI-006: empty base path merge (FM-6)

**Spec:** RFC 3986 §5.2.3

If the base has an authority and an empty path, the merged path is
`"/" + reference_path`.  A base of `http://example` (no trailing slash)
has an empty path; resolving `foo` against it must produce
`http://example/foo`, not `http://examplefoo`.

**Likely miss:** String concatenation of `scheme + "://" + authority + path`
without the merge-paths slash insertion step produces a wrong authority.

---

### IRI-007: host case-folding (FM-7)

**Spec:** RFC 3986 §6.2.2.1; RDF Concepts §3.1; W3C public-rdf-comments 2011

`http://EXAMPLE.COM/s` and `http://example.com/s` are distinct RDF IRIs.

**Likely miss:** A parser that lowercases the host component at parse time
will unify them, creating a spurious merge and corrupting graph identity.

---

### IRI-008: NFC normalization (FM-8)

**Spec:** RFC 3987 §5.3.2.2; RDF Concepts §3.1; W3C/rdf-tests #200

`%C3%A9` (NFC precomposed é) and `cafe%CC%81` (NFD: e + combining accent)
are distinct RDF IRIs.  RFC 3987 recommends but does not mandate NFC.
RDF mandates string equality.

**Likely miss:** A parser built on a Unicode library that normalizes at
string creation (e.g., `NSString` on macOS, some ICU configurations) will
silently unify these, collapsing two graph subjects into one.

---

## Integration

Tests are run via:

```
cargo test -p rdf-diff adversary_iri
```

or via `xtask verify --adversary-iri` once `v1-ci-wiring` lands.

The Rust module `crates/testing/rdf-diff/tests/adversary_iri.rs` enumerates
these fixture files and registers one test case per file.  The `#[ignore]`
gates follow the same lifecycle as other `rdf-diff` tests: unignored once
both the main parser and a reference oracle exist.
