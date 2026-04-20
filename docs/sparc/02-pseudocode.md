# SPARC-02 — Pseudocode

> **Supersedes** the engine-scoped v1. Rewritten 2026-04-18 for the
> Zed-extension + LSP scope.

This phase is **filled just-in-time**, one refinement phase ahead of
implementation. The goal is not to pre-design every algorithm before
writing code; it is to make sure non-trivial ones are thought through at
a level above Rust *before* we commit to types and ownership.

## Rules of engagement

1. **Only the algorithms that deserve it.** A lexer rule does not need
   pseudocode. Error-tolerant Turtle recovery, SPARQL scope resolution,
   JSON-LD context validation, incremental re-parse, idempotent
   formatting — those do.
2. **Each sketch lives next to the phase it unblocks.**
3. **Pseudocode is not code.** Typed Rust-ish prose: function
   signatures, invariants, recovery rules, references to spec clauses.
4. **Every sketch cites the spec clause it implements.**
5. **Error-recovery strategy** is part of every parser sketch, not an
   afterthought.

## Planned sections (filled per phase)

- **§1 IRI parsing and relative resolution** (phase A): RFC 3987 ABNF
  implementation; base + relative resolution per RFC 3986 §5.
- **§2 Turtle error-tolerant parser** (phase A): `.`-boundary resync;
  prefix and base state machine; CST representation (`rowan`/`cstree`).
- **§3 Turtle formatter** (phase A/E): prefix reuse policy,
  indentation, line-wrapping of long lists, blank-node anonymisation
  rules. Idempotency proof sketch.
- **§4 RDF/XML event-based parser** (phase B): streaming `quick-xml`
  consumer; mapping RDF/XML patterns to facts; handling
  `rdf:parseType=Collection`, `Resource`, `Literal`.
- **§5 JSON-LD context validation** (phase B): `@context` value rules
  without full expand/compact; scoped contexts; type-scoped rules.
- **§6 SPARQL scope resolution** (phase C): prefix map; variable
  binding sites; subquery boundaries; aggregate scope.
- **§7 ShEx deterministic recognition** (phase D): triple expression
  determinism check (for highlighting / authoring aid, not for
  validation).
- **§8 Incremental parsing** (phase G): rope → changed-range → smallest
  re-parsed subtree (where cheap); worst-case full re-parse.
- **§9 LSP completion context detection** (phase F): token stream +
  cursor position → completion kind (keyword / prefix / local name /
  snippet / vocab term).
- **§10 Rename scope computation** (phase G): per-language rules for
  which occurrences of a symbol are the same symbol.

Sections are added **before** the corresponding crate or feature begins.

---

## §9 LSP completion context detection (phase F)

**Spec reference:** LSP 3.17 §3.15 (`textDocument/completion`); Turtle grammar §6 prefix declarations; SPARQL 1.1 §4.1.1 prefix declarations; ShEx 2.1 §3 shape expressions; Datalog (no published spec — project-internal grammar).

**Goal:** given `(text: &str, cursor_byte: usize, lang: Language)`, return a `CompletionKind` that tells `handle_completion` which candidate set to build.

```
enum CompletionKind {
    None,                        // no meaningful completion at this position
    PrefixIri,                   // cursor is inside a prefix-declaration IRI
    LocalName { prefix: String }, // cursor is after "known_prefix:"
    VocabTerm,                   // cursor is in predicate or object position
    Keyword,                     // cursor follows a recognized keyword opener
    JsonLdKeyword,               // cursor is inside a JSON string that starts "@"
    TriXElement,                 // cursor is on a TriX element name
    ShExProperty,                // cursor is inside a ShEx shape body
    DatalogRelation,             // cursor follows ":-" in a Datalog rule head
}
```

### Step 0 — position conversion

`lsp_types::Position` carries `(line, character)` in UTF-16 code units. Before the backward scan, convert to a byte offset:

```
fn position_to_byte_offset(text: &str, pos: lsp_types::Position) -> usize {
    // Iterate lines; for the target line, count UTF-16 code units up to
    // pos.character and return the corresponding byte index.
    // Phase F: linear scan is acceptable; Phase G replaces with rope lookup.
    // Invariant: if pos is past end-of-text, clamp to text.len().
}
```

### Step 1 — language dispatch

```
fn completion_context(text: &str, cursor_byte: usize, lang: Language) -> CompletionKind {
    match lang {
        Language::NTriples | Language::NQuads => CompletionKind::None,
        Language::Turtle | Language::TriG | Language::N3 => turtle_context(text, cursor_byte),
        Language::Sparql  => sparql_context(text, cursor_byte),
        Language::RdfXml  => rdfxml_context(text, cursor_byte),
        Language::JsonLd  => jsonld_context(text, cursor_byte),
        Language::TriX    => trix_context(text, cursor_byte),
        Language::ShEx    => shex_context(text, cursor_byte),
        Language::Datalog => datalog_context(text, cursor_byte),
    }
}
```

**NT / NQ:** these formats have no abbreviated syntax (no prefixes, no keywords beyond the implicit grammar). Return `None` unconditionally.

### Step 2 — Turtle / TriG / N3 backward scan (`turtle_context`)

Scan the byte slice `text[..cursor_byte]` from right to left, skipping whitespace and comments, to identify the nearest token context. Three cases in priority order:

**Case A — prefix declaration context**

```
// Pattern: the cursor is on the RHS of "@prefix label: " or "PREFIX label: "
// Trigger tokens: "@prefix" (Turtle/N3) or "PREFIX" (SPARQL-style in TriG/N3)
// Detection: scan left; if the first non-whitespace non-comment region is
//   a "<"-delimited IRI fragment or an empty position after ":", and
//   the token before the colon is a prefix label, and the token before that
//   is "@prefix" or "PREFIX" →
return CompletionKind::PrefixIri;
// Candidate set: well-known namespace IRIs (rdf, rdfs, owl, xsd, skos, …)
//   drawn from rdf-vocab registry.
```

**Case B — local name after known prefix**

```
// Pattern: text immediately left of cursor matches /[A-Za-z_][A-Za-z0-9_\-\.]*:/
//   where the part before ":" is a prefix declared earlier in the document.
// Detection: scan left to the nearest ":" that is not inside a "<…>" or string;
//   extract the prefix label; look it up in the document's prefix map
//   (built by a lightweight pre-scan of "@prefix"/"PREFIX" declarations).
// If the prefix is known →
return CompletionKind::LocalName { prefix };
// Candidate set: all terms in the vocabulary bound to that prefix namespace
//   (via rdf-vocab term table).
```

**Case C — predicate or object position**

```
// Pattern: cursor is after a subject token (IRI, prefixed name, or blank node)
//   and optional whitespace, in a position where a predicate is expected;
//   OR after a predicate and whitespace where an object is expected.
// Detection: a full position analysis is out of scope for Phase F.
//   Approximate heuristic: if neither Case A nor Case B matched and the
//   token left of the cursor is a full IRI or prefixed name (not a keyword) →
return CompletionKind::VocabTerm;
// Candidate set: all terms from the loaded vocabularies, filtered by
//   expected role (properties for predicate, classes/individuals for object).
//   Phase F: no role filtering; return all terms.
```

### Step 3 — SPARQL backward scan (`sparql_context`)

SPARQL uses the same three cases as Turtle for prefix-declaration and prefixed-name contexts. The trigger keywords are `PREFIX` (case-insensitive) and `BASE`. Additionally:

```
// SPARQL keywords (SELECT, WHERE, FILTER, OPTIONAL, …) are completed when
// the cursor follows a word-boundary with no preceding colon.
// Detection: scan left to the nearest whitespace or punctuation; if the
//   partial token matches a SPARQL keyword prefix →
return CompletionKind::Keyword;
```

### Step 4 — RDF/XML context (`rdfxml_context`)

```
// RDF/XML uses XML syntax. Detect the cursor position relative to XML tokens:
// - Inside a "<" → element name completion from RDF/XML production set
//   (rdf:Description, rdf:type, rdf:about, rdf:resource, rdf:datatype, …).
// - Inside an attribute value position → attribute-value completion.
// - Inside element content → property element name from loaded vocabularies.
// Phase F: return CompletionKind::VocabTerm for all positions; element-vs-
//   attribute discrimination is Phase G.
```

### Step 5 — JSON-LD context (`jsonld_context`)

```
// JSON-LD: inspect the byte context:
// - If cursor is inside a JSON string that begins with "@" →
return CompletionKind::JsonLdKeyword;
//   Candidate set: @context, @id, @type, @value, @graph, @language,
//                  @base, @vocab, @container, @reverse, @set, @list, @none.
// - If cursor is in a value position (after ":") →
return CompletionKind::VocabTerm;
//   Candidate set: vocabulary IRIs from rdf-vocab.
// - Otherwise → CompletionKind::None.
```

### Step 6 — TriX context (`trix_context`)

```
// TriX documents are XML. Completion is element-name based:
// - After "<" → suggest: triple, uri, plainLiteral, typedLiteral, TriX, graph.
// Phase F: return CompletionKind::TriXElement for all positions after "<".
// Otherwise: CompletionKind::None.
```

### Step 7 — ShEx context (`shex_context`)

```
// ShEx: detect the nearest enclosing "{" that is not closed by a matching "}".
// If found and the cursor is inside that shape body →
return CompletionKind::ShExProperty;
// Candidate set: property names (predicates) from the loaded vocabularies.
// The enclosing shape's rdf:type constraint (if present) narrows the
//   candidate set in Phase G; Phase F returns all properties.
// If no open "{" → CompletionKind::None.
```

### Step 8 — Datalog context (`datalog_context`)

```
// Datalog: scan left for ":-" (rule neck operator).
// If ":-" is found before the nearest "." (clause terminator) →
return CompletionKind::DatalogRelation;
// Candidate set: relation names declared as facts or rule heads in the
//   current document (a lightweight pre-scan collects these).
// If in a head position (before ":-") → also CompletionKind::DatalogRelation
//   (user may be starting a new head predicate).
// Otherwise → CompletionKind::None.
```

### Error recovery

If the backward scan encounters a lexically invalid region (e.g., unclosed string or IRI), stop the scan at that boundary and return `CompletionKind::None`. Do not propagate scan errors; a failed completion is better than a panic.

### Phase G extensions (out of scope here)

- Rope-based byte-offset conversion (replaces linear scan).
- Role-filtered vocab candidates (property vs. class vs. individual).
- Incremental prefix-map updates (avoid full re-scan on every keystroke).
- Trigger character support (`:`  and `<` trigger completion automatically).
