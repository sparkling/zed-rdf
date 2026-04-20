---
agent_id: pf-tester
cohort: cohort-a
hive: phase-f
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/rdf-lsp/tests/**
---

# pf-tester — LSP integration tests

You are cohort-A agent `pf-tester`. Your job is to write LSP integration
tests for `rdf-lsp` using `lsp-server`'s `Connection` API in test mode.

## Read first

1. `.claude-flow/phase-f/arch-memo.md` — architect memo for context on
   server capabilities and handler signatures.
2. `docs/adr/0025-phase-f-execution-plan.md` — scope and exit gate.
3. `crates/rdf-lsp/src/lib.rs` — the `run_server()` entry point.
4. `crates/rdf-lsp/Cargo.toml` — available dependencies.

## Goal

Create `crates/rdf-lsp/tests/lsp_integration.rs` with LSP integration
tests using a pipe-based test client that exercises the JSON-RPC message
loop.

### Test approach

Use `lsp-server`'s `Connection::memory()` (or a `crossbeam-channel`
pipe pair) to create an in-process client/server pair. Spawn `run_server()`
on a background thread. Send JSON-RPC messages from the test thread.

If `lsp-server` does not expose a memory transport, use OS pipes:
```rust
let (client_reader, server_writer) = ...;
let (server_reader, client_writer) = ...;
// Spawn server thread with server_reader/server_writer.
// Test thread uses client_reader/client_writer.
```

### Required tests

1. **Server initializes successfully.**
   - Send `initialize` request with standard client capabilities.
   - Assert response contains `serverInfo` and the expected capabilities:
     hover, completion, definition, documentSymbol, formatting,
     textDocumentSync=Full.

2. **didOpen invalid Turtle produces diagnostics.**
   - Send `textDocument/didOpen` with a `.ttl` URI and text
     `"<bad turtle"` (invalid Turtle).
   - Assert a `textDocument/publishDiagnostics` notification is received
     with at least one diagnostic.

3. **didOpen valid N-Triples produces empty diagnostics.**
   - Send `textDocument/didOpen` with a `.nt` URI and text
     `"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n"`.
   - Assert a `textDocument/publishDiagnostics` notification is received
     with zero diagnostics.

4. **didChange updates diagnostics.**
   - Open a valid `.ttl` document.
   - Send `textDocument/didChange` with invalid text.
   - Assert diagnostics notification contains errors.

5. **shutdown is handled cleanly.**
   - Send `shutdown` request; assert response is `null`.
   - Send `exit` notification; assert server thread terminates.

### Helper utilities

Create a `TestClient` struct in the test file that wraps the channel
pair and provides:
- `send_request(method, params) -> serde_json::Value`
- `send_notification(method, params)`
- `recv_response() -> serde_json::Value`
- `recv_notification() -> serde_json::Value`

Use `serde_json::json!` for message construction.

## Acceptance

- `cargo test -p rdf-lsp` green (all 5 tests pass).
- Tests are in `crates/rdf-lsp/tests/lsp_integration.rs`.
- No test uses `std::thread::sleep` for synchronisation; use channel
  blocking instead.

## Claims

Claim `crates/rdf-lsp/tests/**` before editing. Release on completion.
Do NOT touch `src/**` — those are owned by `pf-lsp-protocol` and
`pf-lsp-features`.

## Memory

- `memory_store` at `testing/integration-tests` in `phase-f` namespace:
  list of tests, pass/fail status.
- `memory_store` exit report at `phase-f` blackboard:
  `pf-tester:done` with test count and pass rate.
