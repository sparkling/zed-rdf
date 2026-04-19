# `rdf-diagnostics`

Shared diagnostic-reporting infrastructure for the zed-rdf parser
family. This crate is **upstream of every Phase A parser** per
[ADR-0017 §4](../../docs/adr/0017-execution-model.md) and is
intentionally tiny: it owns the diagnostic boundary type and nothing
else.

## Public surface

```rust
pub enum Severity { Error, Warning, Info, Hint }
pub struct Span { pub start: usize, pub end: usize }        // byte half-open
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,     // e.g. "NT-LITESC-001"
    pub message: String,
    pub span: Span,
    pub hint: Option<String>,
    pub related: Vec<Related>,
}
pub struct Related { pub span: Span, pub message: String }
pub struct DiagnosticBag { /* push / extend / is_fatal / into_vec */ }
pub fn render(diagnostic: &Diagnostic, source: &str) -> String;
```

## How parsers use it

1. Add the dep with `default-features = false` so parser crates stay
   LSP-agnostic:

   ```toml
   [dependencies]
   rdf-diagnostics = { path = "../rdf-diagnostics", default-features = false }
   ```

2. Hold a `DiagnosticBag` for the duration of a parse, push into it,
   and hand `bag.into_vec()` back on exit alongside whatever facts /
   CST you produce. Fatality is a function of the bag:
   `bag.is_fatal()` iff at least one `Severity::Error` was pushed.

3. Mint a stable `code` per spec-reading pin. Codes live next to their
   justification under `docs/spec-readings/`. Once minted a code never
   changes.

   ```rust
   use rdf_diagnostics::{Diagnostic, Span};
   bag.push(
       Diagnostic::error(
           "NT-LITESC-001",
           format!("invalid UCHAR escape at byte {offset}: surrogate"),
           Span::new(offset, offset + 6),
       )
       .with_hint("use a non-surrogate Unicode scalar value"),
   );
   ```

4. For CLI / snapshot output call `render(&diag, source)`. The LSP
   crate builds its own range-aware renderer; do **not** feed
   `render()` output to the LSP client.

## Feature flags

- `lsp` *(off by default)* — adds `impl From<Diagnostic> for
  lsp_types::Diagnostic`. Enabled only by the `rdf-lsp` crate; parser
  crates depend on `rdf-diagnostics` with `default-features = false`
  per ADR-0004.

## Stability contract

Breaking this crate's public API is a workspace-wide event. Adding a
new `Severity` variant, renaming a `Diagnostic` field, or changing
`Span`'s offset convention requires an ADR amendment. Additive
changes (new builder methods, new optional fields via builder) do not.
