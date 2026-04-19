# sparql-syntax-shadow

Independent shadow SPARQL 1.1 grammar-only parser for the
verification-v1 sweep (ADR-0020). Implements `rdf_diff::Parser` and emits
a canonical AST-as-Facts encoding suitable for cross-implementation
diffing.

**This crate is not published** and must not appear in runtime
`[dependencies]` of any workspace member. It is gated behind
`--features shadow`.

## Purpose

ADR-0019 §3 requires a disjoint second implementation of `sparql-syntax`
produced by a different agent cohort from a different prompt lineage. This
crate is that implementation. It is compared against the `oxsparql-syntax`
oracle and the main `sparql-syntax` crate by the diff harness in
`crates/testing/rdf-diff`.

## Scope

Grammar-only: no query execution, no dataset evaluation, no algebraic
normalisation.

Covered SPARQL 1.1 constructs:

- SELECT / CONSTRUCT / ASK / DESCRIBE queries
- SPARQL 1.1 Update: INSERT DATA, DELETE DATA, DELETE WHERE,
  DELETE/INSERT...WHERE, LOAD, CLEAR, DROP, CREATE, ADD, MOVE, COPY
- Federated query: SERVICE, SERVICE SILENT
- Property paths: sequence (`/`), alternative (`|`), inverse (`^`),
  negation (`!`), zero-or-one (`?`), zero-or-more (`*`), one-or-more (`+`)
- BIND scoping and expression aliases in SELECT
- Subquery projections `{ SELECT ... }`
- Inline VALUES, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET
- Literal lexical-form preservation (no numeric normalisation, no escape
  decoding) — raw lexical form recorded for grammar-level comparison
- Language tags (lowercased per RFC 5646)
- Datatype IRIs
- Blank-node labels and anonymous blank nodes

## AST-as-Facts Encoding

The parser assigns each AST node a synthetic subject IRI
`_shadow:N` (N = 0-based counter; the document root is always
`_shadow:0`). Structural relationships are expressed as RDF-style triples
stored in `rdf_diff::Facts`.

### Predicate namespace

All predicates use the prefix `sparql:` expanding to
`https://shadow.sparql/ast#`.

| Predicate | Object | Notes |
|-----------|--------|-------|
| `rdf:type` | `sparql:Document`, `sparql:Query`, ... | RDF type of node |
| `sparql:query` | node IRI | Query sub-tree |
| `sparql:queryForm` | `sparql:Select`, `sparql:Construct`, `sparql:Ask`, `sparql:Describe` | Query form |
| `sparql:modifier` | `"DISTINCT"` / `"REDUCED"` | SELECT modifier |
| `sparql:projection` | `sparql:Star` or `sparql:VarList` | Projection kind |
| `sparql:projectionItem` | node | One projected item (index-tagged) |
| `sparql:var` | `"?name"` | Variable |
| `sparql:alias` | `"?name"` | AS alias in SELECT |
| `sparql:expr` | node | Expression sub-tree |
| `sparql:dataset` | node | FROM / FROM NAMED clause |
| `sparql:named` | `"true"` / `"false"` | FROM NAMED flag |
| `sparql:graphIri` | IRI string | Named-graph IRI |
| `sparql:where` | node | WHERE pattern |
| `sparql:element` | node | Element within a group (index-tagged) |
| `sparql:triple` | node | Triple pattern |
| `sparql:subject` | term | Triple subject |
| `sparql:predicate` | term | Triple predicate |
| `sparql:predicatePath` | node | Property path predicate |
| `sparql:object` | term | Triple object |
| `sparql:pathKind` | `"iri"`, `"sequence"`, `"alternative"`, `"inverse"`, `"negated"`, `"zeroOrOne"`, `"zeroOrMore"`, `"oneOrMore"`, `"group"` | Path kind |
| `sparql:pathChild` | node | Child of unary path |
| `sparql:pathLeft` | node | Left of binary path |
| `sparql:pathRight` | node | Right of binary path |
| `sparql:filter` | node | FILTER expression |
| `sparql:optional` | node | OPTIONAL sub-pattern |
| `sparql:minus` | node | MINUS sub-pattern |
| `sparql:left` / `sparql:right` | nodes | UNION children |
| `sparql:graph` | term | GRAPH name (IRI or variable) |
| `sparql:graphPattern` | node | GRAPH inner pattern |
| `sparql:service` | node | SERVICE element |
| `sparql:silent` | `"true"` / `"false"` | SERVICE SILENT flag |
| `sparql:endpoint` | term | SERVICE endpoint |
| `sparql:servicePattern` | node | SERVICE inner pattern |
| `sparql:bind` | node | BIND expression |
| `sparql:bindVar` | `"?name"` | BIND target variable |
| `sparql:values` | node | VALUES clause |
| `sparql:valuesVar` | node | Variable in VALUES header |
| `sparql:valuesRow` | node | Row in VALUES |
| `sparql:valuesCell` | node | Cell in row |
| `sparql:undef` | `"true"` | UNDEF cell |
| `sparql:value` | term | Bound value in cell |
| `sparql:subquery` | node | Sub-SELECT node |
| `sparql:groupBy` | node | GROUP BY anchor |
| `sparql:having` | node | HAVING anchor |
| `sparql:orderBy` | node | ORDER BY anchor |
| `sparql:condition` | node | Condition within modifier |
| `sparql:direction` | `"ASC"` / `"DESC"` | ORDER BY direction |
| `sparql:limit` | `"N"^^xsd:integer` | LIMIT value |
| `sparql:offset` | `"N"^^xsd:integer` | OFFSET value |
| `sparql:updateOp` | node | Update operation |
| `sparql:opKind` | `"InsertData"`, `"DeleteData"`, `"DeleteWhere"`, `"Modify"`, `"Load"`, `"Clear"`, `"Drop"`, `"Create"`, `"Add"`, `"Move"`, `"Copy"` | Update kind |
| `sparql:source` | IRI | LOAD source |
| `sparql:into` | IRI | LOAD INTO graph |
| `sparql:graphRef` | IRI / `"DEFAULT"` / `"NAMED"` / `"ALL"` | CLEAR/DROP target |
| `sparql:deleteTriple` / `sparql:insertTriple` | node | Template triple |
| `sparql:using` | node | USING clause |
| `sparql:with` | IRI | WITH graph |
| `sparql:base` | IRI | BASE declaration |
| `sparql:prefix` | node | PREFIX declaration |
| `sparql:prefixLabel` | `"label"` | Prefix label |
| `sparql:prefixIri` | IRI | Prefix expansion IRI |
| `sparql:exprKind` | `"var"`, `"iri"`, `"literal"`, `"bool"`, `"binop"`, `"not"`, `"neg"`, `"builtin"`, `"aggregate"`, `"exists"`, `"notExists"`, `"in"`, `"notIn"` | Expression kind |
| `sparql:op` | `"="`, `"!="`, `"<"`, ... | Binary operator |
| `sparql:lhs` / `sparql:rhs` | node | Binary operand |
| `sparql:literal` | literal string | Literal value (quoted form + datatype/lang) |
| `sparql:iri` | IRI string | IRI constant |
| `sparql:funcName` | `"STRLEN"`, ... | Built-in / aggregate name |
| `sparql:arg` | node | Function argument (index-tagged) |
| `sparql:distinct` | `"true"` / `"false"` | Aggregate DISTINCT flag |
| `sparql:star` | `"true"` / `"false"` | Aggregate `*` flag |
| `sparql:separator` | `"..."` | GROUP_CONCAT separator |
| `sparql:inMember` / `sparql:notInMember` | node | IN / NOT IN list element |
| `sparql:pattern` | node | EXISTS / NOT EXISTS pattern |
| `sparql:index` | `"N"` | 0-based position tag for ordered lists |

### Canonical form rules

1. All IRIs stored in angle-bracket form: `<https://...>`. Prefixed names
   stored as-is (e.g. `ex:Foo`) — prefix resolution is semantic, not
   grammatical.
2. The `a` shorthand is expanded to
   `<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>`.
3. Literal lexical forms include surrounding quotes exactly as lexed.
   Language tags are lower-cased (per RFC 5646). Plain literals annotated
   with `^^<...xsd#string>`.
4. Blank-node labels stored as `_:label`.
5. Anonymous blank nodes stored as `_:anon`.
6. Variable names stored as `?name`.
7. Numeric literals carry their XSD datatype:
   `"42"^^<...xsd#integer>`, `"3.14"^^<...xsd#decimal>`,
   `"1e5"^^<...xsd#double>`.
8. Graph context is threaded through to triples inside `GRAPH` blocks.

## Usage

```rust
use sparql_syntax_shadow::SparqlShadowParser;
use rdf_diff::Parser as _;

let parser = SparqlShadowParser;
let outcome = parser.parse(b"SELECT * WHERE { ?s ?p ?o }").unwrap();
println!("{} facts emitted", outcome.facts.set.len());
```

## Running tests

```bash
cargo test -p sparql-syntax-shadow --features shadow
cargo clippy -p sparql-syntax-shadow --features shadow -- -D warnings
```
