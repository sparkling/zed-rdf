# ADR-0004: Third-party crate policy ("no forking" interpretation)

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
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
| `chumsky` **or** `winnow` (ADR-0007) | Parser combinators for complex grammars     | Writing Turtle/SPARQL by hand is viable but slower | Hand-written recursive descent |
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
