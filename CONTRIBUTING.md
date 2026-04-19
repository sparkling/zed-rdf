# Contributing to zed-rdf

Thanks for your interest.

## Before you start

- Read [`docs/sparc/`](docs/sparc/) for the current scope and plan.
- Read [`docs/adr/`](docs/adr/) for accepted architectural decisions.
- Architecturally significant changes start with a new ADR — copy
  [`docs/adr/0000-madr-template.md`](docs/adr/0000-madr-template.md) to
  the next free number and open a PR containing the ADR alone before
  the implementation lands.

## Contributor licence grant

By submitting a contribution to this project, You agree that Your
contribution is licensed under the terms of the project's dual
**Apache-2.0 OR MIT** licence (see [`LICENSE-APACHE`](LICENSE-APACHE)
and [`LICENSE-MIT`](LICENSE-MIT)). No separate CLA is required; this
follows the [Rust project standard][rust-licensing].

[rust-licensing]: https://rustc-dev-guide.rust-lang.org/appendix/licensing.html

## Workflow

1. Open a PR from a feature branch.
2. Include failing tests first; implementation second; docs third.
3. Link any relevant ADR; author a new ADR if the change is
   architecturally significant.
4. CI must be green. CI enforces:
   - `cargo fmt --check`,
   - `cargo clippy -- -D warnings`,
   - `cargo test` (fast lane),
   - `cargo deny check` (per [ADR-0004](docs/adr/0004-third-party-crate-policy.md)),
   - the W3C conformance suites on `main` merges.

## Testing

Layered per [ADR-0006](docs/adr/0006-testing-strategy.md): unit,
property (`proptest`), fuzz (`cargo-fuzz`), snapshot (`insta`),
W3C manifest conformance, LSP integration.

## Code style

- `cargo fmt` + `cargo clippy -D warnings`.
- `#![forbid(unsafe_code)]` unless the crate is on the ADR-0001
  `unsafe` allow-list (currently empty).
- Edition 2024.
- Prefer spec vocabulary in type names (`Iri`, `PrefixedName`,
  `BlankNodeLabel`, `VariableOccurrence`, …) — see
  [ADR-0003](docs/adr/0003-ddd-bounded-contexts.md) on ubiquitous
  language.
