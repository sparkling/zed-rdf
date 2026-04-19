# Adversary Brief: IRI Handling

Cohort: verification-v1-adv (cohort B)
Format: IRI parsing, validation, and resolution (cross-cutting all RDF serializations)
Spec references: RFC 3987 (IRI) https://www.rfc-editor.org/rfc/rfc3987
                 RFC 3986 (URI) https://www.rfc-editor.org/rfc/rfc3986
                 W3C RDF Concepts §3.1 https://www.w3.org/TR/rdf11-concepts/#section-IRIs
                 W3C RDF 1.1 Turtle §2.2 (base IRI resolution)
                 W3C SPARQL 1.1 §3 (IRI resolution)
Errata: RFC 3987 errata (IETF errata IDs 3937, 4198); RFC 3986 errata (IETF errata IDs 4005, 5428)
        W3C public-rdf-comments: "IRI normalization and comparison" (2011, revisited 2022)
        W3C SPARQL WG comment "relative IRI resolution in SPARQL BASE" (2012)

---

## Failure Mode 1: Scheme-relative and path-relative IRI resolution

Spec: RFC 3986 §5.2.2 defines a strict reference-resolution algorithm. The "remove-dots" step (§5.2.4) is mandatory for all resolved IRIs.

Sketch (as Turtle):
```turtle
@base <http://example/a/b/c> .
<../d> <p> <o> .        # resolves to http://example/a/d
<../../d> <p> <o> .     # resolves to http://example/d
<../../..> <p> <o> .    # resolves to http://example/
<../../../d> <p> <o> .  # resolves to http://example/d  (cannot go above root)
```

Divergence hypothesis: Implementations that do not apply the "cannot go above root" rule in remove-dots will produce `http://d` for the last case (treating the path root as the authority boundary) or raise an error. RFC 3986 §5.2.4 step 2C requires that a leading `/../` that cannot be resolved further is simply treated as `/`.

Rationale: This is a known source of divergence; the IETF errata 4005 clarifies the path-segment boundary during remove-dots. Many hand-rolled resolvers get this wrong.

---

## Failure Mode 2: Fragment and query component preservation during resolution

Spec: RFC 3986 §5.2.2: when a reference has a non-empty fragment, the fragment is appended to the resolved path. When the reference is just a fragment (`#foo`), the entire base IRI (including path) is kept and only the fragment changes.

Sketch (as Turtle):
```turtle
@base <http://example/doc#section1> .
<#section2> <p> <o> .   # must resolve to http://example/doc#section2
```

Divergence hypothesis: An implementation that strips the fragment from the base before resolution will correctly resolve relative IRIs, BUT an implementation that strips nothing and re-appends the base fragment when the reference has no fragment will double-apply fragments. Pure-fragment references (`#foo`) must replace the fragment of the base, not concatenate.

Rationale: RFC 3986 §5.2.2 step for R.fragment: "T.fragment = R.fragment" (unconditional replacement). The base fragment is never preserved when resolving any reference that has any component set.

---

## Failure Mode 3: IRI character validity — bidirectional and private-use code points

Spec: RFC 3987 §2.2 restricts `ipath-noscheme`, `iuserinfo`, etc., to specific Unicode ranges. Private-use characters (U+E000–U+F8FF, U+F0000–U+FFFFD, U+100000–U+10FFFD) ARE allowed in IRIs. Surrogate code points (U+D800–U+DFFF) are NOT allowed and must never appear as characters (only as percent-encoded pairs in legacy contexts).

Sketch:
```
<http://example/\uD800path>   # surrogate: invalid
<http://example/\uE001path>   # private use: valid IRI character
```

Divergence hypothesis: An implementation that uses a UTF-16 string internally and validates by character range may admit surrogates (since they appear as valid `char` values in some languages/runtimes). An implementation that is overly restrictive may reject private-use code points that are explicitly permitted by RFC 3987 §2.2.

Errata reference: RFC 3987 errata 3937 clarifies that surrogates must not appear as Unicode scalar values in IRI character sequences, only in percent-encoding.

---

## Failure Mode 4: Percent-encoding normalization and case sensitivity

Spec: RFC 3987 §3.1 (converting IRI to URI) and RFC 3986 §2.1: percent-encoding sequences are case-insensitive in the sense that `%2F` and `%2f` encode the same octet, but RDF comparison is character-for-character — `%2F` and `%2f` are DIFFERENT IRI strings unless normalized.

Sketch:
```
<http://example/caf%c3%a9>   # lowercase percent-encoding
<http://example/caf%C3%A9>   # uppercase percent-encoding
```

Divergence hypothesis: These are two distinct IRIs in RDF (no normalization is mandated by the RDF spec). An implementation that normalizes percent-encoding to uppercase (as recommended by RFC 3986 §6.2.2.1) before storing/comparing will incorrectly unify them. An implementation that case-folds hex digits in percent sequences during Turtle parsing but not during SPARQL comparison will produce inconsistent equality results.

Rationale: RDF Concepts §3.1 states IRI equality is a simple string comparison; RFC 3986's normalization recommendations are not mandated by RDF.

---

## Failure Mode 5: IRI absoluteness check — opaque IRIs and `urn:` scheme

Spec: NT/Turtle require absolute IRIs (with scheme). `urn:example:foo` is absolute (has scheme `urn`). `urn:example:foo#bar` is absolute with fragment.

Sketch (NT):
```
<urn:example:foo> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:example:Bar> .
<urn:isbn:0451450523> <http://schema.org/name> "The Name" .
```

Divergence hypothesis: An implementation that validates "absoluteness" by checking for `://` (authority separator) rather than by checking for a scheme followed by `:` will reject valid `urn:` IRIs. The RFC 3986 definition of absolute URI is scheme `:` hier-part; no authority is required.

Rationale: `urn:`, `data:`, `tag:`, and other authority-less schemes are valid absolute IRIs. A regex like `^https?://` or a check for `://` is a common but wrong absoluteness test.

---

## Failure Mode 6: Base IRI resolution with empty path

Spec: RFC 3986 §5.2.2: if the base IRI has an empty path and the reference path is also empty (pure query or fragment reference), specific rules apply about merging.

Sketch (Turtle):
```turtle
@base <http://example> .   # no trailing slash — path is empty
<foo> <p> <o> .            # must resolve to http://example/foo? or http://examplefoo?
```

Divergence hypothesis: RFC 3986 §5.2.3 "Merge Paths" rule says: if the base has an authority and an empty path, the merged path is `"/" + reference_path`. So `<foo>` against `<http://example>` (no trailing slash) must give `<http://example/foo>`, NOT `<http://examplefoo>`. An implementation that does string concatenation without the "add slash" step will produce wrong output.

Rationale: This edge case is frequently wrong in simple resolvers; the Merge Paths algorithm in RFC 3986 is not obvious.

---

## Failure Mode 7: IRI host normalization — case folding

Spec: RFC 3987 §5.3.2.1 / RFC 3986 §6.2.2.1: the host component of an http/https IRI SHOULD be lowercased for normalization. However, RDF does NOT mandate normalization; two IRIs differing only in host case are distinct IRIs.

Sketch:
```
<http://EXAMPLE.COM/s>   vs.   <http://example.com/s>
```

Divergence hypothesis: An implementation that lowercases the host during IRI parsing will unify these two IRIs, which RDF treats as distinct. An implementation that does NOT lowercase will correctly preserve them but may fail interoperability tests that assume normalization. The implementing hive may apply RFC 3986 §6.2 normalization at parse time rather than treating normalization as an application-level concern.

Errata reference: W3C RDF Concepts, no formal errata, but public-rdf-comments thread "IRI normalization and comparison" (2011) clarifies that the RDF spec intentionally does not mandate normalization.

---

## Failure Mode 8: Unicode NFC normalization of IRI characters

Spec: RFC 3987 §5.3.2.2 recommends (not mandates) NFC normalization of IRI characters before comparison. RDF Concepts §3.1 does not mandate it.

Sketch:
```
<http://example/caf\u00E9>   # precomposed e-acute (NFC)
<http://example/cafe\u0301>  # decomposed e + combining acute (NFD)
```

Divergence hypothesis: These are two distinct IRIs in RDF. An implementation that applies NFC normalization to IRI characters at parse time will unify them. This is a subtle trap when reusing a URI library that performs normalization "helpfully."

Rationale: The divergence between RFC 3987's recommendation and RDF's strict string-equality IRI comparison is a known implementer trap. No formal errata; see WG discussion in w3c/rdf-tests issue #200.

---

## Summary of Divergence Hypotheses

| # | Area | Likely miss |
|---|------|-------------|
| 1 | Remove-dots: above-root path | Wrong result or error for `../../..` |
| 2 | Fragment resolution: pure `#foo` | Double-fragment or base-fragment leak |
| 3 | Surrogate / private-use code points | Admit surrogates or reject private-use |
| 4 | Percent-encoding case | Unify `%2F` and `%2f` as equal |
| 5 | Absoluteness: `urn:` / no authority | Reject valid authority-less schemes |
| 6 | Empty base path + merge | Concatenate without slash insertion |
| 7 | Host case-folding | Unify differing-case hosts |
| 8 | NFC normalization | Unify NFC/NFD as equal |
