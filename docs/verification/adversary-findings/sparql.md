# Adversary Brief: SPARQL 1.1

Cohort: verification-v1-adv (cohort B)
Format: SPARQL 1.1 (SELECT, CONSTRUCT, ASK, DESCRIBE; Graph patterns; Update)
Spec references: W3C SPARQL 1.1 Query https://www.w3.org/TR/sparql11-query/
                 W3C SPARQL 1.1 Update https://www.w3.org/TR/sparql11-update/
                 W3C SPARQL 1.1 Federated https://www.w3.org/TR/sparql11-federated-query/
                 W3C SPARQL 1.1 Service Description https://www.w3.org/TR/sparql11-service-description/
                 W3C RDF 1.1 Concepts §3.1 (IRI equality)
Errata: SPARQL 1.1 errata https://www.w3.org/2013/sparql-errata
        Errata IDs: SE-1 (blank nodes in CONSTRUCT), SE-2 (OPTIONAL scoping),
                    SE-3 (aggregate scope), SE-4 (RAND() determinism)
        public-sparql-dev mailing list: "Filter scope in OPTIONAL" (2011);
        "GROUP BY with expressions" (2013); "MINUS semantics" (2014)

---

## Failure Mode 1: OPTIONAL scoping and variable binding leak

Spec: SPARQL 1.1 §12.3.1: a variable bound in the right-hand side of OPTIONAL is in scope only if the OPTIONAL's left side matches. If the left side fails, the variable is unbound for that solution.

Sketch:
```sparql
SELECT * WHERE {
  ?s <p> ?o .
  OPTIONAL { ?s <q> ?x . FILTER(?x > 5) }
  FILTER(?x > 3)   # FILTER here sees ?x from OPTIONAL
}
```

Divergence hypothesis: The outer FILTER references `?x`. Under the semantics of SPARQL §12.3.1 combined with §17.3 (FILTER evaluation), when `?x` is unbound (OPTIONAL did not match), the FILTER expression `?x > 3` evaluates to an error (not `false`), and error in FILTER causes the solution to be discarded. An implementation that treats "unbound variable in FILTER" as `false` will over-discard solutions; one that treats it as `true` will over-include them.

Errata reference: SPARQL 1.1 errata SE-2; public-sparql-dev thread "Filter scope in OPTIONAL" (2011).

---

## Failure Mode 2: MINUS semantics vs NOT EXISTS

Spec: SPARQL 1.1 §12.6: MINUS removes solutions from the left that "share at least one variable" with a right-side solution. If the patterns share NO variables, MINUS has no effect (it does not remove anything).

Sketch:
```sparql
SELECT * WHERE {
  ?s <p> ?o .
  MINUS { ?x <q> ?y . }   # no shared variable with ?s, ?o
}
```

Divergence hypothesis: An implementation that treats MINUS as NOT EXISTS (checking whether the pattern has any solution in the dataset, regardless of shared variables) will incorrectly remove solutions when the MINUS pattern matches something but shares no variables. The correct behavior is that MINUS has no effect here.

Rationale: SPARQL §8.3 Table-1 example and §12.6 formal definition are both needed to understand the shared-variable requirement. Many implementations confuse MINUS with `FILTER NOT EXISTS`.

---

## Failure Mode 3: Blank nodes in CONSTRUCT templates

Spec: SPARQL 1.1 §10.3 (errata SE-1): blank nodes in the CONSTRUCT template are "locally scoped" — each solution mapping produces a fresh set of blank nodes. A blank node label in the CONSTRUCT template is not shared across different result rows.

Sketch:
```sparql
CONSTRUCT { ?s <p> _:b . _:b <q> ?o . }
WHERE { ?s <r> ?o . }
```

With two result rows, the two `_:b` nodes in row 1 are the same blank node (within that row), but distinct from the `_:b` nodes in row 2.

Divergence hypothesis: An implementation that treats the CONSTRUCT template blank node label as a single global blank node (one node shared across all result rows) will produce an incorrect merged graph where all result rows' generated triples share one blank node. Correct behavior requires one fresh blank node per template per solution row.

Errata reference: SPARQL 1.1 errata SE-1 ("blank node in CONSTRUCT is local to each solution").

---

## Failure Mode 4: Aggregate function scope and GROUP BY expressions

Spec: SPARQL 1.1 §11.4: in a SELECT with GROUP BY, only grouped variables and aggregated expressions may appear in the SELECT clause. Aggregates inside expressions inside GROUP BY are forbidden.

Sketch:
```sparql
SELECT (COUNT(?o) AS ?cnt) (SUM(?v) AS ?total)
WHERE { ?s <p> ?o ; <q> ?v . }
GROUP BY ?s
HAVING (?cnt > 2)
```

Divergence hypothesis: `?cnt` is not a "grouped variable" — it is an alias for an aggregate defined in the SELECT clause. HAVING references it. SPARQL §11.4.2 says HAVING clauses may reference aggregate aliases defined in SELECT. An implementation that evaluates HAVING before projecting SELECT aliases will fail to resolve `?cnt` in HAVING, either erroring or treating it as unbound.

Errata reference: SPARQL 1.1 errata SE-3; public-sparql-dev thread "GROUP BY with expressions" (2013).

---

## Failure Mode 5: IRI relative resolution in SPARQL BASE

Spec: SPARQL 1.1 §3.1: the BASE declaration in SPARQL queries uses the same RFC 3986 resolution algorithm as Turtle. Multiple BASE declarations in a single query are allowed; each takes effect from its point in the query text onward.

Sketch:
```sparql
BASE <http://example/a/>
SELECT * WHERE {
  <foo> <p> ?o .   # must be http://example/a/foo
  BASE <http://example/b/>
  <bar> ?q ?r .    # SPARQL: BASE mid-query — is this valid?
}
```

Divergence hypothesis: SPARQL 1.1 §3.1 syntax only allows BASE as a prologue declaration (before SELECT/CONSTRUCT/etc.), NOT inside a WHERE clause. An implementation that parses `BASE` inside a group graph pattern may accept this silently or apply the second BASE, producing wrong IRIs. An implementation that correctly rejects it as a parse error is right. The divergence is whether mid-query BASE is silently accepted or correctly rejected.

Rationale: SPARQL grammar production `Prologue ::= (BaseDecl | PrefixDecl)*` — BASE is only in Prologue, before the query body. Reusing the Turtle parser for IRI handling may accidentally permit BASE anywhere.

---

## Failure Mode 6: GRAPH pattern with variable matching default graph

Spec: SPARQL 1.1 §13.3: the `GRAPH ?g { ... }` pattern matches named graphs only, not the default graph. The default graph is not accessible via `GRAPH`.

Sketch:
```sparql
SELECT * WHERE { GRAPH ?g { ?s ?p ?o . } }
```

Divergence hypothesis: If the dataset has only a default graph (no named graphs), this query must return no results. An implementation that includes the default graph in the set of graphs matched by `GRAPH ?g` will incorrectly return solutions. If the dataset has named graphs, `?g` must bind to their IRIs, never to a blank node or to a special "default graph" IRI unless the default graph has been explicitly named.

Rationale: SPARQL §13.3 distinguishes "default graph" from "named graphs"; the formal semantics in §18.5 formalizes this distinction. Implementations that represent the default graph as a named entity may expose it via `GRAPH ?g`.

---

## Failure Mode 7: FILTER NOT EXISTS vs OPTIONAL/FILTER(!BOUND)

Spec: SPARQL 1.1 §12.5: `FILTER NOT EXISTS { P }` checks whether pattern P has at least one solution in the current solution's context. This is NOT equivalent to `OPTIONAL { P } FILTER (!BOUND(?x))` in all cases when P has solutions with no new variables.

Sketch:
```sparql
# These are NOT equivalent when P shares all variables with the outer pattern
SELECT * WHERE {
  ?s ?p ?o .
  FILTER NOT EXISTS { ?s ?p ?o . }   # should return nothing
}

SELECT * WHERE {
  ?s ?p ?o .
  OPTIONAL { ?s ?p ?o . }
  FILTER (!BOUND(?o))                 # different semantics
}
```

Divergence hypothesis: An implementation that rewrites `FILTER NOT EXISTS` as `OPTIONAL/FILTER(!BOUND)` during optimization will produce different results for patterns where the inner and outer patterns overlap on all variables. The `NOT EXISTS` pattern should find no solutions for the first query; the rewritten version may behave differently.

Rationale: SPARQL §12.5 formal semantics defines `FILTER NOT EXISTS` in terms of `NOTEXISTS` evaluation, not in terms of LEFT JOIN. The equivalence to `OPTIONAL/FILTER(!BOUND)` is only approximate.

---

## Failure Mode 8: UPDATE — INSERT DATA with blank nodes

Spec: SPARQL 1.1 Update §3.1.1: `INSERT DATA` must NOT contain variables or blank nodes in the WHERE clause (there is no WHERE clause). Blank nodes in `INSERT DATA` are treated as newly minted blank nodes in the target graph, local to that update operation.

Sketch:
```sparql
INSERT DATA {
  GRAPH <http://example/g> {
    _:b <p> <o> .
    _:b <q> <o2> .
  }
}
```

Divergence hypothesis: Both `_:b` references within a single `INSERT DATA` operation refer to the same blank node. Across multiple `INSERT DATA` operations, the same label `_:b` refers to different blank nodes. An implementation that generates a fresh blank node for every occurrence of `_:b` in a single `INSERT DATA` will split what should be one node into two. An implementation that shares blank node identity across multiple `INSERT DATA` operations will incorrectly merge distinct nodes.

Rationale: SPARQL Update §3.1.1: "Blank node labels are scoped to the request." One `INSERT DATA` call = one scope.

---

## Failure Mode 9: Property paths — negated property set and inverse

Spec: SPARQL 1.1 §9.3: a negated property set `!(p1|p2)` matches any predicate that is NOT p1 and NOT p2. Inverse paths `^p` match triples in the reverse direction. Combined: `^!(p1|p2)` matches any inverse predicate not in the set.

Sketch:
```sparql
SELECT * WHERE {
  ?s !(rdf:type|owl:sameAs) ?o .
  ?s ^rdf:type ?class .
  ?s ^!(rdf:type) ?x .
}
```

Divergence hypothesis: An implementation that evaluates `^!(rdf:type)` as "all triples where `?s` is the object and predicate is not `rdf:type`" is correct. One that evaluates it as `!(^rdf:type)` (negate then invert) produces a different result. The grammar production `PathNegatedPropertySet` applies to atoms inside the negation, then the `^` reverses direction of the whole.

Rationale: SPARQL §9.3 grammar `PathElt ::= PathPrimary PathMod?` and `PathPrimary ::= '^' PathElt` — the `^` wraps the entire element including the negated set. Operator precedence in property path evaluation is a known source of divergence; no formal errata, but public-sparql-dev discussion "inverse negated property paths" (2012).

---

## Summary of Divergence Hypotheses

| # | Area | Likely miss |
|---|------|-------------|
| 1 | OPTIONAL + FILTER(unbound) | Treat unbound as false instead of error |
| 2 | MINUS shared-variable semantics | Treat MINUS as NOT EXISTS |
| 3 | CONSTRUCT blank node scope | Share blank node across result rows |
| 4 | HAVING references SELECT aggregate alias | Evaluate HAVING before SELECT projection |
| 5 | BASE mid-query silently accepted | Accept BASE inside WHERE clause |
| 6 | GRAPH ?g includes default graph | Expose default graph via GRAPH variable |
| 7 | FILTER NOT EXISTS rewrite | Rewrite as OPTIONAL/FILTER(!BOUND) incorrectly |
| 8 | INSERT DATA blank node scope | Fresh node per occurrence vs. per operation |
| 9 | Inverse negated property path precedence | Evaluate as !(^p) instead of ^(!p) |
