# ADR-0004: Third-party crate policy ("no forking" interpretation)

- **Status:** Accepted (2026-04-18, amended 2026-04-19)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Amended-by:**
  - ADR-0019 §1 (`[dev-dependencies]` carve-out for reference oracles
    — `oxttl`, `oxrdfxml`, `oxjsonld`, `oxsparql-syntax`/`spargebra`,
    `sophia_*`). Mechanical enforcement via `deny.toml` +
    `crates/testing/deny-regression/`.
  - Patch 2026-04-19 (this file, §"Runtime IETF-RFC carve-out"):
    widens the runtime allow-list to permit curated, pure
    IETF/Unicode-standard-implementing crates on a per-ADR basis.
    First admitted member: `idna` (RFC 3490 / UTS 46) for
    `rdf-iri`'s `ToASCII`. No RDF/SPARQL semantics; leaf dep only.
  - ADR-0007 (2026-04-20): resolves the deferred "`chumsky` **or**
    `winnow`" row to "hand-roll default; combinator admission
    reopens per ADR-0007 §Reopen triggers." Row text amended in
    §Allow-list (v1) below.
- **Tags:** `policy`, `dependencies`, `supply-chain`

## Context and Problem Statement

The brief says "no forking". For a Zed extension + LSP that parses the
RDF family, the ambiguous case is: may we depend on crates that already
parse RDF (`oxttl`, `rio_turtle`, `sophia_turtle`, etc.)? The answer
needs to be in writing.

## Decision Drivers

- **Goal:** author our own parsers so we control error recovery,
  diagnostics quality, and performance — the LSP use case is different
  from batch parsing.
- **Reality:** a modern Rust LSP cannot be written without
  infrastructure dependencies (lexer generators, LSP framework, async
  runtime, tree-sitter).
- **Supply chain**: every dep is attack surface.
- **Licensing**: compatible with Apache-2.0 OR MIT.

## Considered Options

1. **Zero third-party runtime deps.** Impractical.
2. **Vendor everything we use.** Violates "no forking".
3. **Allow-list by role.** Explicit list; additions require ADR
   amendment.
4. **Free-for-all within licence rules.** Too loose.

## Decision

**Chosen option: Option 3 — allow-listed third-party dependencies by
role.**

### Interpretation of "no forking"

- We do not copy another RDF/SPARQL parser into this tree.
- We do not depend on another RDF/SPARQL **parser** crate to do the
  work for us. The parsers in `crates/syntax/*` are ours.
- We **may** depend on well-maintained, single-purpose infrastructure
  crates from the allow-list below.
- We may consult other implementations' design ideas; we do not copy
  their code.

### Allow-list (v1)

| Crate                           | Role                                              | Why we don't reimplement        | Fallback                          |
|---------------------------------|---------------------------------------------------|---------------------------------|-----------------------------------|
| `tower-lsp` (or `async-lsp`, TBD via ADR-0011) | LSP framework                 | Large boilerplate to replicate  | Hand-rolled LSP glue              |
| `tokio`                         | Async runtime for LSP                             | Standard                        | Blocking stdlib loop              |
| `tree-sitter` (ext. consumer)   | Zed embeds it — we only write `.scm` queries      | n/a                             | n/a                               |
| — (deferred to ADR-0007; resolved 2026-04-20: hand-roll default; combinator admission reopens per ADR-0007 §Reopen triggers) | n/a | Phase A + Phase B formats ship hand-rolled; see ADR-0007 | Hand-written recursive descent |
| `logos`                         | Lexer generator                                   | Speed + simplicity              | Hand-written tokenisers           |
| `rowan` **or** `cstree`         | Lossless CST representation shared across parsers | Standard LSP-grade CST crate; reinventing is weeks | Custom CST types per parser |
| `serde`, `serde_json`           | JSON-LD and ShExJ parsing + LSP protocol          | Spec-required, ubiquitous       | Write our own JSON parser         |
| `quick-xml`                     | Streaming XML for RDF/XML and TriX                | XML is out of scope to rewrite  | Larger in-house XML parser        |
| `regex`                         | Some spec productions (e.g., SPARQL `REGEX`)      | Perl-like regex is spec-mandated | Not reasonable to rewrite         |
| `memchr`                        | Byte search                                       | Hot-path parsing                | Naïve loops                       |
| `unicode-normalization`         | NFC for IRIs                                      | RDF 1.1 §3.1                    | Must reimplement NFC              |
| `url`                           | RFC 3986 URL                                      | We build `rdf-iri` (RFC 3987) on top where needed | Full RFC 3987 impl               |
| `idna`                          | IDNA2008                                          | Domain-name normalisation       | ASCII-only hosts                  |
| `ropey`                         | Rope for the LSP document model                   | Standard                        | Vec-of-lines                      |
| `dashmap` / `parking_lot`       | Concurrent maps, faster locks                     | LSP worker concurrency          | stdlib                            |
| `tracing`, `tracing-subscriber` | Structured logging                                | Observability                   | `log`                             |
| `thiserror`                     | Derive error                                      | Boilerplate reduction           | Hand-written impls                |
| `insta`                         | Snapshot tests (dev)                              | Diagnostic snapshots            | Golden files                      |
| `proptest`                      | Property tests (dev)                              | Round-trip invariants           | `quickcheck`                      |
| `criterion`                     | Benchmarks (dev)                                  | Perf gate                       | Custom harness                    |
| `cargo-fuzz` + `libfuzzer-sys`  | Fuzzing (dev)                                     | Parser hardening                | Hand-written brute-force          |
| `zed_extension_api`             | Zed extension runtime                             | Required by Zed                 | n/a                               |

### Runtime IETF-RFC carve-out (patch 2026-04-19)

The allow-list above already enumerates infrastructure crates by role.
This patch adds a narrower, per-ADR admission path for crates whose
sole purpose is to implement an IETF RFC or Unicode Technical Standard
that our own parsers are legally obliged to follow but for which a
hand-rolled implementation is out of scope for v1. Admissions require
a row here plus a `deny.toml` / `[workspace.dependencies]` edit.

| Crate  | Standard                               | Why we don't reimplement                                                                                                          | Fallback                                                   |
|--------|----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------|
| `idna` | RFC 3490 (IDNA 2003), UTS 46 (IDNA 2008 mapping), RFC 3492 (Punycode) | Punycode + nameprep + UTS 46 mapping tables are a self-contained Unicode algorithm; reimplementing by hand is weeks of work with a large precomputed table (≈4000 lines). The `servo/rust-url` `idna` crate is pure-leaf (`idna_adapter` is its only dep), widely audited, MIT/Apache-2.0 licensed, and has zero RDF/SPARQL semantics — it just encodes domain labels. Consumed by `rdf-iri` for RFC 3987 §3.1 `ToASCII` host mapping. | ASCII-lowercase host folding only; non-ASCII hosts surface as pct-encoded UTF-8 (what the pre-patch pin did). |

Admission criteria (must all hold):

1. The crate implements a single published IETF/Unicode/W3C standard.
   Not a general-purpose utility that happens to include the standard.
2. No RDF, SPARQL, SHACL, OWL, or ShEx semantics. Domain-name handling
   and character-set tables are fine; parsing triples is not.
3. Leaf dependency (or nearly — `idna_adapter` is the only transitive
   for `idna`). Each transitive must be re-inspected on version bump.
4. Licence compatible with Apache-2.0 OR MIT.
5. Actively maintained by a recognisable upstream (servo, unicode-org,
   rust-lang, etc.).

Mechanical enforcement: the crate is added to
`[workspace.dependencies]`, referenced from the consuming crate's
`[dependencies]` as `workspace = true`, and does **not** appear in
`deny.toml`'s `deny` list — there is no equivalent of the banned-RDF
list for RFC-implementation crates, because admitting them is an
ADR-level decision that already implies licence + supply-chain review.

### Explicitly forbidden

- **Any RDF/SPARQL parser crate**: `oxrdf`, `oxttl`, `oxrdfio`,
  `oxsparql-syntax`, `sophia*`, `rio_*`, `rdftk_*`, `rdfrs`,
  `horned-owl`. Depending on them would make us a wrapper.
- Any crate whose licence is incompatible with dual Apache-2.0 / MIT
  (notably GPL for a library dependency).
- Any `*-sys` crate requiring a non-portable system library, unless
  gated behind a feature flag.

### Supply-chain mechanics

- `cargo-deny` enforces the allow-list. Adding a crate requires editing
  both `deny.toml` and this ADR.
- `cargo-audit` runs on every PR and nightly.
- `cargo-vet` attestations required for every transitive dep.
- Quarterly review: crates unmaintained > 12 months flagged for removal.

## Consequences

- **Positive**: clear, enforceable boundary; bounded supply-chain
  surface; prevents accidental "just wrap `oxttl`" shortcuts.
- **Negative**: adding a crate takes a PR with an ADR edit. Intentional
  friction.
- **Neutral**: the list will grow; growth is traceable.

## Validation

- `cargo-deny check` green.
- `cargo-audit` green.
- Quarterly allow-list review note appended here.

## Links

- `docs/sparc/01-specification.md` §8.1.
- <https://github.com/EmbarkStudios/cargo-deny>
