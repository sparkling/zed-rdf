# RDF/XML W3C Test Suite — Stub Allow-List

This file documents known failures in `crates/rdf-xml/tests/w3c_manifest.rs`
while `RdfXmlParser` is still a Phase B stub (always returns `Err`).

## Status

**Parser**: stub — `RdfXmlParser::parse` always returns `Err`  
**Negative-syntax tests**: 41 — all PASS (stub rejects correctly)  
**Positive/eval tests**: 132 — all FAIL (stub rejects, should accept)

## Allowlisted: All positive/eval tests

Every `rdft:TestXMLEval` entry in `external/tests/rdfxml/manifest.ttl` is
implicitly allowlisted until `pb-rdf-xml` delivers a real parser.

The test `positive_eval_tests_stubbed_parser_output` does NOT panic on these
failures — it logs them and exits cleanly.  Remove entries (or drop the stub
guard entirely) once the parser passes them.

## Negative-syntax tests (should NOT be on this list)

Negative-syntax tests (`rdft:TestXMLNegativeSyntax`) must always pass.  If any
appear in CI failure output, they are a genuine regression and must be fixed.

## Phase B graduation criteria

When `pb-rdf-xml` implements the parser, update `w3c_manifest.rs`:

1. Change `positive_eval_tests_stubbed_parser_output` to panic on any
   `Err` result (i.e. treat all failures as real test failures).
2. Remove the stub-detection guard and this ALLOWLIST.md.
3. Add a proper per-test allowlist for any genuinely failing W3C cases
   (document the reason: spec ambiguity, known limitation, etc.).
