# W3C RDF Test Suites — Pinned Source

This file records the upstream source(s) vendored under `external/tests/`
so a reader can reproduce the tree on demand. Update only via a
deliberate `git subtree pull` (or repeat-`add`) with a new SHA.

## w3c/rdf-tests

| Field              | Value                                                            |
| ------------------ | ---------------------------------------------------------------- |
| Repository         | <https://github.com/w3c/rdf-tests>                               |
| Pinned commit SHA  | `1d6be010606d31b8a5ee62bc0eac666da004c020`                       |
| Pinned branch head | `master` (as of 2026-04-19)                                      |
| Vendored on        | 2026-04-19                                                       |
| Vendored by        | `git subtree add --prefix=external/tests/w3c-rdf-tests --squash` |
| Local prefix       | `external/tests/w3c-rdf-tests/`                                  |
| License            | W3C Test Suite dual licence (BSD-3-Clause or W3C Software Licence and Document Licence) — see `w3c-rdf-tests/LICENSE.md` and <https://www.w3.org/Consortium/Legal/2008/04-testsuite-copyright.html> |

### Refresh procedure

```bash
# from repo root, on a branch dedicated to the pin bump
git subtree pull --prefix=external/tests/w3c-rdf-tests \
    https://github.com/w3c/rdf-tests.git <new-sha> --squash
# then re-run the pruning in external/tests/README.md §Pruning
```

### Prune budget

Upstream clone is ~41 MB. After pruning (see `README.md` §Pruning) the
vendored tree is ~18 MB. Only RDF / SPARQL test artefacts (`*.ttl`,
`*.nt`, `*.nq`, `*.trig`, `*.rdf`, `*.rq`, `*.srx`, `*.srj`, `*.ru`,
`*.tsv`, `*.csv`, `*.json`, `*.md`, `*.txt`) plus the shared vocabulary
under `w3c-rdf-tests/ns/` are retained. Reports (`reports/`),
pre-built archives (`TESTS.zip`, `TESTS.tar.gz`), HAML templates, CI
scaffolding, and EARL dumps are removed.
