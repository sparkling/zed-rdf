# Phase B Adversary Failure-Mode Briefs

Agent: `pb-adv-redteam`
Namespace: `phase-b-adv`
Date: 2026-04-20

These briefs are the sole input for adversary fixture agents
`pb-adv-rdfxml`, `pb-adv-jsonld`, `pb-adv-trix`, and `pb-adv-n3`.
Each entry is self-contained. Do not read the implementing cohort's
code or memory namespace (`phase-b`) when consuming these briefs.

---

## Format: rdfxml

### FM-1: xml:lang inheritance through nested elements

- Input pattern: An outer `rdf:Description` or property element carries
  `xml:lang="fr"`. Inner property elements contain plain string content
  with no `xml:lang` attribute of their own.
- Expected: Each inner string literal inherits the language tag `"fr"`;
  the produced RDF literal has datatype `rdf:langString` and language
  tag `fr`.
- Common bug: Parser copies `xml:lang` only onto the element that
  declares it and treats child elements as plain `xsd:string` literals
  (no lang tag). Alternatively the parser only inherits one level deep
  and misses grandchild elements.
- Severity: high

### FM-2: xml:lang reset to empty string removes language

- Input pattern: Outer element has `xml:lang="en"`. An inner element
  explicitly sets `xml:lang=""` (empty string), which per XML 1.0
  §2.12 removes the language designation.
- Expected: The literal value on the inner element has no language tag
  (plain literal / `xsd:string`). The outer `"en"` language MUST NOT
  bleed through.
- Common bug: Parser treats `xml:lang=""` as absent and continues
  inheriting the enclosing `"en"`, producing an incorrect `"en"`-tagged
  literal.
- Severity: high

### FM-3: rdf:parseType="Literal" serialises XML content verbatim

- Input pattern: A property element carries `rdf:parseType="Literal"`
  and its content is a mixed XML fragment with nested elements,
  namespace declarations, and character data — e.g.
  `<ex:p rdf:parseType="Literal"><b xmlns="http://www.w3.org/1999/xhtml">bold</b> text</ex:p>`.
- Expected: The object is an `rdf:XMLLiteral`. Its lexical form is the
  canonical XML serialisation of the child nodes (as specified in the
  W3C C14N algorithm); the datatype IRI is
  `http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral`.
- Common bug: Parser extracts only the text content, discarding element
  markup, and emits a plain `xsd:string`. Or parser emits the raw
  unparsed XML source bytes rather than canonical form (namespace
  prefix ordering, attribute ordering wrong). Or parser emits
  `rdf:XMLLiteral` but does not canonicalise, so namespace declarations
  introduced by ancestors are missing from the canonical form.
- Severity: high

### FM-4: rdf:ID creates a reification triple AND an IRI subject

- Input pattern: A property element carries `rdf:ID="stmt1"`. The
  document base is `http://example.org/base`.
- Expected: Two things happen simultaneously:
  (a) The triple described by the property element is asserted normally.
  (b) A reification is created: four triples with subject
  `<http://example.org/base#stmt1>` asserting
  `rdf:type rdf:Statement`, `rdf:subject`, `rdf:predicate`,
  `rdf:object` for the triple from (a).
  The IRI `http://example.org/base#stmt1` also becomes the subject of
  those reification triples — it is NOT a blank node.
- Common bug: Parser creates only the reification (missing the original
  asserted triple), or creates only the asserted triple (skipping
  reification), or uses a blank node as the reification subject instead
  of the fragment IRI.
- Severity: high

### FM-5: Relative IRI resolution with a base URI that has a path component

- Input pattern: Document base (from `xml:base` or the retrieval URI)
  is `http://example.org/a/b/c`. Property attributes or `rdf:about`
  values contain relative IRIs such as `../d` or `foo`.
- Expected: Resolution follows RFC 3986 §5.2. `../d` resolves to
  `http://example.org/a/d`; `foo` resolves to
  `http://example.org/a/b/foo`.
- Common bug: Parser strips the path entirely and resolves against the
  host (`http://example.org/d`), or uses naive string concatenation
  without removing the last path segment, or does not handle `../`
  traversal above the base path correctly.
- Severity: high

### FM-6: Blank node ID scope — rdf:nodeID reuse across sibling properties

- Input pattern: Two sibling property elements on different subject
  nodes both carry `rdf:nodeID="b1"`. The blank node `b1` appears as
  both subject in one triple and object in another.
- Expected: Both occurrences of `rdf:nodeID="b1"` refer to the same
  blank node throughout the document. The two triples share one blank
  node identity, not two independently-minted blank nodes.
- Common bug: Parser creates a fresh blank node each time it sees
  `rdf:nodeID="b1"` on a new element, breaking the identity. This is
  distinct from anonymous blank nodes (which are always fresh); named
  blank nodes must be interned per document.
- Severity: medium

### FM-7: rdf:about="" refers to the document itself (empty relative IRI)

- Input pattern: A top-level `rdf:Description` carries
  `rdf:about=""`. The document retrieval URI is
  `http://example.org/doc.rdf`.
- Expected: The subject IRI is `http://example.org/doc.rdf` — the
  document base with no fragment, i.e. the document node itself.
- Common bug: Parser treats `rdf:about=""` as a blank node, or as
  `http://example.org/` (drops path), or raises a parse error claiming
  an empty IRI is invalid.
- Severity: medium

---

## Format: jsonld

### FM-1: @base in @context with relative @id values

- Input pattern: `@context` contains `"@base": "http://example.org/base/"`.
  Node objects in the document have `"@id": "item/1"` (a relative IRI).
- Expected: The node's IRI is resolved against `@base` to produce
  `http://example.org/base/item/1`. The document retrieval base (if
  different) is overridden by the explicit `@base` in context.
- Common bug: Parser ignores `@base` and resolves relative `@id` values
  against the document's own URL, producing a wrong IRI. Or parser
  resolves correctly on the first node but forgets the base for
  subsequent nodes after traversing an array.
- Severity: high

### FM-2: Term definition with @type: @id coerces string value to IRI

- Input pattern: Context defines `"knows": {"@id": "http://xmlns.com/foaf/0.1/knows", "@type": "@id"}`.
  Document node has `"knows": "http://example.org/bob"` (a plain JSON string).
- Expected: Because `@type` is `@id`, the string is coerced to an IRI;
  the produced triple has an IRI object `<http://example.org/bob>`, not
  a plain literal `"http://example.org/bob"`.
- Common bug: Parser ignores the `@type: @id` coercion rule and emits a
  plain `xsd:string` literal, or emits `rdf:langString`, because it
  processes the value as a JSON string without consulting the term
  definition.
- Severity: high

### FM-3: Scoped context (@context inside a node) limits term redefinitions

- Input pattern: A node type `"ex:Widget"` carries an embedded
  `"@context"` that redefines `"name"` to map to a different IRI than
  the outer context. Sibling nodes outside this node do not have the
  embedded context.
- Expected: Inside the `"ex:Widget"` node (and its descendants), `"name"`
  maps to the locally-scoped IRI. Outside that node, `"name"` reverts
  to the outer context definition. The scoped redefinition does NOT
  leak into the parent or sibling scope.
- Common bug: Parser propagates the embedded context's term redefinition
  globally after encountering the first scoped node, causing subsequent
  sibling nodes to use the wrong term mapping. Or parser ignores the
  embedded context entirely, using only the outer definition everywhere.
- Severity: high

### FM-4: @container: @list produces rdf:List structure

- Input pattern: Context defines
  `"items": {"@id": "http://example.org/items", "@container": "@list"}`.
  Document value is `"items": ["a", "b", "c"]`.
- Expected: The parser emits an `rdf:List` chain:
  a blank node with `rdf:first "a"`, `rdf:rest` pointing to another
  blank node with `rdf:first "b"`, `rdf:rest` pointing to another blank
  node with `rdf:first "c"`, `rdf:rest rdf:nil`. The `items` property
  links directly to the head blank node. Total: 7 triples for the list
  structure.
- Common bug: Parser emits one triple per array element with the same
  predicate, producing three separate `<subject> items "a"`, `"b"`,
  `"c"` triples rather than a proper `rdf:List` chain. Or parser emits
  the list structure but omits the final `rdf:rest rdf:nil` triple,
  leaving the list open-ended.
- Severity: high

### FM-5: Invalid @vocab using a relative IRI must be rejected

- Input pattern: Context contains `"@vocab": "relative"` (a relative
  IRI, not an absolute one) in JSON-LD 1.1 processing mode, OR
  `"@vocab": "_:b0"` (a blank node identifier as vocab).
- Expected: The processor MUST raise an `invalid vocab mapping` error
  and reject the document. A blank node or relative IRI is not a valid
  `@vocab` value.
- Common bug: Parser silently accepts the relative or blank-node vocab
  and generates malformed triple predicates (relative IRIs or
  `_:`-prefixed predicates), which are not valid RDF. Or parser
  resolves the relative vocab against the base without raising an error.
- Severity: high

### FM-6: @context array with null entry clears prior context

- Input pattern: `"@context": [{"term": "http://example.org/term"}, null, {"other": "http://example.org/other"}]`.
  The `null` entry in the array is a context reset instruction.
- Expected: After processing the `null`, all previously accumulated
  term definitions are cleared. Only `"other"` is in scope for the
  remainder; `"term"` produces no IRI mapping and must not appear as a
  predicate in the output triples.
- Common bug: Parser treats the `null` as a no-op and preserves `"term"`
  in scope, producing spurious triples. Or parser raises a parse error
  on `null` in a context array.
- Severity: medium

### FM-7: @graph keyword at top level and named graph semantics

- Input pattern: Document uses `"@graph": [...]` at the top level
  without a `"@id"` key (unnamed/default graph).
- Expected: All triples produced from the `@graph` array belong to the
  default graph. No named-graph quad wrapper is emitted.
  If `"@id"` is present alongside `"@graph"`, a named graph quad is
  emitted with that IRI.
- Common bug: Parser emits a named-graph quad using the document URL as
  the graph IRI even when no `"@id"` is provided (treating anonymous
  `@graph` as named). Or parser discards the `@graph` contents entirely
  and emits nothing.
- Severity: medium

---

## Format: trix

### FM-1: Wrong or missing TriX namespace URI is a fatal error

- Input pattern: A well-formed XML document with root element
  `<TriX xmlns="http://wrong.example.org/ns/">` (wrong namespace)
  or `<TriX>` with no namespace declaration at all. Triples inside
  are syntactically valid TriX content.
- Expected: The parser MUST reject the document. TriX requires the
  namespace `http://www.w3.org/2004/03/trix/trix-1/`. Any other
  namespace URI, including a slight variant, is not TriX.
- Common bug: Parser ignores namespace URIs entirely and parses by
  element local names only, accepting documents that claim to be TriX
  but use a different namespace — which may be a different version or
  an attacker-controlled namespace.
- Severity: high

### FM-2: triple element with wrong child count

- Input pattern: A `<triple>` element inside `<graph>` contains
  either two child elements (missing the object) or four child
  elements (extra element beyond subject, predicate, object).
- Expected: The parser MUST reject the malformed triple. A conforming
  TriX `<triple>` has exactly three child elements: one for the
  subject position, one for the predicate (must be a URI), and one
  for the object.
- Common bug: Parser ignores extra children beyond the third and
  silently emits a triple using only the first three, discarding the
  violation. Or parser crashes with an array-out-of-bounds panic
  rather than emitting a structured error.
- Severity: high

### FM-3: bnode element with IRI-like content remains a blank node

- Input pattern: A `<bnode>` element whose text content looks exactly
  like a valid absolute IRI — e.g.
  `<bnode>http://example.org/resource</bnode>`.
- Expected: The subject or object is a blank node whose label is the
  string `http://example.org/resource` (treated as an opaque blank
  node identifier). It MUST NOT be interpreted as an IRI resource, even
  though the string is IRI-shaped.
- Common bug: Parser detects the IRI-like string and silently promotes
  the blank node to an IRI resource, conflating two distinct RDF node
  types. This breaks blank node identity guarantees for any downstream
  consumer that merges graphs.
- Severity: high

### FM-4: Predicate position must be a URI, not a blank node

- Input pattern: A `<triple>` whose second child element (predicate
  slot) is `<bnode>b1</bnode>` instead of `<uri>...</uri>`.
- Expected: The parser MUST reject the triple. RDF does not allow blank
  nodes in the predicate position. TriX `<bnode>` in position 2 is a
  structural violation.
- Common bug: Parser applies the same node-creation logic to all three
  child elements, creating a blank-node predicate and emitting a
  non-RDF-compliant triple rather than raising an error.
- Severity: high

### FM-5: Multiple graph elements in one document, shared blank node scope

- Input pattern: A TriX document contains two `<graph>` blocks. Both
  blocks contain `<bnode>b1</bnode>` references. One block is a named
  graph; the other is anonymous.
- Expected: Blank node `b1` in the first graph and blank node `b1` in
  the second graph are DISTINCT nodes. TriX blank node scope is
  per-graph, not per-document.
- Common bug: Parser interns blank node labels globally across the whole
  document, causing the two graphs to share what should be separate
  blank nodes — breaking graph isolation.
- Severity: medium

---

## Format: n3

### FM-1: @keywords declaration with bare word shadowing rdf:type shorthand

- Input pattern:
  ```
  @keywords a, is, of.
  @prefix ex: <http://example.org/> .
  ex:Alice a ex:Person .
  ```
  The `@keywords` declaration names `a` as a keyword; `a` should
  continue to expand to `rdf:type` per the N3 spec keyword rules.
- Expected: The triple `<http://example.org/Alice> rdf:type <http://example.org/Person>`
  is produced. `@keywords` with `a` in its list explicitly opts `a`
  into keyword interpretation rather than treating it as a local term.
- Common bug: Parser treats `a` after `@keywords` as a user-defined
  bare word rather than preserving the `rdf:type` shorthand, producing
  a triple with the wrong predicate or raising a parse error claiming
  `a` is undefined.
- Severity: high

### FM-2: Quoted formula (graph literal) — variable scope isolation

- Input pattern:
  ```
  @prefix ex: <http://example.org/> .
  { ex:Alice ex:knows ex:Bob . } a ex:Formula .
  ```
  A statement is made about the quoted formula as an object.
- Expected: The parser produces a graph literal (formula) object
  containing one triple. The triple inside the formula does not appear
  in the default graph. The formula is the object of `a ex:Formula`,
  not of the triples inside it.
- Common bug: Parser leaks the triples inside the quoted formula into
  the default graph, treating `{ ... }` as merely grouping rather than
  as a distinct scope. Or parser crashes on the formula-as-subject/
  object construction entirely.
- Severity: high

### FM-3: Nested quoted formula — two levels of scope

- Input pattern:
  ```
  @prefix ex: <http://example.org/> .
  { { ex:A ex:B ex:C . } ex:D ex:E . } a ex:NestedFormula .
  ```
  An outer formula contains an inner formula as a subject.
- Expected: The inner formula containing `ex:A ex:B ex:C` is an opaque
  formula object. The outer formula contains one triple with the inner
  formula as subject, `ex:D` as predicate, and `ex:E` as object. Neither
  inner nor outer triple appears in the default graph.
- Common bug: Parser handles one level of nesting but confuses the scope
  at the second level, either collapsing both formulas into the same
  graph or emitting inner triples into the outer formula rather than
  treating the inner `{ }` as an atomic object. Stack-based parsers
  may pop one too many scopes.
- Severity: high

### FM-4: => implication operator produces rdf:implies triple

- Input pattern:
  ```
  @prefix log: <http://www.w3.org/2000/10/swap/log#> .
  @prefix ex: <http://example.org/> .
  { ex:A ex:B ex:C . } => { ex:D ex:E ex:F . } .
  ```
- Expected: The parser produces one triple whose subject is the formula
  `{ ex:A ex:B ex:C . }`, predicate is
  `<http://www.w3.org/2000/10/swap/log#implies>` (i.e. `log:implies`,
  the semantic equivalent of `=>`), and object is the formula
  `{ ex:D ex:E ex:F . }`. Both formulas are formula-typed objects;
  neither is flattened into the default graph.
- Common bug: Parser treats `=>` as syntax sugar for a graph merge and
  emits all contained triples into the default graph, discarding the
  implication structure. Or parser drops the `=>` statement entirely as
  unrecognised syntax, emitting no triple. Or parser crashes on `=>`.
- Severity: high

### FM-5: @prefix redefinition mid-document — earlier triples unaffected

- Input pattern:
  ```
  @prefix ex: <http://example.org/v1/> .
  ex:foo ex:bar ex:baz .
  @prefix ex: <http://example.org/v2/> .
  ex:foo ex:bar ex:baz .
  ```
- Expected: The first triple uses `v1` expansions; the second triple uses
  `v2` expansions. Redeclaring a prefix only affects triples that appear
  after the new declaration.
- Common bug: Parser uses the last `@prefix` binding for all triples
  (reads declarations lazily or in a second pass), so the first triple
  is incorrectly rewritten to use `v2`. Or parser raises a hard error on
  prefix redefinition, rejecting a valid N3 document.
- Severity: medium

### FM-6: Bare `is ... of` path construct inverts predicate direction

- Input pattern:
  ```
  @prefix ex: <http://example.org/> .
  ex:Bob is ex:knows of ex:Alice .
  ```
  This is equivalent to `ex:Alice ex:knows ex:Bob .`
- Expected: The parser produces the triple
  `<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob>` —
  subject and object are swapped compared to the surface order.
- Common bug: Parser emits the triple in surface order (Bob knows Alice)
  rather than the inverted (Alice knows Bob), treating `is ... of` as
  normal predicate syntax. Or parser raises a parse error because `is`
  and `of` are not recognised as N3 path keywords when `@keywords` has
  not been declared.
- Severity: medium
