# TriG — W3C Manifest Divergences

Owner: `pa-w3c-vendor`.

## Current state (post-triage, 2026-04-19 — allow-list closed)

- Corpus root: `external/tests/trig/`
- Files discovered: 470
- Harness mode: **live** (`rdf-turtle::TriGParser` driving `rdf-diff`).
- `xtask verify` outcome: 712 entries, **712 pass / 0 divergences / 0 allow-listed** → strict pass-rate **100 %**.
- Workspace: `cargo test --workspace --all-features --no-fail-fast` green; `cargo clippy --workspace --all-features -- -D warnings` clean.

## Class taxonomy

The original 86-divergence set resolved to five underlying causes. Four
were parser bugs fixed in this pass; the remaining base-undeclared
cluster is allow-listed pending a harness change.

### C1. Last triple in a graph block may omit `.` — **FIXED**

TriG §2.5 lets the last triple in a `{ … }` block drop its trailing
`.`:

```
GRAPH :g { :s :p :o }     # no dot before }
```

Our grammar required a `.` after every triple unconditionally. The
block-closing `}` was never reached, so the parser reported
`expected '.' after triple statement`.

Fix: new helper `parse_triple_stmt_in_block` accepts either `.` or
`}` as the terminator and returns a `bool` signalling the block
close. `parse_graph_body` uses it in place of the outer-level
`parse_triple_stmt`. See `grammar.rs`.

Member test-ids (TriG, 14):

- `trig-kw-graph-01`, `trig-kw-graph-04`, `trig-kw-graph-05`
- plus the multi-fixture variants covered by the block-body cases in
  the blankNodePropertyList cluster.

### C2. Blank-node and anonymous `[]` graph names — **FIXED**

TriG §2.6 permits `GRAPH _:label { … }`, `_:label { … }`,
`GRAPH [] { … }`, and `[] { … }` as graph headers. Our
`trig_graph_block` accepted only `IRIREF` / `pname` / `GRAPH <iri>`.

Fixes:

1. `looks_like_graph_block` extended to recognise a `BNodeLabel` or
   empty `[]` prefix followed by `{`. For `[`, the lookahead verifies
   the second token is `]` and the third is `{` — this avoids
   mis-classifying `[ :p :o ] .` (a triples subject) as a graph
   header.
2. `trig_graph_block` itself gained arms for `BNodeLabel` (minted via
   `bnode_for_label`) and `LBracket` (minted via `fresh_bnode`).
3. The `GRAPH` branch grew a second arm for `GRAPH []`.
4. `graph_name_from_tok` now recognises `BNodeLabel`.

Member test-ids (TriG):

- `trig-kw-graph-06`, `trig-kw-graph-07`
- `alternating_bnode_graphs`, `anonymous_blank_node_graph`

### C3. Bare `blankNodePropertyList` subject — **FIXED** (mirror of the TTL B class)

Inherits the TTL B fix: `[ :p :o ] .` and `[ :p :o ] { … }` now parse
as expected. See `../ttl/w3c-divergences.md` → class B.

Member test-ids (TriG): `trig-syntax-bnode-08`, `trig-syntax-bnode-09`,
`trig-syntax-bnode-10`, `sole_blankNodePropertyList`,
`nested_blankNodePropertyLists`,
`blankNodePropertyList_containing_collection`,
`blankNodePropertyList_as_object_containing_objectList`,
`blankNodePropertyList_as_object_containing_objectList_of_two_objects`.

### C4. Collection subject without predicate-object-list — **FIXED (negative side)**

W3C `trig-syntax-bad-list-01` … `04` exercise the negative path:

```
( 1 2 3 ) .        # collection alone, no predicate
{ ( 1 2 3 ) }
{ ( ) }
```

Turtle §2.5's optional-predicateObjectList rule applies **only** to
the `blankNodePropertyList` branch — **not** to collections. An
initial pass over-generalised the fix, accidentally accepting bare
collections. The corrective change restricts the optional path to
`SubjectKind::BlankNodePropertyList`, so bare collections are once
again rejected.

Member test-ids: `trig-syntax-bad-list-01` … `trig-syntax-bad-list-04`.

### C5. SPARQL-keyword / pname lexical ambiguity — **FIXED** (mirror of TTL class F)

Inherits the TTL F fix. Member test-id: `trig-subm-02`.

### G. TTL-BASE-001 relative IRIs without harness base — **CLOSED**

Same story as the TTL G class. 17 unique TriG tests (34 entries due to
double manifest pass) needed `mf:assumedTestBase` threading in the
harness. Closed on 2026-04-19 by the same two-part change:
`TriGParser::parse_with_base` added as an inherent method, and
`xtask/verify/src/manifest.rs` wires the manifest-level
`mf:assumedTestBase` plus the action filename through to
`parse_for_language`. See `../ttl/w3c-divergences.md` → class G for
the full rationale.

Member test-ids (TriG, 17):

- `trig-subm-01`, `trig-subm-27`
- `trig-syntax-datatypes-01`, `trig-syntax-datatypes-02`
- `trig-syntax-kw-01`, `trig-syntax-kw-02`
- `trig-syntax-number-01` … `trig-syntax-number-11`

### H. Tolerant trailing `.` after SPARQL PREFIX/BASE — **CLOSED**

Mirror of the TTL H class — closed on 2026-04-19 by the same grammar
tightening. `rdf-turtle`'s `directive_prefix` / `directive_base` now
emit `DirectiveTerminator` for a stray `.` after `PREFIX <iri>` /
`BASE <iri>`. See `../ttl/w3c-divergences.md` → class H.

Member test-ids: `trig-syntax-bad-base-03`, `trig-syntax-bad-prefix-05`.

## Tally

| class | count | status                                  |
| ----- | ----- | --------------------------------------- |
| C1    | 14    | fixed                                   |
| C2    | 4     | fixed                                   |
| C3    | 16    | fixed                                   |
| C4    | 8     | fixed (negative path restored)          |
| C5    | 2     | fixed                                   |
| G     | 34    | closed (harness wired)                  |
| H     | 4     | closed (fixture + strict grammar)       |

## Deferred

None. The verification-v1 Phase-A exit gate reports zero divergences
and zero allow-listed entries for TriG.
