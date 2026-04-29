# zed-rdf

A [Zed editor](https://zed.dev) extension and Rust LSP for the **RDF family** of
languages — 11 languages covered end-to-end by a single language server.

> **Editor-only.** This is a language tool: syntax, structure, and static
> lookups. No triple store, no SPARQL execution, no reasoning, no network.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#licence)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](#building)
[![Zed](https://img.shields.io/badge/zed-extension-black.svg)](https://zed.dev/docs/extensions)

---

## Table of contents

- [What you get](#what-you-get)
- [Supported languages](#supported-languages)
- [Feature matrix](#feature-matrix)
- [Screenshots](#screenshots)
- [Install from Zed](#install-from-zed)
- [Install from source](#install-from-source)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Performance](#performance)
- [Verification discipline](#verification-discipline)
- [Publishing the extension to Zed](#publishing-the-extension-to-zed)
- [Building](#building)
- [Testing](#testing)
- [Project layout](#project-layout)
- [Contributing](#contributing)
- [Roadmap](#roadmap)
- [Acknowledgements](#acknowledgements)
- [Licence](#licence)

---

## What you get

`zed-rdf` ships two pieces that work together:

1. **`rdf-lsp`** — a standalone Rust [Language Server
   Protocol](https://microsoft.github.io/language-server-protocol/) server
   that understands all 11 RDF-family languages. It runs anywhere an LSP
   client is supported (Zed, Neovim, VS Code with a generic LSP client,
   Helix…).
2. **The Zed extension** (`extensions/zed-rdf/`) — a thin WASM launcher
   that wires up file extensions, tree-sitter grammars, and highlight
   queries, and starts `rdf-lsp` when one of the supported file types is
   opened.

You can use the LSP independently of Zed. The Zed extension just gives
you a one-click install.

## Supported languages

| Language       | Extensions                   | Highlight | Diagnostics | Hover | Completion | Format |
|----------------|------------------------------|:---------:|:-----------:|:-----:|:----------:|:------:|
| Turtle         | `.ttl`, `.turtle`            | ✅        | ✅          | ✅    | ✅         | ✅     |
| TriG           | `.trig`                      | ✅        | ✅          | ✅    | ✅         | ✅     |
| N-Triples      | `.nt`                        | ✅        | ✅          | ✅    | ✅         | ✅     |
| N-Quads        | `.nq`                        | ✅        | ✅          | ✅    | ✅         | ✅     |
| RDF/XML        | `.rdf`                       | ✅ (XML)  | ✅          | ✅    | ✅         | —      |
| JSON-LD        | `.jsonld`                    | ✅ (JSON) | ✅          | ✅    | ✅         | —      |
| TriX           | `.trix`                      | ✅ (XML)  | ✅          | ✅    | ✅         | —      |
| Notation3 (N3) | `.n3`                        | ✅        | ✅          | ✅    | ✅         | —      |
| SPARQL         | `.sparql`, `.rq`, `.ru`      | ✅        | ✅          | ✅    | ✅         | —      |
| ShEx           | `.shex`                      | ✅        | ✅          | ✅    | ✅         | —      |
| Datalog        | `.dl`                        | ✅        | ✅          | ✅    | ✅         | —      |

**Coverage:** 149/149 W3C SPARQL 1.1 test-suite syntax entries pass.
RDF/XML + JSON-LD syntax suites: 100 % green. NT, N-Quads, Turtle, TriG
W3C manifests: 100 % green.

## Feature matrix

### Parsing and diagnostics

- **Error-tolerant parsers** — recovery at statement boundaries; a single
  typo in a Turtle file doesn't silence all downstream highlighting.
- **Structured diagnostics** — every error carries a stable code
  (e.g. `SPARQL-BIND-001`, `SPARQL-PROLOGUE-001`) keyed to the spec
  sections in `docs/spec-readings/`.
- **Adversary-hardened** — each parser has been fuzzed against a shared
  hive of adversary fixtures (see [Verification](#verification-discipline)).

### LSP features

| Feature                | Implementation notes                                                                   |
|------------------------|----------------------------------------------------------------------------------------|
| **Diagnostics**        | Push model (`publishDiagnostics`); full-sync; one diagnostic per error with code.      |
| **Hover**              | Vocabulary lookup against 513 curated terms across 11 well-known vocabularies.         |
| **Completion**         | Per-language keyword lists + vocab-aware prefix expansion.                             |
| **Goto definition**    | Turtle `@prefix` → IRI resolution; SPARQL variable → first binding site.               |
| **Document symbols**   | Hierarchical: prefixes, blank node scopes, rule heads.                                 |
| **Formatting**         | Deterministic pretty-printer for NT, N-Quads, Turtle, TriG (idempotent under re-run).  |
| **Rename**             | Turtle prefix labels (all `<label>:` occurrences); SPARQL variables (`?var`/`$var`).   |
| **Code actions**       | Sort prefixes, add missing prefix, extract prefix (13 well-known namespaces built in). |
| **Semantic tokens**    | 9-type legend, delta-encoded. 562 µs for 10k-line Turtle (178× under the 100 ms gate). |
| **Incremental parse**  | Parse cache with skip-on-no-change; statement-boundary-aware re-parse window.          |

### Included vocabularies

All with label + description for hover:

`xsd`, `rdf`, `rdfs`, `owl`, `skos`, `sh` (SHACL), `dcterms`, `dcat`,
`foaf`, `schema` (Schema.org), `prov` — **513 terms total**.

### Zed-side extras

- Tree-sitter-based syntax highlighting (grammars pinned by commit).
- File type associations, bracket matching, auto-indent, folding.
- Works with Zed's built-in formatter key binding (`Cmd-Shift-I`).

## Screenshots

Coming once the extension is live in the Zed registry. Preview locally
with [Install from source](#install-from-source).

## Install from Zed

> The extension is pending submission to the public registry. See
> [Publishing the extension to Zed](#publishing-the-extension-to-zed) for
> status.

Once published:

1. Open Zed.
2. `Cmd-Shift-X` (macOS) / `Ctrl-Shift-X` (Linux) → **Extensions**.
3. Search for **rdf** and click **Install**.
4. Install `rdf-lsp` binary (see [Building](#building)) and ensure it's on
   your `PATH`. The extension looks for `rdf-lsp` via
   `which rdf-lsp`.

## Install from source

### 1. Install prerequisites

```bash
# Rust stable
rustup install stable
rustup default stable

# Zed extension WASM target
rustup target add wasm32-wasip2
```

### 2. Build and install `rdf-lsp`

```bash
cd /path/to/zed-rdf
cargo install --path crates/rdf-lsp
# places `rdf-lsp` in ~/.cargo/bin (make sure it's on PATH)
rdf-lsp --version
```

### 3. Install the Zed extension as a dev extension

```bash
# From Zed:
# Cmd-Shift-P -> "zed: install dev extension" -> pick extensions/zed-rdf/
```

Open any `.ttl` / `.sparql` / `.shex` / etc. file — you should see
highlighting immediately and diagnostics after a brief parse.

## Configuration

The extension needs no configuration out of the box. `rdf-lsp` is
launched by Zed when a supported file is opened.

To point at a specific binary (e.g. for development), set the language
server binary path in your Zed `settings.json`:

```json
{
  "lsp": {
    "rdf-lsp": {
      "binary": {
        "path": "/absolute/path/to/rdf-lsp"
      }
    }
  }
}
```

## Architecture

```
┌───────────────────────────┐   ┌─────────────────────────────────┐
│ Zed editor                │   │ rdf-lsp (standalone Rust binary)│
│                           │   │                                 │
│  ┌─────────────────────┐  │   │  dispatch.rs   (LSP glue)       │
│  │ zed-rdf extension   │  │   │   │                             │
│  │  (WASM, cdylib)     │◀─┼───┼──┤  hover / completion / …      │
│  │                     │  │   │   │                             │
│  │  launches rdf-lsp   │  │   │  11 parsers (rdf-ntriples,      │
│  │  via `which`        │  │   │   rdf-turtle, rdf-xml,          │
│  │                     │  │   │   rdf-jsonld, rdf-trix,         │
│  │  tree-sitter queries│  │   │   rdf-n3, sparql-syntax,        │
│  │  (highlight only)   │  │   │   shex-syntax, datalog-syntax)  │
│  └─────────────────────┘  │   │                                 │
└───────────────────────────┘   └─────────────────────────────────┘
```

Full details in [`docs/sparc/03-architecture.md`](docs/sparc/03-architecture.md)
and the ADRs under [`docs/adr/`](docs/adr/).

## Performance

Benchmarked with [criterion](https://github.com/bheisler/criterion.rs); see
`crates/<parser>/benches/`. Targets and measured results at v0.1.0:

| Metric                              | Measured   | Target      |
|-------------------------------------|------------|-------------|
| N-Triples parse throughput          | ≥ 200 MB/s | ≥ 200 MB/s  |
| Turtle parse throughput             | ≥ 80 MB/s  | ≥ 80 MB/s   |
| SPARQL parse throughput             | ≥ 1000 q/s | ≥ 1000 q/s  |
| Semantic-tokens on 10k-line Turtle  | 562 µs     | ≤ 100 ms    |

CI fails on > 10 % regression against committed baselines
(`bench/baselines/`).

## Verification discipline

Every main parser is verified three ways ([ADR-0019](docs/adr/0019-independent-verification.md)):

1. **Shadow parser** — an independent implementation in a disjoint
   cohort (model-diverse, namespace-isolated from the main parser's
   memory). Diff-harnessed output must match canonical facts.
2. **Oracle adapter** — third-party parsers (`oxttl`, `oxrdfxml`,
   `oxjsonld`, `spargebra`, `sophia`) via `rdf-diff-oracles`.
3. **Adversary hive** — a separate cohort authors failure-mode fixtures
   without access to implementation memory. At v0.1.0: 33 findings
   documented, 24 vetoes fired.

Plus:

- **Fuzzing.** Per-crate `cargo-fuzz` targets with a 3-invariant contract
  (no panics, structured rejection shape, linear output bound). CI runs
  60-second smoke on every PR and 30-minute nightly soak across 12 targets.
- **W3C conformance gates.** 0 allow-list for NT/N-Quads/Turtle/TriG/SPARQL/
  RDF-XML/JSON-LD.

## Publishing the extension to Zed

The public Zed extension registry is at
**<https://github.com/zed-industries/extensions>**. The flow (verified
2026-04-20):

### One-time prerequisites

- ✅ Repository has a permissive license at the root
  (`LICENSE-APACHE` / `LICENSE-MIT` — both accepted).
- ✅ `extension.toml` has a stable `id`, `name`, `version`, `description`,
  `authors`, and a correct `repository` URL.
- ✅ `extension.toml`'s `zed_extension_api` version pin is current.
  Verify against [crates.io/zed_extension_api](https://crates.io/crates/zed_extension_api)
  before submitting.
- ✅ The host repo (`zed-rdf`) is public on GitHub (see
  [Publishing to a public repo](#publishing-this-repo-to-github) below).

### Submitting a new extension

1. **Tag your release.** The `version` field in `extension.toml` must
   match the state at a specific commit SHA.
   ```bash
   git tag -a v0.1.1 -m "zed-rdf v0.1.1"
   git push origin v0.1.0
   ```

2. **Fork the registry** to a **personal** GitHub account (not an org —
   staff need push access to your PR branch):
   ```bash
   gh repo fork zed-industries/extensions --clone
   cd extensions
   ```

3. **Add this repo as a submodule** under `extensions/zed-rdf/`:
   ```bash
   git submodule add \
       https://github.com/<your-user>/zed-rdf.git \
       extensions/zed-rdf
   cd extensions/zed-rdf
   git checkout v0.1.1     # exact commit matching extension.toml version
   cd ../..
   ```

4. **Add an entry to `extensions.toml`** (alphabetically sorted):
   ```toml
   [rdf]
   submodule = "extensions/rdf"
   version = "0.1.1"
   path = "extensions/zed-rdf"    # subdir inside the submodule
   ```
   The `path` key is critical — this is a monorepo; the extension lives
   at `extensions/zed-rdf/` inside the outer repo.

5. **Sort and validate**:
   ```bash
   pnpm install          # one-time
   pnpm sort-extensions
   ```

6. **Open a PR** to `zed-industries/extensions`. Zed CI will:
   - Validate the license.
   - Validate the `extension.toml` schema.
   - Build your extension to WASM from source at the pinned commit.
   - Run `dangerfile.ts` sort / metadata checks.

7. **Wait for review and merge.** Once merged, the extension appears in
   Zed's in-app Extensions UI within minutes (next registry refresh).

### Updating after publish

Same flow: bump `version` in both `extension.toml` here and `extensions.toml`
in your registry fork, point the submodule at the new commit SHA, open a PR.

### Publishing this repo to GitHub

```bash
cd /path/to/zed-rdf
gh repo create zed-rdf --public \
    --description "Zed extension + Rust LSP for RDF, SPARQL, ShEx, Datalog" \
    --source=. --remote=origin --push
git push --tags   # so phase-i/done and v0.1.0 reach the remote
```

Template PRs to use as guidance (both recently merged monorepo-layout
LSP-launching extensions):

- [zed-industries/extensions#5494](https://github.com/zed-industries/extensions/pull/5494)
  — csskit-lsp (merged 2026-04-13). Monorepo with `path` key.
- [zed-industries/extensions#5035](https://github.com/zed-industries/extensions/pull/5035)
  — rovo language server.

## Building

```bash
# All workspace crates
cargo build --workspace --release

# Just the LSP binary (for use outside Zed)
cargo install --path crates/rdf-lsp

# The Zed extension (produces WASM)
cd extensions/zed-rdf
cargo build --release --target wasm32-wasip2
```

## Testing

```bash
# Full workspace test suite
cargo test --workspace --all-features

# Clippy gate (used in CI)
cargo clippy --workspace --all-features -- -D warnings

# Dependency / license audit
cargo deny check

# W3C conformance sweep (requires vendored suites in external/tests/)
cargo run -p xtask -- verify

# Fuzz smoke (60 seconds per target; needs cargo-fuzz + nightly)
cd crates/rdf-ntriples/fuzz
cargo +nightly fuzz run parse_ntriples -- -max_total_time=60 -detect_leaks=0
```

## Project layout

```
zed-rdf/
├── crates/                      # all Rust crates
│   ├── rdf-diagnostics/         # shared diagnostic infrastructure
│   ├── rdf-iri/                 # RFC 3987 IRI parser / RFC 3986 resolver
│   ├── rdf-ntriples/            # N-Triples 1.1 + N-Quads 1.1
│   ├── rdf-turtle/              # Turtle 1.1 + TriG 1.1
│   ├── rdf-xml/                 # RDF/XML
│   ├── rdf-jsonld/              # JSON-LD 1.1 syntax + @context
│   ├── rdf-trix/                # TriX
│   ├── rdf-n3/                  # Notation3
│   ├── sparql-syntax/           # SPARQL 1.1 query + update
│   ├── shex-syntax/             # ShEx compact syntax
│   ├── datalog-syntax/          # Datalog
│   ├── rdf-vocab/               # 513 terms across 11 vocabs
│   ├── rdf-format/              # pretty-printers
│   ├── rdf-lsp/                 # the language server binary
│   ├── syntax/                  # shadow parsers (verification only)
│   └── testing/                 # rdf-diff harness + oracle adapters
├── extensions/
│   └── zed-rdf/                 # WASM extension (cdylib)
├── docs/
│   ├── adr/                     # 28 architecture decision records
│   ├── sparc/                   # SPARC methodology phases 01-05
│   ├── spec-readings/           # annotated W3C spec pins
│   ├── runbooks/                # ops procedures
│   └── verification/            # adversary findings, vetoes
├── xtask/                       # workspace task runner (W3C verify)
├── external/tests/              # vendored W3C test suites (subtree)
├── .github/workflows/           # CI: test, clippy, deny, fuzz, tree-sitter
├── CHANGELOG.md
├── CONTRIBUTING.md
└── README.md
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). The short version:

1. Open an issue first for anything larger than a typo.
2. All code goes in via PR; two-reviewer rule for parser crates.
3. CI must be green (`cargo test`, clippy `-D warnings`, `cargo deny`,
   fuzz smoke, tree-sitter query parse).
4. New languages / features require an ADR under `docs/adr/`.

## Roadmap

**Shipped in v0.1.0** — all 11 language parsers, full LSP, Zed
extension scaffold, 100 % W3C conformance on the syntax gates.

**Out of scope** — intentionally. See
[`docs/sparc/01-specification.md`](docs/sparc/01-specification.md) §2:
no triple store, no query execution, no OWL/SHACL/ShEx validation, no
Datalog evaluation, no network I/O. The sign on the door.

**Candidate for future work**:

- RDF 1.2 syntax (triple terms) behind a feature flag — waiting on REC.
- JSON-LD 1.1 expand/compact (currently only `@context` well-formedness).
- Workspace symbols across files.
- Additional tree-sitter grammars for RDF/XML and JSON-LD (currently
  delegated to Zed's built-in XML/JSON grammars).

## Acknowledgements

- [W3C RDF and SPARQL working groups](https://www.w3.org/groups/wg/rdf-star/)
  for the specs and test suites.
- [Oxigraph](https://github.com/oxigraph/oxigraph) for `oxttl`, `oxrdfxml`,
  `oxjsonld`, `spargebra` — used as oracle parsers in our verification
  harness.
- [Sophia](https://github.com/pchampin/sophia_rs) — also used as an oracle.
- The [tree-sitter-turtle](https://github.com/nicowillis/tree-sitter-turtle),
  [tree-sitter-sparql](https://github.com/GordianDziwis/tree-sitter-sparql),
  and [tree-sitter-shex](https://github.com/nicowillis/tree-sitter-shex)
  grammar authors.
- [Zed](https://zed.dev) for the editor and extension API.

## Licence

Dual-licensed under **Apache-2.0 OR MIT** at your option. See
[`LICENSE-APACHE`](LICENSE-APACHE) and [`LICENSE-MIT`](LICENSE-MIT).

Contributions are accepted under the same terms — see
[`CONTRIBUTING.md`](CONTRIBUTING.md).
