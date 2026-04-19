# Vendored W3C RDF / SPARQL Test Suites

This directory hosts the upstream W3C `rdf-tests` suite, squashed in at
a pinned commit (see [`PINS.md`](./PINS.md)), plus a thin
xtask-oriented language layout mapped onto it via symlinks.

- `w3c-rdf-tests/` — squashed subtree (do not edit files directly;
  refresh via `git subtree pull`).
- `nt/`, `nq/`, `ttl/`, `trig/`, `rdfxml/`, `sparql/` — relative symlinks
  into `w3c-rdf-tests/`. `xtask/verify` walks these roots; each points
  at the upstream directory that owns the canonical `manifest.ttl` for
  that language. See §Mapping below.

## Mapping

| xtask language root | Upstream path                         | Manifest                      | Suite type                                                         |
| ------------------- | ------------------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `nt/`               | `w3c-rdf-tests/rdf/rdf11/rdf-n-triples` | `manifest.ttl`                | Positive / negative syntax (no eval)                               |
| `nq/`               | `w3c-rdf-tests/rdf/rdf11/rdf-n-quads`   | `manifest.ttl`                | Positive / negative syntax (no eval)                               |
| `ttl/`              | `w3c-rdf-tests/rdf/rdf11/rdf-turtle`    | `manifest.ttl`                | Positive / negative syntax **and** eval (Turtle → N-Triples)       |
| `trig/`             | `w3c-rdf-tests/rdf/rdf11/rdf-trig`      | `manifest.ttl`                | Positive / negative syntax **and** eval (TriG → N-Quads)           |
| `rdfxml/`           | `w3c-rdf-tests/rdf/rdf11/rdf-xml`       | `manifest.ttl` (subdirs)      | Mostly eval (RDF/XML → N-Triples)                                  |
| `sparql/`           | `w3c-rdf-tests/sparql/sparql11`         | `manifest.ttl` (aggregates)   | Query / update syntax + eval; federation; entailment               |

"Eval" tests require the parser to produce a specific output graph; the
harness compares the parsed output against the reference N-Triples /
N-Quads provided alongside the input. "Positive / negative syntax"
tests only require accept / reject parity.

The `w3c-rdf-tests/ns/` directory holds the shared test vocabulary
(`test-manifest.ttl`, `rdftest.ttl`, `test-dawg.ttl`, `test-query.ttl`,
`test-update.ttl`) that every language manifest imports; it is
deliberately kept out of the per-language symlinks because the
manifests reach it via HTTP URIs, not relative paths.

The `rdf/rdf12/` tree from upstream is retained under `w3c-rdf-tests/`
for future work but is **not** wired into a top-level language root —
RDF 1.2 is outside Phase A's scope (ADR-0018 §4).

## Counts (pinned commit `1d6be01`)

| Language | Entries under symlink | Of which test / manifest files |
| -------- | --------------------- | ------------------------------ |
| nt       | 74                    | 73                             |
| nq       | 91                    | 90                             |
| ttl      | 433                   | 431                            |
| trig     | 470                   | 468                            |
| rdfxml   | 307                   | 306                            |
| sparql   | 1163                  | 1156                           |

"Entries under symlink" counts file nodes visible beneath the language
root after following the symlink; "test / manifest files" counts those
matching the RDF / SPARQL extensions xtask cares about (`*.nt`, `*.nq`,
`*.ttl`, `*.trig`, `*.rdf`, `*.rq`, `*.srx`, `*.srj`, `*.ru`).

## Deferred suites

The Phase A exit gate (ADR-0018 §4) only requires N-Triples, N-Quads,
Turtle, and TriG to pass 100 %. The following suites are vendored
but **not** required-green yet, and any divergences they surface are
captured in `docs/verification/adversary-findings/<lang>/w3c-divergences.md`:

- `rdfxml/` — depends on a vendored / shadow RDF-XML parser, not yet
  landed in `crates/`. The `rdf-diff-oracles` registry has the adapter
  slot reserved (`oxrdfxml_adapter`) but it is inert in the current
  xtask stub path.
- `sparql/` — query + update syntax only is in-scope for Phase C;
  eval, federation (`service-description/`), entailment regimes
  (`entailment/`), HTTP update (`http-rdf-update/`, `protocol/`), and
  JSON-LD (`json-res/`) are deferred to later phases.
- `rdf/rdf12/**` — the RDF 1.2 star-extensions suite is vendored but
  intentionally not mapped to a language root. Phase A targets RDF 1.1
  only.

## Pruning

Upstream `w3c/rdf-tests` at the pinned SHA is ~41 MB including EARL
reports, pre-built archives, and CI scaffolding. After the pruning
pass in the vendoring commit, `external/tests/w3c-rdf-tests/` is
~18 MB. The following upstream content is **removed** on vendor-in
and on every refresh; add to this list rather than re-adding files
ad-hoc:

- `**/reports/` (EARL implementation reports — not input to the suite)
- `**/TESTS.zip`, `**/TESTS.tar.gz` (pre-built archives of the same files)
- `**/*.haml` (HAML templates for the HTML index)
- `**/*.html` (rendered index pages; generated)
- `**/earl.*` (EARL result dumps)
- `.github/`, `Gemfile*`, `Rakefile`, `*.rb`, `w3c.json`,
  `manifest-frame.jsonld`, `local-biblio.js`, `local-gen/`,
  `.gitignore`, `.gitattributes` (CI / build scaffolding)

If `du -sh w3c-rdf-tests` rises materially above 20 MB after a refresh,
re-run the prune pass before committing.

## xtask contract

`xtask/verify/src/main.rs` discovers corpora via `walk_files()` which
dereferences symlinks (see the function doc for the rationale). The
`build_plan()` probe `external/tests/` existence check also
dereferences the language-root symlinks. On a fresh checkout without
this directory, xtask falls back to `external/fact-oracles/fixtures/smoke/`.

Running the gate:

```bash
cargo run -p xtask -- verify          # vendored suite; fail-closed unless real divergences surface
cargo run -p xtask -- verify --smoke  # force the smoke fallback, e.g. in a minimal checkout
```
