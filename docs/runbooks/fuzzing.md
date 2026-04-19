# Fuzzing runbook

How to reproduce, triage, and extend the `cargo-fuzz` harnesses for
the Phase-A parsers. ADR references: ADR-0018 §§3–4 (fuzz policy),
ADR-0017 §4 (parser scope), ADR-0020 §1.4 (diff-harness integration).

## Layout

Per-parser `fuzz/` directories, one binary per target:

| Crate              | Target                      | Invariant gated                 |
| ------------------ | --------------------------- | ------------------------------- |
| `rdf-iri`          | `parse`                     | No panic; IRI-PCT-001 round-trip |
| `rdf-iri`          | `resolve`                   | No panic; resolve idempotence    |
| `rdf-iri`          | `normalise_idempotence`     | `normalise` is a fixed point     |
| `rdf-ntriples`     | `parse_ntriples`            | No panic; structured `Diagnostics` |
| `rdf-ntriples`     | `parse_nquads`              | Graph term shape; `Diagnostics`  |
| `rdf-turtle`       | `parse_turtle`              | No panic; linear fact bound      |
| `rdf-turtle`       | `parse_trig`                | Graph shape on accept            |
| `rdf-turtle`       | `bnode_scope_invariants`    | TTL-BNPFX-001 document-scope     |

CI workflows:

- `.github/workflows/fuzz-smoke.yml` — 60 s per target on every PR and
  nightly; fails on any crash.
- `.github/workflows/fuzz-nightly.yml` — 30 min per target nightly,
  with corpus minimisation (`cargo fuzz cmin`) and artifact upload.

### CI matrix (verified 2026-04-19 against HEAD `2dd83ab`)

Both workflows enumerate the same eight `(crate, target)` cells; the
matrix is `fail-fast: false` so one target's crash never masks another.
Bumping this list means editing both workflow files **and** the
"Layout" table above in lockstep — there is no generator.

| Crate          | Targets                                                  |
| -------------- | -------------------------------------------------------- |
| `rdf-iri`      | `parse`, `resolve`, `normalise_idempotence`              |
| `rdf-ntriples` | `parse_ntriples`, `parse_nquads`                         |
| `rdf-turtle`   | `parse_turtle`, `parse_trig`, `bnode_scope_invariants`   |

### Nightly toolchain pin

`cargo-fuzz` needs nightly for `-Z build-std` + sanitizer plumbing, so
the workspace-root `rust-toolchain.toml` (stable + `wasm32-wasip2`) is
**deliberately not changed**. Each fuzz workflow instead pins
`FUZZ_TOOLCHAIN: nightly-2026-03-01` and installs it via
`dtolnay/rust-toolchain@master` with `components: rust-src`. Bumping
the pin is a routine PR; do both `fuzz-smoke.yml` and
`fuzz-nightly.yml` in the same change.

### Cache key strategy

Both workflows use `Swatinem/rust-cache@v2` keyed on
`shared-key: fuzz-<workflow>-<crate>` with
`workspaces: crates/<crate>/fuzz`. Consequences:

- Cache is **per-crate**, not per-target — all targets inside a crate
  share one build cache, which is correct because they link the same
  dependency closure.
- `save-if: ${{ github.ref == 'refs/heads/main' }}` means PR runs read
  the cache but never write it. This prevents a long-running fuzz PR
  from poisoning `main`'s cache with stale artefacts.
- Smoke and nightly have **separate** cache namespaces (`fuzz-smoke-*`
  vs `fuzz-nightly-*`). They never cross-pollinate — nightly's
  coverage-instrumented build differs enough that sharing would cost
  more than it saves.
- Corpora are not cached here; they are shipped between runs via the
  `fuzz-nightly-corpus-<crate>-<target>` artifact, retention 30 days.

## Prerequisites

`cargo-fuzz` needs a nightly toolchain (`-Z build-std`, sanitizer
plumbing). The workspace's `rust-toolchain.toml` pins stable; install
a nightly side-by-side:

```bash
rustup toolchain install nightly
cargo +nightly install cargo-fuzz --locked
```

CI pins `nightly-2026-03-01` (see `FUZZ_TOOLCHAIN` in both
workflows); local runs can drift, but pin when a CI crash is being
reproduced.

## Run a target locally

From a parser crate's `fuzz/` directory:

```bash
cd crates/rdf-turtle/fuzz
cargo +nightly fuzz build --all-targets       # compile, no run
cargo +nightly fuzz run parse_turtle -- -max_total_time=5
```

The 5-second run is the Phase-A acceptance check: every target
completes without a crash on `main` at 8ded010. For a longer local
soak, raise `-max_total_time` or omit it (runs until ctrl-C).

## Reproduce a CI crash

Download the `fuzz-smoke-crash-<crate>-<target>` (or the nightly
equivalent) artifact from the failing run, extract the
`crash-<hash>` file, then replay it against the target:

```bash
cd crates/rdf-iri/fuzz
cargo +nightly fuzz run parse <path/to/crash-abcdef0123>
```

libFuzzer will hit the same panic / assertion immediately. Attach
`RUST_BACKTRACE=1` for a stack trace.

## Minimise a crash input

Once reproduced, shrink the reproducer before filing:

```bash
cargo +nightly fuzz tmin parse <path/to/crash-abcdef0123>
```

The minimised file lands under `fuzz/artifacts/<target>/`.

## Seed a corpus

The corpus is **not** committed — gitignored under
`crates/*/fuzz/corpus/`. To prime a local run with W3C RDF test
fixtures:

```bash
# Turtle example; adapt for other parsers.
mkdir -p crates/rdf-turtle/fuzz/corpus/parse_turtle
cp external/tests/ttl/*.ttl crates/rdf-turtle/fuzz/corpus/parse_turtle/
cargo +nightly fuzz run parse_turtle -- -max_total_time=60
```

libFuzzer discovers and deduplicates corpus entries automatically.

## Grow or minimise an existing corpus

```bash
# Merge a new directory of inputs into the canonical corpus:
cargo +nightly fuzz run parse_turtle -- \
    -merge=1 corpus/parse_turtle/ /path/to/new-inputs/

# Minimise the canonical corpus to its shortest cover:
cargo +nightly fuzz cmin parse_turtle
```

The nightly workflow runs `cmin` automatically and uploads the
result as a `fuzz-nightly-corpus-*` artifact.

## Adding a target

1. Create `crates/<parser>/fuzz/fuzz_targets/<name>.rs` with a
   `#![no_main]` + `fuzz_target!` block. Gate only shape-level
   invariants (no panic, structured `Diagnostics`) — never string-
   compare diagnostic messages.
2. Add a `[[bin]]` entry to the crate's `fuzz/Cargo.toml`.
3. Add a matrix row to both workflow files (smoke + nightly).
4. Run `cargo +nightly fuzz build --all-targets` locally.
5. Open a PR; the smoke workflow's 60 s run is the merge gate.

## Escalation

If two consecutive nightlies surface a crash in the same target, the
`cu-fuzz-triage` on-call bumps `fuzz-nightly.yml`'s `FUZZ_MAX_SECONDS`
to `7200` (2 hours) and the cron to `0 5 * * 0` (weekly). Revert when
the triaging PR lands.
